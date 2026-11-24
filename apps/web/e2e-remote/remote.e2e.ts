import { expect, test, type Page } from "@playwright/test";

// The remote browser proof for the lab ingress.
//
// This suite is lab-gated by e2e-remote/gate.ts and the remote Playwright
// config. It uses the real https://quantrule.dev gateway through the VIP, with
// no route mocks and no certificate-error bypass. The client OS or NSS store
// must already trust the private CA; that trust is what makes `page.goto`
// succeed. The untrusted-CA negative is the CLI `deploy/v3/lab.sh
// --remote-client` check with an empty trust store, because a browser negative
// would need a second host or a second CA and is not reliable on one lab.
//
// The token comes from QR_REMOTE_TOKEN and is never stored in the repository.

const TOKEN = process.env.QR_REMOTE_TOKEN ?? "";
const DIGEST = /^sha256:[0-9a-f]{64}$/;

test.describe.configure({ mode: "serial" });

test.beforeAll(() => {
  if (
    process.env.QR_REMOTE_BROWSER !== "1" ||
    process.env.QR_REMOTE_URL !== "https://quantrule.dev"
  ) {
    throw new Error(
      "Unverified: the remote browser suite is lab-gated; set QR_REMOTE_BROWSER=1 and QR_REMOTE_URL=https://quantrule.dev",
    );
  }
  if (TOKEN === "") {
    throw new Error(
      "Unverified: QR_REMOTE_TOKEN is required; no token is stored in the repository",
    );
  }
});

async function enterToken(page: Page): Promise<void> {
  const field = page.getByLabel("Gateway bearer token");
  await expect(field).toBeVisible();
  await field.fill(TOKEN);
  await page.getByRole("button", { name: "Use token" }).click();
}

test("loads over TLS, accepts the token, and reads /version", async ({ page }) => {
  const versionResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/v1/version") &&
      response.request().method() === "GET",
    { timeout: 30_000 },
  );
  await page.goto("/");
  const version = await versionResponse;
  expect(version.status()).toBe(200);
  const body = (await version.json()) as { api: string; image_digest: string };
  expect(body.api).toBe("quantrule.api/v1");
  expect(body.image_digest).toMatch(DIGEST);

  const gated = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/v1/profiles") &&
      response.request().method() === "GET" &&
      response.status() === 200,
    { timeout: 30_000 },
  );
  await enterToken(page);
  await expect(page.getByText("connected", { exact: true })).toBeVisible({
    timeout: 30_000,
  });
  expect((await gated).status()).toBe(200);

  await expect(page.getByText(body.image_digest, { exact: true })).toBeVisible();
  await expect(page.locator(".notice.error")).toHaveCount(0);

  const profiles = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/v1/profiles") &&
      response.request().method() === "GET" &&
      response.status() === 200,
    { timeout: 30_000 },
  );
  await page.getByRole("button", { name: /Profiles/ }).click();
  expect((await profiles).status()).toBe(200);
  await expect(page.getByRole("heading", { name: "Profiles" })).toBeVisible();
  await expect(page.locator(".notice.error")).toHaveCount(0);

  const datasets = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/v1/catalog/datasets") &&
      response.request().method() === "GET" &&
      response.status() === 200,
    { timeout: 30_000 },
  );
  await page.getByRole("button", { name: /Datasets/ }).click();
  expect((await datasets).status()).toBe(200);
  await expect(page.getByRole("heading", { name: "Datasets" })).toBeVisible();
  await expect(page.locator(".notice.error")).toHaveCount(0);
});

test("a wrong token is refused without echoing it", async ({ page }) => {
  const wrong = "qr_remote_wrong_token_not_in_repo";
  await page.goto("/");
  await page.getByLabel("Gateway bearer token").fill(wrong);
  await page.getByRole("button", { name: "Use token" }).click();

  await expect(page.getByText("token required", { exact: true })).toBeVisible({
    timeout: 30_000,
  });
  await expect(page.locator(".notice.error")).toContainText("auth.unauthorized");
  expect(await page.locator("body").innerText()).not.toContain(wrong);
});
