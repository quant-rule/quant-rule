import { expect, test } from "@playwright/test";

test("the core local workflow runs end to end", async ({ page }) => {
  await page.goto("/");

  const nav = page.getByRole("navigation");

  await nav.getByRole("button", { name: /Settings/ }).click();
  await page.getByLabel("Workspace root").fill("/mock/workspace");
  await page.getByLabel("Research memory database").fill("/mock/workspace/research.db");

  await nav.getByRole("button", { name: /Workspace/ }).click();
  await page.getByRole("button", { name: "Open workspace" }).click();
  await expect(page.getByText("workspace open", { exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Datasets" })).toBeVisible();

  await nav.getByRole("button", { name: /^Data/ }).click();
  await page.getByRole("button", { name: /BTCUSD_M1_2024/ }).click();
  await expect(page.getByText("527,040")).toBeVisible();
  await expect(page.getByText("2024-01-01T00:00:00Z")).toBeVisible();

  await nav.getByRole("button", { name: /^Configs/ }).click();
  await page
    .getByRole("button", { name: /btcusd-2024\/run.json/ })
    .click();
  await expect(page.getByRole("cell", { name: "BTCUSD", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Use in Run" }).click();

  await expect(page.getByRole("heading", { name: "Run", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Run model", exact: true }).click();
  await expect(page.getByText("27,655")).toBeVisible();
  await expect(page.getByRole("cell", { name: "1h", exact: true })).toBeVisible();

  await nav.getByRole("button", { name: /^Memory/ }).click();
  await page.getByRole("button", { name: "Open memory" }).click();
  await expect(page.getByText("quantrule.research-memory/v1")).toBeVisible();

  await page.getByLabel("Run key").fill("run-1");
  await page.getByRole("button", { name: "Record current run" }).click();
  await expect(page.getByText("Recorded run #1.")).toBeVisible();
  await expect(page.getByText("run-1")).toBeVisible();

  await page.getByRole("button", { name: /run-1/ }).click();
  await expect(page.getByRole("heading", { name: "Evidence for run #1" })).toBeVisible();
  await page.getByLabel("Metric name").fill("sharpe");
  await page.getByLabel("Metric value").fill("1.5");
  await page.getByRole("button", { name: "Add metric" }).click();
  await expect(page.getByText("Metric added.")).toBeVisible();
  await page.getByLabel("Claim statement").fill("the run is deterministic");
  await page.getByRole("button", { name: "Add claim" }).click();
  await expect(page.getByText("Claim added.")).toBeVisible();
  await page.getByLabel("Note body").fill("first review");
  await page.getByRole("button", { name: "Add note" }).click();
  await expect(page.getByText("Note added.")).toBeVisible();

  await page.getByLabel("Export path").fill("/mock/workspace/export.json");
  await page.getByRole("button", { name: "Export", exact: true }).click();
  await expect(page.getByText("Exported to /mock/workspace/export.json.")).toBeVisible();
});
