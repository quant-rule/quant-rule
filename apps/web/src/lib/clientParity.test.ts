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
import { downloadArtifact, sha256Hex } from "./artifacts";
import { COMMANDS, SECTIONS, missingCommands } from "./capabilities";
import { inspectCandles, inspectEvents, inspectTimeframes, verifyRun } from "./inspect";
import type {
  ConfigSummary,
  DatasetRecord,
  DatasetSummary,
  HealthReport,
  InspectEvents,
  ProfileSummary,
  RunDetail,
  RunDigest,
  RunSummary,
  TimeframeInspection,
} from "./types";

interface ParityResponses {
  health: HealthReport;
  datasets: DatasetSummary[];
  dataset: DatasetRecord;
  configs: ConfigSummary[];
  runs: RunSummary[];
  run: RunDetail;
  candles: DatasetSummary;
  timeframes: TimeframeInspection;
  events: InspectEvents;
  digest: RunDigest;
  profiles: ProfileSummary[];
}

interface ApiFixture {
  profile: string;
  dataset_id: string;
  config_id: string;
  run_id: string;
  timeframe: string;
  commands: string[];
  artifact: {
    name: string;
    body: string;
    bytes: number;
    sha256: string;
  };
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
    if (path === `/runs/${fixture.run_id}/digest`) {
      return respond(fixture.responses.digest);
    }
    if (path === `/runs/${fixture.run_id}/artifacts/${fixture.artifact.name}`) {
      return new Response(new TextEncoder().encode(fixture.artifact.body), {
        status: 200,
        headers: { "content-type": "application/octet-stream" },
      });
    }
    if (path === "/inspect/candles") {
      return respond(fixture.responses.candles);
    }
    if (path === "/inspect/timeframes") {
      return respond(fixture.responses.timeframes);
    }
    if (path === "/inspect/events") {
      return respond(fixture.responses.events);
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

  const commands = COMMANDS.map((item) => item.command);
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

  const candles = await inspectCandles(fixture.dataset_id);
  const timeframes = await inspectTimeframes(fixture.dataset_id);
  const events = await inspectEvents(fixture.dataset_id, fixture.timeframe);
  const digest = await verifyRun(fixture.run_id);
  const reference = run.artifacts[0];
  if (reference === undefined) {
    throw new Error("the fixture run carries no artifact");
  }
  const bytes = await downloadArtifact(fixture.run_id, reference);
  const artifact = {
    name: reference.name,
    bytes: bytes.byteLength,
    sha256: await sha256Hex(bytes),
  };

  return {
    profile: currentProfile(),
    commands,
    health,
    datasets,
    dataset,
    configs,
    config,
    runs,
    run,
    candles,
    timeframes,
    events,
    digest,
    artifact,
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
    expect(seen).toHaveLength(12);
    expect(seen.map((request) => request.path)).toEqual([
      "/health",
      "/catalog/datasets",
      `/catalog/datasets/${API.dataset_id}`,
      "/catalog/configs",
      "/runs",
      `/runs/${API.run_id}`,
      "/profiles",
      "/inspect/candles",
      "/inspect/timeframes",
      "/inspect/events",
      `/runs/${API.run_id}/digest`,
      `/runs/${API.run_id}/artifacts/${API.artifact.name}`,
    ]);
    for (const request of seen) {
      expect(request.profile).toBe(API.profile);
    }
  });

  it("every shell command has exactly one browser control", () => {
    const commands = COMMANDS.map((item) => item.command);
    expect(commands).toEqual(API.commands);
    expect(missingCommands(commands, API.commands)).toEqual([]);
    expect(new Set(commands).size).toBe(commands.length);
  });

  it("every command names a screen that exists", () => {
    const sections = new Set(SECTIONS.map((section) => section.id));
    for (const command of COMMANDS) {
      expect(sections.has(command.section)).toBe(true);
    }
  });

  it("every control has a stable test id in its component", () => {
    for (const command of COMMANDS) {
      const source = readFileSync(new URL(`./${command.component}`, import.meta.url), "utf8");
      expect(source, `${command.command} → ${command.component}`).toContain(
        `data-testid="${command.testId}"`,
      );
    }
  });

  it("a missing browser control is reported as a command gap", () => {
    const browser = COMMANDS.map((item) => item.command).filter(
      (name) => name !== "verify",
    );
    expect(missingCommands(browser, API.commands)).toEqual(["verify"]);
  });

  it("the fixture artifact matches its recorded size and digest", async () => {
    const bytes = new TextEncoder().encode(API.artifact.body);
    expect(bytes.byteLength).toBe(API.artifact.bytes);
    expect(await sha256Hex(bytes)).toBe(API.artifact.sha256);
    expect(API.responses.run.artifacts[0]?.sha256).toBe(API.artifact.sha256);
    expect(API.responses.run.artifacts[0]?.bytes).toBe(API.artifact.bytes);
  });

  it("a substituted artifact fails the download check", async () => {
    const seeded: ApiFixture = {
      ...API,
      artifact: { ...API.artifact, body: "{\"ok\":false}\n" },
    };
    install(seeded);
    setProfile(seeded.profile);

    const run = await call<RunDetail>(`/runs/${seeded.run_id}`);
    const reference = run.artifacts[0]!;
    await expect(downloadArtifact(seeded.run_id, reference)).rejects.toThrow(
      "does not match the run record",
    );
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
