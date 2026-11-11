import { describe, expect, it } from "vitest";

import { formatCount, formatTime, timeframeCode } from "./format";

describe("format", () => {
  it("formats UTC seconds and nulls", () => {
    expect(formatTime(0)).toBe("1970-01-01T00:00:00Z");
    expect(formatTime(null)).toBe("—");
  });

  it("formats counts without a locale dependency", () => {
    expect(formatCount(1234567)).toBe("1,234,567");
  });

  it("maps seconds to stable timeframe codes", () => {
    expect(timeframeCode(60)).toBe("1m");
    expect(timeframeCode(300)).toBe("5m");
    expect(timeframeCode(3600)).toBe("1h");
    expect(timeframeCode(90)).toBe("90s");
  });
});
