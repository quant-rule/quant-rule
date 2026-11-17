import { errorText } from "./api";
import { loadOperations } from "./operations";
import type { OperationsReport } from "./types";

/** What the Operations screen knows about the evidence service. */
export interface OperationsView {
  report: OperationsReport | null;
  loading: boolean;
  error: string | null;
}

let current = $state<OperationsView>({
  report: null,
  loading: false,
  error: null,
});

/** Read the current operations state. */
export function operations(): OperationsView {
  return current;
}

/** Load the retention status once; a manual Refresh repeats it. */
export async function load(): Promise<void> {
  current = { ...current, loading: true, error: null };
  try {
    const report = await loadOperations();
    current = { report, loading: false, error: null };
  } catch (error) {
    current = { ...current, report: null, loading: false, error: errorText(error) };
  }
}

/** Drop the snapshot when the screen unmounts. */
export function reset(): void {
  current = { report: null, loading: false, error: null };
}
