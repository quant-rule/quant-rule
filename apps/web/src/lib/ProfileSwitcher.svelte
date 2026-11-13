<script lang="ts">
  import { load as loadDatasets } from "./datasets.svelte";
  import { profiles, select } from "./profiles.svelte";

  const view = $derived(profiles());

  async function change(event: Event): Promise<void> {
    const value = (event.currentTarget as HTMLSelectElement).value;
    await select(value);
    await loadDatasets();
  }
</script>

<label class="switcher">
  <span class="muted small">Profile</span>
  <select value={view.active} onchange={(event) => void change(event)} aria-label="Active profile">
    {#if view.items.length === 0 || !view.items.some((profile) => profile.profile_id === view.active)}
      <option value={view.active}>{view.active} (unknown)</option>
    {/if}
    {#each view.items as profile (profile.profile_id)}
      <option value={profile.profile_id}>{profile.profile_id}</option>
    {/each}
  </select>
  {#if view.error}
    <span class="badge error" title={view.error}>profile error</span>
  {/if}
</label>

<style>
  .switcher {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  select {
    font: inherit;
    color: inherit;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 0.35rem 0.6rem;
  }
</style>
