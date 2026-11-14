/**
 * The simulation runs list.
 *
 * The store holds one server resource. A failed read drops the rows instead of
 * presenting stale research, and the create call re-reads the list so the
 * screen shows what the server recorded.
 */

import { errorText } from "./api";
import {
  createRun,
  listRuns,
  type SimulationCreate,
  type SimulationSummary,
} from "./simulation";

/** What the simulation runs screen knows. */
export interface Simulations {
  items: SimulationSummary[];
  loading: boolean;
  error: string | null;
  creating: boolean;
}

let current = $state<Simulations>({
  items: [],
  loading: false,
  error: null,
  creating: false,
});

/** Read the current runs state. */
export function simulations(): Simulations {
  return current;
}

/** Read the server's list; a failure clears the rows. */
export async function load(): Promise<void> {
  const first = current.items.length === 0;
  current = { ...current, loading: first, error: null };
  try {
    const items = await listRuns({ limit: 100 });
    current = { ...current, items, loading: false, error: null };
  } catch (error) {
    current = { ...current, items: [], loading: false, error: errorText(error) };
  }
}

/** Create one run, re-read the list, and return the new id or null. */
export async function create(input: SimulationCreate): Promise<string | null> {
  current = { ...current, creating: true, error: null };
  try {
    const simId = await createRun(input);
    current = { ...current, creating: false };
    await load();
    return simId;
  } catch (error) {
    current = { ...current, creating: false, error: errorText(error) };
    return null;
  }
}
