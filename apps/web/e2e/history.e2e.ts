import { expect, test } from "@playwright/test";
import type { Route } from "@playwright/test";
import { installFixture } from "./fixtures";

const RUN = "sha256:" + "b".repeat(64);
const DATA = "sha256:" + "c".repeat(64);

async function openHistory(
  page: import("@playwright/test").Page,
): Promise<void> {
  let archived = false;
  let erased = false;
  await installFixture(page, {
    routes: async (path: string, route: Route) => {
      const decoded = decodeURIComponent(path);
      if (decoded === "/runs") {
        await route.fulfill({
          json: erased
            ? []
            : [
                {
                  run_id: RUN,
                  status: "failed",
                  config_id: "cfg_abc",
                  data_version: DATA,
                  submitted_at: "2026-11-20T00:00:00Z",
                  elapsed_ms: null,
                  ...(archived ? { archived_at: "2026-11-20T01:00:00Z" } : {}),
                },
              ],
        });
        return true;
      }
      if (decoded === `/runs/${RUN}/archive`) {
        archived = true;
        await route.fulfill({
          json: {
            run_id: RUN,
            status: "failed",
            config_id: "cfg_abc",
            data_version: DATA,
            submitted_at: "2026-11-20T00:00:00Z",
            elapsed_ms: null,
            archived_at: "2026-11-20T01:00:00Z",
          },
        });
        return true;
      }
      if (decoded === `/runs/${RUN}` && route.request().method() === "DELETE") {
        erased = true;
        await route.fulfill({ json: { record_id: RUN, bytes: 0 } });
        return true;
      }
      return false;
    },
  });
  await page.goto("/");
  await page.getByRole("button", { name: /^History/ }).click();
}

test("archives and erases a run with typed confirmation", async ({ page }) => {
  await openHistory(page);
  const table = page.getByTestId("history-table");
  await expect(table).toContainText("sha256:bbbb");
  await expect(table).toContainText("failed");

  await page.getByTestId("history-archive").click();
  await expect(table).toContainText("2026-11-20T01:00:00Z");
  await expect(page.getByTestId("history-erase")).toBeVisible();

  await page.getByTestId("history-erase").click();
  const confirm = page.getByTestId("history-confirm-input");
  await confirm.fill("wrong");
  await expect(page.getByTestId("history-confirm-erase")).toBeDisabled();
  await confirm.fill(RUN);
  await page.getByTestId("history-confirm-erase").click();
  await expect(page.getByTestId("history-table")).toHaveCount(0);
  await expect(page.getByText("No run records.")).toBeVisible();
});

test("the stage tab has no archive and no archive filter", async ({ page }) => {
  await installFixture(page, {
    routes: async (path: string, route: Route) => {
      if (decodeURIComponent(path) === "/stages/runs") {
        await route.fulfill({
          json: [
            {
              stage_run_id: "sha256:" + "d".repeat(64),
              kind: "model",
              state: "succeeded",
              config_id: "cfg_abc",
              submitted_at: "2026-11-20T00:00:00Z",
              artifact_id: null,
              evidence: false,
            },
          ],
        });
        return true;
      }
      return false;
    },
  });
  await page.goto("/");
  await page.getByRole("button", { name: /^History/ }).click();
  await page.getByTestId("history-tab-stage").click();
  await expect(page.getByTestId("history-table")).toContainText("succeeded");
  await expect(page.getByTestId("history-archive")).toHaveCount(0);
  await expect(page.getByTestId("history-include-archived")).toHaveCount(0);
});
