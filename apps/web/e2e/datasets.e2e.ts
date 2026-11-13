import { expect, test } from "@playwright/test";
import type { Route } from "@playwright/test";

const DIGEST = "sha256:" + "a".repeat(64);
const DATA = "sha256:" + "b".repeat(64);

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
      json: [{ profile_id: "default", name: "Default", datasets: 0, runs: 0, simulations: 0 }],
    });
    return true;
  }
  return false;
}

test("a tolerant upload shows the skipped rows", async ({ page }) => {
  let tolerant = false;

  await page.route("**/api/v1/**", async (route) => {
    if (await shell(route)) {
      return;
    }
    const request = route.request();
    const path = new URL(request.url()).pathname.replace("/api/v1", "");
    if (path === "/catalog/datasets" && request.method() === "GET") {
      return route.fulfill({ json: [] });
    }
    if (path === "/catalog/datasets/tolerant" && request.method() === "POST") {
      tolerant = true;
      return route.fulfill({
        status: 201,
        json: {
          summary: {
            data_version: DATA,
            name: "candles.csv",
            candles: 4,
            first_bar: 1_704_067_200,
            last_bar: 1_704_067_200,
            bytes: 128,
          },
          object_key: "datasets/bbb",
          sha256: DATA,
          quality: {
            rows_read: 8,
            accepted: 4,
            rejected: 4,
            duplicates: 1,
            out_of_order: 1,
            invalid: 2,
            skipped: [
              { line: 4, reason: "duplicate" },
              { line: 5, reason: "misaligned" },
              { line: 6, reason: "out_of_order" },
              { line: 7, reason: "invalid" },
            ],
          },
          created_at: "2026-11-13T00:00:00Z",
        },
      });
    }
    return route.fulfill({ status: 500, json: {} });
  });

  await page.goto("/");
  await page.getByRole("button", { name: /Datasets/ }).click();
  await expect(page.getByText("No datasets. Upload a CSV.")).toBeVisible();

  await page.getByLabel("CSV file").setInputFiles({
    name: "candles.csv",
    mimeType: "text/csv",
    buffer: Buffer.from("date,open,high,low,close,volume\n"),
  });
  await page.getByLabel("Skip bad rows").check();
  await page.getByRole("button", { name: "Upload" }).click();

  await expect(page.getByText(/4 of 8 rows skipped/)).toBeVisible();
  await expect(page.getByText(/line 4/)).toBeVisible();
  expect(tolerant).toBe(true);
});
