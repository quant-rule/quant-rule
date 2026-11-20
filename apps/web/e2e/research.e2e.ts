import { expect, test } from "@playwright/test";
import type { Page, Route } from "@playwright/test";

const DIGEST = "sha256:" + "a".repeat(64);
const DATA = "sha256:" + "c".repeat(64);
const RUN = "sha256:" + "b".repeat(64);
const ID = "MODEL-2026-11-17-01";

const MANIFEST = {
  schema: "quantrule.improvement-run/v3",
  improvement_id: ID,
  tier: "model",
  hypothesis: "the fixture hypothesis",
  created_utc: "2026-11-17T00:00:00Z",
  changed: "model",
  frozen: ["strategy", "portfolio"],
  partition: "model_dev",
  data_version: DATA,
  config_id: "cfg_abc",
  criteria_id: null,
  green_criteria: { return_target: 0.1 },
  holdout_access: "none",
  baseline: null,
  code_commit: "c236539f",
  seed: 0,
  command: "run submit",
  analytics: true,
};

function detail(state: "created" | "submitted" | "decided", decision: string | null = null) {
  return {
    improvement_id: ID,
    tier: "model",
    state,
    decision,
    decision_note: decision === null ? null : "the fixture decision",
    manifest: MANIFEST,
    pipeline_run_id: state === "created" ? null : RUN,
    canonical_digest: state === "decided" ? DIGEST : null,
    created_at: "2026-11-17T00:00:00Z",
    submitted_at: state === "created" ? null : "2026-11-17T00:05:00Z",
    decided_at: state === "decided" ? "2026-11-17T00:10:00Z" : null,
  };
}

const BASELINE = {
  baseline_id: "MODEL-2026-11-04-04..05",
  tier: "model",
  run_id: "MODEL-2026-11-04-04..05",
  improvement_id: null,
  accepted_by: "Human",
  accepted_on: "2026-11-04",
  scope: "1m, 4 ATR over 20 bars",
  supersedes: null,
  created_at: "2026-11-04T00:00:00Z",
};

const INVALIDATION = {
  invalidation_id: "INVALIDATION-2026-11-10-01",
  date: "2026-11-10",
  commit: "worktree",
  removed: "MODEL-2026-11-10-01",
  reason: "the fixture tombstone",
  superseded_by: null,
  created_at: "2026-11-10T00:00:00Z",
};

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
      json: [{ profile_id: "default", name: "Default", datasets: 1, runs: 0, simulations: 0 }],
    });
    return true;
  }
  return false;
}

async function mockResearch(page: Page): Promise<void> {
  let state: "created" | "submitted" | "decided" = "created";
  await page.route("**/api/v1/**", async (route) => {
    if (await shell(route)) {
      return;
    }
    const request = route.request();
    const path = decodeURIComponent(new URL(request.url()).pathname).replace("/api/v1", "");
    if (path === "/research/runs" && request.method() === "GET") {
      return route.fulfill({
        json: [
          {
            improvement_id: ID,
            tier: "model",
            state,
            decision: state === "decided" ? "accepted" : null,
            created_at: "2026-11-17T00:00:00Z",
            submitted_at: state === "created" ? null : "2026-11-17T00:05:00Z",
            decided_at: state === "decided" ? "2026-11-17T00:10:00Z" : null,
          },
        ],
      });
    }
    if (path === "/research/runs" && request.method() === "POST") {
      return route.fulfill({ status: 201, json: detail("created") });
    }
    if (path === `/research/runs/${ID}` && request.method() === "GET") {
      return route.fulfill({ json: detail(state, state === "decided" ? "accepted" : null) });
    }
    if (path === `/research/runs/${ID}/submit` && request.method() === "POST") {
      state = "submitted";
      return route.fulfill({ status: 202, json: detail("submitted") });
    }
    if (path === `/research/runs/${ID}/compare` && request.method() === "GET") {
      return route.fulfill({
        json: {
          improvement_id: ID,
          tier: "model",
          candidate_run_id: RUN,
          baseline_id: BASELINE.baseline_id,
          candidate_digest: DIGEST,
          baseline_digest: null,
          rows: [{ metric: "1m.coverage_share", candidate: 1, baseline: null, delta: null }],
          note: "the baseline has no pipeline run; baseline metrics are unavailable",
        },
      });
    }
    if (path === `/research/runs/${ID}/decision` && request.method() === "POST") {
      state = "decided";
      return route.fulfill({ json: detail("decided", "accepted") });
    }
    if (path === "/research/baselines" && request.method() === "GET") {
      return route.fulfill({ json: [BASELINE] });
    }
    if (path === "/research/baselines/model" && request.method() === "GET") {
      return route.fulfill({ json: BASELINE });
    }
    if (path === "/research/invalidations" && request.method() === "GET") {
      return route.fulfill({ json: [INVALIDATION] });
    }
    return route.fulfill({ status: 404, json: {} });
  });
}

test("pre-registers, submits, compares, and decides explicitly", async ({ page }) => {
  await mockResearch(page);
  await page.goto("/");
  await page.getByRole("button", { name: /^Research/ }).click();

  await expect(page.getByText(/An improvement run is a coupled pipeline run/)).toBeVisible();
  await expect(page.getByText(/An accepted decision needs a succeeded pipeline run/)).toBeVisible();
  await expect(page.getByText(/Seeded historical baselines have no pipeline run/)).toBeVisible();

  await page.getByLabel("Hypothesis").fill("the fixture hypothesis");
  await page.getByLabel("Data version").fill(DATA);
  await page.getByLabel("Config id").fill("cfg_abc");
  await page.getByLabel("Code commit").fill("c236539f");
  await page.getByRole("button", { name: "Create pre-registration" }).click();

  await expect(page.getByTestId("research-table")).toContainText(ID);
  await expect(page.getByTestId("research-detail")).toContainText("created");
  await page.getByRole("button", { name: "Submit pipeline run" }).click();
  await expect(page.getByTestId("research-detail")).toContainText("submitted");

  await page.getByRole("button", { name: "Compare with baseline" }).click();
  await expect(page.getByTestId("research-compare")).toContainText("1m.coverage_share");

  await page.getByTestId("research-decide").getByLabel("Decision").selectOption("accepted");
  await page.getByTestId("research-decide").getByLabel("Note").fill("the fixture passed");
  await page.getByRole("button", { name: "Record decision" }).click();
  await expect(page.getByTestId("research-detail")).toContainText("decided");
  await expect(page.getByTestId("research-detail")).toContainText("accepted");
});

test("reads the seeded baseline limitation and the invalidation tombstone", async ({ page }) => {
  await mockResearch(page);
  await page.goto("/");
  await page.getByRole("button", { name: /^Research/ }).click();

  await expect(page.getByTestId("baseline-table")).toContainText("MODEL-2026-11-04-04..05");
  await page.getByRole("button", { name: "Show model" }).first().click();
  await expect(page.getByTestId("baseline-detail")).toContainText("1m, 4 ATR");
  await expect(page.getByTestId("invalidation-list")).toContainText("INVALIDATION-2026-11-10-01");
});
