import { afterEach, describe, expect, it } from "bun:test";

import { archive, canArchive, erase, HISTORY_KINDS, list } from "./history";

const originalFetch = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = originalFetch;
});

function respond(body: unknown): void {
  globalThis.fetch = (async () =>
    new Response(JSON.stringify(body), {
      status: 200,
      headers: { "content-type": "application/json" },
    })) as unknown as typeof fetch;
}

describe("history kinds", () => {
  it("lists every runtime kind and refuses stage archive", () => {
    expect(HISTORY_KINDS.map((kind) => kind.id)).toEqual([
      "run",
      "stage",
      "research",
      "simulation",
      "dataset",
    ]);
    expect(canArchive("stage")).toBe(false);
    expect(canArchive("run")).toBe(true);
    expect(canArchive("dataset")).toBe(true);
  });
});

describe("history list", () => {
  it("normalizes a run and carries its archived state", async () => {
    respond([
      {
        run_id: "sha256:" + "a".repeat(64),
        status: "failed",
        config_id: "cfg",
        data_version: "sha256:" + "b".repeat(64),
        submitted_at: "2026-11-20T00:00:00Z",
        elapsed_ms: null,
        archived_at: "2026-11-20T01:00:00Z",
      },
    ]);
    const rows = await list("run", true);
    expect(rows).toEqual([
      {
        kind: "run",
        id: "sha256:" + "a".repeat(64),
        state: "failed",
        timestamp: "2026-11-20T00:00:00Z",
        archivedAt: "2026-11-20T01:00:00Z",
      },
    ]);
  });

  it("reads the dataset history filter", async () => {
    let requested = "";
    globalThis.fetch = (async (input: RequestInfo | URL) => {
      requested = String(input);
      return new Response(
        JSON.stringify([
          {
            data_version: "sha256:" + "c".repeat(64),
            name: "candles.csv",
            candles: 10,
            first_bar: 1,
            last_bar: 10,
            bytes: 128,
            archived_at: "2026-11-20T00:00:00Z",
          },
        ]),
        { status: 200, headers: { "content-type": "application/json" } },
      );
    }) as unknown as typeof fetch;
    const rows = await list("dataset", true);
    expect(requested).toContain("/api/v1/catalog/datasets?include_archived=true");
    expect(rows[0]?.bytes).toBe(128);
    expect(rows[0]?.archivedAt).toBe("2026-11-20T00:00:00Z");
  });
});

describe("history actions", () => {
  it("archives through the kind path", async () => {
    let requested = "";
    globalThis.fetch = (async (input: RequestInfo | URL) => {
      requested = String(input);
      return new Response(JSON.stringify({ sim_id: "sim-1" }), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    }) as unknown as typeof fetch;
    await archive("simulation", "sim-1");
    expect(requested).toContain("/api/v1/simulation/runs/sim-1/archive");
  });

  it("refuses an erase when the confirmation differs", async () => {
    let called = false;
    globalThis.fetch = (async () => {
      called = true;
      return new Response("{}", { status: 200 });
    }) as unknown as typeof fetch;
    await expect(erase("run", "sha256:a", "sha256:b")).rejects.toThrow(
      "confirmation",
    );
    expect(called).toBe(false);
  });
});
