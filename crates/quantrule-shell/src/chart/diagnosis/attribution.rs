//! The portfolio-attribution page.

use quantrule_analytics::diagnosis::types::Diagnosis;

use super::{
    ACCENT, FAIL, MARGIN, PASS, Rect, SECOND, UNKNOWN, base, draw_series, empty, money, panel,
    series_bounds, text_right, time_label,
};
use crate::chart::{Canvas, ChartError};

/// Renders `50-portfolio-attribution.png`.
pub(super) fn render(diagnosis: &Diagnosis) -> Result<Vec<u8>, ChartError> {
    let subtitle = format!(
        "{} -> {} | net {} | max drawdown {}",
        money(diagnosis.portfolio.initial_cash),
        money(diagnosis.portfolio.final_equity),
        money(diagnosis.portfolio.net_pnl),
        super::percent(diagnosis.portfolio.max_drawdown)
    );
    let mut canvas = base("QuantRule portfolio attribution", &subtitle);
    draw_equity(&mut canvas, diagnosis);
    draw_drawdown(&mut canvas, diagnosis);
    draw_waterfall(&mut canvas, diagnosis);
    draw_monthly(&mut canvas, diagnosis);
    canvas.encode_png().map_err(ChartError::Encode)
}

fn draw_equity(canvas: &mut Canvas, diagnosis: &Diagnosis) {
    panel(canvas, MARGIN, 78, 1_216, 262, "Equity vs buy-and-hold");
    let equity = &diagnosis.portfolio.equity;
    let benchmark = &diagnosis.portfolio.benchmark;
    if equity.is_empty() {
        empty(canvas, MARGIN + 12, 110, "no equity point");
        return;
    }
    let (min, max) = series_bounds(&[equity, benchmark]);
    draw_series(
        canvas,
        benchmark,
        Rect::new(MARGIN + 16, 112, 1_184, 188),
        min,
        max,
        SECOND,
    );
    draw_series(
        canvas,
        equity,
        Rect::new(MARGIN + 16, 112, 1_184, 188),
        min,
        max,
        ACCENT,
    );
    canvas.text(
        MARGIN + 12,
        312,
        &time_label(equity[0].0),
        crate::chart::MUTED,
        1,
    );
    text_right(
        canvas,
        1_236,
        312,
        &time_label(equity[equity.len() - 1].0),
        crate::chart::MUTED,
    );
    canvas.fill_rect(MARGIN + 12, 326, 8, 8, ACCENT);
    canvas.text(MARGIN + 26, 326, "equity", crate::chart::MUTED, 1);
    canvas.fill_rect(MARGIN + 110, 326, 8, 8, SECOND);
    canvas.text(MARGIN + 124, 326, "buy and hold", crate::chart::MUTED, 1);
    text_right(
        canvas,
        1_236,
        326,
        &format!(
            "final {} | total return {} | exposure {}",
            money(diagnosis.portfolio.final_equity),
            super::percent(diagnosis.portfolio.total_return),
            super::short_share(diagnosis.portfolio.exposure)
        ),
        crate::chart::MUTED,
    );
}

fn draw_drawdown(canvas: &mut Canvas, diagnosis: &Diagnosis) {
    panel(canvas, MARGIN, 352, 1_216, 158, "Drawdown vs limit");
    let drawdown = &diagnosis.portfolio.drawdown;
    if drawdown.is_empty() {
        empty(canvas, MARGIN + 12, 384, "no drawdown point");
        return;
    }
    let points: Vec<(i64, f64)> = drawdown
        .iter()
        .map(|(date, value)| (*date, value * 100.0))
        .collect();
    let max = diagnosis
        .portfolio
        .max_drawdown_limit
        .map_or_else(
            || {
                points
                    .iter()
                    .map(|(_, value)| *value)
                    .fold(0.0_f64, f64::max)
            },
            |limit| limit * 100.0,
        )
        .max(
            points
                .iter()
                .map(|(_, value)| *value)
                .fold(0.0_f64, f64::max),
        )
        .max(1.0)
        * 1.1;
    draw_series(
        canvas,
        &points,
        Rect::new(MARGIN + 16, 388, 1_184, 76),
        0.0,
        max,
        FAIL,
    );
    if let Some(limit) = diagnosis.portfolio.max_drawdown_limit {
        let y = 388
            + (((max - limit * 100.0) / max) * 76.0)
                .round()
                .clamp(0.0, 76.0) as i32;
        canvas.line(MARGIN + 16, y, MARGIN + 1_200, y, UNKNOWN);
        text_right(
            canvas,
            1_236,
            y - 11,
            &format!("limit {}", super::percent(limit)),
            UNKNOWN,
        );
    }
    if let Some(halt) = diagnosis.portfolio.halt_time {
        let index = drawdown
            .iter()
            .position(|(date, _)| *date >= halt.div_euclid(86_400) * 86_400)
            .unwrap_or(drawdown.len().saturating_sub(1));
        let x =
            MARGIN + 16 + ((index as f64 / drawdown.len().max(2) as f64) * 1_184.0).round() as i32;
        canvas.line(x, 388, x, 464, UNKNOWN);
        canvas.text(x + 4, 466, "halt", UNKNOWN, 1);
    }
    canvas.text(
        MARGIN + 12,
        490,
        &format!(
            "max drawdown {} | max duration {} bars",
            super::percent(diagnosis.portfolio.max_drawdown),
            diagnosis.portfolio.max_drawdown_duration_bars
        ),
        crate::chart::MUTED,
        1,
    );
}

fn draw_waterfall(canvas: &mut Canvas, diagnosis: &Diagnosis) {
    panel(canvas, MARGIN, 524, 600, 152, "Cost waterfall");
    let steps = &diagnosis.portfolio.cost_waterfall;
    let max = steps
        .iter()
        .map(|step| step.amount.abs())
        .fold(0.0_f64, f64::max);
    if steps.is_empty() || max == 0.0 {
        empty(canvas, MARGIN + 12, 556, "no cost step");
        return;
    }
    for (index, step) in steps.iter().enumerate().take(5) {
        let y = 552 + index as i32 * 20;
        canvas.text(MARGIN + 12, y + 4, step.label, crate::chart::TEXT, 1);
        let width = ((step.amount.abs() / max) * 260.0).round() as i32;
        let color = if step.amount < 0.0 { FAIL } else { PASS };
        canvas.fill_rect(MARGIN + 170, y, width, 12, color);
        text_right(
            canvas,
            MARGIN + 588,
            y + 4,
            &money(step.amount),
            crate::chart::TEXT,
        );
    }
}

fn draw_monthly(canvas: &mut Canvas, diagnosis: &Diagnosis) {
    panel(canvas, 648, 524, 600, 152, "Monthly returns");
    let months = &diagnosis.portfolio.monthly_returns;
    if months.is_empty() {
        empty(canvas, 660, 556, "no month");
        return;
    }
    let max = months
        .iter()
        .filter_map(|month| month.return_fraction)
        .map(f64::abs)
        .fold(0.0_f64, f64::max);
    if max == 0.0 {
        empty(canvas, 660, 556, "no monthly return");
        return;
    }
    let baseline = 606;
    let width = ((560 / months.len().max(1)) as i32).clamp(4, 32);
    for (index, month) in months.iter().enumerate() {
        let x = 660 + (index as i32 * 560) / months.len().max(1) as i32;
        if let Some(value) = month.return_fraction {
            let extent = ((value.abs() / max) * 28.0).round() as i32;
            let color = if value < 0.0 { FAIL } else { PASS };
            if value < 0.0 {
                canvas.fill_rect(x, baseline, width, extent.min(28), color);
            } else {
                canvas.fill_rect(x, baseline - extent.min(28), width, extent.min(28), color);
            }
        }
        if index % 2 == 0 {
            canvas.text(
                x,
                baseline + 32,
                &format!("{:02}", month.month),
                crate::chart::MUTED,
                1,
            );
        }
    }
    canvas.text(
        660,
        656,
        &format!(
            "{} months | net {}",
            months.len(),
            money(diagnosis.portfolio.net_pnl)
        ),
        crate::chart::MUTED,
        1,
    );
}
