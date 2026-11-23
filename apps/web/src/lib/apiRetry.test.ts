import { afterEach, describe, expect, it } from "bun:test";
import { get } from "svelte/store";

import {
  ApiError,
  call,
  fetchBytes,
  setProfile,
  setToken,
  uploadDataset,
  DEFAULT_PROFILE,
} from "./api";
import {
  beginRetry,
  clearRetry,
  endRetry,
  finalMessage,
  resetTransport,
  retryDelayMs,
  retryMessage,
  retryableMethod,
  retryableStatus,
  transport,
} from "./transport";

const original = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = original;
  resetTransport();
  setProfile(DEFAULT_PROFILE);
  setToken(null);
});

type Step =
  | { status: number; body?: unknown }
  | { error: Error };

/** Serve one scripted step per call; the last step repeats. */
function scripted(steps: Step[]): { calls: number } {
  const counter = { calls: 0 };
  globalThis.fetch = (async () => {
    const step = steps[Math.min(counter.calls, steps.length - 1)]!;
    counter.calls += 1;
    if ("error" in step) {
      throw step.error;
    }
    return new Response(JSON.stringify(step.body ?? {}), {
      status: step.status,
      headers: { "content-type": "application/json" },
    });
  }) as unknown as typeof fetch;
  return counter;
}

describe("retry policy", () => {
  it("uses the fixed delay schedule", () => {
    expect(retryDelayMs(1)).toBe(250);
    expect(retryDelayMs(2)).toBe(1_000);
    expect(retryDelayMs(3)).toBeNull();
    expect(retryDelayMs(0)).toBeNull();
    expect(retryDelayMs(1.5)).toBeNull();
  });

  it("retries only the transient statuses", () => {
    for (const status of [408, 429, 502, 503, 504]) {
      expect(retryableStatus(status), String(status)).toBe(true);
    }
    for (const status of [200, 204, 400, 401, 403, 404, 409, 500, 501]) {
      expect(retryableStatus(status), String(status)).toBe(false);
    }
  });

  it("allows only GET", () => {
    expect(retryableMethod(undefined)).toBe(true);
    expect(retryableMethod("GET")).toBe(true);
    expect(retryableMethod("get")).toBe(true);
    for (const method of ["POST", "PUT", "PATCH", "DELETE"]) {
      expect(retryableMethod(method), method).toBe(false);
    }
  });
});

describe("retrying a read", () => {
  it("retries a transient GET twice and returns the successful value", async () => {
    const script = scripted([
      {
        status: 503,
        body: { error: { code: "service.unavailable", message: "down" } },
      },
      { error: new TypeError("network down") },
      { status: 200, body: { ok: true } },
    ]);

    const value = await call<{ ok: boolean }>("/health");

    expect(value).toEqual({ ok: true });
    expect(script.calls).toBe(3);
    expect(get(transport)).toEqual({ reconnecting: false, lastError: null });
  });

  it("retries a GET byte read", async () => {
    let calls = 0;
    globalThis.fetch = (async () => {
      calls += 1;
      if (calls === 1) {
        return new Response(
          JSON.stringify({ error: { code: "service.unavailable", message: "down" } }),
          { status: 503, headers: { "content-type": "application/json" } },
        );
      }
      return new Response(new TextEncoder().encode("artifact bytes"), {
        status: 200,
        headers: { "content-type": "application/octet-stream" },
      });
    }) as unknown as typeof fetch;

    const bytes = await fetchBytes("/runs/x/artifacts/y");

    expect(new TextDecoder().decode(bytes)).toBe("artifact bytes");
    expect(calls).toBe(2);
  });

  it("stops after three attempts and keeps a stable last message", async () => {
    const script = scripted([{ error: new TypeError("network down") }]);

    await expect(call("/health")).rejects.toThrow("network down");

    expect(script.calls).toBe(3);
    expect(get(transport)).toEqual({
      reconnecting: false,
      lastError: "The gateway did not answer after 3 attempts.",
    });
  });

  it("does not retry an invalid response body", async () => {
    let calls = 0;
    globalThis.fetch = (async () => {
      calls += 1;
      return new Response("not json", {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    }) as unknown as typeof fetch;

    await expect(call("/health")).rejects.toBeInstanceOf(SyntaxError);
    expect(calls).toBe(1);
  });
});

describe("single-attempt requests", () => {
  it("never retries a mutation", async () => {
    const mutations: [string, RequestInit][] = [
      ["/signals", { method: "POST", body: "{}" }],
      ["/catalog/packs", { method: "POST", body: "{}" }],
      ["/runs", { method: "POST", body: "{}" }],
      ["/runs/x/cancel", { method: "POST" }],
      ["/simulation/runs/sim-1/decision", { method: "POST", body: "{}" }],
      ["/runs/x/archive", { method: "POST" }],
    ];

    for (const [path, init] of mutations) {
      const script = scripted([{ error: new TypeError("network down") }]);
      await expect(call(path, init)).rejects.toThrow("network down");
      expect(script.calls, path).toBe(1);
    }
    expect(get(transport)).toEqual({ reconnecting: false, lastError: null });
  });

  it("never retries a dataset upload", async () => {
    const script = scripted([{ error: new TypeError("network down") }]);
    const file = new File(["1704067200,1,2,0.5,1.5,10\n"], "candles.csv");

    await expect(uploadDataset(file)).rejects.toThrow("network down");
    expect(script.calls).toBe(1);
    expect(get(transport)).toEqual({ reconnecting: false, lastError: null });
  });

  it("does not retry an aborted GET", async () => {
    let calls = 0;
    globalThis.fetch = (async () => {
      calls += 1;
      const error = new Error("aborted");
      error.name = "AbortError";
      throw error;
    }) as unknown as typeof fetch;

    await expect(call("/health")).rejects.toThrow("aborted");
    expect(calls).toBe(1);
    expect(get(transport).lastError).toBeNull();
  });

  it("never retries a terminal status", async () => {
    for (const status of [400, 401, 403, 404, 409, 500]) {
      const script = scripted([
        { status, body: { error: { code: "terminal", message: "no retry" } } },
      ]);
      await expect(call("/runs/x"), String(status)).rejects.toBeInstanceOf(ApiError);
      expect(script.calls, String(status)).toBe(1);
      expect(get(transport), String(status)).toEqual({
        reconnecting: false,
        lastError: null,
      });
    }
  });
});

describe("connection state", () => {
  it("moves through retry, exhausted budget and success", () => {
    resetTransport();
    expect(get(transport)).toEqual({ reconnecting: false, lastError: null });

    beginRetry(retryMessage(1, null));
    expect(get(transport)).toEqual({
      reconnecting: true,
      lastError: "The gateway did not answer on attempt 1 of 3.",
    });

    beginRetry(retryMessage(2, 503));
    expect(get(transport)).toEqual({
      reconnecting: true,
      lastError: "The gateway answered HTTP 503 on attempt 2 of 3.",
    });

    endRetry(finalMessage(3, 503));
    expect(get(transport)).toEqual({
      reconnecting: false,
      lastError: "The gateway answered HTTP 503 after 3 attempts.",
    });

    beginRetry(retryMessage(1, null));
    clearRetry();
    expect(get(transport)).toEqual({ reconnecting: false, lastError: null });
  });
});
