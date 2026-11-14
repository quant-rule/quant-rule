<script lang="ts">
  import { errorText } from "./api";
  import { indicatorName } from "./chart/labels";
  import {
    markClassForKey,
    MARK_CLASSES,
    type MarkClassKey,
  } from "./chart/markClasses";
  import {
    appendMark,
    draftForClass,
    lastSessionMark,
    removeMarkById,
  } from "./chart/marks";
  import { arrowPath, diamondPath } from "./chart/paths";
  import { barAtPointer, barCenter, DEFAULT_LAYOUT, viewBoxX } from "./chart/scale";
  import { buildChartScene } from "./chart/scene";
  import type { ChartMark, ChartSeries } from "./chart/types";
  import { deleteMark, postMark } from "./simulation";

  interface Props {
    series: ChartSeries;
    simulationId: string;
    /** The parent owns the list; acknowledged marks are appended in place. */
    marks?: ChartMark[];
  }

  let {
    series,
    simulationId,
    marks = $bindable([]),
  }: Props = $props();

  const scene = $derived(buildChartScene(series, marks));
  const area = $derived(scene.regions);
  let hoverBar = $state<number | null>(null);
  let activeBar = $state<number | null>(null);
  let selectedKey = $state<MarkClassKey | null>(null);
  let note = $state("");
  let busy = $state(false);
  let error = $state<string | null>(null);
  let status = $state<string | null>(null);
  let postedIds = $state<string[]>([]);
  let surface = $state<SVGSVGElement | null>(null);

  const targetBar = $derived(hoverBar ?? activeBar);
  const undoable = $derived(lastSessionMark(marks, postedIds));

  function barAt(event: PointerEvent | MouseEvent): number | null {
    if (scene.barCount === 0) {
      return null;
    }
    const element = surface ?? (event.currentTarget as HTMLElement);
    const rect = element.getBoundingClientRect();
    const x = viewBoxX(event.clientX, rect.left, rect.width, area.width);
    return x === null ? null : barAtPointer(x, DEFAULT_LAYOUT, scene.barCount);
  }

  function onPointerMove(event: PointerEvent): void {
    hoverBar = barAt(event);
  }

  function onPointerLeave(): void {
    hoverBar = null;
  }

  function onClick(event: MouseEvent): void {
    const bar = barAt(event);
    if (bar !== null) {
      activeBar = bar;
      hoverBar = bar;
    }
  }

  function onChartKeyDown(event: KeyboardEvent): void {
    if (scene.barCount === 0) {
      return;
    }
    const current = activeBar ?? hoverBar ?? 0;
    let next: number | null = null;
    switch (event.key) {
      case "ArrowLeft":
        next = Math.max(0, current - 1);
        break;
      case "ArrowRight":
        next = Math.min(scene.barCount - 1, current + 1);
        break;
      case "Home":
        next = 0;
        break;
      case "End":
        next = scene.barCount - 1;
        break;
      default:
        return;
    }
    event.preventDefault();
    activeBar = next;
    hoverBar = null;
  }

  function isTyping(target: EventTarget | null): boolean {
    if (!(target instanceof HTMLElement)) {
      return false;
    }
    return (
      target.tagName === "INPUT" ||
      target.tagName === "TEXTAREA" ||
      target.tagName === "SELECT" ||
      target.isContentEditable
    );
  }

  function onWindowKeyDown(event: KeyboardEvent): void {
    if (event.ctrlKey || event.metaKey || event.altKey || isTyping(event.target)) {
      return;
    }
    const markClass = markClassForKey(event.key);
    if (markClass !== null) {
      event.preventDefault();
      void capture(markClass.key);
      return;
    }
    if (event.key === "z" || event.key === "Z") {
      event.preventDefault();
      void undo();
    }
  }

  async function capture(key: MarkClassKey): Promise<void> {
    const markClass = markClassForKey(key);
    if (markClass === null || busy) {
      return;
    }
    selectedKey = key;
    const bar = hoverBar ?? activeBar;
    if (bar === null) {
      error = "Point at a bar, click one, or use the arrow keys before choosing a mark.";
      status = null;
      return;
    }

    const draft = draftForClass(markClass, series.chart, bar, note);
    busy = true;
    error = null;
    status = null;
    try {
      const ack = await postMark(simulationId, draft);
      marks = appendMark(marks, {
        ...draft,
        mark_id: ack.mark_id,
        origin: "human",
      });
      postedIds = [...postedIds, ack.mark_id];
      note = "";
      status = `${markClass.label} at bar ${bar}.`;
    } catch (failure) {
      // The selected class, bar, and note stay put so the user can correct and retry.
      error = errorText(failure);
    } finally {
      busy = false;
    }
  }

  async function undo(): Promise<void> {
    if (busy) {
      return;
    }
    const last = lastSessionMark(marks, postedIds);
    if (last === null) {
      error = "No mark from this session to undo.";
      status = null;
      return;
    }
    busy = true;
    error = null;
    status = null;
    try {
      await deleteMark(simulationId, last.mark_id);
      marks = removeMarkById(marks, last.mark_id);
      postedIds = postedIds.filter((id) => id !== last.mark_id);
      status = `Removed ${last.mark_id}.`;
    } catch (failure) {
      error = errorText(failure);
    } finally {
      busy = false;
    }
  }
</script>

<svelte:window onkeydown={onWindowKeyDown} />

<section class="chart-shell stack">
  <div class="row">
    <h3>Window {series.chart}</h3>
    <span class="badge">{scene.barCount} bars</span>
    <span class="badge">{marks.length} {marks.length === 1 ? "mark" : "marks"}</span>
    <span class="muted small" aria-live="polite" data-testid="bar-readout">
      Bar {targetBar ?? "—"}
    </span>
    <button onclick={() => void undo()} disabled={busy || undoable === null} data-testid="mark-undo">
      Undo (Z)
    </button>
  </div>

  <div class="toolbar" aria-label="Mark classes">
    {#each MARK_CLASSES as markClass (markClass.key)}
      <button
        class:selected={selectedKey === markClass.key}
        aria-pressed={selectedKey === markClass.key}
        onclick={() => void capture(markClass.key)}
        disabled={busy}
        data-testid={`mark-class-${markClass.key}`}
      >
        <span class="key">{markClass.key}</span>
        {markClass.label}
      </button>
    {/each}
  </div>

  <label class="note">
    <span class="muted small">Note (optional, {note.length}/500)</span>
    <textarea
      bind:value={note}
      maxlength="500"
      rows="2"
      placeholder="Why this bar: the visual reason and the event or indicator reason."
      data-testid="mark-note"
    ></textarea>
  </label>

  {#if error}
    <div class="notice error" role="alert" data-testid="mark-error">{error}</div>
  {/if}
  {#if status}
    <p class="muted small" role="status" data-testid="mark-status">{status}</p>
  {/if}

  {#if scene.barCount === 0}
    <div class="notice">This window has no candles.</div>
  {:else}
    <div class="legend muted small" aria-label="Legend">
      <span><i class="swatch fast"></i>EMA 20</span>
      <span><i class="swatch mid"></i>EMA 50</span>
      <span><i class="swatch slow"></i>EMA 200</span>
      <span><i class="swatch level"></i>Confirmed level</span>
      <span><i class="swatch event"></i>Event</span>
    </div>
    <!-- The chart surface is a keyboard application region: arrows choose a bar
         and digits capture a mark. Svelte's static role table does not treat
         `application` as interactive, so both checks are inapplicable here. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <div
      class="chart-frame"
      role="application"
      aria-label={`Simulation chart, window ${series.chart}. Use arrow keys to choose a bar and 1 through 8 to mark it.`}
      tabindex="0"
      data-testid="chart-application"
      onpointermove={onPointerMove}
      onpointerleave={onPointerLeave}
      onclick={onClick}
      onkeydown={onChartKeyDown}
    >
      <svg
        bind:this={surface}
        viewBox={`0 0 ${area.width} ${area.height}`}
        role="img"
        aria-label={`Candles, volume, moving averages, events, levels, and marks for window ${series.chart}.`}
        data-testid="simulation-chart"
      >
        <rect width={area.width} height={area.height} class="background" />

        {#each scene.priceTicks as tick (tick.key)}
          <line
            x1={area.left}
            x2={area.left + area.plotWidth}
            y1={tick.y}
            y2={tick.y}
            class="grid"
          />
          <text
            x={area.left - 8}
            y={tick.y}
            text-anchor="end"
            dominant-baseline="middle"
            class="axis"
          >{tick.label}</text>
        {/each}

        {#each scene.levels as level (level.key)}
          <g
            class="level {level.kindCode === 1 ? 'high' : level.kindCode === -1 ? 'low' : ''} {level.scaleCode === 2 ? 'major' : 'minor'}"
          >
            <line x1={level.x1} x2={level.x2} y1={level.y} y2={level.y} />
            <title>{level.label}</title>
          </g>
        {/each}

        <path d={scene.candles.wicks} class="wicks" />
        <path d={scene.candles.up} class="candle up" />
        <path d={scene.candles.down} class="candle down" />

        <path d={scene.ema.fast} class="ema fast" />
        <path d={scene.ema.mid} class="ema mid" />
        <path d={scene.ema.slow} class="ema slow" />

        <line
          x1={area.left}
          x2={area.left + area.plotWidth}
          y1={area.volumeBottom}
          y2={area.volumeBottom}
          class="separator"
        />
        <path d={scene.volume.up} class="volume up" />
        <path d={scene.volume.down} class="volume down" />
        <text x={area.left - 8} y={area.volumeTop + 10} text-anchor="end" class="axis">
          Vol
        </text>

        <rect
          x={area.left}
          y={area.panelTop}
          width={area.plotWidth}
          height={area.panelHeight}
          class="panel"
        />
        <text x={area.left} y={area.panelTop - 4} class="panel-title">
          {indicatorName(series.panel.indicator_code)}
        </text>
        {#if scene.panel.zeroY !== null}
          <line
            x1={area.left}
            x2={area.left + area.plotWidth}
            y1={scene.panel.zeroY}
            y2={scene.panel.zeroY}
            class="panel-zero"
          />
        {/if}
        <path d={scene.panel.path} class="panel-line" />
        <text
          x={area.left + area.plotWidth}
          y={area.panelTop + 10}
          text-anchor="end"
          class="axis"
        >{scene.panel.maxLabel}</text>
        <text
          x={area.left + area.plotWidth}
          y={area.panelBottom - 4}
          text-anchor="end"
          class="axis"
        >{scene.panel.minLabel}</text>

        <g data-testid="event-layer">
          {#each scene.events as marker (marker.key)}
            <g
              class="event-marker {marker.kind} {marker.direction > 0
                ? 'bullish'
                : marker.direction < 0
                  ? 'bearish'
                  : 'neutral'}"
            >
              {#if marker.kind === "level"}
                <circle cx={marker.x} cy={marker.y} r="3" />
              {:else if marker.kind === "divergence"}
                <path d={diamondPath(marker.x, marker.y)} />
              {:else if marker.kind === "signal"}
                <path d={arrowPath(marker.x, marker.y, marker.direction >= 0)} />
              {:else}
                <rect x={marker.x - 3} y={marker.y - 3} width="6" height="6" />
              {/if}
              <title>{marker.name} — {marker.detail}</title>
            </g>
          {/each}
        </g>

        <g data-testid="mark-layer">
          {#each scene.marks as marker (marker.key)}
            <g
              class="mark {marker.quality} {marker.action}"
              data-mark-id={marker.markId}
              data-mark-class={`${marker.action}-${marker.quality}-${marker.side}`}
            >
              <path d={marker.path} />
              <title>{marker.label}</title>
            </g>
          {/each}
        </g>

        {#if targetBar !== null}
          <line
            x1={barCenter(DEFAULT_LAYOUT, scene.barCount, targetBar)}
            x2={barCenter(DEFAULT_LAYOUT, scene.barCount, targetBar)}
            y1={area.priceTop}
            y2={area.volumeBottom}
            class="crosshair"
          />
          <rect
            x={barCenter(DEFAULT_LAYOUT, scene.barCount, targetBar) - scene.slotWidth / 2}
            y={area.priceTop}
            width={scene.slotWidth}
            height={area.priceHeight}
            class="bar-highlight"
          />
        {/if}
      </svg>
    </div>
    <p class="muted small">
      Point at a bar and press 1–8 to capture a mark; Z removes the last mark from this session.
      The server is the record.
    </p>
  {/if}
</section>

<style>
  .chart-shell {
    max-width: 72rem;
    gap: 0.7rem;
  }

  .toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }

  .toolbar button {
    font-size: 0.8rem;
    padding: 0.35rem 0.55rem;
  }

  .toolbar button.selected {
    border-color: var(--accent);
    color: var(--accent);
  }

  .key {
    font-family: var(--mono);
    color: var(--muted);
    margin-right: 0.25rem;
  }

  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 0.8rem;
    align-items: center;
  }

  .legend span {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
  }

  .swatch {
    display: inline-block;
    width: 16px;
    height: 3px;
    border-radius: 2px;
    background: var(--muted);
  }

  .swatch.fast {
    background: #e3b341;
  }

  .swatch.mid {
    background: #3d8bfd;
  }

  .swatch.slow {
    background: #c58af9;
  }

  .swatch.level {
    background: var(--accent-2);
  }

  .swatch.event {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
  }

  .note {
    display: grid;
    gap: 0.25rem;
  }

  textarea {
    font: inherit;
    color: inherit;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 0.45rem 0.6rem;
    resize: vertical;
  }

  .chart-frame {
    border: 1px solid var(--line);
    border-radius: 10px;
    background: #0b0e13;
    overflow: hidden;
  }

  .chart-frame:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  svg {
    display: block;
    width: 100%;
    height: auto;
  }

  .background {
    fill: #0b0e13;
  }

  .grid {
    stroke: #1b2431;
    stroke-width: 1;
  }

  .separator {
    stroke: var(--line);
    stroke-width: 1;
  }

  .axis {
    fill: var(--muted);
    font-size: 10px;
    font-family: var(--mono);
  }

  .panel-title {
    fill: var(--muted);
    font-size: 10px;
    font-family: var(--mono);
  }

  .wicks {
    stroke: #8b98a9;
    stroke-width: 1;
    fill: none;
  }

  .candle.up {
    fill: #35d0a5;
  }

  .candle.down {
    fill: #ff6b6b;
  }

  .ema {
    fill: none;
    stroke-width: 1.4;
  }

  .ema.fast {
    stroke: #e3b341;
  }

  .ema.mid {
    stroke: #3d8bfd;
  }

  .ema.slow {
    stroke: #c58af9;
  }

  .volume.up {
    fill: rgba(53, 208, 165, 0.45);
  }

  .volume.down {
    fill: rgba(255, 107, 107, 0.45);
  }

  .panel {
    fill: rgba(22, 27, 34, 0.55);
    stroke: #1b2431;
    stroke-width: 1;
  }

  .panel-line {
    fill: none;
    stroke: var(--accent-2);
    stroke-width: 1.4;
  }

  .panel-zero {
    stroke: var(--line);
    stroke-width: 1;
  }

  .level line {
    stroke: var(--muted);
    stroke-width: 1;
    stroke-dasharray: 4 4;
  }

  .level.major line {
    stroke-width: 1.4;
    stroke-dasharray: none;
  }

  .level.high line {
    stroke: var(--accent-2);
  }

  .level.low line {
    stroke: var(--accent);
  }

  .event-marker {
    fill: var(--muted);
  }

  .event-marker.bullish {
    fill: var(--accent);
  }

  .event-marker.bearish {
    fill: var(--error);
  }

  .event-marker.neutral {
    fill: var(--muted);
  }

  .mark.good {
    color: var(--ok);
  }

  .mark.bad {
    color: var(--error);
  }

  .mark.neutral {
    color: var(--muted);
  }

  .mark path {
    fill: currentColor;
  }

  .mark.exit path {
    fill: #0b0e13;
    stroke: currentColor;
    stroke-width: 1.5;
  }

  .crosshair {
    stroke: var(--muted);
    stroke-width: 1;
    stroke-dasharray: 2 3;
    pointer-events: none;
  }

  .bar-highlight {
    fill: rgba(61, 139, 253, 0.07);
    pointer-events: none;
  }
</style>
