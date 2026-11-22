import { expect, test } from "@playwright/test";
import type { Page, Route } from "@playwright/test";

/**
 * Profile scope and disconnection.
 *
 * A profile switch must drop every profile-scoped row before the next read
 * completes, and a disconnected gateway must clear the open run detail rather
 * than leave it beside an error. Both are asserted at the screen boundary.
 */

const DIGEST = "sha256:" + "a".repeat(64);
const DATA = "sha256:" + "b".repeat(64);
const DEFAULT_RUN = "sha256:" + "c".repeat(64);
const ALPHA_RUN = "sha256:" + "d".repeat(64);
const DEFAULT_STAGE = "sha256:" + "1".repeat(64);
const ALPHA_STAGE = "sha256:" + "2".repeat(64);
const IMAGE = "sha256:" + "e".repeat(64);

interface Profile {
  profile_id: string;
  name: string;
  datasets: number;
  runs: number;
  simulations: number;
}

interface Server {
  online: boolean;
  /** Held open until the test releases the alpha simulation response. */
  gate: Promise<void> | null;
}

const PROFILES: Profile[] = [
  { profile_id: "default", name: "Default", datasets: 0, runs: 1, simulations: 1 },
  { profile_id: "alpha", name: "Alpha", datasets: 0, runs: 1, simulations: 1 },
];

function run(runId: string, configId: string) {
  return {
    run_id: runId,
    status: "succeeded",
    config_id: configId,
    data_version: DATA,
    submitted_at: "2026-11-13T12:00:00Z",
    elapsed_ms: 1_200,
  };
}

function detail(runId: string, configId: string) {
  return {
    ...run(runId, configId),
    owner: configId === "cfg_default" ? "default" : "alpha",
    stages: [{ name: "data", state: "done", elapsed_ms: 1_200 }],
    provenance: {
      data_version: DATA,
      config_hash: DATA,
      model_version: "M1.0",
      image_digest: IMAGE,
    },
    artifacts: [],
    digest: DIGEST,
    error: null,
  };
}

function researchSummary(profile: string) {
  return {
    improvement_id: profile === "alpha" ? "MODEL-ALPHA-01" : "MODEL-DEFAULT-01",
    tier: "model",
    state: "created",
    decision: null,
    created_at: "2026-11-17T00:00:00Z",
    submitted_at: null,
    decided_at: null,
  };
}

function researchBaseline(profile: string) {
  return {
    baseline_id: profile === "alpha" ? "BASE-ALPHA" : "BASE-DEFAULT",
    tier: "model",
    run_id: profile === "alpha" ? "MODEL-ALPHA-01" : "MODEL-DEFAULT-01",
    improvement_id: null,
    accepted_by: "Human",
    accepted_on: "2026-11-04",
    scope: "the profile fixture",
    supersedes: null,
    created_at: "2026-11-04T00:00:00Z",
  };
}

function researchInvalidation(profile: string) {
  return {
    invalidation_id: profile === "alpha" ? "INV-ALPHA" : "INV-DEFAULT",
    date: "2026-11-10",
    commit: "worktree",
    removed: profile === "alpha" ? "MODEL-ALPHA-01" : "MODEL-DEFAULT-01",
    reason: "the profile fixture",
    superseded_by: null,
    created_at: "2026-11-10T00:00:00Z",
  };
}

function stage(stageRunId: string) {
  return {
    stage_run_id: stageRunId,
    kind: "model",
    state: "succeeded",
    config_id: "cfg_default",
    submitted_at: "2026-11-17T00:00:00Z",
    artifact_id: DIGEST,
    evidence: false,
  };
}

function config(profile: string) {
  return {
    config_id: profile === "alpha" ? "cfg_alpha" : "cfg_default",
    name: profile === "alpha" ? "alpha/run.json" : "default/run.json",
    origin: "repository",
    valid: true,
    message: null,
    config_hash: DIGEST,
  };
}

function summary(simId: string, premise: string) {
  return {
    sim_id: simId,
    premise,
    status: "draft",
    windows: 2,
    marks: 0,
    bundle: DIGEST,
    marked_windows: 0,
  };
}

async function install(page: Page, server: Server): Promise<void> {
  await page.route("**/api/v1/**", async (route: Route) => {
    if (!server.online) {
      await route.abort();
      return;
    }
    const request = route.request();
    const path = new URL(request.url()).pathname.replace("/api/v1", "");
    const profile = request.headers()["x-quantrule-profile"] ?? "default";

    if (path === "/version") {
      return route.fulfill({
        json: { product: "1.0.0", api: "quantrule.api/v1", image_digest: IMAGE, contracts: [] },
      });
    }
    if (path === "/health") {
      return route.fulfill({
        json: {
          v: "quantrule.api/v1",
          state: "healthy",
          services: [],
          queue_depth: 0,
          workers: 1,
          image_digest: IMAGE,
        },
      });
    }
    if (path === "/profiles") {
      return route.fulfill({ json: PROFILES });
    }
    if (path === "/catalog/datasets") {
      return route.fulfill({ json: [] });
    }
    if (path === "/catalog/configs") {
      return route.fulfill({ json: [config(profile)] });
    }
    if (path === "/catalog/packs") {
      return route.fulfill({ json: [] });
    }
    if (path === "/signals") {
      return route.fulfill({ json: [] });
    }
    if (path === "/runs") {
      return route.fulfill({
        json: [
          profile === "alpha"
            ? run(ALPHA_RUN, "cfg_alpha")
            : run(DEFAULT_RUN, "cfg_default"),
        ],
      });
    }
    if (path === `/runs/${DEFAULT_RUN}`) {
      return route.fulfill({ json: detail(DEFAULT_RUN, "cfg_default") });
    }
    if (path === `/runs/${ALPHA_RUN}`) {
      return route.fulfill({ json: detail(ALPHA_RUN, "cfg_alpha") });
    }
    if (path === "/stages/runs") {
      return route.fulfill({
        json: [profile === "alpha" ? stage(ALPHA_STAGE) : stage(DEFAULT_STAGE)],
      });
    }
    if (path === "/research/runs") {
      return route.fulfill({ json: [researchSummary(profile)] });
    }
    if (path === "/research/baselines") {
      return route.fulfill({ json: [researchBaseline(profile)] });
    }
    if (path === "/research/invalidations") {
      return route.fulfill({ json: [researchInvalidation(profile)] });
    }
    if (path === "/simulation/runs") {
      if (profile === "alpha" && server.gate !== null) {
        await server.gate;
      }
      return route.fulfill({
        json: [
          profile === "alpha"
            ? summary("sim_alpha", "alpha premise")
            : summary("sim_default", "default premise"),
        ],
      });
    }
    return route.fulfill({
      status: 404,
      json: { error: { code: "test.unhandled", message: `unmocked ${path}` } },
    });
  });
}

test("a profile switch clears runs and simulations before the new read", async ({ page }) => {
  let openGate!: () => void;
  const gate = new Promise<void>((resolve) => {
    openGate = resolve;
  });
  const server: Server = { online: true, gate };
  await install(page, server);
  await page.goto("/");

  await page.getByRole("button", { name: /Simulation/ }).click();
  await expect(page.getByText("default premise")).toBeVisible();

  await page.getByLabel("Active profile").selectOption("alpha");
  // The alpha response is held open, so a row still on screen would be stale.
  await expect(page.getByText("default premise")).toHaveCount(0);
  await expect(page.getByText("Loading simulation runs…")).toBeVisible();

  openGate();
  await expect(page.getByText("alpha premise")).toBeVisible();

  await page.getByRole("button", { name: /Runs/ }).click();
  await expect(page.getByRole("row", { name: /dddddddd/ })).toBeVisible();
  await expect(page.getByText(/cccccccc/)).toHaveCount(0);
});

test("a profile switch clears the open config detail", async ({ page }) => {
  const server: Server = { online: true, gate: null };
  await install(page, server);
  await page.goto("/");

  await page.getByRole("button", { name: /Configs/ }).click();
  await expect(page.getByTestId("config-table")).toContainText("default/run.json");
  await page.getByTestId("config-detail-open").first().click();
  await expect(page.getByTestId("config-detail")).toContainText("cfg_default");

  await page.getByLabel("Active profile").selectOption("alpha");
  await expect(page.getByTestId("config-detail")).toHaveCount(0);
  await expect(page.getByTestId("config-table")).toContainText("alpha/run.json");
});

test("a profile switch clears and reloads stage rows", async ({ page }) => {
  const server: Server = { online: true, gate: null };
  await install(page, server);
  await page.goto("/");

  await page.getByRole("button", { name: /Workbench/ }).click();
  await page.getByRole("tab", { name: "Model" }).click();
  await expect(page.getByTestId("stage-table")).toContainText("111111");

  await page.getByLabel("Active profile").selectOption("alpha");
  await expect(page.getByTestId("stage-table")).toContainText("222222");
  await expect(page.getByTestId("stage-table")).not.toContainText("111111");
});

test("a profile switch clears and reloads research rows and registries", async ({ page }) => {
  const server: Server = { online: true, gate: null };
  await install(page, server);
  await page.goto("/");

  await page.getByRole("button", { name: /^Research/ }).click();
  await expect(page.getByTestId("research-table")).toContainText("MODEL-DEFAULT-01");
  await expect(page.getByTestId("baseline-table")).toContainText("BASE-DEFAULT");
  await expect(page.getByTestId("invalidation-list")).toContainText("INV-DEFAULT");

  await page.getByLabel("Active profile").selectOption("alpha");
  await expect(page.getByTestId("research-table")).toContainText("MODEL-ALPHA-01");
  await expect(page.getByTestId("research-table")).not.toContainText("MODEL-DEFAULT-01");
  await expect(page.getByTestId("baseline-table")).toContainText("BASE-ALPHA");
  await expect(page.getByTestId("baseline-table")).not.toContainText("BASE-DEFAULT");
  await expect(page.getByTestId("invalidation-list")).toContainText("INV-ALPHA");
  await expect(page.getByTestId("invalidation-list")).not.toContainText("INV-DEFAULT");
});

test("disconnection clears the open run detail", async ({ page }) => {
  const server: Server = { online: true, gate: null };
  await install(page, server);
  await page.goto("/");

  await page.getByRole("button", { name: /Runs/ }).click();
  await page.getByRole("button", { name: `Open ${DEFAULT_RUN}` }).click();
  await expect(page.getByRole("heading", { name: "Run detail" })).toBeVisible();

  server.online = false;
  await page.getByRole("button", { name: /Connection/ }).click();
  await page.getByRole("button", { name: "Refresh" }).click();

  await page.getByRole("button", { name: /Runs/ }).click();
  await expect(page.getByText("Disconnected. The run list is not shown.")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Run detail" })).toHaveCount(0);
  await expect(page.getByRole("row", { name: /cccccccc/ })).toHaveCount(0);
});
