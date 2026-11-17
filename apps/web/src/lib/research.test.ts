import { afterEach, describe, expect, it } from "bun:test";

import {
  baselinePath,
  baselinesPath,
  canCreate,
  comparePath,
  compareRun,
  createRun,
  decideRun,
  invalidationsPath,
  runsPath,
  submitRun,
  type CreateImprovement,
} from "./research";

const original = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = original;
});

interface Seen {
  url: string;
  method: string;
  body: string | null;
}

function install(response: unknown): Seen[] {
  const seen: Seen[] = [];
  globalThis.fetch = (async (input: Parameters<typeof fetch>[0], init?: RequestInit) => {
    seen.push({
      url: String(input),
      method: init?.method ?? "GET",
      body: typeof init?.body === "string" ? init.body : null,
    });
    return new Response(JSON.stringify(response), {
      status: 200,
      headers: { "content-type": "application/json" },
    });
  }) as unknown as typeof fetch;
  return seen;
}

function draft(): CreateImprovement {
  return {
    tier: "model",
    hypothesis: "the fixture hypothesis",
    partition: "model_dev",
    data_version: "sha256:" + "c".repeat(64),
    config_id: "cfg_abc",
    criteria_id: null,
    green_criteria: { return_target: 0.1 },
    baseline: null,
    code_commit: "c236539f",
    seed: 0,
    analytics: true,
  };
}

describe("research API paths", () => {
  it("builds filtered list, comparison, and registry paths", () => {
    expect(runsPath({ tier: "model", state: "created", limit: 5 })).toBe(
      "/research/runs?tier=model&state=created&limit=5",
    );
    expect(runsPath()).toBe("/research/runs");
    expect(comparePath("MODEL-2026-11-17-01", "MODEL-2026-11-04-04..05")).toBe(
      "/research/runs/MODEL-2026-11-17-01/compare?baseline=MODEL-2026-11-04-04..05",
    );
    expect(comparePath("MODEL-2026-11-17-01")).toBe(
      "/research/runs/MODEL-2026-11-17-01/compare",
    );
    expect(baselinesPath()).toBe("/research/baselines");
    expect(baselinePath("model")).toBe("/research/baselines/model");
    expect(invalidationsPath()).toBe("/research/invalidations");
  });

  it("requires the pre-registration fields before enabling create", () => {
    expect(canCreate(draft())).toBe(true);
    expect(canCreate({ ...draft(), hypothesis: " " })).toBe(false);
    expect(canCreate({ ...draft(), data_version: "" })).toBe(false);
    expect(canCreate({ ...draft(), config_id: "" })).toBe(false);
    expect(canCreate({ ...draft(), green_criteria: {} })).toBe(false);
    expect(canCreate({ ...draft(), seed: -1 })).toBe(false);
  });

  it("posts the create, submit, compare, and decision requests once each", async () => {
    const created = {
      improvement_id: "MODEL-2026-11-17-01",
      tier: "model",
      state: "created",
      decision: null,
      created_at: "2026-11-17T00:00:00Z",
      submitted_at: null,
      decided_at: null,
      decision_note: null,
      manifest: {
        schema: "quantrule.improvement-run/v3",
        improvement_id: "MODEL-2026-11-17-01",
        tier: "model",
        hypothesis: "the fixture hypothesis",
        created_utc: "2026-11-17T00:00:00Z",
        changed: "model",
        frozen: ["strategy", "portfolio"],
        partition: "model_dev",
        data_version: "sha256:" + "c".repeat(64),
        config_id: "cfg_abc",
        criteria_id: null,
        green_criteria: { return_target: 0.1 },
        holdout_access: "none",
        baseline: null,
        code_commit: "c236539f",
        seed: 0,
        command: "run submit",
        analytics: true,
      },
      pipeline_run_id: null,
      canonical_digest: null,
    };
    let seen = install(created);
    await createRun(draft());
    expect(seen).toHaveLength(1);
    expect(seen[0]!.url).toBe("/api/v1/research/runs");
    expect(seen[0]!.method).toBe("POST");
    expect(JSON.parse(seen[0]!.body!)).toMatchObject({
      tier: "model",
      partition: "model_dev",
      config_id: "cfg_abc",
    });

    seen = install({ ...created, state: "submitted" });
    await submitRun("MODEL-2026-11-17-01");
    expect(seen[0]!.url).toBe(
      "/api/v1/research/runs/MODEL-2026-11-17-01/submit",
    );
    expect(seen[0]!.method).toBe("POST");

    seen = install({
      improvement_id: "MODEL-2026-11-17-01",
      tier: "model",
      candidate_run_id: "sha256:" + "b".repeat(64),
      baseline_id: null,
      candidate_digest: null,
      baseline_digest: null,
      rows: [],
      note: null,
    });
    await compareRun("MODEL-2026-11-17-01", "MODEL-2026-11-04-04..05");
    expect(seen[0]!.url).toBe(
      "/api/v1/research/runs/MODEL-2026-11-17-01/compare?baseline=MODEL-2026-11-04-04..05",
    );

    seen = install({ ...created, state: "decided", decision: "rejected" });
    await decideRun("MODEL-2026-11-17-01", "rejected", "the fixture failed");
    expect(seen[0]!.url).toBe(
      "/api/v1/research/runs/MODEL-2026-11-17-01/decision",
    );
    expect(JSON.parse(seen[0]!.body!)).toEqual({
      decision: "rejected",
      note: "the fixture failed",
    });
  });
});
