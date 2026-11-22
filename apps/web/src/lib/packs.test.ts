import { afterEach, describe, expect, it } from "bun:test";

import { sha256Hex } from "./artifacts";
import {
  createPack,
  downloadPack,
  fetchPack,
  freezePack,
  listPacks,
  packsPath,
  retirePack,
  validatePack,
  type PackRecord,
} from "./packs";

const original = globalThis.fetch;
const PACK_ID = "sha256:" + "a".repeat(64);

afterEach(() => {
  globalThis.fetch = original;
});

interface Seen {
  url: string;
  method: string;
  body: unknown;
  contentType: string | null;
}

function install(response: unknown, status = 200): Seen[] {
  const seen: Seen[] = [];
  globalThis.fetch = (async (input: Parameters<typeof fetch>[0], init?: RequestInit) => {
    const headers = new Headers(init?.headers);
    seen.push({
      url: String(input),
      method: init?.method ?? "GET",
      body: init?.body,
      contentType: headers.get("content-type"),
    });
    return new Response(JSON.stringify(response), {
      status,
      headers: { "content-type": "application/json" },
    });
  }) as unknown as typeof fetch;
  return seen;
}

function record(bytes: Uint8Array, digest: string): PackRecord {
  return {
    summary: {
      pack_id: PACK_ID,
      kind: "strategy",
      name: "parity-pack",
      version: "1.0.0",
      owner: "operator",
      state: "frozen",
      bytes: bytes.byteLength,
      created_at: "2026-11-22T00:00:00Z",
    },
    object_key: "packs/" + "a".repeat(64),
    sha256: digest,
    validations: [],
    events: [],
  };
}

describe("packsPath", () => {
  it("renders the kind and state filters", () => {
    expect(packsPath()).toBe("/catalog/packs");
    expect(packsPath({ kind: "strategy", state: "frozen" })).toBe(
      "/catalog/packs?kind=strategy&state=frozen",
    );
  });
});

describe("pack transport", () => {
  it("lists and reads one record", async () => {
    const seen = install([]);
    await listPacks({ kind: "strategy", state: "frozen" });
    expect(seen[0]?.url).toBe("/api/v1/catalog/packs?kind=strategy&state=frozen");
    expect(seen[0]?.method).toBe("GET");

    const pack = record(new TextEncoder().encode("{}"), PACK_ID);
    const read = install(pack);
    await expect(fetchPack(PACK_ID)).resolves.toEqual(pack);
    expect(read[0]?.url).toBe(`/api/v1/catalog/packs/${encodeURIComponent(PACK_ID)}`);
  });

  it("sends the local file bytes to create", async () => {
    const seen = install({});
    const file = new File(["{\"schema\":\"quantrule.pack/v1\"}\n"], "pack.json", {
      type: "application/json",
    });
    await createPack(file);
    expect(seen[0]?.url).toBe("/api/v1/catalog/packs");
    expect(seen[0]?.method).toBe("POST");
    expect(seen[0]?.contentType).toBe("application/json");
    expect(seen[0]?.body).toBe(file);
  });

  it("uses one POST per lifecycle action", async () => {
    const seen = install({});
    await validatePack(PACK_ID);
    await freezePack(PACK_ID);
    await retirePack(PACK_ID);
    expect(seen.map((item) => [item.method, item.url])).toEqual([
      ["POST", `/api/v1/catalog/packs/${encodeURIComponent(PACK_ID)}/validate`],
      ["POST", `/api/v1/catalog/packs/${encodeURIComponent(PACK_ID)}/freeze`],
      ["POST", `/api/v1/catalog/packs/${encodeURIComponent(PACK_ID)}/retire`],
    ]);
  });

  it("verifies a download against the record and refuses a substitute", async () => {
    const bytes = new TextEncoder().encode("{\"kind\":\"strategy\"}\n");
    globalThis.fetch = (async () =>
      new Response(bytes, {
        status: 200,
        headers: { "content-type": "application/json" },
      })) as unknown as typeof fetch;
    const digest = await sha256Hex(bytes);
    const valid = record(bytes, digest);
    await expect(downloadPack(valid)).resolves.toBeInstanceOf(ArrayBuffer);

    const substituted = record(bytes, "sha256:" + "b".repeat(64));
    await expect(downloadPack(substituted)).rejects.toThrow(
      "do not match the catalog",
    );
  });
});
