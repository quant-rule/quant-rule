<script lang="ts">
  import { onMount } from "svelte";
  import { create, load, profiles } from "./profiles.svelte";

  const view = $derived(profiles());
  let profileId = $state("");
  let name = $state("");

  onMount(() => {
    void load();
  });

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (profileId.trim() === "" || name.trim() === "") {
      return;
    }
    if (await create(profileId.trim(), name.trim())) {
      profileId = "";
      name = "";
    }
  }
</script>

<section class="stack">
  <div class="row">
    <h2>Profiles</h2>
    <span class="badge">{view.items.length} total</span>
    <button onclick={() => void load()} disabled={view.loading}>Refresh</button>
  </div>

  {#if view.error}
    <div class="notice error" role="alert">{view.error}</div>
  {/if}

  {#if view.loading && view.items.length === 0}
    <div class="notice">Loading profiles…</div>
  {:else if view.items.length === 0}
    <div class="notice">No profiles. Create one.</div>
  {:else}
    <table>
      <thead>
        <tr>
          <th>Profile</th>
          <th>Name</th>
          <th>Datasets</th>
          <th>Runs</th>
          <th>Simulations</th>
        </tr>
      </thead>
      <tbody>
        {#each view.items as item (item.profile_id)}
          <tr>
            <td class="mono">
              {item.profile_id}
              {#if item.profile_id === view.active}<span class="badge ok">active</span>{/if}
            </td>
            <td>{item.name}</td>
            <td>{item.datasets}</td>
            <td>{item.runs}</td>
            <td>{item.simulations}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}

  <form class="row" onsubmit={(event) => void submit(event)}>
    <input
      bind:value={profileId}
      placeholder="profile-id"
      aria-label="New profile id"
      pattern="[a-z0-9][a-z0-9-]*"
    />
    <input bind:value={name} placeholder="Name" aria-label="New profile name" />
    <button type="submit" disabled={profileId.trim() === "" || name.trim() === ""}>
      Create profile
    </button>
  </form>
</section>

<style>
  section {
    max-width: 70rem;
    gap: 0.8rem;
  }

  form {
    flex-wrap: wrap;
  }

  input {
    font: inherit;
    color: inherit;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 0.4rem 0.6rem;
    min-width: 12rem;
  }
</style>
