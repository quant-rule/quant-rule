import { expect, test } from "@playwright/test";
import type { Page, Route } from "@playwright/test";
import { createHash } from "node:crypto";

const DIGEST = "sha256:" + "a".repeat(64);
const RUN = "sha256:" + "b".repeat(64);
const DATA = "sha256:" + "c".repeat(64);
const CONFIG = "sha256:" + "d".repeat(64);
const IMAGE = "sha256:" + "e".repeat(64);
const ARTIFACT_BODY = "{\"ok\":true}\n";
const ARTIFACT = "sha256:" + createHash("sha256").update(ARTIFACT_BODY).digest("hex");
const ARTIFACT_BYTES = Buffer.byteLength(ARTIFACT_BODY);

const PROVENANCE = {
  data_version: DATA,
  config_hash: CONFIG,
  model_version: "M1.0",
  image_digest: IMAGE,
};

function list(status: string) {
  return [
    {
      run_id: RUN,
      status,
      config_id: "cfg_abc",
      data_version: DATA,
      submitted_at: "2026-11-13T12:00:00Z",
      elapsed_ms: 1_200,
    },
  ];
}

function detail(status: string) {
  return {
    run_id: RUN,
    status,
    owner: "default",
    stages: [
      { name: "data", state: status === "failed" ? "failed" : "done", elapsed_ms: 1_200 },
      { name: "model", state: status === "failed" ? "skipped" : "done", elapsed_ms: 2_400 },
      { name: "backtest", state: status === "failed" ? "pending" : "done", elapsed_ms: 3_600 },
      { name: "evidence", state: status === "failed" ? "pending" : "done", elapsed_ms: 120 },
    ],
    provenance: PROVENANCE,
    artifacts: [{ name: "results.json", bytes: ARTIFACT_BYTES, sha256: ARTIFACT }],
    digest: status === "succeeded" ? DIGEST : null,
    error: status === "failed" ? "data stage failed" : null,
  };
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
      json: [{ profile_id: "default", name: "Default", datasets: 1, runs: 1, simulations: 0 }],
    });
    return true;
  }
  return false;
}

async function mockDetail(
  page: Page,
  status: string,
  artifactBody = ARTIFACT_BODY,
): Promise<void> {
  await page.route("**/api/v1/**", async (route) => {
    if (await shell(route)) {
      return;
    }
    const request = route.request();
    const path = decodeURIComponent(new URL(request.url()).pathname).replace("/api/v1", "");
    if (path === "/runs") {
      return route.fulfill({ json: list(status) });
    }
    if (path === `/runs/${RUN}/digest`) {
      return route.fulfill({ json: { run_id: RUN, digest: DIGEST } });
    }
    if (path === `/runs/${RUN}/artifacts/results.json`) {
      return route.fulfill({
        status: 200,
        headers: { "content-type": "application/octet-stream" },
        body: Buffer.from(artifactBody),
      });
    }
    if (path.startsWith("/runs/") && request.method() === "GET") {
      return route.fulfill({ json: detail(status) });
    }
    return route.fulfill({ status: 500, json: {} });
  });
}

async function openDetail(
  page: Page,
  status: string,
  artifactBody = ARTIFACT_BODY,
): Promise<void> {
  await page.addInitScript(() => {
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: {
        writeText: async (text: string) => {
          (window as unknown as { copied: string }).copied = text;
        },
      },
    });
  });
  await mockDetail(page, status, artifactBody);
  await page.goto("/");
  await page.getByRole("button", { name: /Runs/ }).click();
  await page.getByRole("button", { name: `Open ${RUN}` }).click();
  await expect(page.getByRole("heading", { name: "Run detail" })).toBeVisible();
}

test("renders stages, provenance, artifacts, and copies the digest", async ({ page }) => {
  await openDetail(page, "succeeded");

  await expect(page.getByText("data", { exact: true })).toBeVisible();
  await expect(page.getByText("evidence", { exact: true })).toBeVisible();
  await expect(page.getByText("M1.0")).toBeVisible();
  await expect(page.getByText(IMAGE)).toBeVisible();
  await expect(page.getByText("Canonical digest")).toBeVisible();
  await expect(page.getByText(DIGEST)).toBeVisible();
  await expect(page.getByText("results.json")).toBeVisible();
  await expect(page.getByText("12 B")).toBeVisible();

  await page.getByRole("button", { name: "Copy canonical digest" }).click();
  await expect(page.getByText("Copied")).toBeVisible();
  expect(
    await page.evaluate(() => (window as unknown as { copied: string }).copied),
  ).toBe(DIGEST);
});

test("a failed run shows the error and hides the artifact section", async ({ page }) => {
  await openDetail(page, "failed");

  await expect(page.getByText("data stage failed")).toBeVisible();
  await expect(page.getByText("Artifacts")).toHaveCount(0);
  await expect(page.getByText("results.json")).toHaveCount(0);
});

test("verify reads the recorded digest", async ({ page }) => {
  await openDetail(page, "succeeded");

  await page.getByRole("button", { name: "Verify canonical digest" }).click();
  await expect(page.getByText("matches the digest above")).toBeVisible();
});

test("download offers an artifact that matches the record", async ({ page }) => {
  await openDetail(page, "succeeded");

  const pending = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download results.json" }).click();
  const download = await pending;
  expect(download.suggestedFilename()).toBe("results.json");
  await expect(page.getByText("results.json matched the record and was saved.")).toBeVisible();
});

test("a substituted artifact is refused", async ({ page }) => {
  await openDetail(page, "succeeded", "{\"ok\":false}\n");

  await page.getByRole("button", { name: "Download results.json" }).click();
  await expect(page.getByRole("alert")).toContainText("does not match the run record");
  await expect(page.getByText(/matched the record and was saved/)).toHaveCount(0);
});
