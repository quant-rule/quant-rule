import { expect, test } from "@playwright/test";
import type { Page, Route } from "@playwright/test";

type RunState = "queued" | "running" | "succeeded" | "failed" | "cancelled";

const DIGEST = "sha256:" + "a".repeat(64);
const RUN_ID = "sha256:" + "b".repeat(64);
const DATA = "sha256:" + "c".repeat(64);

function run(status: RunState) {
  return {
    run_id: RUN_ID,
    status,
    config_id: "cfg_abc",
    data_version: DATA,
    submitted_at: "2026-11-13T12:00:00Z",
    elapsed_ms: null,
  };
}

async function shell(route: Route): Promise<boolean> {
  const path = new URL(route.request().url()).pathname.replace("/api/v1", "");
  if (path === "/version") {
    await route.fulfill({
      json: {
        product: "1.0.0",
        api: "quantrule.api/v1",
        image_digest: DIGEST,
        contracts: [],
      },
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
      json: [{ profile_id: "default", name: "Default", datasets: 0, runs: 0, simulations: 0 }],
    });
    return true;
  }
  return false;
}

async function openRuns(page: Page): Promise<void> {
  await page.goto("/");
  await page.getByRole("button", { name: /Runs/ }).click();
}

test("polls a queued run through running to succeeded", async ({ page }) => {
  await page.clock.install();
  const sequence: RunState[] = ["queued", "running", "succeeded"];
  let reads = 0;

  await page.route("**/api/v1/**", async (route) => {
    if (await shell(route)) {
      return;
    }
    const path = new URL(route.request().url()).pathname.replace("/api/v1", "");
    if (path === "/runs") {
      const status = sequence[Math.min(reads, sequence.length - 1)];
      reads += 1;
      return route.fulfill({ json: [run(status)] });
    }
    return route.fulfill({ status: 500, json: {} });
  });

  await openRuns(page);
  await expect(page.getByText("queued", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: /^Cancel/ })).toBeVisible();

  await page.clock.fastForward(2_100);
  await expect(page.getByText("running", { exact: true })).toBeVisible();

  await page.clock.fastForward(2_100);
  await expect(page.getByText("succeeded", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: /^Cancel/ })).toHaveCount(0);
});

test("cancels an active run and shows the server's new state", async ({ page }) => {
  let status: RunState = "queued";

  await page.route("**/api/v1/**", async (route) => {
    if (await shell(route)) {
      return;
    }
    const request = route.request();
    const path = new URL(request.url()).pathname.replace("/api/v1", "");
    if (path === `/runs/${encodeURIComponent(RUN_ID)}/cancel` && request.method() === "POST") {
      status = "cancelled";
      return route.fulfill({ json: { run_id: RUN_ID, status, owner: "default" } });
    }
    if (path === "/runs") {
      return route.fulfill({ json: [run(status)] });
    }
    return route.fulfill({ status: 500, json: {} });
  });

  await openRuns(page);
  await page.getByRole("button", { name: /^Cancel/ }).click();
  await expect(page.getByText("cancelled", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: /^Cancel/ })).toHaveCount(0);
});

test("an error clears the rows instead of showing stale runs", async ({ page }) => {
  await page.route("**/api/v1/**", async (route) => {
    if (await shell(route)) {
      return;
    }
    const path = new URL(route.request().url()).pathname.replace("/api/v1", "");
    if (path === "/runs") {
      return route.fulfill({
        status: 503,
        json: { error: { code: "queue.unavailable", message: "broker down" } },
      });
    }
    return route.fulfill({ status: 500, json: {} });
  });

  await openRuns(page);
  await expect(page.getByText(/queue\.unavailable/)).toBeVisible();
  await expect(page.getByRole("table")).toHaveCount(0);
});

test("the list survives a reload because the server is the truth", async ({ page }) => {
  await page.route("**/api/v1/**", async (route) => {
    if (await shell(route)) {
      return;
    }
    const path = new URL(route.request().url()).pathname.replace("/api/v1", "");
    if (path === "/runs") {
      return route.fulfill({ json: [run("succeeded")] });
    }
    return route.fulfill({ status: 500, json: {} });
  });

  await openRuns(page);
  await expect(page.getByText("succeeded", { exact: true })).toBeVisible();

  await page.reload();
  await page.getByRole("button", { name: /Runs/ }).click();
  await expect(page.getByText("succeeded", { exact: true })).toBeVisible();
});
