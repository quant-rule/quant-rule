//! The multi-panel market chart.
//!
//! The top section shows the selected OHLCV bars and a volume strip; the four
//! sections below show the continuous Direction, Momentum, Structure, and
//! Progression resolutions on one shared x-axis. The renderer copies values from
//! [`ChartSeries`]; it does not compute an axis, an indicator, or an event.

use quantrule_analytics::chart::{ChartBar, ChartSeries};
use quantrule_data::{Timeframe, format_utc_timestamp};

use super::{
    BACKGROUND, Canvas, ChartError, DOJI, DOWN, GRID, LEFT, MUTED, PANEL, PLOT_WIDTH, RIGHT, Rgb,
    TEXT, UP, price_precision, text_right, text_width, time_label,
};

/// The rendered image width in pixels.
const WIDTH: u32 = 1_280;
/// The rendered image height in pixels.
const HEIGHT: u32 = 960;
/// The top of the candle panel.
const CANDLE_TOP: i32 = 72;
/// The bottom of the candle panel.
const CANDLE_BOTTOM: i32 = 400;
/// The top of the volume strip.
const VOLUME_TOP: i32 = 418;
/// The bottom of the volume strip.
const VOLUME_BOTTOM: i32 = 478;
/// The top of the first axis panel.
const AXIS_TOP: i32 = 500;
/// The height of one axis panel.
const AXIS_HEIGHT: i32 = 100;
/// The gap between axis panels.
const AXIS_GAP: i32 = 10;
/// The baseline of the shared x-axis labels.
const LABEL_Y: i32 = 938;
/// The number of horizontal price and axis divisions.
const DIVISIONS: i32 = 4;
/// The number of x-axis labels.
const TIME_LABELS: usize = 5;
/// The widest candle body.
const MAX_BODY_WIDTH: i32 = 24;
/// The zero line inside an axis panel.
const AXIS_ZERO: Rgb = Rgb::new(90, 104, 118);
/// Direction's panel line.
const DIRECTION_COLOR: Rgb = Rgb::new(38, 166, 154);
/// Momentum's panel line.
const MOMENTUM_COLOR: Rgb = Rgb::new(255, 167, 38);
/// Structure's panel line.
const STRUCTURE_COLOR: Rgb = Rgb::new(171, 71, 188);
/// Progression's panel line.
const PROGRESSION_COLOR: Rgb = Rgb::new(102, 187, 106);
/// The four panel labels.
const AXIS_LABELS: [&str; 4] = ["DIRECTION", "MOMENTUM", "STRUCTURE", "PROGRESSION"];
/// The four panel colors, in the same order as `AXIS_LABELS`.
const AXIS_COLORS: [Rgb; 4] = [
    DIRECTION_COLOR,
    MOMENTUM_COLOR,
    STRUCTURE_COLOR,
    PROGRESSION_COLOR,
];

/// Renders the market chart from the selected bars.
///
/// The caller selects the timeframe and the last `n` bars. This function draws
/// exactly the supplied series.
///
/// # Errors
///
/// Returns [`ChartError::Empty`] when the series has no bars and
/// [`ChartError::Encode`] when the PNG encoder fails.
pub(crate) fn render_market_png(
    series: &ChartSeries,
    timeframe: Timeframe,
) -> Result<Vec<u8>, ChartError> {
    if series.is_empty() {
        return Err(ChartError::Empty);
    }

    let mut canvas = Canvas::new(WIDTH, HEIGHT, BACKGROUND);
    draw_market(&mut canvas, series, timeframe)?;
    canvas.encode_png().map_err(ChartError::Encode)
}

fn draw_market(
    canvas: &mut Canvas,
    series: &ChartSeries,
    timeframe: Timeframe,
) -> Result<(), ChartError> {
    let bars = &series.bars;
    let Some(first) = bars.first() else {
        return Err(ChartError::Empty);
    };
    let Some(last) = bars.last() else {
        return Err(ChartError::Empty);
    };

    draw_header(
        canvas,
        timeframe,
        bars.len(),
        first.candle.timestamp(),
        last.candle.timestamp(),
    );
    draw_panels(canvas);
    draw_time_grid(canvas, bars);
    draw_price_panel(canvas, bars);
    draw_volume_panel(canvas, bars);
    for (index, label) in AXIS_LABELS.iter().enumerate() {
        draw_axis_panel(canvas, bars, index, label, AXIS_COLORS[index]);
    }
    draw_borders(canvas);
    Ok(())
}

fn draw_header(canvas: &mut Canvas, timeframe: Timeframe, count: usize, first: i64, last: i64) {
    canvas.text(
        18,
        16,
        &format!("QuantRule {} market chart", timeframe.code()),
        TEXT,
        2,
    );
    canvas.text(
        18,
        44,
        &format!(
            "last {count} bars | {} to {} UTC",
            format_utc_timestamp(first),
            format_utc_timestamp(last)
        ),
        MUTED,
        1,
    );
}

fn draw_panels(canvas: &mut Canvas) {
    canvas.fill_rect(
        LEFT,
        CANDLE_TOP,
        PLOT_WIDTH,
        CANDLE_BOTTOM - CANDLE_TOP,
        PANEL,
    );
    canvas.fill_rect(
        LEFT,
        VOLUME_TOP,
        PLOT_WIDTH,
        VOLUME_BOTTOM - VOLUME_TOP,
        PANEL,
    );
    for index in 0..AXIS_LABELS.len() {
        let top = axis_top(index);
        canvas.fill_rect(LEFT, top, PLOT_WIDTH, AXIS_HEIGHT, PANEL);
    }
}

fn axis_top(index: usize) -> i32 {
    AXIS_TOP + index as i32 * (AXIS_HEIGHT + AXIS_GAP)
}

fn draw_time_grid(canvas: &mut Canvas, bars: &[ChartBar]) {
    let count = bars.len();
    let span = bars.last().zip(bars.first()).map_or(0, |(last, first)| {
        last.candle.timestamp() - first.candle.timestamp()
    });
    let date_only = span > 7 * 86_400;
    let bottom = axis_top(AXIS_LABELS.len() - 1) + AXIS_HEIGHT;

    for label in 0..TIME_LABELS {
        let index = if count <= 1 {
            0
        } else {
            label * (count - 1) / (TIME_LABELS - 1)
        };
        let x = x_center(index, count);
        canvas.line(x, CANDLE_TOP, x, bottom, GRID);
        let text = time_label(bars[index].candle.timestamp(), date_only);
        let width = text_width(&text, 1);
        let text_x = (x - width / 2).clamp(LEFT - 20, RIGHT - width);
        canvas.text(text_x, LABEL_Y, &text, MUTED, 1);
    }
}

fn draw_price_panel(canvas: &mut Canvas, bars: &[ChartBar]) {
    let (min_price, max_price) = price_bounds(bars);
    let precision = price_precision(max_price - min_price);
    for division in 0..=DIVISIONS {
        let ratio = f64::from(division) / f64::from(DIVISIONS);
        let price = max_price - (max_price - min_price) * ratio;
        let y = CANDLE_TOP + ((CANDLE_BOTTOM - CANDLE_TOP) * division) / DIVISIONS;
        canvas.line(LEFT, y, RIGHT, y, GRID);
        text_right(
            canvas,
            LEFT - 8,
            y - 4,
            &format!("{price:.precision$}"),
            TEXT,
            1,
        );
    }

    let count = bars.len().max(1);
    let step = f64::from(PLOT_WIDTH) / count as f64;
    let body_width = ((step * 0.7).floor() as i32).clamp(1, MAX_BODY_WIDTH);
    let half = body_width / 2;
    for (index, bar) in bars.iter().enumerate() {
        let candle = bar.candle;
        let x = x_center(index, count);
        let color = candle_color(&candle);
        let high_y = price_y(candle.high(), min_price, max_price);
        let low_y = price_y(candle.low(), min_price, max_price);
        let open_y = price_y(candle.open(), min_price, max_price);
        let close_y = price_y(candle.close(), min_price, max_price);
        canvas.line(x, high_y, x, low_y, color);
        let top = open_y.min(close_y);
        let bottom = open_y.max(close_y);
        canvas.fill_rect(x - half, top, body_width, (bottom - top).max(1), color);
    }
}

fn draw_volume_panel(canvas: &mut Canvas, bars: &[ChartBar]) {
    let max_volume = bars
        .iter()
        .filter_map(|bar| bar.candle.volume())
        .fold(0.0_f64, f64::max);
    for division in 0..=2 {
        let y = VOLUME_TOP + ((VOLUME_BOTTOM - VOLUME_TOP) * division) / 2;
        canvas.line(LEFT, y, RIGHT, y, GRID);
    }
    if max_volume > 0.0 {
        text_right(
            canvas,
            LEFT - 8,
            VOLUME_TOP + 4,
            &format!("{max_volume:.0}"),
            MUTED,
            1,
        );
    }

    let count = bars.len().max(1);
    let step = f64::from(PLOT_WIDTH) / count as f64;
    let body_width = ((step * 0.7).floor() as i32).clamp(1, MAX_BODY_WIDTH);
    let half = body_width / 2;
    for (index, bar) in bars.iter().enumerate() {
        let Some(volume) = bar.candle.volume() else {
            continue;
        };
        let height = if max_volume > 0.0 {
            ((volume / max_volume) * f64::from(VOLUME_BOTTOM - VOLUME_TOP))
                .round()
                .max(1.0) as i32
        } else {
            1
        };
        let x = x_center(index, count);
        canvas.fill_rect(
            x - half,
            VOLUME_BOTTOM - height.min(VOLUME_BOTTOM - VOLUME_TOP),
            body_width,
            height.min(VOLUME_BOTTOM - VOLUME_TOP),
            candle_color(&bar.candle),
        );
    }
}

fn draw_axis_panel(canvas: &mut Canvas, bars: &[ChartBar], index: usize, label: &str, color: Rgb) {
    let top = axis_top(index);
    let height = AXIS_HEIGHT;
    canvas.text(LEFT + 4, top + 4, label, color, 1);

    for division in 0..=DIVISIONS {
        let ratio = f64::from(division) / f64::from(DIVISIONS);
        let value = 1.0 - 2.0 * ratio;
        let y = top + (height * division) / DIVISIONS;
        canvas.line(LEFT, y, RIGHT, y, GRID);
        text_right(canvas, LEFT - 8, y - 4, &format!("{value:+.1}"), MUTED, 1);
    }
    let zero_y = top + height / 2;
    canvas.line(LEFT, zero_y, RIGHT, zero_y, AXIS_ZERO);

    let value = |bar: &ChartBar| match index {
        0 => bar.direction,
        1 => bar.momentum,
        2 => bar.structure,
        _ => bar.progression,
    };
    let count = bars.len();
    let mut previous: Option<(i32, i32)> = None;
    for (bar_index, bar) in bars.iter().enumerate() {
        let x = x_center(bar_index, count);
        match value(bar) {
            Some(value) => {
                let y = axis_y(value, top, height);
                if let Some((previous_x, previous_y)) = previous {
                    canvas.line(previous_x, previous_y, x, y, color);
                } else {
                    canvas.fill_rect(x - 1, y - 1, 2, 2, color);
                }
                previous = Some((x, y));
            }
            None => {
                previous = None;
            }
        }
    }
}

fn draw_borders(canvas: &mut Canvas) {
    for (top, bottom) in [(CANDLE_TOP, CANDLE_BOTTOM), (VOLUME_TOP, VOLUME_BOTTOM)] {
        canvas.line(LEFT, top, RIGHT, top, GRID);
        canvas.line(LEFT, bottom, RIGHT, bottom, GRID);
        canvas.line(LEFT, top, LEFT, bottom, GRID);
        canvas.line(RIGHT, top, RIGHT, bottom, GRID);
    }
    for index in 0..AXIS_LABELS.len() {
        let top = axis_top(index);
        let bottom = top + AXIS_HEIGHT;
        canvas.line(LEFT, top, RIGHT, top, GRID);
        canvas.line(LEFT, bottom, RIGHT, bottom, GRID);
        canvas.line(LEFT, top, LEFT, bottom, GRID);
        canvas.line(RIGHT, top, RIGHT, bottom, GRID);
    }
}

fn x_center(index: usize, count: usize) -> i32 {
    let count = count.max(1);
    let step = f64::from(PLOT_WIDTH) / count as f64;
    (f64::from(LEFT) + (index as f64 + 0.5) * step).round() as i32
}

fn price_bounds(bars: &[ChartBar]) -> (f64, f64) {
    let mut min_price = f64::INFINITY;
    let mut max_price = f64::NEG_INFINITY;
    for bar in bars {
        min_price = min_price.min(bar.candle.low());
        max_price = max_price.max(bar.candle.high());
    }
    if max_price <= min_price {
        let padding = if min_price == 0.0 {
            1.0
        } else {
            min_price.abs() * 0.01
        };
        min_price -= padding;
        max_price += padding;
    } else {
        let padding = (max_price - min_price) * 0.05;
        min_price -= padding;
        max_price += padding;
    }
    (min_price, max_price)
}

fn price_y(price: f64, min_price: f64, max_price: f64) -> i32 {
    let span = max_price - min_price;
    if span <= 0.0 {
        return CANDLE_TOP + (CANDLE_BOTTOM - CANDLE_TOP) / 2;
    }
    let offset = (max_price - price) / span * f64::from(CANDLE_BOTTOM - CANDLE_TOP);
    (f64::from(CANDLE_TOP) + offset).round() as i32
}

fn axis_y(value: f64, top: i32, height: i32) -> i32 {
    let value = value.clamp(-1.0, 1.0);
    let offset = (1.0 - value) / 2.0 * f64::from(height);
    (f64::from(top) + offset).round() as i32
}

fn candle_color(candle: &quantrule_data::Candle) -> Rgb {
    if candle.close() > candle.open() {
        UP
    } else if candle.close() < candle.open() {
        DOWN
    } else {
        DOJI
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use quantrule_analytics::chart::{ChartBar, ChartSeries};
    use quantrule_data::{Candle, Timeframe};

    use super::super::ChartError;
    use super::render_market_png;

    fn candle(index: usize, close: f64) -> Candle {
        Candle::new(
            index as u64,
            1_700_000_000 + index as i64 * 60,
            10.0,
            11.0,
            9.0,
            close,
            Some(10.0),
        )
        .expect("the test candle is valid")
    }

    fn series() -> ChartSeries {
        ChartSeries {
            timeframe_secs: 60,
            bars: vec![
                ChartBar {
                    bar_index: 0,
                    candle: candle(0, 10.5),
                    direction: Some(0.2),
                    momentum: Some(-0.1),
                    structure: None,
                    progression: Some(0.4),
                },
                ChartBar {
                    bar_index: 1,
                    candle: candle(1, 10.0),
                    direction: Some(0.5),
                    momentum: None,
                    structure: Some(-0.3),
                    progression: None,
                },
            ],
        }
    }

    #[test]
    fn the_market_chart_has_the_expected_dimensions() {
        let bytes = render_market_png(&series(), Timeframe::M1).expect("the chart renders");
        let decoder = png::Decoder::new(Cursor::new(bytes));
        let reader = decoder.read_info().expect("the PNG header is valid");
        assert_eq!(reader.info().width, 1_280);
        assert_eq!(reader.info().height, 960);
        assert_eq!(reader.info().color_type, png::ColorType::Rgb);
    }

    #[test]
    fn the_market_chart_is_deterministic() {
        let first = render_market_png(&series(), Timeframe::M1).expect("the first chart");
        let second = render_market_png(&series(), Timeframe::M1).expect("the second chart");
        assert_eq!(first, second);
    }

    #[test]
    fn an_empty_series_is_an_error() {
        let empty = ChartSeries {
            timeframe_secs: 60,
            bars: Vec::new(),
        };
        assert!(matches!(
            render_market_png(&empty, Timeframe::M1),
            Err(ChartError::Empty)
        ));
    }

    #[test]
    fn one_bar_with_missing_axes_renders() {
        let one = ChartSeries {
            timeframe_secs: 60,
            bars: vec![ChartBar {
                bar_index: 0,
                candle: candle(0, 10.5),
                direction: None,
                momentum: None,
                structure: None,
                progression: None,
            }],
        };
        assert!(render_market_png(&one, Timeframe::M1).is_ok());
    }
}
