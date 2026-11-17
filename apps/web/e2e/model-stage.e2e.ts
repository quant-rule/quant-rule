import { expect, test } from "@playwright/test";
import type { Page, Route } from "@playwright/test";
import { createHash } from "node:crypto";

const DIGEST = "sha256:" + "a".repeat(64);
const DATA = "sha256:" + "c".repeat(64);
const CONFIG = "sha256:" + "d".repeat(64);
const IMAGE = "sha256:" + "e".repeat(64);
const STAGE = "sha256:" + "f".repeat(64);
const ARTIFACT = "sha256:" + "9".repeat(64);
const EVENTS = "timeframe_secs,event_index,event_code\n300,0,1\n900,0,2\n300,1,3\n";
const REPORT =
  '{"model_version":"1.3.0","view_contract":"quantrule.model-view/v4",' +
  '"data_version":"fixture","requested_period":{"from":null,"to":null},' +
  '"flat_report":{"timeframes":[{"timeframe_secs":900,"bars":0,"expected_bars":1,"coverage_share":0.0},' +
  '{"timeframe_secs":300,"bars":3,"expected_bars":3,"coverage_share":1.0}]}}';
const EVENTS_SHA = "sha256:" + createHash("sha256").update(EVENTS).digest("hex");
const REPORT_SHA = "sha256:" + createHash("sha256").update(REPORT).digest("hex");

function list(state: string) {
  return [
    {
      stage_run_id: STAGE,
      kind: "model",
      state,
      config_id: "cfg_abc",
      submitted_at: "2026-11-17T00:00:00Z",
      artifact_id: state === "succeeded" ? ARTIFACT : null,
      evidence: false,
    },
  ];
}

function detail(state: string) {
  return {
    stage_run_id: STAGE,
    kind: "model",
    state,
    owner: "default",
    config_id: "cfg_abc",
    input: { kind: "data_version", data_version: DATA },
    provenance: {
      data_version: DATA,
      config_hash: CONFIG,
      model_version: "M1.0",
      image_digest: IMAGE,
    },
    artifact:
      state === "succeeded"
        ? {
            artifact_id: ARTIFACT,
            files: [
              {
                name: "events.csv",
                bytes: Buffer.byteLength(EVENTS),
                sha256: EVENTS_SHA,
              },
              {
                name: "report.json",
                bytes: Buffer.byteLength(REPORT),
                sha256: REPORT_SHA,
              },
            ],
          }
        : null,
    submitted_at: "2026-11-17T00:00:00Z",
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
      json: [{ profile_id: "default", name: "Default", datasets: 1, runs: 1, simulations: 0 }],
    });
    return true;
  }
  return false;
}

async function mockStage(page: Page, state: string): Promise<{ deleted: () => boolean }> {
  let deleted = false;
  let currentState = state;
  await page.route("**/api/v1/**", async (route) => {
    if (await shell(route)) {
      return;
    }
    const request = route.request();
    const path = decodeURIComponent(new URL(request.url()).pathname).replace("/api/v1", "");
    if (path === "/stages/runs" && request.method() === "GET") {
      return route.fulfill({ json: deleted ? [] : list(currentState) });
    }
    if (path === "/stages/model/runs" && request.method() === "POST") {
      return route.fulfill({
        status: 202,
        json: { stage_run_id: STAGE, state: "queued", created: true },
      });
    }
    if (path === `/stages/runs/${STAGE}/cancel` && request.method() === "POST") {
      currentState = "cancelled";
      return route.fulfill({ status: 202, json: detail("cancelled") });
    }
    if (path === `/stages/runs/${STAGE}` && request.method() === "DELETE") {
      deleted = true;
      return route.fulfill({ status: 204, body: "" });
    }
    if (path === `/stages/runs/${STAGE}/artifacts/events.csv`) {
      return route.fulfill({
        status: 200,
        headers: { "content-type": "text/csv" },
        body: Buffer.from(EVENTS),
      });
    }
    if (path === `/stages/runs/${STAGE}/artifacts/report.json`) {
      return route.fulfill({
        status: 200,
        headers: { "content-type": "application/json" },
        body: Buffer.from(REPORT),
      });
    }
    if (path === `/stages/runs/${STAGE}` && request.method() === "GET") {
      return route.fulfill({ json: detail(currentState) });
    }
    return route.fulfill({ status: 500, json: {} });
  });
  return { deleted: () => deleted };
}

async function openWorkbench(page: Page): Promise<void> {
  await page.goto("/");
  await page.getByRole("button", { name: /Workbench/ }).click();
  await page.getByRole("tab", { name: "Model" }).click();
}

test("submits, inspects, downloads, and deletes a model stage", async ({ page }) => {
  const stage = await mockStage(page, "succeeded");
  await openWorkbench(page);

  await expect(page.getByRole("tab", { name: "Data" })).toBeVisible();
  await page.getByLabel("Data version").fill(DATA);
  await page.getByLabel("Config id").fill("cfg_abc");
  await page.getByRole("button", { name: "Run model stage" }).click();

  const table = page.getByTestId("stage-table");
  await expect(table).toBeVisible();
  await expect(table).toContainText("model");
  await page.getByRole("button", { name: `Open ${STAGE}` }).click();

  await expect(page.getByTestId("stage-detail")).toContainText("M1.0");
  await expect(page.getByTestId("stage-detail")).toContainText("events.csv");
  await page.getByLabel("Event timeframe").fill("5m");
  await page.getByRole("button", { name: "Read events" }).click();
  await expect(page.getByText("2 event(s) at 5m.")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Coverage", exact: true })).toBeVisible();
  await expect(page.getByTestId("stage-detail")).toContainText("1.0000");

  const pending = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download events.csv" }).click();
  const download = await pending;
  expect(download.suggestedFilename()).toBe("events.csv");
  await expect(page.getByText("events.csv matched the stage record and was saved.")).toBeVisible();

  await page.getByTestId("stage-delete").click();
  await expect(page.getByText("No stage runs. Submit a model stage above.")).toBeVisible();
  expect(stage.deleted()).toBe(true);
});

test("cancels a running stage", async ({ page }) => {
  await mockStage(page, "running");
  await openWorkbench(page);
  await page.getByRole("button", { name: `Open ${STAGE}` }).click();
  await page.getByTestId("stage-cancel").click();
  await expect(page.getByTestId("stage-detail")).toContainText("cancelled");
});
