import { expect, test } from "@playwright/test";
import type { Page, Route } from "@playwright/test";

type Status = "draft" | "marking" | "reviewed" | "checked" | "decided";
type Origin = "human" | "oracle";
type Action = "entry" | "exit";
type Quality = "good" | "bad" | "neutral";
type Side = "long" | "short";
type DecisionKind = "test" | "refine" | "stop";

const DIGEST = "sha256:" + "a".repeat(64);
const DATA = "sha256:" + "b".repeat(64);
const RUN_ID = "sim_682";
const CREATED_ID = "sim_created";
const STAMP = "2026-11-14T10:00:00Z";

const WINDOWS = [
  { index: 0, start: 0, end: 8 },
  { index: 1, start: 100, end: 108 },
];

interface WindowRow {
  index: number;
  start: number;
  end: number;
}

interface MarkRow {
  mark_id: string;
  chart: number;
  bar: number;
  marked_time_utc: string;
  action: Action;
  quality: Quality;
  side: Side;
  origin: Origin;
  note: string;
}

interface DecisionRow {
  decision: DecisionKind;
  note: string;
  decided_at_utc: string;
}

interface RunRow {
  sim_id: string;
  premise: string;
  bundle: string;
  inputs: string;
  seed: number;
  criteria: string;
  status: Status;
  windows: WindowRow[];
  marks: MarkRow[];
  decision: DecisionRow | null;
}

interface CreateBody {
  premise: string;
  bundle: string;
  inputs: string;
  seed: number;
  criteria: string;
}

interface Server {
  run: RunRow | null;
  online: boolean;
  lockMarks: boolean;
  created: CreateBody | null;
  markPosts: number;
  oracleCalls: number;
  checkCalls: number;
  decisionCalls: number;
}

const CHECK = {
  events: 42,
  mean_net: 0.12,
  hit_rate: 0.57,
  control: 80,
  control_mean_net: 0.05,
  control_hit_rate: 0.51,
  positive_half_years: 4,
  passed: true,
  failures: [] as string[],
};

function seededRun(status: Status): RunRow {
  return {
    sim_id: RUN_ID,
    premise: "Higher lows into a confirmed level",
    bundle: DIGEST,
    inputs: "EMA stack and RSI zone",
    seed: 7,
    criteria: "min_events=30; mean net > 0; hit rate > 0.5; 3 of 5 half-years",
    status,
    windows: WINDOWS.map((window) => ({ ...window })),
    marks: [],
    decision: null,
  };
}

function summary(run: RunRow): Record<string, unknown> {
  return {
    sim_id: run.sim_id,
    premise: run.premise,
    status: run.status,
    windows: run.windows.length,
    marks: run.marks.filter((mark) => mark.origin === "human").length,
    bundle: run.bundle,
    marked_windows: new Set(run.marks.map((mark) => mark.chart)).size,
  };
}

function detail(run: RunRow): Record<string, unknown> {
  return {
    sim_id: run.sim_id,
    premise: run.premise,
    status: run.status,
    bundle: run.bundle,
    inputs: run.inputs,
    inputs_digest: DATA,
    seed: run.seed,
    criteria: run.criteria,
    counts: {
      windows: run.windows.length,
      marked_windows: new Set(run.marks.map((mark) => mark.chart)).size,
      human_marks: run.marks.filter((mark) => mark.origin === "human").length,
      oracle_marks: run.marks.filter((mark) => mark.origin === "oracle").length,
    },
    decision: run.decision,
    created_at: STAMP,
  };
}

function series(run: RunRow, chart: number): Record<string, unknown> {
  const base = 100 + chart * 10;
  const candles = Array.from({ length: 8 }, (_, bar) => ({
    bar,
    open: base + bar,
    high: base + bar + 2,
    low: base + bar - 1,
    close: base + bar + 1,
    volume: (bar + 1) * 10,
  }));
  const closes = candles.map((candle) => candle.close);
  return {
    sim_id: run.sim_id,
    chart,
    candles,
    ema_fast: closes,
    ema_mid: closes,
    ema_slow: closes,
    panel: { indicator_code: 2, values: [48, 52, 45, 60, 63, 58, 66, 70] },
    events: [],
    levels: [],
  };
}

function oracleMarks(run: RunRow): MarkRow[] {
  return [
    {
      mark_id: `${run.sim_id}-oracle-1`,
      chart: 0,
      bar: 2,
      marked_time_utc: STAMP,
      action: "entry",
      quality: "good",
      side: "long",
      origin: "oracle",
      note: "oracle: higher low into the level",
    },
    {
      mark_id: `${run.sim_id}-oracle-2`,
      chart: 0,
      bar: 5,
      marked_time_utc: STAMP,
      action: "exit",
      quality: "bad",
      side: "long",
      origin: "oracle",
      note: "oracle: exit before the level",
    },
  ];
}

async function shell(route: Route): Promise<boolean> {
  const path = new URL(route.request().url()).pathname.replace("/api/v1", "");
  if (path === "/version") {
    await route.fulfill({
      json: { product: "1.0.0", api: "quantrule.api/v1", image_digest: DIGEST, contracts: [] },
    });
    return true;
  }
  if (path === "/health") {
    await route.fulfill({
      json: {
        v: "quantrule.api/v1",
        state: "healthy",
        services: [],
        queue_depth: 0,
        workers: 1,
        image_digest: DIGEST,
      },
    });
    return true;
  }
  if (path === "/profiles") {
    await route.fulfill({
      json: [
        { profile_id: "default", name: "Default", datasets: 1, runs: 0, simulations: 1 },
      ],
    });
    return true;
  }
  return false;
}

async function installServer(
  page: Page,
  initial: RunRow | null,
): Promise<Server> {
  const server: Server = {
    run: initial,
    online: true,
    lockMarks: false,
    created: null,
    markPosts: 0,
    oracleCalls: 0,
    checkCalls: 0,
    decisionCalls: 0,
  };

  await page.route("**/api/v1/**", async (route: Route) => {
    if (!server.online) {
      await route.fulfill({
        status: 503,
        json: { error: { code: "internal", message: "gateway is offline" } },
      });
      return;
    }
    if (await shell(route)) {
      return;
    }

    const request = route.request();
    const path = new URL(request.url()).pathname.replace("/api/v1", "");
    const run = server.run;

    if (path === "/simulation/runs" && request.method() === "GET") {
      await route.fulfill({ json: run === null ? [] : [summary(run)] });
      return;
    }
    if (path === "/simulation/runs" && request.method() === "POST") {
      const body = request.postDataJSON() as CreateBody;
      server.created = body;
      server.run = {
        sim_id: CREATED_ID,
        ...body,
        status: "draft",
        windows: WINDOWS.map((window) => ({ ...window })),
        marks: [],
        decision: null,
      };
      await route.fulfill({ status: 201, json: { sim_id: CREATED_ID } });
      return;
    }

    if (run === null) {
      await route.fulfill({
        status: 404,
        json: { error: { code: "simulation.not_found", message: "unknown simulation" } },
      });
      return;
    }

    const base = `/simulation/runs/${encodeURIComponent(run.sim_id)}`;
    if (path === base && request.method() === "GET") {
      await route.fulfill({ json: detail(run) });
      return;
    }
    if (path === `${base}/windows` && request.method() === "POST") {
      await route.fulfill({ json: { windows: run.windows } });
      return;
    }
    if (path.startsWith(`${base}/windows/`) && path.endsWith("/series")) {
      const chart = Number(path.split("/").at(-2));
      await route.fulfill({ json: series(run, chart) });
      return;
    }
    if (path === `${base}/marks` && request.method() === "GET") {
      await route.fulfill({ json: { marks: run.marks } });
      return;
    }
    if (path === `${base}/marks` && request.method() === "POST") {
      server.markPosts += 1;
      if (run.status === "decided" || server.lockMarks) {
        await route.fulfill({
          status: 409,
          json: {
            error: { code: "simulation.locked", message: "marking is closed" },
          },
        });
        return;
      }
      const body = request.postDataJSON() as Omit<MarkRow, "mark_id" | "origin" | "marked_time_utc">;
      const mark: MarkRow = {
        ...body,
        mark_id: `m${server.markPosts}`,
        origin: "human",
        marked_time_utc: STAMP,
      };
      run.marks.push(mark);
      if (run.status === "draft") {
        run.status = "marking";
      }
      await route.fulfill({ status: 201, json: { mark_id: mark.mark_id } });
      return;
    }
    if (path.startsWith(`${base}/marks/`) && request.method() === "DELETE") {
      const markId = decodeURIComponent(path.split("/").at(-1) ?? "");
      run.marks = run.marks.filter((mark) => mark.mark_id !== markId);
      await route.fulfill({ status: 204 });
      return;
    }
    if (path === `${base}/oracle` && request.method() === "POST") {
      server.oracleCalls += 1;
      run.marks = [
        ...run.marks.filter((mark) => mark.origin === "human"),
        ...oracleMarks(run),
      ];
      if (run.status === "draft" || run.status === "marking") {
        run.status = "reviewed";
      }
      await route.fulfill({ json: { marks: run.marks } });
      return;
    }
    if (path === `${base}/check` && request.method() === "POST") {
      server.checkCalls += 1;
      if (run.status === "reviewed") {
        run.status = "checked";
      }
      await route.fulfill({ json: CHECK });
      return;
    }
    if (path === `${base}/decision` && request.method() === "POST") {
      server.decisionCalls += 1;
      const body = request.postDataJSON() as { decision: DecisionKind; note: string };
      run.decision = {
        decision: body.decision,
        note: body.note,
        decided_at_utc: STAMP,
      };
      run.status = "decided";
      await route.fulfill({ status: 201, json: run.decision });
      return;
    }

    await route.fulfill({
      status: 404,
      json: { error: { code: "test.unhandled", message: `${path} is not fixtured` } },
    });
  });

  return server;
}

async function openSimulation(page: Page): Promise<void> {
  await page.goto("/");
  await page.getByRole("button", { name: /Simulation/ }).first().click();
  await expect(page.getByTestId("simulation-runs")).toBeVisible();
}

async function openRun(page: Page, run: RunRow): Promise<void> {
  await openSimulation(page);
  await page.getByRole("button", { name: `Open ${run.sim_id}` }).click();
  await expect(page.getByTestId("simulation-run")).toBeVisible();
}

async function markBarOne(page: Page, note: string): Promise<void> {
  await expect(page.getByTestId("simulation-chart")).toBeVisible();
  await page.getByTestId("chart-application").focus();
  await page.keyboard.press("End");
  await page.keyboard.press("ArrowLeft");
  await page.getByTestId("mark-note").fill(note);
  await page.getByTestId("mark-class-1").click();
}

test("creates a run and walks premise, mark, oracle, review, check, decision", async ({ page }) => {
  const server = await installServer(page, null);
  await openSimulation(page);
  await expect(page.getByTestId("simulation-empty")).toHaveText("No simulation runs.");

  const premise = "Higher lows into a confirmed level";
  await page.getByTestId("sim-create-premise").fill(premise);
  await page.getByTestId("sim-create-bundle").fill(DIGEST);
  await page.getByTestId("sim-create-inputs").fill("EMA stack and RSI zone");
  await page.getByTestId("sim-create-seed").fill("7");
  await page
    .getByTestId("sim-create-criteria")
    .fill("min_events=30; mean net > 0; 3 of 5 half-years");
  await page.getByTestId("sim-create-submit").click();

  await expect(page.getByTestId("sim-status")).toHaveText("draft");
  await expect(page.getByTestId("sim-run-meta")).toContainText(premise);
  await expect(page.getByTestId("simulation-chart")).toBeVisible();
  const download = page.waitForEvent("download");
  await page.getByTestId("chart-export").click();
  expect((await download).suggestedFilename()).toContain("window-");
  expect(server.created).toEqual({
    premise,
    bundle: DIGEST,
    inputs: "EMA stack and RSI zone",
    seed: 7,
    criteria: "min_events=30; mean net > 0; 3 of 5 half-years",
  });

  await expect(page.getByTestId("sim-marks-empty")).toBeVisible();
  await expect(page.getByTestId("sim-check-empty")).toBeVisible();
  await expect(page.getByTestId("sim-review-empty")).toBeVisible();

  await markBarOne(page, "higher low into the level");
  await expect(page.getByTestId("sim-marks").locator("tbody tr")).toHaveCount(1);
  await expect(page.getByTestId("sim-marks")).toContainText("human");
  await expect(page.getByTestId("sim-review-human-total")).toHaveText("1");
  await expect(page.getByTestId("sim-review-oracle-total")).toHaveText("0");
  await expect(page.getByTestId("sim-review-waiting")).toBeVisible();

  await page.getByTestId("sim-oracle").click();
  await expect(page.getByTestId("sim-status")).toHaveText("reviewed");
  await expect(page.getByTestId("sim-marks").locator("tbody tr")).toHaveCount(3);
  await expect(page.getByTestId("sim-marks").locator('[data-origin="oracle"]')).toHaveCount(2);
  await expect(page.getByTestId("sim-review-oracle-total")).toHaveText("2");
  expect(server.oracleCalls).toBe(1);

  await page.getByTestId("sim-check-run").click();
  await expect(page.getByTestId("sim-status")).toHaveText("checked");
  await expect(page.getByTestId("sim-check-verdict")).toHaveText("passed");
  await expect(page.getByTestId("sim-check-table")).toContainText("42");
  expect(server.checkCalls).toBe(1);

  await page.getByTestId("sim-decision-test").check();
  await page
    .getByTestId("sim-decision-note")
    .fill("The boundary holds; send it to Strategy Research.");
  await page.getByTestId("sim-decision-submit").click();

  await expect(page.getByTestId("sim-status")).toHaveText("decided");
  await expect(page.getByTestId("sim-decision-record")).toContainText("test");
  await expect(page.getByTestId("sim-decision-record")).toContainText(
    "The boundary holds",
  );
  await expect(page.getByTestId("sim-lock")).toBeVisible();
  await expect(page.getByTestId("mark-class-1")).toBeDisabled();
  await expect(page.getByTestId("mark-note")).toBeDisabled();
  expect(server.decisionCalls).toBe(1);

  const posts = server.markPosts;
  await page.getByTestId("chart-application").focus();
  await page.keyboard.press("1");
  await expect(page.getByTestId("sim-marks").locator("tbody tr")).toHaveCount(3);
  expect(server.markPosts).toBe(posts);
});

test("a decided run opens locked and refuses every mark", async ({ page }) => {
  const run = seededRun("decided");
  run.decision = {
    decision: "stop",
    note: "The pattern does not survive the check.",
    decided_at_utc: STAMP,
  };
  const server = await installServer(page, run);
  await openRun(page, run);

  await expect(page.getByTestId("sim-status")).toHaveText("decided");
  await expect(page.getByTestId("sim-lock")).toBeVisible();
  await expect(page.getByTestId("sim-decision-record")).toContainText("stop");
  await expect(page.getByTestId("sim-decision-submit")).toHaveCount(0);
  await expect(page.getByTestId("mark-class-1")).toBeDisabled();
  await expect(page.getByTestId("mark-undo")).toBeDisabled();

  await page.getByTestId("chart-application").focus();
  await page.keyboard.press("1");
  await page.getByTestId("mark-class-1").click({ force: true });
  await expect(page.getByTestId("sim-marks-empty")).toBeVisible();
  expect(server.markPosts).toBe(0);
});

test("a server-locked run refuses a mark and keeps the note", async ({ page }) => {
  const run = seededRun("marking");
  const server = await installServer(page, run);
  server.lockMarks = true;
  await openRun(page, run);

  await expect(page.getByTestId("simulation-chart")).toBeVisible();
  await page.getByTestId("chart-application").focus();
  await page.keyboard.press("End");
  await page.keyboard.press("ArrowLeft");
  await page.getByTestId("mark-note").fill("this note must survive the refusal");
  await page.getByTestId("mark-class-1").click();

  await expect(page.getByTestId("mark-error")).toContainText("simulation.locked");
  await expect(page.getByTestId("mark-note")).toHaveValue(
    "this note must survive the refusal",
  );
  await expect(page.getByTestId("sim-marks-empty")).toBeVisible();
  expect(server.markPosts).toBe(1);
});

test("shows empty states for no windows, marks, and check", async ({ page }) => {
  const run = seededRun("draft");
  run.windows = [];
  await installServer(page, run);
  await openRun(page, run);

  await expect(page.getByTestId("sim-no-windows")).toBeVisible();
  await expect(page.getByTestId("sim-marks-empty")).toBeVisible();
  await expect(page.getByTestId("sim-check-empty")).toBeVisible();
  await expect(page.getByTestId("sim-review-empty")).toBeVisible();
  await expect(page.getByTestId("simulation-chart")).toHaveCount(0);
});

test("a disconnected gateway clears the simulation rows", async ({ page }) => {
  const run = seededRun("marking");
  const server = await installServer(page, run);
  await openSimulation(page);

  await expect(page.getByTestId("simulation-table")).toContainText(run.premise);
  server.online = false;
  await page.getByTestId("simulation-refresh").click();

  await expect(page.getByTestId("simulation-disconnected")).toBeVisible();
  await expect(page.getByTestId("simulation-table")).toHaveCount(0);
  await expect(page.getByText(run.premise)).toHaveCount(0);
  await expect(page.getByTestId("simulation-create")).toHaveCount(0);
});
