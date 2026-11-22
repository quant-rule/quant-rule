import { expect, test } from "@playwright/test";
import { createHash } from "node:crypto";
import { installFixture } from "./fixtures";

const PACK_BODY =
  "{\"kind\":\"strategy\",\"name\":\"parity-pack\",\"owner\":\"operator\",\"payload\":{},\"schema\":\"quantrule.pack/v1\",\"version\":\"1.0.0\"}\n";
const SIGNAL_BODY = "{\"schema\":\"quantrule.signal-artifact/v1\",\"rows\":[]}\n";
const PACK = "sha256:" + createHash("sha256").update(PACK_BODY).digest("hex");
const SIGNAL = "sha256:" + createHash("sha256").update(SIGNAL_BODY).digest("hex");
const MODEL = "sha256:" + "9".repeat(64);
const DATA = "sha256:" + "d".repeat(64);

type PackState = "draft" | "validated" | "frozen" | "retired";

function summary(state: PackState) {
  return {
    pack_id: PACK,
    kind: "strategy",
    name: "parity-pack",
    version: "1.0.0",
    owner: "operator",
    state,
    bytes: Buffer.byteLength(PACK_BODY),
    created_at: "2026-11-22T00:00:00Z",
  };
}

function record(state: PackState) {
  return {
    summary: summary(state),
    object_key: "packs/" + PACK.slice(7),
    sha256: PACK,
    validations: [],
    events: [],
  };
}

function signalRecord() {
  return {
    summary: {
      artifact_id: SIGNAL,
      kind: "signal",
      bytes: Buffer.byteLength(SIGNAL_BODY),
      created_at: "2026-11-22T00:04:00Z",
    },
    object_key: "artifacts/" + SIGNAL.slice(7),
    sha256: SIGNAL,
  };
}

test("a pack moves through validate, freeze, retire and downloads", async ({ page }) => {
  let state: PackState = "draft";
  await installFixture(page, {
    routes: async (rawPath, route) => {
      const path = decodeURIComponent(rawPath);
      if (path === "/catalog/packs" && route.request().method() === "GET") {
        await route.fulfill({ json: [summary(state)] });
        return true;
      }
      if (path === `/catalog/packs/${PACK}`) {
        await route.fulfill({ json: record(state) });
        return true;
      }
      if (path === `/catalog/packs/${PACK}/validate`) {
        state = "validated";
        await route.fulfill({ json: record(state) });
        return true;
      }
      if (path === `/catalog/packs/${PACK}/freeze`) {
        state = "frozen";
        await route.fulfill({ json: record(state) });
        return true;
      }
      if (path === `/catalog/packs/${PACK}/retire`) {
        state = "retired";
        await route.fulfill({ json: record(state) });
        return true;
      }
      if (path === `/catalog/packs/${PACK}/bytes`) {
        await route.fulfill({
          status: 200,
          headers: { "content-type": "application/json" },
          body: Buffer.from(PACK_BODY),
        });
        return true;
      }
      return false;
    },
  });
  await page.goto("/");
  await page.getByRole("button", { name: /^Packs/ }).click();
  await expect(page.getByTestId("packs-table")).toContainText("parity-pack");
  await page.getByRole("button", { name: `Open pack ${PACK}` }).click();
  await expect(page.getByTestId("pack-detail")).toContainText("State");
  await expect(page.getByTestId("pack-detail")).toContainText("draft");

  await page.getByTestId("pack-validate").click();
  await expect(page.getByTestId("pack-detail")).toContainText("validated");
  await page.getByTestId("pack-freeze").click();
  await expect(page.getByTestId("pack-detail")).toContainText("frozen");
  await page.getByTestId("pack-retire").click();
  await expect(page.getByTestId("pack-detail")).toContainText("retired");

  const pending = page.waitForEvent("download");
  await page.getByTestId("pack-fetch").click();
  expect(await pending).not.toBeNull();
  await expect(page.getByText(/matched the record and was saved/)).toBeVisible();
});

test("a pack is registered from a local file", async ({ page }) => {
  await installFixture(page, {
    routes: async (rawPath, route) => {
      const path = decodeURIComponent(rawPath);
      if (path === "/catalog/packs" && route.request().method() === "GET") {
        await route.fulfill({ json: [summary("draft")] });
        return true;
      }
      if (path === "/catalog/packs" && route.request().method() === "POST") {
        await route.fulfill({ status: 201, json: record("draft") });
        return true;
      }
      return false;
    },
  });
  await page.goto("/");
  await page.getByRole("button", { name: /^Packs/ }).click();
  await page.getByTestId("pack-create").setInputFiles({
    name: "parity-pack.json",
    mimeType: "application/json",
    buffer: Buffer.from(PACK_BODY),
  });
  await expect(page.getByTestId("pack-created")).toBeVisible();
  await expect(page.getByTestId("pack-detail")).toContainText("parity-pack");
});

test("signal create computes alpha and the download is the document", async ({ page }) => {
  let signals: ReturnType<typeof signalRecord>[] = [];
  await installFixture(page, {
    routes: async (rawPath, route) => {
      const path = decodeURIComponent(rawPath);
      if (path === "/catalog/packs") {
        await route.fulfill({ json: [summary("frozen")] });
        return true;
      }
      if (path === "/signals" && route.request().method() === "GET") {
        await route.fulfill({ json: signals });
        return true;
      }
      if (path === "/signals" && route.request().method() === "POST") {
        const artifact = signalRecord();
        signals = [artifact];
        await route.fulfill({
          status: 201,
          json: {
            schema: "quantrule.signal-response/v1",
            artifact,
            created: true,
          },
        });
        return true;
      }
      if (path === `/signals/${SIGNAL}`) {
        await route.fulfill({ json: signalRecord() });
        return true;
      }
      if (path === `/signals/${SIGNAL}/bytes`) {
        await route.fulfill({
          status: 200,
          headers: { "content-type": "application/octet-stream" },
          body: Buffer.from(SIGNAL_BODY),
        });
        return true;
      }
      return false;
    },
  });
  await page.goto("/");
  await page.getByRole("button", { name: /^Packs/ }).click();
  await page.getByRole("tab", { name: "Signals" }).click();
  await page.getByLabel("Model artifact id").fill(MODEL);
  await page.getByLabel("Data version").fill(DATA);
  await page.getByLabel("Config id").fill("cfg_abc");
  await page.getByLabel("Strategy pack").selectOption(PACK);
  await page.getByTestId("signal-create").click();
  await expect(page.getByTestId("signal-created")).toBeVisible();
  await expect(page.getByTestId("signal-table")).toContainText("signal");
  await page.getByRole("button", { name: `Open signal ${SIGNAL}` }).click();
  await expect(page.getByTestId("signal-detail")).toContainText("computed alpha");

  const pending = page.waitForEvent("download");
  await page.getByTestId("signal-fetch").click();
  expect(await pending).not.toBeNull();
  await expect(page.getByText(/matched the record and was saved/)).toBeVisible();
});

test("a v2 deployment disables signal create with the same message", async ({ page }) => {
  await installFixture(page, {
    routes: async (rawPath, route) => {
      const path = decodeURIComponent(rawPath);
      if (path === "/catalog/packs") {
        await route.fulfill({ json: [summary("frozen")] });
        return true;
      }
      if (path === "/signals" && route.request().method() === "GET") {
        await route.fulfill({ json: [] });
        return true;
      }
      if (path === "/signals" && route.request().method() === "POST") {
        await route.fulfill({
          status: 503,
          json: {
            v: "quantrule.api/v1",
            error: {
              code: "service.unavailable",
              message: "the strategy service is not configured",
            },
          },
        });
        return true;
      }
      return false;
    },
  });
  await page.goto("/");
  await page.getByRole("button", { name: /^Packs/ }).click();
  await page.getByRole("tab", { name: "Signals" }).click();
  await page.getByLabel("Model artifact id").fill(MODEL);
  await page.getByLabel("Data version").fill(DATA);
  await page.getByLabel("Config id").fill("cfg_abc");
  await page.getByLabel("Strategy pack").selectOption(PACK);
  await page.getByTestId("signal-create").click();
  await expect(page.getByRole("status")).toContainText(
    "does not run the strategy service",
  );
  await expect(page.getByTestId("signal-create")).toBeDisabled();
});
