import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

interface Profile {
  profile_id: string;
  name: string;
  datasets: number;
  runs: number;
  simulations: number;
}

interface Dataset {
  data_version: string;
  name: string;
  candles: number;
  first_bar: number;
  last_bar: number;
  bytes: number;
}

interface Mock {
  profiles: Profile[];
  datasets: Record<string, Dataset[]>;
}

const DIGEST = "sha256:" + "a".repeat(64);

function profile(id: string, name: string, datasets: number): Profile {
  return { profile_id: id, name, datasets, runs: 0, simulations: 0 };
}

function dataset(fill: string, name: string, candles: number): Dataset {
  return {
    data_version: "sha256:" + fill.repeat(64),
    name,
    candles,
    first_bar: 1_704_067_200,
    last_bar: 1_704_067_200 + (candles - 1) * 60,
    bytes: candles * 32,
  };
}

function seed(): Mock {
  return {
    profiles: [profile("default", "Default", 1), profile("alpha", "Alpha", 1)],
    datasets: {
      default: [dataset("a", "btc.csv", 2)],
      alpha: [dataset("b", "eth.csv", 3)],
    },
  };
}

async function mockApi(page: Page, mock: Mock): Promise<void> {
  await page.route("**/api/v1/**", async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname.replace("/api/v1", "");
    const active = request.headers()["x-quantrule-profile"] ?? "default";

    if (path === "/version") {
      return route.fulfill({
        json: {
          product: "1.0.0",
          api: "quantrule.api/v1",
          image_digest: DIGEST,
          contracts: [],
        },
      });
    }
    if (path === "/health") {
      return route.fulfill({
        json: {
          v: "quantrule.api/v1",
          state: "healthy",
          services: [],
          queue_depth: 0,
          workers: 1,
          image_digest: DIGEST,
        },
      });
    }
    if (path === "/profiles" && request.method() === "GET") {
      return route.fulfill({ json: mock.profiles });
    }
    if (path === "/profiles" && request.method() === "POST") {
      const body = request.postDataJSON() as { profile_id: string; name: string };
      const created = profile(body.profile_id, body.name, 0);
      mock.profiles.push(created);
      return route.fulfill({ status: 201, json: created });
    }
    if (path === "/catalog/configs" && request.method() === "GET") {
      return route.fulfill({ json: [] });
    }
    if (path === "/catalog/datasets" && request.method() === "GET") {
      if (!mock.profiles.some((item) => item.profile_id === active)) {
        return route.fulfill({
          status: 404,
          json: {
            v: "quantrule.api/v1",
            error: {
              code: "profile.not_found",
              message: `profile ${active} is unknown`,
            },
          },
        });
      }
      return route.fulfill({ json: mock.datasets[active] ?? [] });
    }
    return route.fulfill({
      status: 500,
      json: { error: { code: "internal", message: `unmocked ${path}` } },
    });
  });
}

async function open(page: Page, mock: Mock): Promise<void> {
  await mockApi(page, mock);
  await page.goto("/");
  await expect(page.getByLabel("Active profile")).toHaveValue("default");
}

test("creates a profile and selects it", async ({ page }) => {
  const mock = seed();
  await open(page, mock);

  await page.getByRole("button", { name: /Profiles/ }).click();
  await page.getByLabel("New profile id").fill("research");
  await page.getByLabel("New profile name").fill("Research");
  await page.getByRole("button", { name: "Create profile" }).click();

  await expect(page.getByLabel("Active profile")).toHaveValue("research");
  await expect(page.getByRole("row", { name: /research/ })).toBeVisible();
  expect(mock.profiles.some((item) => item.profile_id === "research")).toBe(true);
});

test("switching a profile changes the dataset list without a page reload", async ({ page }) => {
  const mock = seed();
  await open(page, mock);
  await page.getByRole("button", { name: /Datasets/ }).click();
  await expect(page.getByRole("cell", { name: "btc.csv" })).toBeVisible();

  await page.evaluate(() => {
    (window as unknown as { marker: string }).marker = "kept";
  });
  await page.getByLabel("Active profile").selectOption("alpha");

  await expect(page.getByRole("cell", { name: "eth.csv" })).toBeVisible();
  await expect(page.getByRole("cell", { name: "btc.csv" })).toHaveCount(0);
  expect(
    await page.evaluate(() => (window as unknown as { marker: string }).marker),
  ).toBe("kept");
});

test("the active profile survives a reload", async ({ page }) => {
  const mock = seed();
  await open(page, mock);
  await page.getByLabel("Active profile").selectOption("alpha");
  await page.reload();

  await expect(page.getByLabel("Active profile")).toHaveValue("alpha");
  await page.getByRole("button", { name: /Datasets/ }).click();
  await expect(page.getByRole("cell", { name: "eth.csv" })).toBeVisible();
});

test("an unknown stored profile shows the error", async ({ page }) => {
  const mock = seed();
  await page.addInitScript(() => {
    localStorage.setItem("quantrule.profile", "ghost");
  });
  await mockApi(page, mock);
  await page.goto("/");

  await expect(page.getByLabel("Active profile")).toHaveValue("ghost");
  await expect(page.getByText("profile error", { exact: true })).toBeVisible();

  await page.getByRole("button", { name: /Datasets/ }).click();
  await expect(page.getByText(/profile\.not_found/)).toBeVisible();
});
