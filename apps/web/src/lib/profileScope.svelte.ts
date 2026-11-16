/**
 * The profile-scoped client boundary.
 *
 * Every row the browser shows belongs to one profile: datasets, configs, runs,
 * and simulation runs. Selecting another profile or losing the gateway clears
 * all four before any new read starts, so a slow or failed request cannot leave
 * the previous profile's rows on screen. The stores keep their own generation
 * counter; this module is only the one place that decides when a scope ends.
 *
 * Panel-local selections (an open run, an open simulation, an inspection
 * result) are cleared by the panel that owns them when the active profile
 * changes.
 */

import { load as loadConfigs, reset as resetConfigs } from "./configs.svelte";
import { load as loadDatasets, reset as resetDatasets } from "./datasets.svelte";
import { select } from "./profiles.svelte";
import { load as loadRuns, reset as resetRuns } from "./runs.svelte";
import { load as loadSimulations, reset as resetSimulations } from "./simulation.svelte";

/** Drop every profile-scoped row. */
export function clearProfileScope(): void {
  resetDatasets();
  resetConfigs();
  resetRuns();
  resetSimulations();
}

/** Read every profile-scoped resource for the active profile. */
export async function loadProfileScope(): Promise<void> {
  await Promise.all([loadDatasets(), loadConfigs(), loadRuns(), loadSimulations()]);
}

/** Select a profile, drop the previous profile's rows, then read the new ones. */
export async function switchProfile(profile: string): Promise<void> {
  await select(profile);
  clearProfileScope();
  await loadProfileScope();
}
