<script lang="ts">
  import { onMount } from "svelte";
  import { currentProfile, errorText } from "./api";
  import {
    HISTORY_KINDS,
    archive,
    canArchive,
    erase,
    list,
    type HistoryEntry,
    type HistoryKind,
  } from "./history";

  let kind = $state<HistoryKind>("run");
  let includeArchived = $state(true);
  let entries = $state<HistoryEntry[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let pending = $state<string | null>(null);
  let erasing = $state<HistoryEntry | null>(null);
  let confirmation = $state("");
  let profile = $state(currentProfile());

  $effect(() => {
    const next = currentProfile();
    if (next !== profile) {
      profile = next;
      entries = [];
      erasing = null;
      confirmation = "";
      void load();
    }
  });

  async function load(): Promise<void> {
    const requested = kind;
    loading = true;
    error = null;
    try {
      const rows = await list(requested, includeArchived);
      if (requested !== kind) {
        return;
      }
      entries = rows;
    } catch (cause) {
      if (requested !== kind) {
        return;
      }
      entries = [];
      error = errorText(cause);
    } finally {
      if (requested === kind) {
        loading = false;
      }
    }
  }

  async function selectKind(next: HistoryKind): Promise<void> {
    kind = next;
    erasing = null;
    confirmation = "";
    entries = [];
    await load();
  }

  async function archiveRow(row: HistoryEntry): Promise<void> {
    pending = `archive:${row.id}`;
    error = null;
    try {
      await archive(row.kind, row.id);
      await load();
    } catch (cause) {
      error = errorText(cause);
    } finally {
      pending = null;
    }
  }

  function askErase(row: HistoryEntry): void {
    erasing = row;
    confirmation = "";
  }

  async function confirmErase(): Promise<void> {
    if (erasing === null) {
      return;
    }
    pending = `erase:${erasing.id}`;
    error = null;
    try {
      await erase(erasing.kind, erasing.id, confirmation);
      erasing = null;
      confirmation = "";
      await load();
    } catch (cause) {
      error = errorText(cause);
    } finally {
      pending = null;
    }
  }

  function short(value: string): string {
    return value.length > 24 ? `${value.slice(0, 21)}…` : value;
  }

  onMount(() => {
    void load();
  });
</script>

<section class="stack">
  <div class="row">
    <h2>History</h2>
    <span class="badge">{entries.length} rows</span>
    <button onclick={() => void load()} disabled={loading}>Refresh</button>
  </div>

  <div class="row" role="tablist" aria-label="History kinds">
    {#each HISTORY_KINDS as tab (tab.id)}
      <button
        role="tab"
        aria-selected={kind === tab.id}
        class:active={kind === tab.id}
        data-testid={`history-tab-${tab.id}`}
        onclick={() => void selectKind(tab.id)}
      >
        {tab.label}
      </button>
    {/each}
  </div>

  {#if kind !== "stage"}
    <label class="row">
      <input
        type="checkbox"
        data-testid="history-include-archived"
        bind:checked={includeArchived}
        onchange={() => void load()}
      />
      Include archived
    </label>
  {/if}

  {#if error}
    <div class="notice error" role="alert">{error}</div>
  {/if}

  {#if erasing}
    <div class="notice" role="dialog" aria-label="Confirm erase">
      <p>
        Erase <strong>{erasing.kind}</strong> <span class="mono">{erasing.id}</span>?
        This cannot be undone.
      </p>
      <label>
        Type the exact id
        <input
          class="mono"
          data-testid="history-confirm-input"
          bind:value={confirmation}
        />
      </label>
      <div class="row">
        <button
          data-testid="history-confirm-erase"
          onclick={() => void confirmErase()}
          disabled={confirmation !== erasing.id || pending !== null}
        >
          Erase
        </button>
        <button onclick={() => (erasing = null)}>Cancel</button>
      </div>
    </div>
  {/if}

  {#if loading && entries.length === 0}
    <div class="notice">Loading history…</div>
  {:else if entries.length === 0}
    <div class="notice">No {kind} records.</div>
  {:else}
    <table data-testid="history-table" aria-busy={loading}>
      <thead>
        <tr>
          <th>Record</th>
          <th>State</th>
          <th>Timestamp (UTC)</th>
          <th>Archived</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#each entries as row (row.kind + row.id)}
          <tr>
            <td class="mono" title={row.id}>{short(row.id)}</td>
            <td>{row.state}</td>
            <td class="mono">{row.timestamp.replace("T", " ")}</td>
            <td>{row.archivedAt ?? "no"}</td>
            <td>
              {#if row.archivedAt === null && canArchive(row.kind)}
                <button
                  data-testid="history-archive"
                  onclick={() => void archiveRow(row)}
                  disabled={pending !== null}
                  aria-label={`Archive ${row.id}`}
                >
                  Archive
                </button>
              {:else if row.archivedAt !== null}
                <button
                  data-testid="history-erase"
                  onclick={() => askErase(row)}
                  disabled={pending !== null}
                  aria-label={`Erase ${row.id}`}
                >
                  Erase
                </button>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>

<style>
  section {
    max-width: 80rem;
    gap: 0.8rem;
  }

  .active {
    background: var(--panel);
    border-color: var(--accent);
  }
</style>
