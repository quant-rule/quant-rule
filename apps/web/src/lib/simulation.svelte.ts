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

/** A counter that makes an in-flight read from a previous profile stale. */
let generation = 0;

/** Drop every row; the next read belongs to the active profile. */
export function reset(): void {
  generation += 1;
  current = { items: [], loading: false, error: null, creating: false };
}

/** Read the current runs state. */
export function simulations(): Simulations {
  return current;
}

/** Read the server's list; a failure clears the rows. */
export async function load(): Promise<void> {
  const at = generation;
  const first = current.items.length === 0;
  current = { ...current, loading: first, error: null };
  try {
    const items = await listRuns({ limit: 100 });
    if (at !== generation) {
      return;
    }
    current = { ...current, items, loading: false, error: null };
  } catch (error) {
    if (at !== generation) {
      return;
    }
    current = { ...current, items: [], loading: false, error: errorText(error) };
  }
}

/** Create one run, re-read the list, and return the new id or null. */
export async function create(input: SimulationCreate): Promise<string | null> {
  const at = generation;
  current = { ...current, creating: true, error: null };
  try {
    const simId = await createRun(input);
    if (at !== generation) {
      return null;
    }
    current = { ...current, creating: false };
    await load();
    return simId;
  } catch (error) {
    if (at !== generation) {
      return null;
    }
    current = { ...current, creating: false, error: errorText(error) };
    return null;
  }
}
