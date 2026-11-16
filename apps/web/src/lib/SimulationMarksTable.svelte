<script lang="ts">
  import { markClassLabel } from "./chart/markClasses";
  import type { ChartMark } from "./chart/types";
  import { sortMarks } from "./simulationReview";

  let {
    marks,
    loading,
    error,
  }: { marks: ChartMark[]; loading: boolean; error: string | null } = $props();

  const ordered = $derived(sortMarks(marks));

  function utc(iso: string | undefined): string {
    if (iso === undefined) {
      return "—";
    }
    return iso.replace("T", " ").replace(".000Z", "Z");
  }
</script>

<div class="row">
  <h3>Marks</h3>
  <span class="badge">{marks.length}</span>
  <span class="muted small">Human and oracle rows are kept distinct.</span>
</div>

{#if error}
  <div class="notice error" role="alert" data-testid="sim-marks-error">
    {error}
  </div>
{:else if loading && ordered.length === 0}
  <div class="notice">Loading marks…</div>
{:else if ordered.length === 0}
  <div class="notice" data-testid="sim-marks-empty">
    No marks yet. Point at a bar and press 1–8.
  </div>
{:else}
  <table data-testid="sim-marks">
    <thead>
      <tr>
        <th>Mark</th>
        <th>Origin</th>
        <th>Window</th>
        <th>Bar</th>
        <th>Class</th>
        <th>Note</th>
        <th>Recorded (UTC)</th>
      </tr>
    </thead>
    <tbody>
      {#each ordered as mark (mark.mark_id)}
        <tr class="mark-row {mark.origin}" data-testid={`sim-mark-${mark.mark_id}`}>
          <td class="mono">{mark.mark_id}</td>
          <td>
            <span
              class="badge {mark.origin === 'human' ? 'ok' : 'warn'}"
              data-origin={mark.origin}
            >
              {mark.origin}
            </span>
          </td>
          <td>{mark.chart}</td>
          <td>{mark.bar}</td>
          <td>{markClassLabel(mark)}</td>
          <td class="note">{mark.note === "" ? "—" : mark.note}</td>
          <td class="mono">{utc(mark.marked_time_utc)}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  .note {
    max-width: 28rem;
    overflow-wrap: anywhere;
  }

  .mark-row.oracle td:first-child {
    border-left: 3px solid var(--warn);
  }

  .mark-row.human td:first-child {
    border-left: 3px solid var(--ok);
  }
</style>
