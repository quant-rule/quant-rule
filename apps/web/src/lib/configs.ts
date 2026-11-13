import type { ConfigSummary } from "./types";

/** Whether a config can be submitted with the chosen dataset. */
export function canSubmit(
  config: ConfigSummary,
  dataVersion: string,
  submitting: string | null,
): boolean {
  return config.valid && dataVersion !== "" && submitting === null;
}
