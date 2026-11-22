import { errorText } from "./api";
import {
  createPack,
  fetchPack,
  freezePack,
  listPacks,
  retirePack,
  validatePack,
  type PackFilter,
  type PackKind,
  type PackRecord,
  type PackState,
  type PackSummary,
} from "./packs";

/** What the Packs screen knows. */
export interface Packs {
  items: PackSummary[];
  filter: PackFilter;
  loading: boolean;
  error: string | null;
  selected: string | null;
  detail: PackRecord | null;
  detailLoading: boolean;
  acting: string | null;
  created: PackRecord | null;
}

const EMPTY: Packs = {
  items: [],
  filter: {},
  loading: false,
  error: null,
  selected: null,
  detail: null,
  detailLoading: false,
  acting: null,
  created: null,
};

let current = $state<Packs>({ ...EMPTY });

/** A counter that makes an in-flight read from a previous profile stale. */
let generation = 0;

/** Drop every row and the open detail; the next read belongs to the active profile. */
export function reset(): void {
  generation += 1;
  current = { ...EMPTY };
}

/** Read the current packs state. */
export function packs(): Packs {
  return current;
}

/** Read the profile's packs, optionally changing the filter. */
export async function load(filter: PackFilter = current.filter): Promise<void> {
  const at = generation;
  current = {
    ...current,
    filter,
    loading: current.items.length === 0,
    error: null,
  };
  try {
    const items = await listPacks(filter);
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

/** Read one pack record. */
export async function select(packId: string): Promise<void> {
  const at = generation;
  current = { ...current, selected: packId, detailLoading: true, error: null };
  try {
    const detail = await fetchPack(packId);
    if (at !== generation || current.selected !== packId) {
      return;
    }
    current = { ...current, detail, detailLoading: false };
  } catch (error) {
    if (at !== generation || current.selected !== packId) {
      return;
    }
    current = { ...current, detail: null, detailLoading: false, error: errorText(error) };
  }
}

/** Register one local document, then show the server's record. */
export async function create(file: File): Promise<void> {
  const at = generation;
  current = { ...current, acting: "create", created: null, error: null };
  try {
    const record = await createPack(file);
    if (at !== generation) {
      return;
    }
    current = {
      ...current,
      acting: null,
      created: record,
      detail: record,
      selected: record.summary.pack_id,
    };
    await load();
  } catch (error) {
    if (at !== generation) {
      return;
    }
    current = { ...current, acting: null, error: errorText(error) };
  }
}

/** Re-validate, freeze, or retire one pack and keep the returned record. */
export async function transition(
  packId: string,
  action: "validate" | "freeze" | "retire",
): Promise<void> {
  const at = generation;
  current = { ...current, acting: `${action}:${packId}`, error: null };
  try {
    const record =
      action === "validate"
        ? await validatePack(packId)
        : action === "freeze"
          ? await freezePack(packId)
          : await retirePack(packId);
    if (at !== generation) {
      return;
    }
    current = {
      ...current,
      acting: null,
      detail: record,
      selected: record.summary.pack_id,
    };
    await load();
  } catch (error) {
    if (at !== generation) {
      return;
    }
    current = { ...current, acting: null, error: errorText(error) };
  }
}

/** The lifecycle state labels the filter offers. */
export const PACK_STATES: PackState[] = ["draft", "validated", "frozen", "retired"];

/** The pack kind labels the filter offers. */
export const PACK_KINDS: PackKind[] = ["strategy", "portfolio", "execution"];
