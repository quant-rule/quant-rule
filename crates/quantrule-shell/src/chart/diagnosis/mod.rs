//! The five deterministic diagnosis pages.
//!
//! Every page copies numbers from [`quantrule_analytics::diagnosis::types::Diagnosis`].
//! The renderer chooses a scale and a position; it computes no statistic. The
//! embedded bitmap font and the fixed palette make the same diagnosis render
//! the same PNG bytes on every run.

mod attribution;
mod execution;
mod funnel;
mod quality;
mod scorecard;

use quantrule_analytics::diagnosis::types::{CheckStatus, Diagnosis};

use super::{BACKGROUND, Canvas, ChartError, GRID, MUTED, PANEL, Rgb, TEXT};

pub(super) const WIDTH: u32 = 1_280;
pub(super) const HEIGHT: u32 = 720;
pub(super) const MARGIN: i32 = 32;

pub(super) const PASS: Rgb = Rgb::new(38, 166, 154);
pub(super) const FAIL: Rgb = Rgb::new(239, 83, 80);
pub(super) const UNKNOWN: Rgb = Rgb::new(255, 167, 38);
pub(super) const ACCENT: Rgb = Rgb::new(66, 165, 245);
pub(super) const SECOND: Rgb = Rgb::new(171, 71, 188);

/// The artifact name and rendered bytes of one page.
pub(crate) type Page = (&'static str, Vec<u8>);

/// Renders every diagnosis page, in contract order.
///
/// # Errors
///
/// Returns [`ChartError::Encode`] when the PNG encoder fails.
pub(crate) fn render_pages(diagnosis: &Diagnosis) -> Result<Vec<Page>, ChartError> {
    Ok(vec![
        ("00-run-scorecard.png", scorecard::render(diagnosis)?),
        ("10-data-quality.png", quality::render(diagnosis)?),
        ("30-strategy-funnel.png", funnel::render(diagnosis)?),
        ("40-execution-quality.png", execution::render(diagnosis)?),
        (
            "50-portfolio-attribution.png",
            attribution::render(diagnosis)?,
        ),
    ])
}

pub(super) fn base(title: &str, subtitle: &str) -> Canvas {
    let mut canvas = Canvas::new(WIDTH, HEIGHT, BACKGROUND);
    canvas.text(MARGIN, 18, title, TEXT, 2);
    canvas.text(MARGIN, 46, subtitle, MUTED, 1);
    canvas
}

pub(super) fn panel(canvas: &mut Canvas, x: i32, y: i32, width: i32, height: i32, title: &str) {
    canvas.fill_rect(x, y, width, height, PANEL);
    canvas.line(x, y, x + width, y, GRID);
    canvas.line(x, y + height, x + width, y + height, GRID);
    canvas.line(x, y, x, y + height, GRID);
    canvas.line(x + width, y, x + width, y + height, GRID);
    canvas.text(x + 8, y + 8, title, TEXT, 1);
}

/// A chart rectangle.
#[derive(Debug, Clone, Copy)]
pub(super) struct Rect {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) width: i32,
    pub(super) height: i32,
}

impl Rect {
    pub(super) const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

pub(super) fn status_color(status: CheckStatus) -> Rgb {
    match status {
        CheckStatus::Pass => PASS,
        CheckStatus::Fail => FAIL,
        CheckStatus::Unknown => UNKNOWN,
    }
}

pub(super) fn draw_status(canvas: &mut Canvas, x: i32, y: i32, status: CheckStatus, label: &str) {
    let color = status_color(status);
    canvas.fill_rect(x, y, 10, 10, color);
    canvas.text(x + 16, y + 1, label, color, 1);
}

pub(super) fn draw_bar_h(canvas: &mut Canvas, rect: Rect, value: f64, max: f64, color: Rgb) {
    canvas.fill_rect(
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        Rgb::new(31, 39, 47),
    );
    if max <= 0.0 || !value.is_finite() {
        return;
    }
    let filled = ((value.abs() / max) * f64::from(rect.width))
        .round()
        .clamp(0.0, f64::from(rect.width)) as i32;
    canvas.fill_rect(rect.x, rect.y, filled, rect.height, color);
}

pub(super) fn draw_bar_v(
    canvas: &mut Canvas,
    baseline: i32,
    rect: Rect,
    value: f64,
    max: f64,
    color: Rgb,
) {
    if max <= 0.0 || !value.is_finite() {
        return;
    }
    let extent = ((value.abs() / max) * f64::from(rect.height))
        .round()
        .clamp(0.0, f64::from(rect.height)) as i32;
    if value >= 0.0 {
        canvas.fill_rect(rect.x, baseline - extent, rect.width, extent, color);
    } else {
        canvas.fill_rect(rect.x, baseline, rect.width, extent, color);
    }
}

pub(super) fn draw_series(
    canvas: &mut Canvas,
    points: &[(i64, f64)],
    rect: Rect,
    min: f64,
    max: f64,
    color: Rgb,
) {
    if points.is_empty() || rect.width <= 0 || rect.height <= 0 || max <= min {
        return;
    }
    let step = if points.len() <= 1 {
        0.0
    } else {
        f64::from(rect.width - 1) / (points.len() - 1) as f64
    };
    let mut previous = None;
    for (index, (_, value)) in points.iter().enumerate() {
        let px = rect.x + (index as f64 * step).round() as i32;
        let py = rect.y
            + (((max - value) / (max - min)) * f64::from(rect.height))
                .round()
                .clamp(0.0, f64::from(rect.height)) as i32;
        if let Some((previous_x, previous_y)) = previous {
            canvas.line(previous_x, previous_y, px, py, color);
        } else {
            canvas.fill_rect(px - 1, py - 1, 2, 2, color);
        }
        previous = Some((px, py));
    }
}

pub(super) fn series_bounds(series: &[&[(i64, f64)]]) -> (f64, f64) {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for points in series {
        for (_, value) in *points {
            min = min.min(*value);
            max = max.max(*value);
        }
    }
    if !min.is_finite() || !max.is_finite() {
        return (0.0, 1.0);
    }
    if max <= min {
        let padding = if min == 0.0 { 1.0 } else { min.abs() * 0.01 };
        return (min - padding, max + padding);
    }
    let padding = (max - min) * 0.06;
    (min - padding, max + padding)
}

pub(super) fn money(value: f64) -> String {
    format!("{:.2}", if value == 0.0 { 0.0 } else { value })
}

pub(super) fn percent(value: f64) -> String {
    format!("{:.2}%", if value == 0.0 { 0.0 } else { value * 100.0 })
}

pub(super) fn short_share(value: Option<f64>) -> String {
    value.map_or_else(|| String::from("-"), percent)
}

pub(super) fn opt_money(value: Option<f64>) -> String {
    value.map_or_else(|| String::from("-"), money)
}

pub(super) fn time_label(timestamp: i64) -> String {
    let text = quantrule_data::format_utc_timestamp(timestamp);
    text[..10].to_owned()
}

pub(super) fn text_right(canvas: &mut Canvas, right: i32, y: i32, text: &str, color: Rgb) {
    let width = super::text_width(text, 1);
    canvas.text(right - width, y, text, color, 1);
}

pub(super) fn empty(canvas: &mut Canvas, x: i32, y: i32, label: &str) {
    canvas.text(x, y, label, MUTED, 1);
}

/// Wraps text to at most `columns` characters per line, splitting on spaces.
pub(super) fn wrap(text: &str, columns: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= columns {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}
