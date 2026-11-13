import { expect, test } from "@playwright/test";
import type { Route } from "@playwright/test";

const DIGEST = "sha256:" + "a".repeat(64);
const DATA = "sha256:" + "b".repeat(64);
const RUN = "sha256:" + "c".repeat(64);

const CONFIGS = [
  {
    config_id: "cfg_valid",
    name: "valid/run.json",
    origin: "repository",
    valid: true,
    message: null,
    config_hash: DIGEST,
  },
  {
    config_id: "cfg_invalid",
    name: "invalid/run.json",
    origin: "repository",
    valid: false,
    message: "missing-data.json: a readable file was expected",
    config_hash: null,
  },
];

const DATASET = {
  data_version: DATA,
  name: "btc.csv",
  candles: 10,
  first_bar: 1_704_067_200,
  last_bar: 1_704_067_740,
  bytes: 320,
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

test("a valid config submits the chosen dataset and opens the run list", async ({ page }) => {
  let submitted: { data_version?: string; config_id?: string } | null = null;

  await page.route("**/api/v1/**", async (route) => {
    if (await shell(route)) {
      return;
    }
    const request = route.request();
    const path = new URL(request.url()).pathname.replace("/api/v1", "");
    if (path === "/catalog/configs") {
      return route.fulfill({ json: CONFIGS });
    }
    if (path === "/catalog/datasets") {
      return route.fulfill({ json: [DATASET] });
    }
    if (path === "/runs" && request.method() === "POST") {
      submitted = request.postDataJSON() as { data_version?: string; config_id?: string };
      return route.fulfill({
        status: 202,
        json: { run_id: RUN, status: "queued", created: true },
      });
    }
    if (path === "/runs" && request.method() === "GET") {
      return route.fulfill({ json: [] });
    }
    return route.fulfill({ status: 500, json: {} });
  });

  await page.goto("/");
  await page.getByRole("button", { name: /Configs/ }).click();

  await expect(page.getByRole("cell", { name: "valid/run.json", exact: true })).toBeVisible();
  await expect(page.getByRole("cell", { name: "invalid/run.json", exact: true })).toBeVisible();
  await expect(page.getByText(/missing-data\.json/)).toBeVisible();
  await expect(page.getByLabel("Dataset")).toHaveValue(DATA);
  await expect(
    page.getByRole("button", { name: "Submit invalid/run.json" }),
  ).toBeDisabled();

  await page.getByRole("button", { name: "Submit valid/run.json" }).click();
  await expect(page.getByText(/Run queued/)).toBeVisible();
  expect(submitted).toEqual({ data_version: DATA, config_id: "cfg_valid" });

  await page.getByRole("button", { name: "View runs" }).click();
  await expect(page.getByRole("heading", { name: "Runs", level: 1 })).toBeVisible();
  await expect(page.getByText("No runs. Submit one from Configs.")).toBeVisible();
});

test("without a dataset every submit is disabled", async ({ page }) => {
  await page.route("**/api/v1/**", async (route) => {
    if (await shell(route)) {
      return;
    }
    const path = new URL(route.request().url()).pathname.replace("/api/v1", "");
    if (path === "/catalog/configs") {
      return route.fulfill({ json: CONFIGS });
    }
    if (path === "/catalog/datasets") {
      return route.fulfill({ json: [] });
    }
    return route.fulfill({ status: 500, json: {} });
  });

  await page.goto("/");
  await page.getByRole("button", { name: /Configs/ }).click();

  await expect(page.getByText("Upload a dataset before submitting.")).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Submit valid/run.json" }),
  ).toBeDisabled();
});
