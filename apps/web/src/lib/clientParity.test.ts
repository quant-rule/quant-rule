/**
 * Cross-client parity: the browser reads one fixture and builds the same view.
 *
 * The shell test (`crates/quantrule-shell/tests/client_parity.rs`) serves the
 * same `tests/fixtures/client-parity/api.json` and builds the same
 * `view.json`. The transport is stubbed at the `fetch` boundary, so no gateway
 * runs and no request leaves the process.
 */
import { afterEach, describe, expect, it } from "bun:test";
import { readFileSync } from "node:fs";

import { DEFAULT_PROFILE, call, currentProfile, setProfile } from "./api";
import type {
  ConfigSummary,
  DatasetRecord,
  DatasetSummary,
  HealthReport,
  ProfileSummary,
  RunDetail,
  RunSummary,
} from "./types";

interface ParityResponses {
  health: HealthReport;
  datasets: DatasetSummary[];
  dataset: DatasetRecord;
  configs: ConfigSummary[];
  runs: RunSummary[];
  run: RunDetail;
  profiles: ProfileSummary[];
}

interface ApiFixture {
  profile: string;
  dataset_id: string;
  config_id: string;
  run_id: string;
  responses: ParityResponses;
}

type View = Record<string, unknown>;

const FIXTURE_URL = new URL("../../../../tests/fixtures/client-parity/api.json", import.meta.url);
const VIEW_URL = new URL("../../../../tests/fixtures/client-parity/view.json", import.meta.url);

const API = JSON.parse(readFileSync(FIXTURE_URL, "utf8")) as ApiFixture;
const EXPECTED = JSON.parse(readFileSync(VIEW_URL, "utf8")) as View;

const original = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = original;
  setProfile(DEFAULT_PROFILE);
});

interface Seen {
  path: string;
  profile: string | null;
}

function respond(value: unknown, status = 200): Response {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json" },
  });
}

/** Serve the shared responses at the `fetch` boundary and record the profile. */
function install(fixture: ApiFixture): Seen[] {
  const seen: Seen[] = [];
  globalThis.fetch = (async (input: Parameters<typeof fetch>[0], init?: RequestInit) => {
    const url = String(input);
    const path = decodeURIComponent(url.split("?")[0]!.replace(/^\/api\/v1/, ""));
    const headers = new Headers(init?.headers);
    seen.push({ path, profile: headers.get("X-QuantRule-Profile") });

    if (path === "/health") {
      return respond(fixture.responses.health);
    }
    if (path === "/catalog/datasets") {
      return respond(fixture.responses.datasets);
    }
    if (path === `/catalog/datasets/${fixture.dataset_id}`) {
      return respond(fixture.responses.dataset);
    }
    if (path === "/catalog/configs") {
      return respond(fixture.responses.configs);
    }
    if (path === "/runs") {
      return respond(fixture.responses.runs);
    }
    if (path === `/runs/${fixture.run_id}`) {
      return respond(fixture.responses.run);
    }
    if (path === "/profiles") {
      return respond(fixture.responses.profiles);
    }
    return respond(
      { v: "quantrule.api/v1", error: { code: "test.unhandled", message: path } },
      404,
    );
  }) as unknown as typeof fetch;
  return seen;
}

/** The normalized view the browser presents for the shared fixture. */
async function browserView(fixture: ApiFixture): Promise<View> {
  setProfile(fixture.profile);

  const health = await call<HealthReport>("/health");
  const datasets = await call<DatasetSummary[]>("/catalog/datasets");
  const dataset = await call<DatasetRecord>(`/catalog/datasets/${fixture.dataset_id}`);
  const configs = await call<ConfigSummary[]>("/catalog/configs");
  const runs = await call<RunSummary[]>("/runs");
  const run = await call<RunDetail>(`/runs/${fixture.run_id}`);
  const profiles = await call<ProfileSummary[]>("/profiles");
  const config = configs.find((item) => item.config_id === fixture.config_id);
  if (config === undefined) {
    throw new Error(`the fixture carries no config ${fixture.config_id}`);
  }

  return {
    profile: currentProfile(),
    health,
    datasets,
    dataset,
    configs,
    config,
    runs,
    run,
    profiles,
  };
}

/** Canonicalize object key order so equality compares values, not insertion. */
function canonical(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map(canonical);
  }
  if (value !== null && typeof value === "object") {
    const entries = Object.entries(value as Record<string, unknown>)
      .sort(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0))
      .map(([key, item]) => [key, canonical(item)]);
    return Object.fromEntries(entries);
  }
  return value;
}

function difference(left: unknown, right: unknown): string | null {
  const client = JSON.stringify(canonical(left), null, 2);
  const expected = JSON.stringify(canonical(right), null, 2);
  if (client === expected) {
    return null;
  }
  return `the client views differ\nbrowser:\n${client}\nexpected:\n${expected}`;
}

describe("client parity", () => {
  it("the browser view matches the shared client parity view", async () => {
    const seen = install(API);
    const view = await browserView(API);

    expect(difference(view, EXPECTED)).toBeNull();
    expect(seen).toHaveLength(7);
    expect(seen.map((request) => request.path)).toEqual([
      "/health",
      "/catalog/datasets",
      `/catalog/datasets/${API.dataset_id}`,
      "/catalog/configs",
      "/runs",
      `/runs/${API.run_id}`,
      "/profiles",
    ]);
    for (const request of seen) {
      expect(request.profile).toBe(API.profile);
    }
  });

  it("a seeded fixture divergence fails the comparison", async () => {
    const seeded: ApiFixture = {
      ...API,
      responses: {
        ...API.responses,
        health: { ...API.responses.health, queue_depth: 3 },
      },
    };
    install(seeded);
    const view = await browserView(seeded);

    const divergence = difference(view, EXPECTED);
    expect(divergence).not.toBeNull();
    expect(divergence).toContain('"queue_depth": 3');
  });
});
