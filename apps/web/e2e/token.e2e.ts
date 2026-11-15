import { expect, test, type Page } from "@playwright/test";
import { installFixture } from "./fixtures";

const TOKEN = "qr_browser_token_0123456789abcdef";

/** The Authorization header on every API request the page made, in order. */
function watchAuthorization(page: Page): string[] {
  const seen: string[] = [];
  page.on("request", (request) => {
    if (request.url().includes("/api/v1/")) {
      seen.push(request.headers()["authorization"] ?? "");
    }
  });
  return seen;
}

async function enterToken(page: Page, token: string): Promise<void> {
  await page.getByLabel("Gateway bearer token").fill(token);
  await page.getByRole("button", { name: "Use token" }).click();
}

test("a gated API shows token required, not a stale connected state", async ({
  page,
}) => {
  await installFixture(page, { token: TOKEN });
  await page.goto("/");

  await expect(page.getByText("token required", { exact: true })).toBeVisible();
  await expect(page.locator(".notice.error")).toContainText("bearer token");
  await expect(page.getByText("connected", { exact: true })).toHaveCount(0);
});

test("entering the token connects and sends the header", async ({ page }) => {
  const authorization = watchAuthorization(page);
  await installFixture(page, { token: TOKEN });
  await page.goto("/");
  await expect(page.getByText("token required", { exact: true })).toBeVisible();

  await enterToken(page, TOKEN);
  await expect(page.getByText("connected", { exact: true })).toBeVisible();
  expect(authorization).toContain(`Bearer ${TOKEN}`);
});

test("the stored token survives a reload", async ({ page }) => {
  await installFixture(page, { token: TOKEN });
  await page.goto("/");
  await enterToken(page, TOKEN);
  await expect(page.getByText("connected", { exact: true })).toBeVisible();

  await page.reload();
  await expect(page.getByText("connected", { exact: true })).toBeVisible();
  await expect(page.getByText("token required", { exact: true })).toHaveCount(0);
});

test("forgetting the token returns to token required", async ({ page }) => {
  await page.addInitScript(
    ([key, value]) => localStorage.setItem(key, value),
    ["quantrule.token", TOKEN],
  );
  await installFixture(page, { token: TOKEN });
  await page.goto("/");
  await expect(page.getByText("connected", { exact: true })).toBeVisible();

  await page.getByRole("button", { name: "Forget token" }).click();
  await expect(page.getByText("token required", { exact: true })).toBeVisible();
  const saved = await page.evaluate(() => localStorage.getItem("quantrule.token"));
  expect(saved).toBeNull();
});

test("the token never enters the URL, the page text, or a cookie", async ({
  page,
}) => {
  const urls: string[] = [];
  page.on("request", (request) => urls.push(request.url()));
  await installFixture(page, { token: TOKEN });
  await page.goto("/");
  await enterToken(page, TOKEN);
  await expect(page.getByText("connected", { exact: true })).toBeVisible();

  expect(page.url()).not.toContain(TOKEN);
  expect(urls.join(" ")).not.toContain(TOKEN);
  expect(await page.locator("body").innerText()).not.toContain(TOKEN);
  expect(await page.getByLabel("Gateway bearer token").inputValue()).toBe("");
  const cookies = await page.context().cookies();
  expect(JSON.stringify(cookies)).not.toContain(TOKEN);
  const saved = await page.evaluate(() => localStorage.getItem("quantrule.token"));
  expect(saved).toBe(TOKEN);
});

test("a wrong token stays token required without echoing the value", async ({
  page,
}) => {
  await installFixture(page, { token: TOKEN });
  await page.goto("/");
  await enterToken(page, "wrong-token");

  await expect(page.getByText("token required", { exact: true })).toBeVisible();
  await expect(page.locator(".notice.error")).toContainText("auth.unauthorized");
  await expect(page.locator(".notice.error")).not.toContainText("wrong-token");
  expect(await page.locator("body").innerText()).not.toContain("wrong-token");
});
