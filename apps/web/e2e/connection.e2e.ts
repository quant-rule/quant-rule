import { expect, test } from "@playwright/test";
import { API, DIGEST, installFixture } from "./fixtures";

test("shows the gateway identity when connected", async ({ page }) => {
  await installFixture(page);
  await page.goto("/");

  await expect(page.getByText("connected", { exact: true })).toBeVisible();
  await expect(page.getByText("1.0.0", { exact: true })).toBeVisible();
  await expect(page.getByText(API, { exact: true })).toBeVisible();
  await expect(page.getByText(DIGEST, { exact: true })).toBeVisible();
});

test("shows a version mismatch instead of connected", async ({ page }) => {
  await installFixture(page, { version: { api: "quantrule.api/v2" } });
  await page.goto("/");

  await expect(page.getByText("version mismatch", { exact: true })).toBeVisible();
  await expect(page.locator(".notice.error")).toContainText("quantrule.api/v2");
  await expect(page.locator(".notice.error")).toContainText(API);
  await expect(page.getByText("connected", { exact: true })).toHaveCount(0);
});

test("shows unreachable when the gateway does not answer", async ({ page }) => {
  await installFixture(page, { unreachable: true });
  await page.goto("/");

  await expect(page.getByText("unreachable", { exact: true })).toBeVisible();
  await expect(page.locator(".notice.error")).toBeVisible();
  await expect(page.getByText("connected", { exact: true })).toHaveCount(0);
});

test("shows degraded and names the down dependency", async ({ page }) => {
  await installFixture(page, {
    health: {
      state: "degraded",
      services: [{ name: "postgres", state: "down", detail: "connection refused" }],
    },
  });
  await page.goto("/");

  await expect(page.getByText("degraded", { exact: true })).toBeVisible();
  await expect(page.getByText("postgres", { exact: true })).toBeVisible();
  await expect(page.getByText("connection refused", { exact: true })).toBeVisible();
});
