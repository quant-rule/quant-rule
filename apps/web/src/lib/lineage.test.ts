import { afterEach, describe, expect, it } from "bun:test";

import { ApiError } from "./api";
import { fetchRunLineage } from "./lineage";

const original = globalThis.fetch;
const RUN = "sha256:" + "b".repeat(64);

afterEach(() => {
  globalThis.fetch = original;
});

function respond(status: number, body: unknown): void {
  globalThis.fetch = (async () =>
    new Response(JSON.stringify(body), {
      status,
      headers: { "content-type": "application/json" },
    })) as unknown as typeof fetch;
}

describe("fetchRunLineage", () => {
  it("reads the recorded reference set", async () => {
    const lineage = {
      run_id: RUN,
      references: [],
      recorded_at: "2026-11-22T00:05:00Z",
    };
    respond(200, lineage);
    await expect(fetchRunLineage(RUN)).resolves.toEqual(lineage);
  });

  it("turns a recorded absence into null", async () => {
    respond(404, {
      v: "quantrule.api/v1",
      error: {
        code: "run.lineage_not_found",
        message: `run ${RUN} has no recorded lineage`,
      },
    });
    await expect(fetchRunLineage(RUN)).resolves.toBeNull();
  });

  it("keeps every other failure visible", async () => {
    respond(404, {
      v: "quantrule.api/v1",
      error: { code: "run.not_found", message: "run is unknown" },
    });
    await expect(fetchRunLineage(RUN)).rejects.toBeInstanceOf(ApiError);
  });
});
