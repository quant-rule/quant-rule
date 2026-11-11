/** Formats a UTC Unix second as a compact ISO-like timestamp. */
export function formatTime(seconds: number | null): string {
  if (seconds === null || !Number.isFinite(seconds)) {
    return "—";
  }
  return new Date(seconds * 1000).toISOString().replace(".000Z", "Z");
}

/** Formats a count with thousands separators without depending on the locale. */
export function formatCount(value: number): string {
  return new Intl.NumberFormat("en-US").format(value);
}

/** Formats a timeframe in seconds as its stable code. */
export function timeframeCode(seconds: number): string {
  if (seconds % 3600 === 0) {
    return `${seconds / 3600}h`;
  }
  if (seconds % 60 === 0) {
    return `${seconds / 60}m`;
  }
  return `${seconds}s`;
}
