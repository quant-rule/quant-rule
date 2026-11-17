import { expect, test } from "@playwright/test";
import type { Page, Route } from "@playwright/test";
import { createHash } from "node:crypto";

const DIGEST = "sha256:" + "a".repeat(64);
const DATA = "sha256:" + "c".repeat(64);
const CONFIG = "sha256:" + "d".repeat(64);
const IMAGE = "sha256:" + "e".repeat(64);
const MODEL = "sha256:" + "9".repeat(64);
const STAGE = "sha256:" + "8".repeat(64);
const DECISIONS =
  '{"schema":"quantrule.strategy-decision/v1","timeframe_secs":60,"bar_index":0,' +
  '"decision":"enter","side":"long","reason":"entry","position_before":"flat",' +
  '"position_after":"long","atr14":1.5}\n' +
  '{"schema":"quantrule.strategy-decision/v1","timeframe_secs":60,"bar_index":1,' +
  '"decision":"hold","side":null,"reason":null,"position_before":"long",' +
  '"position_after":"long","atr14":1.5}\n';
const DECISIONS_SHA = "sha256:" + createHash("sha256").update(DECISIONS).digest("hex");
const SUMMARY = "QuantRule strategy run\n";
const PROVENANCE =
  '{"schema":"quantrule.stage-provenance/v1","stage":"strategy","artifact":"decisions"}\n';

function detail(state: string) {
  return {
    stage_run_id: STAGE,
    kind: "strategy",
    state,
    owner: "default",
    config_id: "cfg_abc",
    input: { kind: "model_artifact", model_artifact_id: MODEL },
    provenance: {
      data_version: DATA,
      config_hash: CONFIG,
      model_version: "M1.0",
      image_digest: IMAGE,
    },
    artifact:
      state === "succeeded"
        ? {
            artifact_id: DIGEST,
            files: [
              {
                name: "decisions.ndjson",
                bytes: Buffer.byteLength(DECISIONS),
                sha256: DECISIONS_SHA,
              },
              {
                name: "provenance.json",
                bytes: Buffer.byteLength(PROVENANCE),
                sha256: "sha256:" + createHash("sha256").update(PROVENANCE).digest("hex"),
              },
              {
                name: "summary.txt",
                bytes: Buffer.byteLength(SUMMARY),
                sha256: "sha256:" + createHash("sha256").update(SUMMARY).digest("hex"),
              },
            ],
          }
        : null,
    submitted_at: "2026-11-17T00:00:01Z",
    error: null,
    attempts: 1,
    evidence: false,
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
      json: [{ profile_id: "default", name: "Default", datasets: 1, runs: 0, simulations: 0 }],
    });
    return true;
  }
  return false;
}

async function mockStrategy(page: Page): Promise<void> {
  await page.route("**/api/v1/**", async (route) => {
    if (await shell(route)) {
      return;
    }
    const request = route.request();
    const path = decodeURIComponent(new URL(request.url()).pathname).replace("/api/v1", "");
    if (path === "/stages/runs" && request.method() === "GET") {
      return route.fulfill({
        json: [
          {
            stage_run_id: STAGE,
            kind: "strategy",
            state: "succeeded",
            config_id: "cfg_abc",
            submitted_at: "2026-11-17T00:00:01Z",
            artifact_id: DIGEST,
            evidence: false,
          },
        ],
      });
    }
    if (path === "/stages/strategy/runs" && request.method() === "POST") {
      return route.fulfill({
        status: 202,
        json: { stage_run_id: STAGE, state: "queued", created: true },
      });
    }
    if (path === `/stages/runs/${STAGE}` && request.method() === "GET") {
      return route.fulfill({ json: detail("succeeded") });
    }
    if (path === `/stages/runs/${STAGE}/artifacts/decisions.ndjson`) {
      return route.fulfill({
        status: 200,
        headers: { "content-type": "application/x-ndjson" },
        body: Buffer.from(DECISIONS),
      });
    }
    return route.fulfill({ status: 404, json: {} });
  });
}

test("submits, explains, downloads, and shows the no-P&L boundary", async ({ page }) => {
  await mockStrategy(page);
  await page.goto("/");
  await page.getByRole("button", { name: /Workbench/ }).click();
  await page.getByRole("tab", { name: "Strategy" }).click();

  await expect(
    page.getByText(/standalone strategy simulates its own position/),
  ).toBeVisible();
  await page.getByLabel("Model artifact id").fill(MODEL);
  await page.getByLabel("Config id").fill("cfg_abc");
  await page.getByRole("button", { name: "Run strategy stage" }).click();

  await expect(page.getByTestId("strategy-table")).toBeVisible();
  await page.getByRole("button", { name: `Open ${STAGE}` }).click();
  await expect(page.getByTestId("strategy-detail")).toContainText("No P&L");
  await expect(page.getByTestId("strategy-decisions")).toContainText("enter");
  await expect(page.getByTestId("strategy-decisions")).toContainText("hold");
  await page.getByLabel("Decision bar").fill("1");
  await page.getByTestId("strategy-explain").click();
  await expect(page.getByTestId("strategy-detail")).toContainText("hold");
  await expect(page.getByTestId("strategy-detail")).toContainText("long");

  const pending = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download decisions.ndjson" }).click();
  const download = await pending;
  expect(download.suggestedFilename()).toBe("decisions.ndjson");
  await expect(page.getByText("decisions.ndjson matched the stage record and was saved.")).toBeVisible();
});
