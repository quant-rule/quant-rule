<script lang="ts">
  import { call } from "./api";
  import { errorMessage } from "./errors";
  import { formatTime } from "./format";
  import { lastModel, recordTimes } from "./session.svelte";
  import type {
    ClaimView,
    MemoryStatusView,
    MetricView,
    NoteView,
    RecordedRunView,
    RunRecordView,
    WorkspaceRecordView,
  } from "./types";

  let {
    memoryPath,
    runPath,
  }: {
    memoryPath: string;
    runPath: string;
  } = $props();

  let status = $state<MemoryStatusView | null>(null);
  let workspaces = $state<WorkspaceRecordView[]>([]);
  let runs = $state<RunRecordView[]>([]);
  let metrics = $state<MetricView[]>([]);
  let claims = $state<ClaimView[]>([]);
  let notes = $state<NoteView[]>([]);
  let selectedWorkspace = $state(0);
  let selectedRun = $state(0);
  let metricName = $state("");
  let metricKind = $state("real");
  let metricValue = $state("");
  let metricText = $state("");
  let claimType = $state("Behaviour Claim");
  let claimStatement = $state("");
  let claimStatus = $state("accepted");
  let noteBody = $state("");
  let runKey = $state("");
  let exportPath = $state("");
  let importPath = $state("");
  let backupPath = $state("");
  let restorePath = $state("");
  let busy = $state(false);
  let error = $state("");
  let notice = $state("");

  async function initialize() {
    error = "";
    notice = "";
    busy = true;
    try {
      status = await call<MemoryStatusView>("memory_status", { db: memoryPath });
      await loadWorkspaces();
      notice = "Memory is ready.";
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = false;
    }
  }

  async function loadWorkspaces() {
    workspaces = await call<WorkspaceRecordView[]>("memory_workspaces", { db: memoryPath });
    if (workspaces.length > 0 && selectedWorkspace === 0) {
      selectedWorkspace = workspaces[0].id;
      await loadRuns(selectedWorkspace);
    }
  }

  async function loadRuns(workspaceId: number) {
    selectedWorkspace = workspaceId;
    runs = await call<RunRecordView[]>("memory_runs", {
      db: memoryPath,
      workspace_id: workspaceId,
    });
    metrics = [];
    claims = [];
    notes = [];
    selectedRun = 0;
  }

  async function loadEvidence(runId: number) {
    metrics = await call<MetricView[]>("memory_metrics", {
      db: memoryPath,
      run_id: runId,
    });
    claims = await call<ClaimView[]>("memory_claims", {
      db: memoryPath,
      run_id: runId,
    });
    notes = await call<NoteView[]>("memory_notes_for_run", {
      db: memoryPath,
      run_id: runId,
    });
  }

  async function selectRun(run: RunRecordView) {
    selectedRun = run.id;
    error = "";
    try {
      await loadEvidence(run.id);
    } catch (cause) {
      error = errorMessage(cause);
    }
  }

  async function addMetric() {
    if (selectedRun === 0 || metricName.trim() === "") {
      return;
    }
    error = "";
    notice = "";
    try {
      await call("memory_metric_set", {
        db: memoryPath,
        run_id: selectedRun,
        name: metricName,
        kind: metricKind,
        value: metricKind === "real" ? Number(metricValue) : null,
        text_value:
          metricKind === "integer" || metricKind === "text" ? metricText : null,
      });
      metricName = "";
      metricValue = "";
      metricText = "";
      notice = "Metric added.";
      await loadEvidence(selectedRun);
    } catch (cause) {
      error = errorMessage(cause);
    }
  }

  async function deleteMetric(metric: MetricView) {
    error = "";
    notice = "";
    try {
      await call("memory_metric_delete", {
        db: memoryPath,
        run_id: metric.run_id,
        name: metric.name,
      });
      notice = "Metric deleted.";
      await loadEvidence(selectedRun);
    } catch (cause) {
      error = errorMessage(cause);
    }
  }

  async function addClaim() {
    if (selectedRun === 0 || claimStatement.trim() === "") {
      return;
    }
    error = "";
    notice = "";
    try {
      await call("memory_claim_add", {
        db: memoryPath,
        run_id: selectedRun,
        claim_type: claimType,
        statement: claimStatement,
        status: claimStatus,
        created_at: Math.floor(Date.now() / 1000),
      });
      claimStatement = "";
      notice = "Claim added.";
      await loadEvidence(selectedRun);
    } catch (cause) {
      error = errorMessage(cause);
    }
  }

  async function deleteClaim(claim: ClaimView) {
    error = "";
    notice = "";
    try {
      await call("memory_claim_delete", { db: memoryPath, id: claim.id });
      notice = "Claim deleted.";
      await loadEvidence(selectedRun);
    } catch (cause) {
      error = errorMessage(cause);
    }
  }

  async function addNote() {
    if (selectedRun === 0 || noteBody.trim() === "") {
      return;
    }
    error = "";
    notice = "";
    try {
      const now = Math.floor(Date.now() / 1000);
      await call("memory_note_add", {
        db: memoryPath,
        workspace_id: selectedWorkspace,
        run_id: selectedRun,
        body: noteBody,
        created_at: now,
        updated_at: now,
      });
      noteBody = "";
      notice = "Note added.";
      await loadEvidence(selectedRun);
    } catch (cause) {
      error = errorMessage(cause);
    }
  }

  async function deleteNote(note: NoteView) {
    error = "";
    notice = "";
    try {
      await call("memory_note_delete", { db: memoryPath, id: note.id });
      notice = "Note deleted.";
      await loadEvidence(selectedRun);
    } catch (cause) {
      error = errorMessage(cause);
    }
  }

  async function record() {
    error = "";
    notice = "";
    busy = true;
    try {
      const times = recordTimes(runPath, Math.floor(Date.now() / 1000));
      const recorded = await call<RecordedRunView>("record_run", {
        db: memoryPath,
        config: runPath,
        run_key: runKey,
        status: "complete",
        started_at: times.startedAt,
        finished_at: times.finishedAt,
        model: lastModel(runPath),
      });
      notice = `Recorded run #${recorded.run_id}.`;
      selectedWorkspace = recorded.workspace_id;
      await loadWorkspaces();
      await loadRuns(recorded.workspace_id);
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = false;
    }
  }

  async function exportMemory() {
    error = "";
    notice = "";
    busy = true;
    try {
      await call("memory_export", { db: memoryPath, out: exportPath });
      notice = `Exported to ${exportPath}.`;
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = false;
    }
  }

  async function importMemory() {
    error = "";
    notice = "";
    busy = true;
    try {
      await call("memory_import", { db: memoryPath, from: importPath });
      notice = `Imported from ${importPath}.`;
      await loadWorkspaces();
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = false;
    }
  }

  async function backupMemory() {
    error = "";
    notice = "";
    busy = true;
    try {
      await call("memory_backup", { db: memoryPath, out: backupPath });
      notice = `Backed up to ${backupPath}.`;
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = false;
    }
  }

  async function restoreMemory() {
    error = "";
    notice = "";
    busy = true;
    try {
      await call("memory_restore", { backup: backupPath, target: restorePath });
      notice = `Restored to ${restorePath}.`;
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = false;
    }
  }
</script>

<div class="stack">
  <div class="card">
    <div class="panel-title">
      <div>
        <h2>Local research memory</h2>
        <p class="muted small mono">{memoryPath || "Set a memory database path."}</p>
      </div>
      <button class="primary" onclick={initialize} disabled={busy || memoryPath.trim() === ""}>
        Open memory
      </button>
    </div>

    {#if status}
      <div class="row">
        <span class="pill ok">{status.schema}</span>
        <span class="pill">version {status.version}</span>
      </div>
    {/if}
    {#if error}
      <p class="error">{error}</p>
    {/if}
    {#if notice}
      <p class="ok">{notice}</p>
    {/if}

    <div class="record">
      <input bind:value={runKey} placeholder="run key, e.g. run-2026-11-11-01" aria-label="Run key" />
      <button
        onclick={record}
        disabled={busy || runPath === "" || runKey.trim() === "" || memoryPath.trim() === ""}
      >
        Record current run
      </button>
      <span class="muted small">
        {runPath ? `config: ${runPath}` : "select a run config first"}
      </span>
    </div>
  </div>

  <div class="split">
    <div class="card">
      <h2>Workspaces</h2>
      <div class="list">
        {#each workspaces as item (item.id)}
          <button
            class:active={selectedWorkspace === item.id}
            onclick={() => loadRuns(item.id)}
          >
            <strong>{item.name}</strong>
            <div class="muted small mono">{item.root}</div>
          </button>
        {:else}
          <div class="muted">No workspaces recorded.</div>
        {/each}
      </div>
    </div>

    <div class="card">
      <h2>Runs</h2>
      <div class="list">
        {#each runs as run (run.id)}
          <button class:active={selectedRun === run.id} onclick={() => selectRun(run)}>
            <div class="row" style="justify-content: space-between">
              <strong>{run.run_key}</strong>
              <span class="muted small">{run.status}</span>
            </div>
            <div class="muted small">
              {formatTime(run.started_at)} · {run.data_version}
            </div>
          </button>
        {:else}
          <div class="muted">No runs recorded.</div>
        {/each}
      </div>
    </div>
  </div>

  {#if selectedRun > 0}
    <div class="card">
      <h2>Evidence for run #{selectedRun}</h2>
      <div class="split">
        <div>
          <h3>Metrics</h3>
          <table>
            <tbody>
              {#each metrics as metric (metric.id)}
                <tr>
                  <th>{metric.name}</th>
                  <td>
                    {#if metric.kind === "null"}
                      <span class="muted">null</span>
                    {:else}
                      {metric.value}
                    {/if}
                  </td>
                  <td>
                    <button class="quiet" onclick={() => deleteMetric(metric)}>
                      Delete
                    </button>
                  </td>
                </tr>
              {:else}
                <tr><td class="muted">No metrics.</td></tr>
              {/each}
            </tbody>
          </table>
          <div class="row">
            <input bind:value={metricName} placeholder="metric name" aria-label="Metric name" />
            <select bind:value={metricKind} aria-label="Metric kind">
              <option value="real">real</option>
              <option value="integer">integer</option>
              <option value="text">text</option>
              <option value="null">null</option>
            </select>
            {#if metricKind === "real"}
              <input bind:value={metricValue} placeholder="number" aria-label="Metric value" />
            {:else if metricKind !== "null"}
              <input bind:value={metricText} placeholder="value" aria-label="Metric text" />
            {/if}
            <button
              onclick={addMetric}
              disabled={metricName.trim() === "" || (metricKind === "real" && metricValue.trim() === "") || ((metricKind === "integer" || metricKind === "text") && metricText.trim() === "")}
            >
              Add metric
            </button>
          </div>
        </div>
        <div>
          <h3>Claims</h3>
          <div class="list">
            {#each claims as claim (claim.id)}
              <div class="item">
                <strong>{claim.claim_type}</strong>
                <div>{claim.statement}</div>
                <div class="muted small">{claim.status}</div>
                <button class="quiet" onclick={() => deleteClaim(claim)}>Delete</button>
              </div>
            {:else}
              <div class="muted">No claims.</div>
            {/each}
            <div class="row">
              <input bind:value={claimType} placeholder="claim type" aria-label="Claim type" />
              <input bind:value={claimStatement} placeholder="statement" aria-label="Claim statement" />
              <input bind:value={claimStatus} placeholder="status" aria-label="Claim status" />
              <button onclick={addClaim} disabled={claimStatement.trim() === ""}>
                Add claim
              </button>
            </div>
          </div>
          <h3>Notes</h3>
          <div class="list">
            {#each notes as note (note.id)}
              <div class="item">
                <div>{note.body}</div>
                <div class="muted small">{formatTime(note.updated_at)}</div>
                <button class="quiet" onclick={() => deleteNote(note)}>Delete</button>
              </div>
            {:else}
              <div class="muted">No notes for this run.</div>
            {/each}
            <div class="row">
              <input bind:value={noteBody} placeholder="note" aria-label="Note body" />
              <button onclick={addNote} disabled={noteBody.trim() === ""}>Add note</button>
            </div>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <div class="card">
    <h2>Export, import, backup, restore</h2>
    <div class="stack">
      <div class="row">
        <input bind:value={exportPath} placeholder="export path" aria-label="Export path" />
        <button onclick={exportMemory} disabled={busy || exportPath.trim() === ""}>Export</button>
        <input bind:value={importPath} placeholder="import path" aria-label="Import path" />
        <button onclick={importMemory} disabled={busy || importPath.trim() === ""}>Import</button>
      </div>
      <div class="row">
        <input bind:value={backupPath} placeholder="backup path" aria-label="Backup path" />
        <button onclick={backupMemory} disabled={busy || backupPath.trim() === ""}>Backup</button>
        <input bind:value={restorePath} placeholder="restore target" aria-label="Restore target" />
        <button onclick={restoreMemory} disabled={busy || backupPath.trim() === "" || restorePath.trim() === ""}>
          Restore
        </button>
      </div>
    </div>
    <p class="muted small" style="margin-top: 0.8rem">
      Import only accepts an empty memory. Backup and restore targets must not
      exist; the app never overwrites a database in place.
    </p>
  </div>
</div>

<style>
  .pill {
    border: 1px solid var(--line);
    border-radius: 999px;
    padding: 0.2rem 0.55rem;
    font-size: 0.78rem;
    color: var(--muted);
  }

  .record {
    display: flex;
    gap: 0.6rem;
    align-items: center;
    flex-wrap: wrap;
    margin-top: 0.8rem;
  }

  .record input {
    flex: 1 1 260px;
  }

  .item {
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 0.55rem 0.65rem;
    background: var(--panel-2);
  }
</style>
