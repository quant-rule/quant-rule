import { afterEach, describe, expect, it } from "bun:test";

import { sha256Hex } from "./artifacts";
import {
  createSignal,
  downloadSignal,
  fetchSignal,
  listSignals,
  signalsPath,
  type ArtifactRecord,
} from "./signals";

const original = globalThis.fetch;
const SIGNAL_ID = "sha256:" + "c".repeat(64);

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

function record(bytes: Uint8Array, digest: string): ArtifactRecord {
  return {
    summary: {
      artifact_id: SIGNAL_ID,
      kind: "signal",
      bytes: bytes.byteLength,
      created_at: "2026-11-22T00:04:00Z",
    },
    object_key: "artifacts/" + "c".repeat(64),
    sha256: digest,
  };
}

const request = {
  model_artifact_id: "sha256:" + "9".repeat(64),
  data_version: "sha256:" + "d".repeat(64),
  config_id: "cfg_abc",
  strategy_pack_id: "sha256:" + "a".repeat(64),
};

describe("signalsPath", () => {
  it("adds the optional limit", () => {
    expect(signalsPath()).toBe("/signals");
    expect(signalsPath(5)).toBe("/signals?limit=5");
  });
});

describe("signal transport", () => {
  it("lists metadata and reads one record", async () => {
    const seen = install([]);
    await listSignals();
    expect(seen[0]?.url).toBe("/api/v1/signals");
    expect(seen[0]?.method).toBe("GET");

    const signal = record(new TextEncoder().encode("{}"), SIGNAL_ID);
    const read = install(signal);
    await expect(fetchSignal(SIGNAL_ID)).resolves.toEqual(signal);
    expect(read[0]?.url).toBe(`/api/v1/signals/${encodeURIComponent(SIGNAL_ID)}`);
  });

  it("posts exactly the four references", async () => {
    const seen = install({});
    await createSignal(request);
    expect(seen).toHaveLength(1);
    expect(seen[0]?.url).toBe("/api/v1/signals");
    expect(seen[0]?.method).toBe("POST");
    expect(JSON.parse(seen[0]?.body ?? "{}")).toEqual(request);
  });

  it("verifies a download against the record and refuses a substitute", async () => {
    const bytes = new TextEncoder().encode("{\"rows\":[]}\n");
    globalThis.fetch = (async () =>
      new Response(bytes, {
        status: 200,
        headers: { "content-type": "application/octet-stream" },
      })) as unknown as typeof fetch;
    const valid = record(bytes, await sha256Hex(bytes));
    await expect(downloadSignal(valid)).resolves.toBeInstanceOf(ArrayBuffer);

    const substituted = record(bytes, "sha256:" + "e".repeat(64));
    await expect(downloadSignal(substituted)).rejects.toThrow(
      "do not match the catalog",
    );
  });
});
