/**
 * The profile-scoped client boundary.
 *
 * Every row the browser shows belongs to one profile: datasets, configs, runs,
 * simulation runs, diagnostic stage runs, and research runs, baselines, and
 * invalidations. Selecting another profile or losing the gateway clears all six
 * scoped stores before any new read starts, so a slow or failed request cannot
 * leave the previous profile's rows on screen. The stores keep their own
 * generation counter; this module is only the one place that decides when a
 * scope ends.
 *
 * Panel-local selections (an open run, an open simulation, an inspection
 * result) are cleared by the panel that owns them when the active profile
 * changes.
 */

import { load as loadConfigs, reset as resetConfigs } from "./configs.svelte";
import { load as loadDatasets, reset as resetDatasets } from "./datasets.svelte";
import { select } from "./profiles.svelte";
import { load as loadResearch, reset as resetResearch } from "./research.svelte";
import { load as loadRuns, reset as resetRuns } from "./runs.svelte";
import { load as loadSimulations, reset as resetSimulations } from "./simulation.svelte";
import { load as loadStages, reset as resetStages } from "./stages.svelte";

/** Drop every profile-scoped row. */
export function clearProfileScope(): void {
  resetDatasets();
  resetConfigs();
  resetRuns();
  resetResearch();
  resetSimulations();
  resetStages();
}

/** Read every profile-scoped resource for the active profile. */
export async function loadProfileScope(): Promise<void> {
  await Promise.all([
    loadDatasets(),
    loadConfigs(),
    loadRuns(),
    loadResearch(),
    loadSimulations(),
    loadStages(),
  ]);
}

/** Select a profile, drop the previous profile's rows, then read the new ones. */
export async function switchProfile(profile: string): Promise<void> {
  await select(profile);
  clearProfileScope();
  await loadProfileScope();
}
