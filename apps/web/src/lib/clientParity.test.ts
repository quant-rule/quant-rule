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

import { DEFAULT_PROFILE, call, callVoid, currentProfile, setProfile, uploadDataset } from "./api";
import { downloadArtifact, sha256Hex } from "./artifacts";
import { COMMANDS, SECTIONS, missingCommands } from "./capabilities";
import { inspectCandles, inspectEvents, inspectTimeframes, verifyRun } from "./inspect";
import {
  downloadStageArtifact,
  explainDecision,
  fetchStageArtifacts,
  stageDecisions,
  stageEventView,
  stagePath,
  type StageFetchView,
} from "./stage";
import {
  compareRun,
  createRun,
  decideRun,
  fetchBaseline,
  fetchRun,
  listBaselines,
  listInvalidations,
  listRuns,
  submitRun,
  type BaselineRecord,
  type ImprovementComparison,
  type ImprovementRunDetail,
  type ImprovementRunSummary,
  type InvalidationRecord,
} from "./research";
import {
  createRun as createSimulation,
  fetchMarks,
  listRuns as listSimulations,
  postCheck,
  postDecision,
  postMark,
  postOracle,
  selectWindows,
} from "./simulation";
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
  SubmitRunResponse,
  StageEventView,
  StageRunDetail,
  StageRunSummary,
  StageSubmitResponse,
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
  stage_submit: StageSubmitResponse;
  stage_list: StageRunSummary[];
  stage: StageRunDetail;
  stage_cancelled: StageRunDetail;
  stage_events: StageEventView;
  stage_fetch: StageFetchView;
  stage_delete: { stage_run_id: string };
  stage_run_strategy: StageSubmitResponse;
  strategy_stage: StageRunDetail;
  strategy_explain: Record<string, unknown>;
  stage_run_portfolio: StageSubmitResponse;
  portfolio_stage: StageRunDetail;
  research_create: ImprovementRunDetail;
  research_submit: ImprovementRunDetail;
  research_list: ImprovementRunSummary[];
  research: ImprovementRunDetail;
  research_compare: ImprovementComparison;
  research_decision: ImprovementRunDetail;
  baselines: BaselineRecord[];
  baseline: BaselineRecord;
  invalidations: InvalidationRecord[];
}

interface ApiFixture {
  profile: string;
  dataset_id: string;
  config_id: string;
  run_id: string;
  timeframe: string;
  stage_id: string;
  stage_timeframe: string;
  strategy_stage_id: string;
  strategy_model_artifact_id: string;
  portfolio_stage_id: string;
  portfolio_decisions_artifact_id: string;
  research_id: string;
  commands: string[];
  artifact: {
    name: string;
    body: string;
    bytes: number;
    sha256: string;
  };
  stage_artifacts: {
    name: string;
    body: string;
    bytes: number;
    sha256: string;
  }[];
  strategy_artifacts: {
    name: string;
    body: string;
    bytes: number;
    sha256: string;
  }[];
  portfolio_artifacts: {
    name: string;
    body: string;
    bytes: number;
    sha256: string;
  }[];
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
    const method = init?.method ?? "GET";
    const headers = new Headers(init?.headers);
    seen.push({ path, profile: headers.get("X-QuantRule-Profile") });

    if (path === "/stages/model/runs" && method === "POST") {
      return respond(fixture.responses.stage_submit, 202);
    }
    if (path === "/stages/strategy/runs" && method === "POST") {
      return respond(fixture.responses.stage_run_strategy, 202);
    }
    if (path === "/stages/portfolio/runs" && method === "POST") {
      return respond(fixture.responses.stage_run_portfolio, 202);
    }
    if (
      path === `/stages/runs/${fixture.portfolio_stage_id}` &&
      method === "GET"
    ) {
      return respond(fixture.responses.portfolio_stage);
    }
    if (
      path === `/stages/runs/${fixture.strategy_stage_id}` &&
      method === "GET"
    ) {
      return respond(fixture.responses.strategy_stage);
    }
    const strategyArtifact = fixture.strategy_artifacts.find(
      (artifact) =>
        path ===
        `/stages/runs/${fixture.strategy_stage_id}/artifacts/${artifact.name}`,
    );
    if (strategyArtifact !== undefined && method === "GET") {
      return new Response(new TextEncoder().encode(strategyArtifact.body), {
        status: 200,
        headers: { "content-type": "application/octet-stream" },
      });
    }
    if (path === "/stages/runs" && method === "GET") {
      return respond(fixture.responses.stage_list);
    }
    if (path === `/stages/runs/${fixture.stage_id}` && method === "GET") {
      return respond(fixture.responses.stage);
    }
    if (path === `/stages/runs/${fixture.stage_id}/cancel` && method === "POST") {
      return respond(fixture.responses.stage_cancelled);
    }
    if (path === `/stages/runs/${fixture.stage_id}` && method === "DELETE") {
      return new Response(null, { status: 204 });
    }
    const stageArtifact = fixture.stage_artifacts.find(
      (artifact) =>
        path ===
        `/stages/runs/${fixture.stage_id}/artifacts/${artifact.name}`,
    );
    if (stageArtifact !== undefined && method === "GET") {
      return new Response(new TextEncoder().encode(stageArtifact.body), {
        status: 200,
        headers: { "content-type": "application/octet-stream" },
      });
    }
    if (path === "/research/runs" && method === "GET") {
      return respond(fixture.responses.research_list);
    }
    if (path === "/research/runs" && method === "POST") {
      return respond(fixture.responses.research_create, 201);
    }
    if (path === `/research/runs/${fixture.research_id}/submit` && method === "POST") {
      return respond(fixture.responses.research_submit, 202);
    }
    if (path === `/research/runs/${fixture.research_id}` && method === "GET") {
      return respond(fixture.responses.research);
    }
    if (path === `/research/runs/${fixture.research_id}/compare` && method === "GET") {
      return respond(fixture.responses.research_compare);
    }
    if (path === `/research/runs/${fixture.research_id}/decision` && method === "POST") {
      return respond(fixture.responses.research_decision);
    }
    if (path === "/research/baselines" && method === "GET") {
      return respond(fixture.responses.baselines);
    }
    if (path === "/research/baselines/model" && method === "GET") {
      return respond(fixture.responses.baseline);
    }
    if (path === "/research/invalidations" && method === "GET") {
      return respond(fixture.responses.invalidations);
    }
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
    if (path === "/catalog/datasets" && method === "POST") {
      return respond(fixture.responses.dataset, 201);
    }
    if (path === "/runs" && method === "POST") {
      return respond(
        { run_id: fixture.run_id, status: "queued", created: true },
        202,
      );
    }
    if (path === `/runs/${fixture.run_id}/cancel` && method === "POST") {
      return respond({ ...fixture.responses.run, status: "cancelled" }, 202);
    }
    if (path === "/profiles" && method === "POST") {
      return respond(
        { profile_id: "parity", name: "Parity", datasets: 0, runs: 0, simulations: 0 },
        201,
      );
    }
    if (path === "/simulation/runs" && method === "GET") {
      return respond([
        {
          sim_id: "sim-1",
          premise: "the parity premise",
          status: "draft",
          windows: 1,
          marks: 0,
          bundle: fixture.artifact.sha256,
          marked_windows: 0,
        },
      ]);
    }
    if (path === "/simulation/runs" && method === "POST") {
      return respond({ sim_id: "sim-3" }, 201);
    }
    if (path === "/simulation/runs/sim-1/windows" && method === "POST") {
      return respond({
        sim_id: "sim-1",
        windows: [{ index: 0, start: 0, end: 120 }],
      });
    }
    if (path === "/simulation/runs/sim-1/marks" && method === "POST") {
      return respond({ mark_id: "mark-1" }, 201);
    }
    if (path === "/simulation/runs/sim-1/marks" && method === "GET") {
      return respond({
        sim_id: "sim-1",
        marks: [
          {
            mark_id: "mark-1",
            chart: 0,
            bar: 12,
            marked_time_utc: "2026-11-14T10:00:00Z",
            action: "entry",
            quality: "good",
            side: "long",
            origin: "human",
            note: "",
          },
        ],
      });
    }
    if (path === "/simulation/runs/sim-1/oracle" && method === "POST") {
      return respond({
        sim_id: "sim-1",
        marks: [
          {
            mark_id: "oracle-1",
            chart: 0,
            bar: 80,
            marked_time_utc: "2026-11-14T10:00:00Z",
            action: "entry",
            quality: "good",
            side: "long",
            origin: "oracle",
            note: "",
          },
        ],
      });
    }
    if (path === "/simulation/runs/sim-1/check" && method === "POST") {
      return respond({
        events: 31,
        mean_net: 0.01,
        hit_rate: 0.6,
        control: 60,
        control_mean_net: 0,
        control_hit_rate: 0.5,
        positive_half_years: 3,
        passed: true,
        failures: [],
        verdict: "pass",
      });
    }
    if (path === "/simulation/runs/sim-1/decision" && method === "POST") {
      return respond({ decision: "test", note: "the parity note" });
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

  const stageSubmit = await call<StageSubmitResponse>("/stages/model/runs", {
    method: "POST",
    body: JSON.stringify({
      kind: "model",
      config_id: fixture.config_id,
      input: { kind: "data_version", data_version: fixture.dataset_id },
    }),
  });
  const stageList = await call<StageRunSummary[]>("/stages/runs?limit=100");
  const stage = await call<StageRunDetail>(stagePath(fixture.stage_id));
  const eventsDetail = await call<StageRunDetail>(stagePath(fixture.stage_id));
  const eventsFile = eventsDetail.artifact?.files.find(
    (file) => file.name === "events.csv",
  );
  const reportFile = eventsDetail.artifact?.files.find(
    (file) => file.name === "report.json",
  );
  if (eventsFile === undefined) {
    throw new Error("the fixture stage carries no events.csv");
  }
  const eventBytes = await downloadStageArtifact(fixture.stage_id, eventsFile);
  const reportBytes =
    reportFile === undefined
      ? null
      : await downloadStageArtifact(fixture.stage_id, reportFile);
  const stageEvents = stageEventView(
    fixture.stage_id,
    fixture.stage_timeframe,
    new TextDecoder().decode(eventBytes),
    reportBytes === null ? null : new TextDecoder().decode(reportBytes),
  );
  const fetchDetail = await call<StageRunDetail>(stagePath(fixture.stage_id));
  const stageFetch = await fetchStageArtifacts(fetchDetail);
  const stageCancel = await call<StageRunDetail>(
    `${stagePath(fixture.stage_id)}/cancel`,
    { method: "POST" },
  );
  await callVoid(stagePath(fixture.stage_id), { method: "DELETE" });
  const stageDelete = { stage_run_id: fixture.stage_id };

  const strategySubmit = await call<StageSubmitResponse>("/stages/strategy/runs", {
    method: "POST",
    body: JSON.stringify({
      kind: "strategy",
      config_id: fixture.config_id,
      input: {
        kind: "model_artifact",
        model_artifact_id: fixture.strategy_model_artifact_id,
      },
    }),
  });
  const strategyStage = await call<StageRunDetail>(
    stagePath(fixture.strategy_stage_id),
  );
  const explainDetail = await call<StageRunDetail>(
    stagePath(fixture.strategy_stage_id),
  );
  const decisionsFile = explainDetail.artifact?.files.find(
    (file) => file.name === "decisions.ndjson",
  );
  if (decisionsFile === undefined) {
    throw new Error("the strategy fixture carries no decisions.ndjson");
  }
  const decisionBytes = await downloadStageArtifact(
    fixture.strategy_stage_id,
    decisionsFile,
  );
  const strategyExplain = explainDecision(
    stageDecisions(new TextDecoder().decode(decisionBytes)),
    1,
  );
  const portfolioSubmit = await call<StageSubmitResponse>(
    "/stages/portfolio/runs",
    {
      method: "POST",
      body: JSON.stringify({
        kind: "portfolio",
        config_id: fixture.config_id,
        input: {
          kind: "decisions_artifact",
          decisions_artifact_id: fixture.portfolio_decisions_artifact_id,
        },
      }),
    },
  );
  const portfolioStage = await call<StageRunDetail>(
    stagePath(fixture.portfolio_stage_id),
  );

  const researchCreate = await createRun({
    tier: "model",
    hypothesis: "the fixture hypothesis",
    partition: "model_dev",
    data_version: fixture.dataset_id,
    config_id: fixture.config_id,
    criteria_id: null,
    green_criteria: { return_target: 0.1 },
    baseline: null,
    code_commit: "c236539f",
    seed: 0,
    analytics: true,
  });
  const researchSubmit = await submitRun(fixture.research_id);
  const researchList = await listRuns();
  const research = await fetchRun(fixture.research_id);
  const researchCompare = await compareRun(
    fixture.research_id,
    "MODEL-2026-11-04-04..05",
  );
  const researchDecision = await decideRun(
    fixture.research_id,
    "accepted",
    "the fixture decision",
  );
  const baselines = await listBaselines();
  const baseline = await fetchBaseline("model");
  const invalidations = await listInvalidations();

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
    stage_submit: stageSubmit,
    stage_list: stageList,
    stage,
    stage_events: stageEvents,
    stage_fetch: stageFetch,
    stage_cancel: stageCancel,
    stage_delete: stageDelete,
    strategy_stage_submit: strategySubmit,
    strategy_stage: strategyStage,
    strategy_explain: strategyExplain,
    portfolio_stage_submit: portfolioSubmit,
    portfolio_stage: portfolioStage,
    research_create: researchCreate,
    research_submit: researchSubmit,
    research_list: researchList,
    research: research,
    research_compare: researchCompare,
    research_decision: researchDecision,
    baselines,
    baseline,
    invalidations,
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
    expect(seen).toHaveLength(38);
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
      "/stages/model/runs",
      "/stages/runs",
      `/stages/runs/${API.stage_id}`,
      `/stages/runs/${API.stage_id}`,
      `/stages/runs/${API.stage_id}/artifacts/events.csv`,
      `/stages/runs/${API.stage_id}/artifacts/report.json`,
      `/stages/runs/${API.stage_id}`,
      `/stages/runs/${API.stage_id}/artifacts/events.csv`,
      `/stages/runs/${API.stage_id}/artifacts/report.json`,
      `/stages/runs/${API.stage_id}/cancel`,
      `/stages/runs/${API.stage_id}`,
      "/stages/strategy/runs",
      `/stages/runs/${API.strategy_stage_id}`,
      `/stages/runs/${API.strategy_stage_id}`,
      `/stages/runs/${API.strategy_stage_id}/artifacts/decisions.ndjson`,
      "/stages/portfolio/runs",
      `/stages/runs/${API.portfolio_stage_id}`,
      "/research/runs",
      `/research/runs/${API.research_id}/submit`,
      "/research/runs",
      `/research/runs/${API.research_id}`,
      `/research/runs/${API.research_id}/compare`,
      `/research/runs/${API.research_id}/decision`,
      "/research/baselines",
      "/research/baselines/model",
      "/research/invalidations",
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

  it("exercises every command at the browser boundary", async () => {
    const seen = install(API);
    const upload = new File(["1704067200,1,2,0.5,1.5,10\n"], "upload.csv");
    await uploadDataset(upload, {});
    await call<SubmitRunResponse>("/runs", {
      method: "POST",
      body: JSON.stringify({ data_version: API.dataset_id, config_id: API.config_id }),
    });
    await call<RunDetail>(`/runs/${API.run_id}/cancel`, { method: "POST" });
    await call<ProfileSummary>("/profiles", {
      method: "POST",
      body: JSON.stringify({ profile_id: "parity", name: "Parity" }),
    });
    setProfile("parity");
    await listSimulations();
    await createSimulation({
      premise: "the parity premise",
      bundle: API.artifact.sha256,
      inputs: "the parity inputs",
      seed: 7,
      criteria: "the parity criteria",
    });
    await selectWindows("sim-1");
    await postMark("sim-1", {
      chart: 0,
      bar: 12,
      action: "entry",
      quality: "good",
      side: "long",
      note: "the parity mark",
    });
    await fetchMarks("sim-1", "human");
    await postOracle("sim-1");
    await postCheck("sim-1");
    await postDecision("sim-1", "test", "the parity note");

    expect(currentProfile()).toBe("parity");
    expect(COMMANDS.length).toBe(46);
    expect(seen.map((request) => request.path)).toEqual(
      expect.arrayContaining([
        "/catalog/datasets",
        "/runs",
        `/runs/${API.run_id}/cancel`,
        "/profiles",
        "/simulation/runs",
        "/simulation/runs/sim-1/windows",
        "/simulation/runs/sim-1/marks",
        "/simulation/runs/sim-1/oracle",
        "/simulation/runs/sim-1/check",
        "/simulation/runs/sim-1/decision",
      ]),
    );
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
