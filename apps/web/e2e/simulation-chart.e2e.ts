import { expect, test } from "@playwright/test";
import type { Page, Route } from "@playwright/test";

const RUN = "sim_fixture";
const CHART = 1;

const SERIES = {
  sim_id: RUN,
  chart: CHART,
  candles: [
    { bar: 0, open: 100, high: 102, low: 99, close: 101, volume: 10 },
    { bar: 1, open: 101, high: 103, low: 100, close: 100, volume: 20 },
    { bar: 2, open: 100, high: 101, low: 98, close: 99, volume: 0 },
    { bar: 3, open: 99, high: 102, low: 98, close: 102, volume: 30 },
    { bar: 4, open: 102, high: 104, low: 101, close: 103, volume: 40 },
    { bar: 5, open: 103, high: 105, low: 102, close: 104, volume: 50 },
    { bar: 6, open: 104, high: 106, low: 103, close: 105, volume: 60 },
    { bar: 7, open: 105, high: 107, low: 104, close: 106, volume: 70 },
  ],
  ema_fast: [101, 101, 100.5, 101, 102, 103, 104, 105],
  ema_mid: [null, 100.5, 100.4, 100.8, 101.8, 102.8, 103.8, 104.8],
  ema_slow: [null, null, null, 100.5, 101.5, 102.5, 103.5, 104.5],
  panel: { indicator_code: 2, values: [48, 52, 45, 60, 63, 58, 66, 70] },
  events: [
    {
      event_code: 8,
      event_type: 3,
      event_subtype_code: 1,
      source_code: 4,
      occurred_bar_index: 1,
      available_bar_index: 2,
      direction_code: 1,
    },
  ],
  levels: [
    {
      reference_code: 1001,
      scale_code: 1,
      kind_code: -1,
      price: 100,
      occurred_bar_index: 0,
      available_bar_index: 1,
    },
  ],
};

interface SeenMark {
  chart: number;
  bar: number;
  action: string;
  quality: string;
  side: string;
  note: string;
}

interface Harness {
  marks: SeenMark[];
  deletes: string[];
}

async function installSimulation(page: Page): Promise<Harness> {
  const harness: Harness = { marks: [], deletes: [] };
  let nextMark = 1;
  await page.route("**/api/v1/**", async (route: Route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname.replace("/api/v1", "");
    if (path === `/simulation/runs/${RUN}/windows/${CHART}/series`) {
      await route.fulfill({ json: SERIES });
      return;
    }
    if (path === `/simulation/runs/${RUN}/marks` && request.method() === "POST") {
      harness.marks.push(request.postDataJSON() as SeenMark);
      const markId = `m${nextMark}`;
      nextMark += 1;
      await route.fulfill({ status: 201, json: { mark_id: markId } });
      return;
    }
    if (path.startsWith(`/simulation/runs/${RUN}/marks/`) && request.method() === "DELETE") {
      harness.deletes.push(path.split("/").at(-1) ?? "");
      await route.fulfill({ status: 204 });
      return;
    }
    await route.fulfill({
      status: 404,
      json: { error: { code: "test.unhandled", message: `${path} is not fixtured` } },
    });
  });
  return harness;
}

async function openHarness(page: Page): Promise<void> {
  await page.goto("/?fixture=simulation-chart");
  await expect(page.getByTestId("simulation-chart")).toBeVisible();
}

async function plotPoint(
  page: Page,
  bar: number,
  count = 8,
): Promise<{ x: number; y: number }> {
  const svg = page.getByTestId("simulation-chart");
  const box = await svg.boundingBox();
  if (box === null) {
    throw new Error("the chart has no bounding box");
  }
  const left = box.x + (box.width * 64) / 960;
  const width = (box.width * 880) / 960;
  return {
    x: left + (width * (bar + 0.5)) / count,
    y: box.y + (box.height * 200) / 558,
  };
}

async function moveToBar(page: Page, bar: number): Promise<void> {
  const point = await plotPoint(page, bar);
  await page.mouse.move(point.x, point.y);
}

test("maps pointer position at both window edges", async ({ page }) => {
  await installSimulation(page);
  await openHarness(page);

  const svg = page.getByTestId("simulation-chart");
  const box = await svg.boundingBox();
  if (box === null) {
    throw new Error("the chart has no bounding box");
  }
  const left = box.x + (box.width * 64) / 960;
  const width = (box.width * 880) / 960;
  const y = box.y + (box.height * 200) / 558;

  await page.mouse.move(left + 0.5, y);
  await expect(page.getByTestId("bar-readout")).toHaveText("Bar 0");

  await page.mouse.move(left + width - 0.5, y);
  await expect(page.getByTestId("bar-readout")).toHaveText("Bar 7");

  await page.mouse.move(left - 2, y);
  await expect(page.getByTestId("bar-readout")).toHaveText("Bar —");

  await page.getByTestId("chart-application").focus();
  await page.keyboard.press("Home");
  await expect(page.getByTestId("bar-readout")).toHaveText("Bar 0");
  await page.keyboard.press("End");
  await expect(page.getByTestId("bar-readout")).toHaveText("Bar 7");
  await page.keyboard.press("ArrowLeft");
  await expect(page.getByTestId("bar-readout")).toHaveText("Bar 6");
});

test("captures a mark with a key, renders it, and undoes it", async ({ page }) => {
  const harness = await installSimulation(page);
  await openHarness(page);
  await moveToBar(page, 4);
  await expect(page.getByTestId("bar-readout")).toHaveText("Bar 4");

  await page.getByTestId("mark-note").fill("higher low into the level");
  const point = await plotPoint(page, 4);
  await page.mouse.click(point.x, point.y);
  await page.keyboard.press("1");

  await expect(page.getByTestId("mark-layer").locator("[data-mark-id]")).toHaveCount(1);
  await expect(page.locator('[data-mark-id="m1"]')).toHaveCount(1);
  await expect(page.getByTestId("mark-status")).toContainText("Entry · good · long");
  await expect(page.getByTestId("mark-note")).toHaveValue("");

  expect(harness.marks).toEqual([
    {
      chart: CHART,
      bar: 4,
      action: "entry",
      quality: "good",
      side: "long",
      note: "higher low into the level",
    },
  ]);

  await page.getByTestId("mark-note").fill("second mark");
  await page.getByTestId("mark-class-2").click();
  await expect(page.getByTestId("mark-layer").locator("[data-mark-id]")).toHaveCount(2);

  await page.keyboard.press("z");
  await expect(page.getByTestId("mark-layer").locator("[data-mark-id]")).toHaveCount(1);
  expect(harness.deletes).toEqual(["m2"]);
  await expect(page.locator('[data-mark-id="m1"]')).toHaveCount(1);
});

test("reaches all eight mark classes from the keyboard", async ({ page }) => {
  const harness = await installSimulation(page);
  await openHarness(page);
  await moveToBar(page, 2);

  const keys = ["1", "2", "3", "4", "5", "6", "7", "8"];
  const classes: string[] = [];
  for (let index = 0; index < keys.length; index += 1) {
    await page.keyboard.press(keys[index] ?? "");
    await expect(page.getByTestId("mark-layer").locator("[data-mark-id]")).toHaveCount(
      index + 1,
    );
  }
  for (const marker of await page.getByTestId("mark-layer").locator("[data-mark-id]").all()) {
    classes.push((await marker.getAttribute("data-mark-class")) ?? "");
  }
  expect(new Set(classes).size).toBe(8);
  expect(harness.marks.map((mark) => `${mark.action}-${mark.quality}-${mark.side}`)).toEqual(
    [...classes],
  );
});

test("a rejected mark keeps the note and shows the error", async ({ page }) => {
  await page.route("**/api/v1/**", async (route) => {
    const path = new URL(route.request().url()).pathname.replace("/api/v1", "");
    if (path === `/simulation/runs/${RUN}/windows/${CHART}/series`) {
      await route.fulfill({ json: SERIES });
      return;
    }
    if (path === `/simulation/runs/${RUN}/marks`) {
      await route.fulfill({
        status: 400,
        json: {
          error: {
            code: "simulation.mark_invalid",
            message: "bar outside the window",
          },
        },
      });
      return;
    }
    await route.fulfill({ status: 404, json: {} });
  });

  await openHarness(page);
  await moveToBar(page, 3);
  await page.getByTestId("mark-note").fill("this note must survive");
  const point = await plotPoint(page, 3);
  await page.mouse.click(point.x, point.y);
  await page.keyboard.press("3");

  await expect(page.getByTestId("mark-error")).toContainText("simulation.mark_invalid");
  await expect(page.getByTestId("mark-note")).toHaveValue("this note must survive");
  await expect(page.getByTestId("mark-layer").locator("[data-mark-id]")).toHaveCount(0);
  await expect(page.getByTestId("bar-readout")).toHaveText("Bar 3");
  await expect(page.getByTestId("mark-class-3")).toHaveAttribute("aria-pressed", "true");
});

test("renders identical path data after a reload", async ({ page }) => {
  await installSimulation(page);
  await openHarness(page);

  const candles = page.getByTestId("simulation-chart").locator("path.candle");
  const before = await candles.evaluateAll((paths) => paths.map((path) => path.getAttribute("d")));
  expect(before.length).toBeGreaterThan(0);

  await page.reload();
  await expect(page.getByTestId("simulation-chart")).toBeVisible();
  const after = await candles.evaluateAll((paths) => paths.map((path) => path.getAttribute("d")));
  expect(after).toEqual(before);
});
