import { afterEach, describe, expect, it } from "bun:test";
import { datasetPath, datasetRecord } from "./datasets";
import type { DatasetRecord } from "./types";

const original = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = original;
});

describe("datasetPath", () => {
  it("encodes the content version", () => {
    expect(datasetPath("sha256:ab/cd")).toBe("/catalog/datasets/sha256%3Aab%2Fcd");
  });
});

describe("datasetRecord", () => {
  it("reads one record through the shared error envelope", async () => {
    const seen: string[] = [];
    const record: DatasetRecord = {
      summary: {
        data_version: "sha256:" + "a".repeat(64),
        name: "candles.csv",
        candles: 10,
        first_bar: 0,
        last_bar: 9,
        bytes: 100,
      },
      object_key: "datasets/" + "a".repeat(64),
      sha256: "sha256:" + "a".repeat(64),
      quality: null,
      created_at: "2026-11-16T00:00:00Z",
    };
    globalThis.fetch = (async (input: Parameters<typeof fetch>[0]) => {
      seen.push(String(input));
      return new Response(JSON.stringify(record), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    }) as unknown as typeof fetch;

    await expect(datasetRecord(record.summary.data_version)).resolves.toEqual(record);
    expect(seen).toEqual([`/api/v1${datasetPath(record.summary.data_version)}`]);
  });
});
