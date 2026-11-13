import { afterEach, describe, expect, it } from "bun:test";
import {
  ApiError,
  call,
  currentProfile,
  DEFAULT_PROFILE,
  setProfile,
  uploadDataset,
} from "./api";

const original = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = original;
  setProfile(DEFAULT_PROFILE);
});

interface Seen {
  url: string;
  init: RequestInit;
}

function capture(status: number, body: unknown): Seen[] {
  const seen: Seen[] = [];
  globalThis.fetch = (async (input: Parameters<typeof fetch>[0], init?: RequestInit) => {
    seen.push({ url: String(input), init: init ?? {} });
    return new Response(JSON.stringify(body), {
      status,
      headers: { "content-type": "application/json" },
    });
  }) as unknown as typeof fetch;
  return seen;
}

function file(): File {
  return new File(
    [
      "date,open,high,low,close,volume\n2024-01-01 00:00:00+00:00,100,101,99,100.5,10\n",
    ],
    "candles.csv",
    { type: "text/csv" },
  );
}

describe("call", () => {
  it("parses a successful response", async () => {
    capture(200, { product: "1.0.0" });
    const value = await call<{ product: string }>("/version");
    expect(value.product).toBe("1.0.0");
  });

  it("turns an error envelope into an ApiError", async () => {
    capture(404, { v: "quantrule.api/v1", error: { code: "run.not_found", message: "missing" } });
    await expect(call("/runs/x")).rejects.toBeInstanceOf(ApiError);
  });

  it("keeps the stable code", async () => {
    capture(409, { error: { code: "run.conflict", message: "already finished" } });
    try {
      await call("/runs/x/cancel");
      throw new Error("expected a failure");
    } catch (error) {
      expect((error as ApiError).code).toBe("run.conflict");
      expect((error as ApiError).status).toBe(409);
    }
  });

  it("sends the active profile", async () => {
    const seen = capture(200, {});
    await call("/version");
    const headers = new Headers(seen[0]?.init.headers);
    expect(headers.get("X-QuantRule-Profile")).toBe(DEFAULT_PROFILE);
    expect(currentProfile()).toBe(DEFAULT_PROFILE);

    setProfile("alpha");
    await call("/version");
    const switched = new Headers(seen[1]?.init.headers);
    expect(switched.get("X-QuantRule-Profile")).toBe("alpha");
  });
});

describe("uploadDataset", () => {
  it("sends the file as CSV and returns the record", async () => {
    const seen = capture(201, {
      summary: {
        data_version: "sha256:" + "a".repeat(64),
        name: "candles.csv",
        candles: 1,
        first_bar: 1_704_067_200,
        last_bar: 1_704_067_200,
        bytes: 64,
      },
      object_key: "datasets/aaa",
      sha256: "sha256:" + "a".repeat(64),
      created_at: "2026-11-13T00:00:00Z",
    });

    const upload = file();
    const record = await uploadDataset(upload);

    expect(seen[0]?.url).toBe("/api/v1/catalog/datasets?name=candles.csv");
    const headers = new Headers(seen[0]?.init.headers);
    expect(headers.get("content-type")).toBe("text/csv");
    expect(headers.get("X-QuantRule-Profile")).toBe(DEFAULT_PROFILE);
    expect(seen[0]?.init.body).toBe(upload);
    expect(record.summary.candles).toBe(1);
  });

  it("keeps the stable code on a rejected upload", async () => {
    capture(400, {
      error: { code: "dataset.invalid", message: "line 2: high is below low" },
    });
    try {
      await uploadDataset(file());
      throw new Error("expected a failure");
    } catch (error) {
      expect((error as ApiError).code).toBe("dataset.invalid");
      expect((error as ApiError).status).toBe(400);
    }
  });
});
