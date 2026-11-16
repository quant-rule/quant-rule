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
    if (path === "/catalog/datasets" || path === "/catalog/configs") {
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
