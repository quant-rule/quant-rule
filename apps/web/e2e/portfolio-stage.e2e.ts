import { expect, test } from "@playwright/test";
import type { Page, Route } from "@playwright/test";
import { createHash } from "node:crypto";

const DIGEST = "sha256:" + "a".repeat(64);
const DATA = "sha256:" + "c".repeat(64);
const CONFIG = "sha256:" + "d".repeat(64);
const IMAGE = "sha256:" + "e".repeat(64);
const DECISIONS = "sha256:" + "7".repeat(64);
const STAGE = "sha256:" + "6".repeat(64);
const STRATEGY_STAGE = "sha256:" + "8".repeat(64);
const METRICS =
  '{"schema":"quantrule.portfolio-metrics/v1","initial_cash":10000.0,' +
  '"final_equity":10020.0,"net_pnl":20.0,"execution_fees":1.0,' +
  '"commission":2.0,"funding_paid":3.0,"funding_received":0.0,' +
  '"max_drawdown":0.01,"exposure":0.5,"halted":false}\n';
const FILLS = "fill_id,side,quantity,price,bar\n0,buy,10,100.0,1\n";
const ORDERS = "order_id,side,state\n0,buy,filled\n";
const REJECTIONS = "order_id,reason\n";
const SUMMARY = "QuantRule portfolio replay\nfinal_equity: 10020\nhalted: false\n";
const EXECUTION_SUMMARY = "QuantRule execution run\ncosts: fee=1 slippage=2\n";

const FILES: Record<string, string> = {
  "execution/orders.csv": ORDERS,
  "execution/fills.csv": FILLS,
  "execution/rejections.csv": REJECTIONS,
  "execution/summary.txt": EXECUTION_SUMMARY,
  "execution/provenance.json":
    '{"schema":"quantrule.stage-provenance/v1","stage":"execution","artifact":"fills"}\n',
  "portfolio/trades.csv": "trade_id,side\n0,long\n",
  "portfolio/equity.csv": "date,equity\n0,10000\n",
  "portfolio/funding.csv": "bar,amount\n",
  "portfolio/risk_events.csv": "bar,drawdown\n",
  "portfolio/metrics.json": METRICS,
  "portfolio/summary.txt": SUMMARY,
  "portfolio/provenance.json":
    '{"schema":"quantrule.stage-provenance/v1","stage":"portfolio","artifact":"portfolio"}\n',
};

function artifactFiles() {
  return Object.entries(FILES).map(([name, body]) => ({
    name,
    bytes: Buffer.byteLength(body),
    sha256: "sha256:" + createHash("sha256").update(body).digest("hex"),
  }));
}

function detail() {
  return {
    stage_run_id: STAGE,
    kind: "portfolio",
    state: "succeeded",
    owner: "default",
    config_id: "cfg_abc",
    input: { kind: "decisions_artifact", decisions_artifact_id: DECISIONS },
    provenance: {
      data_version: DATA,
      config_hash: CONFIG,
      model_version: "M1.0",
      image_digest: IMAGE,
    },
    artifact: { artifact_id: DIGEST, files: artifactFiles() },
    submitted_at: "2026-11-17T00:00:02Z",
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

async function mockPortfolio(page: Page): Promise<void> {
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
            stage_run_id: STRATEGY_STAGE,
            kind: "strategy",
            state: "succeeded",
            config_id: "cfg_abc",
            submitted_at: "2026-11-17T00:00:01Z",
            artifact_id: DECISIONS,
            evidence: false,
          },
          {
            stage_run_id: STAGE,
            kind: "portfolio",
            state: "succeeded",
            config_id: "cfg_abc",
            submitted_at: "2026-11-17T00:00:02Z",
            artifact_id: DIGEST,
            evidence: false,
          },
        ],
      });
    }
    if (path === "/stages/portfolio/runs" && request.method() === "POST") {
      return route.fulfill({
        status: 202,
        json: { stage_run_id: STAGE, state: "queued", created: true },
      });
    }
    if (path === `/stages/runs/${STAGE}` && request.method() === "GET") {
      return route.fulfill({ json: detail() });
    }
    const artifact = Object.entries(FILES).find(
      ([name]) => path === `/stages/runs/${STAGE}/artifacts/${name}`,
    );
    if (artifact !== undefined) {
      return route.fulfill({
        status: 200,
        headers: { "content-type": "application/octet-stream" },
        body: Buffer.from(artifact[1]),
      });
    }
    return route.fulfill({ status: 404, json: {} });
  });
}

test("submits, reads the published bundles, and downloads a nested artifact", async ({ page }) => {
  await mockPortfolio(page);
  await page.goto("/");
  await page.getByRole("button", { name: /Workbench/ }).click();
  await page.getByRole("tab", { name: "Portfolio" }).click();

  await expect(page.getByText(/P&L is not evidence/)).toBeVisible();
  await page.getByLabel("Decisions artifact id").fill(DECISIONS);
  await page.getByLabel("Config id").fill("cfg_abc");
  await page.getByRole("button", { name: "Run portfolio stage" }).click();

  await expect(page.getByTestId("portfolio-table")).toBeVisible();
  await expect(page.getByTestId("portfolio-table")).not.toContainText("sha256:88888888");
  await page.getByRole("button", { name: `Open ${STAGE}` }).click();
  await expect(page.getByTestId("portfolio-detail")).toContainText("not an accepted result");
  await expect(page.getByTestId("portfolio-metrics")).toContainText("10020");
  await expect(page.getByTestId("portfolio-metrics")).toContainText("Exposure");
  await expect(page.getByTestId("portfolio-metrics")).toContainText("0.500000");
  await expect(page.getByTestId("portfolio-execution-summary")).toContainText("slippage=2");
  await expect(page.getByTestId("portfolio-fills")).toContainText("buy");
  await expect(page.getByTestId("portfolio-orders")).toContainText("filled");
  await expect(page.getByTestId("portfolio-rejections")).toContainText("reason");

  const pending = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download portfolio/metrics.json" }).click();
  const download = await pending;
  expect(download.suggestedFilename()).toBe("metrics.json");
  await expect(
    page.getByText("portfolio/metrics.json matched the stage record and was saved."),
  ).toBeVisible();
});

test("a strategy stage does not leak into the portfolio tab", async ({ page }) => {
  await mockPortfolio(page);
  await page.goto("/");
  await page.getByRole("button", { name: /Workbench/ }).click();
  await page.getByRole("tab", { name: "Strategy" }).click();
  await page.getByRole("button", { name: `Open ${STRATEGY_STAGE}` }).click();
  await page.getByRole("tab", { name: "Portfolio" }).click();
  await expect(page.getByTestId("portfolio-detail")).toHaveCount(0);
  await expect(page.getByTestId("portfolio-table")).toContainText("sha256:66666666");
});
