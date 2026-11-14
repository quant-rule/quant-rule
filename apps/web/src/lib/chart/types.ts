/**
 * The simulation chart's input and output types.
 *
 * The series endpoint returns numbers and codes only. The chart keeps those
 * shapes verbatim and maps a code to a word at render time through
 * `labels.generated.ts`; it never invents a reading.
 */

/** Whether a mark records an entry or an exit. */
export type MarkAction = "entry" | "exit";

/** Whether a mark is a good or bad example; the oracle may also be neutral. */
export type MarkQuality = "good" | "bad" | "neutral";

/** The direction a mark refers to. */
export type Side = "long" | "short";

/** Who produced a mark. */
export type MarkOrigin = "human" | "oracle";

/** One candle in a simulation window. */
export interface ChartCandle {
  bar: number;
  open: number;
  high: number;
  low: number;
  close: number;
  /** A missing volume is null and is distinct from zero. */
  volume: number | null;
}

/** One published event, in the model's numeric record shape. */
export interface ChartEvent {
  event_code: number;
  event_type: number;
  event_subtype_code: number;
  source_code: number;
  occurred_bar_index: number;
  available_bar_index: number;
  /** -1 bearish, 0 neutral, +1 bullish. */
  direction_code: number;
}

/** One confirmed swing level. */
export interface ChartLevel {
  reference_code: number;
  scale_code: number;
  kind_code: number;
  price: number;
  occurred_bar_index: number;
  available_bar_index: number;
}

/** The one indicator panel a window carries. */
export interface IndicatorPanel {
  indicator_code: number;
  values: (number | null)[];
}

/** The series for one chart window, as the endpoint publishes it. */
export interface ChartSeries {
  sim_id: string;
  chart: number;
  candles: ChartCandle[];
  ema_fast: (number | null)[];
  ema_mid: (number | null)[];
  ema_slow: (number | null)[];
  panel: IndicatorPanel;
  events: ChartEvent[];
  levels: ChartLevel[];
}

/** The fields a human supplies for one mark. */
export interface MarkDraft {
  chart: number;
  bar: number;
  action: MarkAction;
  quality: MarkQuality;
  side: Side;
  note: string;
}

/** One acknowledged or loaded mark. */
export interface ChartMark extends MarkDraft {
  mark_id: string;
  origin: MarkOrigin;
  marked_time_utc?: string;
}
