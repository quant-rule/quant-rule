//! The strategy-funnel page.

use quantrule_analytics::diagnosis::types::Diagnosis;

use super::{
    ACCENT, FAIL, MARGIN, PASS, Rect, base, draw_series, empty, money, panel, series_bounds,
    text_right, time_label,
};
use crate::chart::{Canvas, ChartError};

/// Renders `30-strategy-funnel.png`.
pub(super) fn render(diagnosis: &Diagnosis) -> Result<Vec<u8>, ChartError> {
    let subtitle = format!(
        "{} entry attempts -> {} fills -> {} trades | {} decisions",
        diagnosis.strategy.entry_attempts,
        diagnosis.strategy.executed_entries,
        diagnosis.portfolio.trades,
        diagnosis.strategy.decisions
    );
    let mut canvas = base("QuantRule strategy funnel", &subtitle);
    draw_series_panel(&mut canvas, diagnosis);
    draw_exit_reasons(&mut canvas, diagnosis);
    draw_hold_buckets(&mut canvas, diagnosis);
    canvas.encode_png().map_err(ChartError::Encode)
}

fn draw_series_panel(canvas: &mut Canvas, diagnosis: &Diagnosis) {
    panel(
        canvas,
        MARGIN,
        78,
        1_216,
        302,
        "Entry attempts and fills by day",
    );
    let points = &diagnosis.strategy.funnel_series;
    if points.is_empty() {
        empty(canvas, MARGIN + 12, 110, "no entry attempt");
        return;
    }
    let attempts: Vec<(i64, f64)> = points
        .iter()
        .map(|point| (point.time, point.entry_attempts as f64))
        .collect();
    let fills: Vec<(i64, f64)> = points
        .iter()
        .map(|point| (point.time, point.entry_fills as f64))
        .collect();
    let (min, max) = series_bounds(&[&attempts, &fills]);
    draw_series(
        canvas,
        &attempts,
        Rect::new(MARGIN + 16, 112, 1_184, 228),
        min,
        max,
        ACCENT,
    );
    draw_series(
        canvas,
        &fills,
        Rect::new(MARGIN + 16, 112, 1_184, 228),
        min,
        max,
        PASS,
    );
    canvas.text(
        MARGIN + 12,
        352,
        &time_label(points[0].time),
        crate::chart::MUTED,
        1,
    );
    text_right(
        canvas,
        1_236,
        352,
        &time_label(points[points.len() - 1].time),
        crate::chart::MUTED,
    );
    canvas.fill_rect(MARGIN + 12, 370, 8, 8, ACCENT);
    canvas.text(MARGIN + 26, 370, "entry attempts", crate::chart::MUTED, 1);
    canvas.fill_rect(MARGIN + 210, 370, 8, 8, PASS);
    canvas.text(MARGIN + 224, 370, "entry fills", crate::chart::MUTED, 1);
    canvas.text(
        MARGIN + 390,
        370,
        &format!(
            "entry fill rate {:.4} | long {} short {}",
            if diagnosis.strategy.entry_attempts > 0 {
                diagnosis.strategy.executed_entries as f64
                    / diagnosis.strategy.entry_attempts as f64
            } else {
                0.0
            },
            diagnosis.strategy.long_entries,
            diagnosis.strategy.short_entries
        ),
        crate::chart::MUTED,
        1,
    );
}

fn draw_exit_reasons(canvas: &mut Canvas, diagnosis: &Diagnosis) {
    panel(canvas, MARGIN, 398, 600, 280, "Exit reason net P&L");
    let reasons = &diagnosis.strategy.exit_reasons;
    let max = reasons
        .iter()
        .map(|reason| reason.net_pnl.abs())
        .fold(0.0_f64, f64::max);
    if reasons.is_empty() || max == 0.0 {
        empty(canvas, MARGIN + 12, 430, "no closed trade");
        return;
    }
    for (index, reason) in reasons.iter().enumerate() {
        let y = 436 + index as i32 * 52;
        canvas.text(MARGIN + 12, y, &reason.reason, crate::chart::TEXT, 1);
        let width = ((reason.net_pnl.abs() / max) * 180.0).round() as i32;
        let color = if reason.net_pnl < 0.0 { FAIL } else { PASS };
        canvas.fill_rect(MARGIN + 110, y - 1, width, 14, color);
        canvas.text(
            MARGIN + 300,
            y,
            &format!("net {}", money(reason.net_pnl)),
            crate::chart::TEXT,
            1,
        );
        canvas.text(
            MARGIN + 110,
            y + 17,
            &format!(
                "{} trades | gross {}",
                reason.trades,
                money(reason.gross_pnl)
            ),
            crate::chart::MUTED,
            1,
        );
    }
    canvas.text(
        MARGIN + 12,
        650,
        "Exit reason is the portfolio's; the strategy reason is separate.",
        crate::chart::MUTED,
        1,
    );
}

fn draw_hold_buckets(canvas: &mut Canvas, diagnosis: &Diagnosis) {
    panel(canvas, 648, 398, 600, 280, "Hold-time net P&L");
    let buckets = &diagnosis.strategy.hold_buckets;
    let max = buckets
        .iter()
        .map(|bucket| bucket.net_pnl.abs())
        .fold(0.0_f64, f64::max);
    if buckets.is_empty() || max == 0.0 {
        empty(canvas, 660, 430, "no closed trade");
        return;
    }
    for (index, bucket) in buckets.iter().enumerate() {
        let y = 430 + index as i32 * 29;
        canvas.text(660, y, &bucket.label, crate::chart::TEXT, 1);
        let width = ((bucket.net_pnl.abs() / max) * 260.0).round() as i32;
        let color = if bucket.net_pnl < 0.0 { FAIL } else { PASS };
        canvas.fill_rect(744, y - 1, width, 14, color);
        canvas.text(
            1_020,
            y,
            &format!("{} trades", bucket.trades),
            crate::chart::MUTED,
            1,
        );
        text_right(canvas, 1_236, y, &money(bucket.net_pnl), color);
    }
    canvas.text(
        660,
        650,
        &format!(
            "mean hold {} bars | net expectancy {}",
            super::opt_money(diagnosis.strategy.average_hold_bars),
            super::opt_money(diagnosis.strategy.net_expectancy)
        ),
        crate::chart::MUTED,
        1,
    );
}
