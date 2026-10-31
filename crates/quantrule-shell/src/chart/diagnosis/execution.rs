//! The execution-quality page.

use quantrule_analytics::diagnosis::types::Diagnosis;

use super::{
    FAIL, MARGIN, PASS, Rect, UNKNOWN, base, draw_bar_h, draw_bar_v, empty, money, panel, percent,
    text_right,
};
use crate::chart::{Canvas, ChartError};

/// Renders `40-execution-quality.png`.
pub(super) fn render(diagnosis: &Diagnosis) -> Result<Vec<u8>, ChartError> {
    let subtitle = format!(
        "{} orders | {} filled | {} rejected | slippage {}",
        diagnosis.execution.orders,
        diagnosis.execution.filled,
        diagnosis.execution.rejected,
        super::opt_money(diagnosis.execution.slippage_average_bps)
    );
    let mut canvas = base("QuantRule execution quality", &subtitle);
    draw_order_states(&mut canvas, diagnosis);
    draw_fill_reasons(&mut canvas, diagnosis);
    draw_rejections(&mut canvas, diagnosis);
    draw_latency(&mut canvas, diagnosis);
    draw_summary(&mut canvas, diagnosis);
    canvas.encode_png().map_err(ChartError::Encode)
}

fn draw_order_states(canvas: &mut Canvas, diagnosis: &Diagnosis) {
    panel(canvas, MARGIN, 78, 600, 196, "Order states");
    let execution = &diagnosis.execution;
    let rows = [
        ("filled", execution.filled, PASS),
        ("rejected", execution.rejected, FAIL),
        ("pending", execution.pending, UNKNOWN),
    ];
    let max = rows.iter().map(|(_, value, _)| *value).max().unwrap_or(0);
    for (index, (label, value, color)) in rows.iter().enumerate() {
        let y = 112 + index as i32 * 34;
        canvas.text(MARGIN + 12, y + 4, label, crate::chart::TEXT, 1);
        draw_bar_h(
            canvas,
            Rect::new(MARGIN + 96, y, 360, 16),
            *value as f64,
            max as f64,
            *color,
        );
        text_right(
            canvas,
            MARGIN + 588,
            y + 4,
            &value.to_string(),
            crate::chart::TEXT,
        );
    }
    canvas.text(
        MARGIN + 12,
        232,
        &format!(
            "fill rate {} | orders {} | fills {}",
            super::short_share(execution.fill_rate),
            execution.orders,
            execution.filled
        ),
        crate::chart::MUTED,
        1,
    );
}

fn draw_fill_reasons(canvas: &mut Canvas, diagnosis: &Diagnosis) {
    panel(canvas, 648, 78, 600, 196, "Fill reasons");
    let reasons = &diagnosis.execution.fill_reasons;
    let max = reasons.iter().map(|reason| reason.count).max().unwrap_or(0);
    for (index, reason) in reasons.iter().enumerate() {
        let y = 112 + index as i32 * 28;
        canvas.text(660, y + 4, &reason.reason, crate::chart::TEXT, 1);
        draw_bar_h(
            canvas,
            Rect::new(744, y, 340, 14),
            reason.count as f64,
            max as f64,
            PASS,
        );
        text_right(
            canvas,
            1_236,
            y + 4,
            &reason.count.to_string(),
            crate::chart::TEXT,
        );
    }
    canvas.text(
        660,
        236,
        &format!(
            "fees {} | slippage total {}",
            money(diagnosis.execution.fees_total),
            money(diagnosis.execution.slippage_total)
        ),
        crate::chart::MUTED,
        1,
    );
}

fn draw_rejections(canvas: &mut Canvas, diagnosis: &Diagnosis) {
    panel(canvas, MARGIN, 292, 600, 250, "Rejection reasons");
    let reasons = &diagnosis.execution.rejection_reasons;
    if reasons.is_empty() {
        empty(canvas, MARGIN + 12, 326, "no rejection");
        return;
    }
    let max = reasons.iter().map(|reason| reason.count).max().unwrap_or(0);
    for (index, reason) in reasons.iter().enumerate().take(13) {
        let y = 324 + index as i32 * 16;
        canvas.text(MARGIN + 12, y, &reason.reason, crate::chart::TEXT, 1);
        draw_bar_h(
            canvas,
            Rect::new(MARGIN + 190, y - 1, 300, 11),
            reason.count as f64,
            max as f64,
            FAIL,
        );
        text_right(
            canvas,
            MARGIN + 588,
            y,
            &reason.count.to_string(),
            crate::chart::TEXT,
        );
    }
    canvas.text(
        MARGIN + 12,
        510,
        &format!(
            "post-halt {} | pre-halt {}",
            diagnosis.execution.post_halt_rejections, diagnosis.execution.pre_halt_rejections
        ),
        if diagnosis.execution.post_halt_rejections > 0 {
            FAIL
        } else {
            PASS
        },
        1,
    );
}

fn draw_latency(canvas: &mut Canvas, diagnosis: &Diagnosis) {
    panel(canvas, 648, 292, 600, 250, "Fill latency (bars)");
    let buckets = &diagnosis.execution.latency;
    let max = buckets.iter().map(|bucket| bucket.fills).max().unwrap_or(0);
    if buckets.is_empty() || max == 0 {
        empty(canvas, 660, 326, "no fill");
        return;
    }
    let baseline = 500;
    for (index, bucket) in buckets.iter().enumerate() {
        let x = 690 + index as i32 * 130;
        draw_bar_v(
            canvas,
            baseline,
            Rect::new(x, baseline, 74, 150),
            bucket.fills as f64,
            max as f64,
            PASS,
        );
        canvas.text(x, baseline + 8, &bucket.label, crate::chart::MUTED, 1);
        canvas.text(
            x,
            baseline + 24,
            &bucket.fills.to_string(),
            crate::chart::TEXT,
            1,
        );
    }
}

fn draw_summary(canvas: &mut Canvas, diagnosis: &Diagnosis) {
    panel(canvas, MARGIN, 554, 1_216, 112, "Summary");
    let execution = &diagnosis.execution;
    canvas.text(
        MARGIN + 12,
        582,
        &format!(
            "slippage per unit {} | average {} bps | fees {}",
            super::opt_money(execution.slippage_average_per_unit),
            super::opt_money(execution.slippage_average_bps),
            money(execution.fees_total)
        ),
        crate::chart::TEXT,
        1,
    );
    canvas.text(
        MARGIN + 12,
        608,
        &format!(
            "rejection share {} | filled share {}",
            percent(if execution.orders > 0 {
                execution.rejected as f64 / execution.orders as f64
            } else {
                0.0
            }),
            percent(if execution.orders > 0 {
                execution.filled as f64 / execution.orders as f64
            } else {
                0.0
            })
        ),
        crate::chart::MUTED,
        1,
    );
    canvas.text(
        MARGIN + 12,
        634,
        "A rejection is not a fill; the order record keeps the refused decision visible.",
        crate::chart::MUTED,
        1,
    );
}
