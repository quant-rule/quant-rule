//! The two full-period market-report charts.
//!
//! Both charts draw the report's precomputed buckets and event markers. The
//! renderer copies values from [`ChartReport`]; it does not compute a bucket, an
//! axis, an event count, or an event position. The same report therefore renders
//! the same PNG bytes.

use quantrule_data::format_utc_timestamp;
use quantrule_model::report::{ChartReport, ReportChartBar, ReportChartEvent};

use super::{
    BACKGROUND, Canvas, ChartError, DOJI, DOWN, GRID, LEFT, MUTED, PANEL, PLOT_WIDTH, RIGHT, Rgb,
    TEXT, UP, price_precision, text_right, text_width, time_label,
};
use crate::labels;

/// The price-and-axes image width in pixels.
const PRICE_AXES_WIDTH: u32 = 1_280;
/// The price-and-axes image height in pixels.
const PRICE_AXES_HEIGHT: u32 = 960;
/// The events image width in pixels.
const EVENTS_WIDTH: u32 = 1_280;
/// The events image height in pixels.
const EVENTS_HEIGHT: u32 = 720;

/// The top of the price panel in the price-and-axes chart.
const PRICE_AXES_CANDLE_TOP: i32 = 72;
/// The bottom of the price panel in the price-and-axes chart.
const PRICE_AXES_CANDLE_BOTTOM: i32 = 400;
/// The top of the volume strip in the price-and-axes chart.
const PRICE_AXES_VOLUME_TOP: i32 = 418;
/// The bottom of the volume strip in the price-and-axes chart.
const PRICE_AXES_VOLUME_BOTTOM: i32 = 478;
/// The top of the first axis panel in the price-and-axes chart.
const PRICE_AXES_AXIS_TOP: i32 = 500;
/// The height of one axis panel.
const PRICE_AXES_AXIS_HEIGHT: i32 = 100;
/// The gap between axis panels.
const PRICE_AXES_AXIS_GAP: i32 = 10;
/// The baseline of the shared x-axis labels in the price-and-axes chart.
const PRICE_AXES_LABEL_Y: i32 = 938;

/// The top of the price panel in the events chart.
const EVENTS_PRICE_TOP: i32 = 72;
/// The bottom of the price panel in the events chart.
const EVENTS_PRICE_BOTTOM: i32 = 492;
/// The top of the volume strip in the events chart.
const EVENTS_VOLUME_TOP: i32 = 522;
/// The bottom of the volume strip in the events chart.
const EVENTS_VOLUME_BOTTOM: i32 = 640;
/// The baseline of the x-axis labels in the events chart.
const EVENTS_LABEL_Y: i32 = 652;

/// The number of horizontal price and axis divisions.
const DIVISIONS: i32 = 4;
/// The number of x-axis labels.
const TIME_LABELS: usize = 5;
/// The widest candle body.
const MAX_BODY_WIDTH: i32 = 24;
/// The zero line inside a continuous axis panel.
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
/// The marker radius in the events chart.
const MARKER_RADIUS: i32 = 4;

/// The panel bounds shared by the two charts.
struct Geometry {
    price_top: i32,
    price_bottom: i32,
    volume_top: i32,
    volume_bottom: i32,
    label_y: i32,
    grid_bottom: i32,
}

/// Renders `price-axes.png`.
///
/// # Errors
///
/// Returns [`ChartError::Empty`] when the report has no chart bars and
/// [`ChartError::Encode`] when the PNG encoder fails.
pub(crate) fn render_report_price_axes_png(report: &ChartReport) -> Result<Vec<u8>, ChartError> {
    if report.bars.is_empty() {
        return Err(ChartError::Empty);
    }

    let mut canvas = Canvas::new(PRICE_AXES_WIDTH, PRICE_AXES_HEIGHT, BACKGROUND);
    draw_price_axes(&mut canvas, report)?;
    canvas.encode_png().map_err(ChartError::Encode)
}

/// Renders `events.png`.
///
/// # Errors
///
/// Returns [`ChartError::Empty`] when the report has no chart bars and
/// [`ChartError::Encode`] when the PNG encoder fails.
pub(crate) fn render_report_events_png(report: &ChartReport) -> Result<Vec<u8>, ChartError> {
    if report.bars.is_empty() {
        return Err(ChartError::Empty);
    }

    let mut canvas = Canvas::new(EVENTS_WIDTH, EVENTS_HEIGHT, BACKGROUND);
    draw_events(&mut canvas, report)?;
    canvas.encode_png().map_err(ChartError::Encode)
}

fn draw_price_axes(canvas: &mut Canvas, report: &ChartReport) -> Result<(), ChartError> {
    let geometry = Geometry {
        price_top: PRICE_AXES_CANDLE_TOP,
        price_bottom: PRICE_AXES_CANDLE_BOTTOM,
        volume_top: PRICE_AXES_VOLUME_TOP,
        volume_bottom: PRICE_AXES_VOLUME_BOTTOM,
        label_y: PRICE_AXES_LABEL_Y,
        grid_bottom: axis_top(AXIS_LABELS.len() - 1) + PRICE_AXES_AXIS_HEIGHT,
    };

    draw_header(canvas, report, "market-report");
    draw_panels(canvas, &geometry);
    for index in 0..AXIS_LABELS.len() {
        let top = axis_top(index);
        canvas.fill_rect(LEFT, top, PLOT_WIDTH, PRICE_AXES_AXIS_HEIGHT, PANEL);
    }
    draw_time_grid(canvas, &report.bars, &geometry);
    draw_price_panel(canvas, &report.bars, &geometry);
    draw_volume_panel(canvas, &report.bars, &geometry);
    for (index, label) in AXIS_LABELS.iter().enumerate() {
        draw_axis_panel(canvas, &report.bars, index, label, AXIS_COLORS[index]);
    }
    draw_borders(canvas, &geometry);
    Ok(())
}

fn draw_events(canvas: &mut Canvas, report: &ChartReport) -> Result<(), ChartError> {
    let geometry = Geometry {
        price_top: EVENTS_PRICE_TOP,
        price_bottom: EVENTS_PRICE_BOTTOM,
        volume_top: EVENTS_VOLUME_TOP,
        volume_bottom: EVENTS_VOLUME_BOTTOM,
        label_y: EVENTS_LABEL_Y,
        grid_bottom: EVENTS_VOLUME_BOTTOM,
    };

    draw_header(canvas, report, "market-report events");
    draw_panels(canvas, &geometry);
    draw_time_grid(canvas, &report.bars, &geometry);
    draw_price_panel(canvas, &report.bars, &geometry);
    draw_volume_panel(canvas, &report.bars, &geometry);
    draw_borders(canvas, &geometry);
    draw_event_markers(canvas, report);
    draw_event_legend(canvas);
    Ok(())
}

fn draw_header(canvas: &mut Canvas, report: &ChartReport, title: &str) {
    let timeframe = report.timeframe_secs.map_or_else(
        || String::from("-"),
        |seconds| labels::timeframe_label(seconds).to_owned(),
    );
    canvas.text(18, 16, &format!("QuantRule {title} {timeframe}"), TEXT, 2);

    let first = report
        .bars
        .first()
        .map(|bar| format_utc_timestamp(bar.open_time))
        .unwrap_or_else(|| String::from("-"));
    let last = report
        .bars
        .last()
        .map(|bar| format_utc_timestamp(bar.last_open_time))
        .unwrap_or_else(|| String::from("-"));
    canvas.text(
        18,
        44,
        &format!(
            "{} source bars | {} buckets | {} to {} UTC",
            report.source_bars,
            report.bars.len(),
            first,
            last
        ),
        MUTED,
        1,
    );
}

fn draw_panels(canvas: &mut Canvas, geometry: &Geometry) {
    canvas.fill_rect(
        LEFT,
        geometry.price_top,
        PLOT_WIDTH,
        geometry.price_bottom - geometry.price_top,
        PANEL,
    );
    canvas.fill_rect(
        LEFT,
        geometry.volume_top,
        PLOT_WIDTH,
        geometry.volume_bottom - geometry.volume_top,
        PANEL,
    );
}

fn axis_top(index: usize) -> i32 {
    PRICE_AXES_AXIS_TOP + index as i32 * (PRICE_AXES_AXIS_HEIGHT + PRICE_AXES_AXIS_GAP)
}

fn draw_time_grid(canvas: &mut Canvas, bars: &[ReportChartBar], geometry: &Geometry) {
    let count = bars.len();
    let span = bars
        .last()
        .zip(bars.first())
        .map_or(0, |(last, first)| last.last_open_time - first.open_time);
    let date_only = span > 7 * 86_400;

    for label in 0..TIME_LABELS {
        let index = if count <= 1 {
            0
        } else {
            label * (count - 1) / (TIME_LABELS - 1)
        };
        let x = x_center(index, count);
        canvas.line(x, geometry.price_top, x, geometry.grid_bottom, GRID);
        let text = time_label(bars[index].open_time, date_only);
        let width = text_width(&text, 1);
        let text_x = (x - width / 2).clamp(LEFT - 20, RIGHT - width);
        canvas.text(text_x, geometry.label_y, &text, MUTED, 1);
    }
}

fn draw_price_panel(canvas: &mut Canvas, bars: &[ReportChartBar], geometry: &Geometry) {
    let (min_price, max_price) = price_bounds(bars);
    let precision = price_precision(max_price - min_price);
    let height = geometry.price_bottom - geometry.price_top;
    for division in 0..=DIVISIONS {
        let ratio = f64::from(division) / f64::from(DIVISIONS);
        let price = max_price - (max_price - min_price) * ratio;
        let y = geometry.price_top + (height * division) / DIVISIONS;
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
        let x = x_center(index, count);
        let color = candle_color(bar);
        let high_y = price_y(bar.high, min_price, max_price, geometry);
        let low_y = price_y(bar.low, min_price, max_price, geometry);
        let open_y = price_y(bar.open, min_price, max_price, geometry);
        let close_y = price_y(bar.close, min_price, max_price, geometry);
        canvas.line(x, high_y, x, low_y, color);
        let top = open_y.min(close_y);
        let bottom = open_y.max(close_y);
        canvas.fill_rect(x - half, top, body_width, (bottom - top).max(1), color);
    }
}

fn draw_volume_panel(canvas: &mut Canvas, bars: &[ReportChartBar], geometry: &Geometry) {
    let max_volume = bars
        .iter()
        .filter_map(|bar| bar.volume)
        .fold(0.0_f64, f64::max);
    let height = geometry.volume_bottom - geometry.volume_top;
    for division in 0..=2 {
        let y = geometry.volume_top + (height * division) / 2;
        canvas.line(LEFT, y, RIGHT, y, GRID);
    }
    if max_volume > 0.0 {
        text_right(
            canvas,
            LEFT - 8,
            geometry.volume_top + 4,
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
        let Some(volume) = bar.volume else {
            continue;
        };
        let bar_height = if max_volume > 0.0 {
            ((volume / max_volume) * f64::from(height)).round().max(1.0) as i32
        } else {
            1
        };
        let x = x_center(index, count);
        canvas.fill_rect(
            x - half,
            geometry.volume_bottom - bar_height.min(height),
            body_width,
            bar_height.min(height),
            candle_color(bar),
        );
    }
}

fn draw_axis_panel(
    canvas: &mut Canvas,
    bars: &[ReportChartBar],
    index: usize,
    label: &str,
    color: Rgb,
) {
    let top = axis_top(index);
    let height = PRICE_AXES_AXIS_HEIGHT;
    canvas.text(LEFT + 4, top + 4, label, color, 1);

    if index == 3 {
        // Progress is unclamped; the panel fixes the domain at -1..+2 and marks
        // a value outside it. The reference lines are the origin (0) and the
        // target (1).
        for division in 0..=3 {
            let y = top + (height * division) / 3;
            canvas.line(LEFT, y, RIGHT, y, GRID);
            let value = 2.0 - f64::from(division);
            text_right(canvas, LEFT - 8, y - 4, &format!("{value:+.1}"), MUTED, 1);
            if division == 1 || division == 2 {
                canvas.line(LEFT, y, RIGHT, y, AXIS_ZERO);
            }
        }
    } else {
        for division in 0..=DIVISIONS {
            let ratio = f64::from(division) / f64::from(DIVISIONS);
            let value = 1.0 - 2.0 * ratio;
            let y = top + (height * division) / DIVISIONS;
            canvas.line(LEFT, y, RIGHT, y, GRID);
            text_right(canvas, LEFT - 8, y - 4, &format!("{value:+.1}"), MUTED, 1);
        }
        let zero_y = top + height / 2;
        canvas.line(LEFT, zero_y, RIGHT, zero_y, AXIS_ZERO);
    }

    let count = bars.len();
    let mut previous: Option<(i32, i32)> = None;
    for (bar_index, bar) in bars.iter().enumerate() {
        let x = x_center(bar_index, count);
        let plotted = match index {
            0 => bar
                .direction
                .map(|value| (axis_y(value, top, height), false)),
            1 => bar
                .momentum
                .map(|value| (axis_y(value, top, height), false)),
            2 => bar
                .structure
                .map(|value| (axis_y(value, top, height), false)),
            _ => bar
                .progression_progress
                .map(|value| progress_point(value, top, height)),
        };
        match plotted {
            Some((y, clipped)) => {
                if let Some((previous_x, previous_y)) = previous {
                    canvas.line(previous_x, previous_y, x, y, color);
                } else {
                    canvas.fill_rect(x - 1, y - 1, 2, 2, color);
                }
                if clipped {
                    let edge = if y <= top { top } else { top + height - 2 };
                    canvas.fill_rect(x - 1, edge, 3, 2, color);
                }
                previous = Some((x, y));
            }
            None => {
                previous = None;
            }
        }
    }
}

fn draw_borders(canvas: &mut Canvas, geometry: &Geometry) {
    for (top, bottom) in [
        (geometry.price_top, geometry.price_bottom),
        (geometry.volume_top, geometry.volume_bottom),
    ] {
        canvas.line(LEFT, top, RIGHT, top, GRID);
        canvas.line(LEFT, bottom, RIGHT, bottom, GRID);
        canvas.line(LEFT, top, LEFT, bottom, GRID);
        canvas.line(RIGHT, top, RIGHT, bottom, GRID);
    }
    if geometry.grid_bottom > geometry.volume_bottom {
        for index in 0..AXIS_LABELS.len() {
            let top = axis_top(index);
            let bottom = top + PRICE_AXES_AXIS_HEIGHT;
            canvas.line(LEFT, top, RIGHT, top, GRID);
            canvas.line(LEFT, bottom, RIGHT, bottom, GRID);
            canvas.line(LEFT, top, LEFT, bottom, GRID);
            canvas.line(RIGHT, top, RIGHT, bottom, GRID);
        }
    }
}

fn draw_event_markers(canvas: &mut Canvas, report: &ChartReport) {
    let count = report.bars.len();
    let geometry = Geometry {
        price_top: EVENTS_PRICE_TOP,
        price_bottom: EVENTS_PRICE_BOTTOM,
        volume_top: EVENTS_VOLUME_TOP,
        volume_bottom: EVENTS_VOLUME_BOTTOM,
        label_y: EVENTS_LABEL_Y,
        grid_bottom: EVENTS_VOLUME_BOTTOM,
    };
    let (min_price, max_price) = price_bounds(&report.bars);

    for event in &report.events {
        let Some(bar) = report.bars.get(event.bucket_index) else {
            continue;
        };
        let x = x_center(event.bucket_index, count);
        let y = price_y(bar.close, min_price, max_price, &geometry);
        let color = direction_color(event);

        match event.event_type_code {
            1 => fill_circle(canvas, x, y, MARKER_RADIUS, color),
            2 => fill_triangle(canvas, x, y, MARKER_RADIUS, color),
            3 => canvas.fill_rect(
                x - MARKER_RADIUS,
                y - MARKER_RADIUS,
                MARKER_RADIUS * 2 + 1,
                MARKER_RADIUS * 2 + 1,
                color,
            ),
            _ => {}
        }
    }
}

fn draw_event_legend(canvas: &mut Canvas) {
    let row_one = 666;
    let row_two = 686;

    let mut x = LEFT;
    fill_circle(canvas, x + 4, row_one + 4, 3, TEXT);
    x += 14;
    canvas.text(x, row_one, "LEVEL", MUTED, 1);
    x += 62;
    fill_triangle(canvas, x + 4, row_one + 4, 3, TEXT);
    x += 14;
    canvas.text(x, row_one, "DIVERGENCE", MUTED, 1);
    x += 104;
    canvas.fill_rect(x, row_one, 7, 7, TEXT);
    x += 14;
    canvas.text(x, row_one, "SIGNAL", MUTED, 1);

    let mut x = LEFT;
    canvas.fill_rect(x, row_two, 7, 7, UP);
    x += 14;
    canvas.text(x, row_two, "BULLISH", MUTED, 1);
    x += 76;
    canvas.fill_rect(x, row_two, 7, 7, DOWN);
    x += 14;
    canvas.text(x, row_two, "BEARISH", MUTED, 1);
    x += 76;
    canvas.fill_rect(x, row_two, 7, 7, DOJI);
    x += 14;
    canvas.text(x, row_two, "NEUTRAL", MUTED, 1);
}

fn direction_color(event: &ReportChartEvent) -> Rgb {
    match event.direction_code {
        code if code > 0 => UP,
        code if code < 0 => DOWN,
        _ => DOJI,
    }
}

fn fill_circle(canvas: &mut Canvas, center_x: i32, center_y: i32, radius: i32, color: Rgb) {
    for offset_y in -radius..=radius {
        for offset_x in -radius..=radius {
            if offset_x * offset_x + offset_y * offset_y <= radius * radius {
                canvas.set(center_x + offset_x, center_y + offset_y, color);
            }
        }
    }
}

fn fill_triangle(canvas: &mut Canvas, center_x: i32, center_y: i32, radius: i32, color: Rgb) {
    for offset_y in -radius..=radius {
        let half = (offset_y + radius) / 2;
        canvas.fill_rect(center_x - half, center_y + offset_y, half * 2 + 1, 1, color);
    }
}

fn x_center(index: usize, count: usize) -> i32 {
    let count = count.max(1);
    let step = f64::from(PLOT_WIDTH) / count as f64;
    (f64::from(LEFT) + (index as f64 + 0.5) * step).round() as i32
}

fn price_bounds(bars: &[ReportChartBar]) -> (f64, f64) {
    let mut min_price = f64::INFINITY;
    let mut max_price = f64::NEG_INFINITY;
    for bar in bars {
        min_price = min_price.min(bar.low);
        max_price = max_price.max(bar.high);
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

fn price_y(price: f64, min_price: f64, max_price: f64, geometry: &Geometry) -> i32 {
    let span = max_price - min_price;
    let height = geometry.price_bottom - geometry.price_top;
    if span <= 0.0 {
        return geometry.price_top + height / 2;
    }
    let offset = (max_price - price) / span * f64::from(height);
    (f64::from(geometry.price_top) + offset).round() as i32
}

fn axis_y(value: f64, top: i32, height: i32) -> i32 {
    let value = value.clamp(-1.0, 1.0);
    let offset = (1.0 - value) / 2.0 * f64::from(height);
    (f64::from(top) + offset).round() as i32
}

fn progress_y(value: f64, top: i32, height: i32) -> i32 {
    top + (((2.0 - value) / 3.0) * f64::from(height)).round() as i32
}

/// The panel position of a progress value and whether it was clipped.
///
/// The panel fixes the domain at `-1..+2`; a value outside it is drawn at the
/// edge and marked, so the marker never implies a value the model did not
/// publish.
fn progress_point(value: f64, top: i32, height: i32) -> (i32, bool) {
    let clamped = value.clamp(-1.0, 2.0);
    (progress_y(clamped, top, height), clamped != value)
}

fn candle_color(bar: &ReportChartBar) -> Rgb {
    if bar.close > bar.open {
        UP
    } else if bar.close < bar.open {
        DOWN
    } else {
        DOJI
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use quantrule_model::report::{ChartReport, ReportChartBar, ReportChartEvent};

    use super::{progress_point, render_report_events_png, render_report_price_axes_png};
    use crate::chart::ChartError;

    fn bar(index: usize, close: f64) -> ReportChartBar {
        ReportChartBar {
            first_bar_index: index,
            last_bar_index: index,
            open_time: 1_735_689_600 + index as i64 * 3_600,
            last_open_time: 1_735_689_600 + index as i64 * 3_600,
            open: close - 0.5,
            high: close + 1.0,
            low: close - 1.0,
            close,
            volume: Some(100.0 + index as f64),
            direction: Some(0.2),
            momentum: Some(-0.1),
            structure: Some(0.3),
            progression_stage_code: Some(2),
            progression_progress: Some(0.6),
        }
    }

    fn report() -> ChartReport {
        ChartReport {
            timeframe_secs: Some(3_600),
            source_bars: 3,
            bucket_size: 1,
            bars: vec![bar(0, 10.5), bar(1, 10.0), bar(2, 11.0)],
            events: vec![
                ReportChartEvent {
                    bucket_index: 0,
                    available_bar_index: 0,
                    occurred_bar_index: 0,
                    event_type_code: 1,
                    event_subtype_code: 1,
                    direction_code: 1,
                },
                ReportChartEvent {
                    bucket_index: 1,
                    available_bar_index: 1,
                    occurred_bar_index: 1,
                    event_type_code: 2,
                    event_subtype_code: 1,
                    direction_code: -1,
                },
                ReportChartEvent {
                    bucket_index: 2,
                    available_bar_index: 2,
                    occurred_bar_index: 2,
                    event_type_code: 3,
                    event_subtype_code: 1,
                    direction_code: 0,
                },
            ],
        }
    }

    #[test]
    fn the_progress_panel_maps_the_fixed_domain_and_marks_clipping() {
        assert_eq!(progress_point(2.0, 0, 300), (0, false));
        assert_eq!(progress_point(1.0, 0, 300), (100, false));
        assert_eq!(progress_point(0.0, 0, 300), (200, false));
        assert_eq!(progress_point(-1.0, 0, 300), (300, false));
        assert_eq!(progress_point(9.0, 0, 300), (0, true));
        assert_eq!(progress_point(-9.0, 0, 300), (300, true));
    }

    #[test]
    fn the_price_axes_chart_has_the_expected_dimensions() {
        let bytes = render_report_price_axes_png(&report()).expect("the chart renders");
        let decoder = png::Decoder::new(Cursor::new(bytes));
        let reader = decoder.read_info().expect("the PNG header is valid");
        assert_eq!(reader.info().width, 1_280);
        assert_eq!(reader.info().height, 960);
        assert_eq!(reader.info().color_type, png::ColorType::Rgb);
    }

    #[test]
    fn the_events_chart_has_the_expected_dimensions() {
        let bytes = render_report_events_png(&report()).expect("the chart renders");
        let decoder = png::Decoder::new(Cursor::new(bytes));
        let reader = decoder.read_info().expect("the PNG header is valid");
        assert_eq!(reader.info().width, 1_280);
        assert_eq!(reader.info().height, 720);
        assert_eq!(reader.info().color_type, png::ColorType::Rgb);
    }

    #[test]
    fn repeated_renders_are_byte_identical() {
        let first = render_report_price_axes_png(&report()).expect("the first chart");
        let second = render_report_price_axes_png(&report()).expect("the second chart");
        assert_eq!(first, second);

        let first = render_report_events_png(&report()).expect("the first chart");
        let second = render_report_events_png(&report()).expect("the second chart");
        assert_eq!(first, second);
    }

    #[test]
    fn an_empty_chart_is_an_error() {
        let empty = ChartReport {
            timeframe_secs: None,
            source_bars: 0,
            bucket_size: 1,
            bars: Vec::new(),
            events: Vec::new(),
        };
        assert!(matches!(
            render_report_price_axes_png(&empty),
            Err(ChartError::Empty)
        ));
        assert!(matches!(
            render_report_events_png(&empty),
            Err(ChartError::Empty)
        ));
    }
}
