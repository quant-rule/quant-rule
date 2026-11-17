import { expect, test } from "@playwright/test";
import type { Route } from "@playwright/test";
import { DIGEST, installFixture } from "./fixtures";

const RUN = "sha256:" + "b".repeat(64);
const MISSING = "sha256:" + "f".repeat(64);

function report() {
  return {
    stage_retention_days: 30,
    stage_store: true,
    last_cleanup: {
      completed_utc: "2026-11-17T08:00:00Z",
      stages_removed: 2,
      objects_removed: 3,
      error: null,
    },
  };
}

async function mockOperations(
  path: string,
  route: Route,
  value: ReturnType<typeof report>,
): Promise<boolean> {
  const decoded = decodeURIComponent(path);
  if (decoded === "/operations") {
    await route.fulfill({ json: value });
    return true;
  }
  if (decoded === `/runs/${RUN}/digest`) {
    await route.fulfill({ json: { run_id: RUN, digest: DIGEST } });
    return true;
  }
  if (decoded === `/runs/${MISSING}/digest`) {
    await route.fulfill({
      status: 404,
      json: {
        v: "quantrule.api/v1",
        error: { code: "run.not_found", message: `run ${MISSING} is unknown` },
      },
    });
    return true;
  }
  return false;
}

async function openOperations(
  page: import("@playwright/test").Page,
  value = report(),
): Promise<void> {
  await installFixture(page, {
    routes: (path, route) => mockOperations(path, route, value),
  });
  await page.goto("/");
  await page.getByRole("button", { name: /^Operations/ }).click();
}

test("shows the connection, profile, retention, and a verified digest", async ({
  page,
}) => {
  await openOperations(page);

  const health = page.getByTestId("operations-health");
  await expect(health).toContainText("connected");
  await expect(health).toContainText("1.0.0");
  await expect(health).toContainText(DIGEST);

  const profile = page.getByTestId("operations-profile");
  await expect(profile).toContainText("default");
  await expect(profile).toContainText("Not stored");

  const retention = page.getByTestId("operations-retention");
  await expect(retention).toContainText("30 days");
  await expect(retention).toContainText("2026-11-17T08:00:00Z");
  await expect(retention).toContainText("2 stage runs and 3 objects removed");
  await expect(retention).toContainText("Only diagnostic stage artifacts expire");
  await expect(retention).toContainText(
    "Runs, research records, and evidence are permanent",
  );

  const verify = page.getByTestId("operations-verify");
  await verify.getByLabel("Run id to verify").fill(RUN);
  await verify.getByRole("button", { name: "Verify digest" }).click();
  await expect(page.getByTestId("operations-verify-digest")).toHaveText(DIGEST);
  await expect(page.getByText("not research evidence")).toBeVisible();
});

test("shows a failed cleanup and a verify error instead of hiding them", async ({
  page,
}) => {
  await openOperations(page, {
    stage_retention_days: 1,
    stage_store: false,
    last_cleanup: {
      completed_utc: "2026-11-17T08:00:00Z",
      stages_removed: 0,
      objects_removed: 0,
      error: "the object store refused a delete",
    },
  });

  const retention = page.getByTestId("operations-retention");
  await expect(retention).toContainText("1 day");
  await expect(retention).toContainText("Failed: the object store refused a delete");
  await expect(retention).toContainText("No stage store is configured");

  const verify = page.getByTestId("operations-verify");
  await verify.getByLabel("Run id to verify").fill(MISSING);
  await verify.getByRole("button", { name: "Verify digest" }).click();
  await expect(page.locator(".notice.error")).toContainText("run.not_found");
  await expect(page.getByTestId("operations-verify-digest")).toHaveCount(0);
});
