import { call, errorText } from "./api";
import { pollDelay } from "./runs";
import type { RunDetail, RunSummary } from "./types";

/** What the runs inbox knows. */
export interface Runs {
  items: RunSummary[];
  loading: boolean;
  error: string | null;
  selected: string | null;
  detail: RunDetail | null;
  detailLoading: boolean;
}

let current = $state<Runs>({
  items: [],
  loading: false,
  error: null,
  selected: null,
  detail: null,
  detailLoading: false,
});

/** Read the current runs state. */
export function runs(): Runs {
  return current;
}

/** Read the server's list; a failure drops the rows rather than showing stale ones. */
export async function load(): Promise<void> {
  const first = current.items.length === 0;
  current = { ...current, loading: first, error: null };
  try {
    const items = await call<RunSummary[]>("/runs?limit=100");
    current = { ...current, items, loading: false, error: null };
  } catch (error) {
    current = { ...current, items: [], loading: false, error: errorText(error) };
  }
}

/** Read one run's detail. */
export async function loadDetail(runId: string): Promise<void> {
  current = { ...current, detailLoading: true, error: null };
  try {
    const detail = await call<RunDetail>(`/runs/${encodeURIComponent(runId)}`);
    current = { ...current, detail, detailLoading: false, error: null };
  } catch (error) {
    current = { ...current, detail: null, detailLoading: false, error: errorText(error) };
  }
}

/** Open one run. */
export async function select(runId: string): Promise<void> {
  current = { ...current, selected: runId, detail: null, detailLoading: true };
  await loadDetail(runId);
}

/** Return to the list. */
export function clearSelection(): void {
  current = { ...current, selected: null, detail: null, detailLoading: false };
}

/**
 * Start the inbox poll.
 *
 * The next delay follows the current rows: 2 s while a run is active and 30 s
 * otherwise. The returned function stops the chain; call it on unmount.
 */
export function startPolling(): () => void {
  let stopped = false;
  let timer: ReturnType<typeof setTimeout> | undefined;

  const tick = async (): Promise<void> => {
    if (stopped) {
      return;
    }
    await load();
    if (current.selected !== null) {
      await loadDetail(current.selected);
    }
    if (stopped) {
      return;
    }
    timer = setTimeout(() => {
      void tick();
    }, pollDelay(current.items));
  };

  void tick();
  return () => {
    stopped = true;
    if (timer !== undefined) {
      clearTimeout(timer);
      timer = undefined;
    }
  };
}

/** Cancel an active run, then read the server's new state. */
export async function cancel(runId: string): Promise<void> {
  try {
    await call(`/runs/${encodeURIComponent(runId)}/cancel`, { method: "POST" });
    await load();
    if (current.selected === runId) {
      await loadDetail(runId);
    }
  } catch (error) {
    current = { ...current, error: errorText(error) };
  }
}
