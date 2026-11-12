import { open, save } from "@tauri-apps/plugin-dialog";

import { isTauri } from "./api";

export interface PickerFilter {
  name: string;
  extensions: string[];
}

/**
 * Opens the native folder picker. The browser fixture returns null; the typed
 * path remains the fallback.
 */
export async function pickDirectory(title: string): Promise<string | null> {
  if (!isTauri()) {
    return null;
  }
  const selected = await open({ directory: true, multiple: false, title });
  return typeof selected === "string" ? selected : null;
}

/** Opens the native file picker with an optional filter. */
export async function pickOpenFile(
  title: string,
  filters?: PickerFilter[],
): Promise<string | null> {
  if (!isTauri()) {
    return null;
  }
  const selected = await open({
    multiple: false,
    directory: false,
    title,
    filters,
  });
  return typeof selected === "string" ? selected : null;
}

/** Opens the native save picker. */
export async function pickSaveFile(
  title: string,
  defaultPath: string,
  filters?: PickerFilter[],
): Promise<string | null> {
  if (!isTauri()) {
    return null;
  }
  const selected = await save({
    title,
    defaultPath: defaultPath || undefined,
    filters,
  });
  return selected ?? null;
}
