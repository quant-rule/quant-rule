import { expect, test } from "@playwright/test";
import { installFixture } from "./fixtures";

const DATA = "sha256:" + "c".repeat(64);

const DATASETS = [
  {
    data_version: DATA,
    name: "candles.csv",
    candles: 3,
    first_bar: 1_704_067_200,
    last_bar: 1_704_067_320,
    bytes: 96,
  },
];

const TIMEFRAMES = {
  data_version: DATA,
  timeframes: [
    { timeframe: "1m", bars: 3, partial_buckets: 0, first_bar: 1_704_067_200, last_bar: 1_704_067_320 },
    { timeframe: "5m", bars: 1, partial_buckets: 1, first_bar: 1_704_067_200, last_bar: 1_704_067_200 },
  ],
};

const EVENTS = {
  data_version: DATA,
  timeframe: "5m",
  events: [
    { type: 1, source: 2, bar: 10 },
    { type: 2, source: 3, bar: 11 },
  ],
};

test("inspects candles, timeframes, and events", async ({ page }) => {
  await installFixture(page, {
    routes: async (path, route) => {
      if (path === "/catalog/datasets") {
        await route.fulfill({ json: DATASETS });
        return true;
      }
      if (path === "/inspect/candles") {
        await route.fulfill({ json: DATASETS[0] });
        return true;
      }
      if (path === "/inspect/timeframes") {
        await route.fulfill({ json: TIMEFRAMES });
        return true;
      }
      if (path === "/inspect/events") {
        await route.fulfill({ json: EVENTS });
        return true;
      }
      return false;
    },
  });

  await page.goto("/");
  await page.getByRole("button", { name: /Inspect/ }).click();
  await expect(page.getByLabel("Data version")).toHaveValue(DATA);

  await page.getByRole("button", { name: "Inspect", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Candles" })).toBeVisible();
  await expect(page.getByRole("row", { name: /Candles 3/ })).toBeVisible();
  await expect(page.getByRole("row", { name: /First bar 1704067200/ })).toBeVisible();

  await page.getByLabel("Inspection").selectOption("timeframes");
  await page.getByRole("button", { name: "Inspect", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Timeframes" })).toBeVisible();
  await expect(page.getByRole("row", { name: /1m 3 0/ })).toBeVisible();
  await expect(page.getByRole("row", { name: /5m 1 1/ })).toBeVisible();

  await page.getByLabel("Inspection").selectOption("events");
  await page.getByRole("button", { name: "Inspect", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Events — 5m" })).toBeVisible();
  await expect(page.getByRole("row", { name: /1 2 10/ })).toBeVisible();
  await expect(page.getByRole("row", { name: /2 3 11/ })).toBeVisible();
});
