import { afterEach, describe, expect, it } from "bun:test";
import { ApiError, call } from "./api";

const original = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = original;
});

function reply(status: number, body: unknown): void {
  globalThis.fetch = (async () =>
    new Response(JSON.stringify(body), {
      status,
      headers: { "content-type": "application/json" },
    })) as unknown as typeof fetch;
}

describe("call", () => {
  it("parses a successful response", async () => {
    reply(200, { product: "1.0.0" });
    const value = await call<{ product: string }>("/version");
    expect(value.product).toBe("1.0.0");
  });

  it("turns an error envelope into an ApiError", async () => {
    reply(404, { v: "quantrule.api/v1", error: { code: "run.not_found", message: "missing" } });
    await expect(call("/runs/x")).rejects.toBeInstanceOf(ApiError);
  });

  it("keeps the stable code", async () => {
    reply(409, { error: { code: "run.conflict", message: "already finished" } });
    try {
      await call("/runs/x/cancel");
      throw new Error("expected a failure");
    } catch (error) {
      expect((error as ApiError).code).toBe("run.conflict");
      expect((error as ApiError).status).toBe(409);
    }
  });
});
