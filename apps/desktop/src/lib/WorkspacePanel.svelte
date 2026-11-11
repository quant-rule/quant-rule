<script lang="ts">
  import type { WorkspaceView } from "./types";

  let {
    root,
    workspace,
    busy,
    onOpen,
  }: {
    root: string;
    workspace: WorkspaceView | null;
    busy: boolean;
    onOpen: () => void;
  } = $props();
</script>

<div class="stack">
  <div class="card">
    <div class="panel-title">
      <div>
        <h2>Local workspace</h2>
        <p class="muted">
          A workspace holds <code>config/</code> run files and
          <code>data/sources/</code> candle files. Nothing leaves this machine.
        </p>
      </div>
      <button class="primary" onclick={onOpen} disabled={busy || root.trim() === ""}>
        {busy ? "Opening…" : "Open workspace"}
      </button>
    </div>

    <div class="path mono">{root || "Set a workspace root in Settings."}</div>

    {#if workspace}
      <div class="grid" style="margin-top: 1rem">
        <div class="stat">
          <div class="muted small">Workspace</div>
          <div class="value">{workspace.name}</div>
        </div>
        <div class="stat">
          <div class="muted small">Datasets</div>
          <div class="value">{workspace.datasets.length}</div>
        </div>
        <div class="stat">
          <div class="muted small">Run configs</div>
          <div class="value">{workspace.runs.length}</div>
        </div>
      </div>
    {:else}
      <div class="empty">
        <h3>Getting started</h3>
        <ol>
          <li>Open a workspace root.</li>
          <li>Inspect a dataset in <strong>Data</strong>.</li>
          <li>Validate a run in <strong>Configs</strong>.</li>
          <li>Run the model and record it in <strong>Memory</strong>.</li>
        </ol>
      </div>
    {/if}
  </div>

  {#if workspace}
    <div class="card">
      <h2>Workspace contents</h2>
      <div class="split">
        <div>
          <h3>Datasets</h3>
          <div class="list">
            {#each workspace.datasets as dataset (dataset.path)}
              <div class="item">
                <strong>{dataset.name}</strong>
                <div class="muted small mono">{dataset.path}</div>
              </div>
            {:else}
              <div class="muted">No CSV sources in data/sources.</div>
            {/each}
          </div>
        </div>
        <div>
          <h3>Run configs</h3>
          <div class="list">
            {#each workspace.runs as run (run)}
              <div class="item">
                <div class="mono small">{run}</div>
              </div>
            {:else}
              <div class="muted">No run.json under config/.</div>
            {/each}
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .path {
    background: #0b0f15;
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 0.6rem 0.7rem;
    color: var(--muted);
    overflow-wrap: anywhere;
  }

  .empty {
    margin-top: 1rem;
    border: 1px dashed var(--line);
    border-radius: 10px;
    padding: 1rem;
  }

  .empty ol {
    margin: 0;
    padding-left: 1.2rem;
    color: var(--muted);
  }

  .empty li {
    margin: 0.25rem 0;
  }

  .item {
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 0.55rem 0.65rem;
    background: var(--panel-2);
    overflow-wrap: anywhere;
  }
</style>
