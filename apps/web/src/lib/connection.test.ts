import { describe, expect, it } from "bun:test";
import { ApiError } from "./api";
import { classify, classifyFailure, EXPECTED_API } from "./connection";
import type { HealthReport, VersionInfo } from "./types";

function version(api = EXPECTED_API): VersionInfo {
  return {
    product: "1.0.0",
    api,
    image_digest: "sha256:" + "a".repeat(64),
    contracts: [],
  };
}

function health(state: HealthReport["state"] = "healthy"): HealthReport {
  return {
    v: EXPECTED_API,
    service: "gateway",
    state,
    services: [],
    queue_depth: 0,
    workers: 1,
    image_digest: "sha256:" + "a".repeat(64),
  };
}

describe("classify", () => {
  it("reports a healthy matching gateway as connected", () => {
    expect(classify(version(), health())).toEqual({
      state: "connected",
      error: null,
    });
  });

  it("reports a different API as mismatch, with the expected version", () => {
    const verdict = classify(version("quantrule.api/v2"), health());
    expect(verdict.state).toBe("mismatch");
    expect(verdict.error).toContain("quantrule.api/v2");
    expect(verdict.error).toContain(EXPECTED_API);
  });

  it("reports a matching gateway with a down dependency as degraded", () => {
    expect(classify(version(), health("down"))).toEqual({
      state: "degraded",
      error: null,
    });
  });

  it("lets a mismatch outrank a degraded service", () => {
    expect(classify(version("quantrule.api/v2"), health("degraded")).state).toBe(
      "mismatch",
    );
  });
});

describe("classifyFailure", () => {
  it("reports a 401 as token required", () => {
    expect(
      classifyFailure(new ApiError(401, "auth.unauthorized", "a bearer token is required")),
    ).toBe("unauthorized");
  });

  it("reports every other failure as unreachable", () => {
    expect(classifyFailure(new ApiError(500, "internal", "boom"))).toBe("unreachable");
    expect(classifyFailure(new Error("connection refused"))).toBe("unreachable");
  });
});
