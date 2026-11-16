import type { ConfigSummary } from "./types";

/** Find one config record in the catalog list. */
export function findConfig(
  items: readonly ConfigSummary[],
  configId: string,
): ConfigSummary | null {
  return items.find((item) => item.config_id === configId) ?? null;
}

/** Whether a config can be submitted with the chosen dataset. */
export function canSubmit(
  config: ConfigSummary,
  dataVersion: string,
  submitting: string | null,
): boolean {
  return config.valid && dataVersion !== "" && submitting === null;
}
