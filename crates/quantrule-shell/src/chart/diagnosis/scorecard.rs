//! The run scorecard page.

use quantrule_analytics::diagnosis::types::Diagnosis;

use super::{
    FAIL, MARGIN, PASS, base, draw_status, empty, money, opt_money, panel, short_share, text_right,
    wrap,
};
use crate::chart::{Canvas, ChartError};

/// Renders `00-run-scorecard.png`.
pub(super) fn render(diagnosis: &Diagnosis) -> Result<Vec<u8>, ChartError> {
    let subtitle = format!(
        "{} | data {} | config {} | model {}",
        diagnosis.identity.lab,
        diagnosis.identity.data_version,
        &diagnosis.identity.config_hash[..diagnosis.identity.config_hash.len().min(12)],
        diagnosis.identity.model_version
    );
    let mut canvas = base("QuantRule diagnosis scorecard", &subtitle);
    draw_checks(&mut canvas, diagnosis);
    draw_funnel(&mut canvas, diagnosis);
    draw_waterfall(&mut canvas, diagnosis);
    draw_triage(&mut canvas, diagnosis);
    draw_footer(&mut canvas, diagnosis);
    canvas.encode_png().map_err(ChartError::Encode)
}

fn draw_checks(canvas: &mut Canvas, diagnosis: &Diagnosis) {
    panel(canvas, MARGIN, 78, 600, 332, "Checks");
    let counts = diagnosis.check_counts;
    canvas.text(
        MARGIN + 12,
        100,
        &format!(
            "{} pass   {} fail   {} unknown",
            counts.pass, counts.fail, counts.unknown
        ),
        if counts.fail > 0 { FAIL } else { PASS },
        1,
    );
    for (index, check) in diagnosis.checks.iter().enumerate().take(17) {
        let y = 124 + index as i32 * 16;
        draw_status(canvas, MARGIN + 12, y, check.status, check.status.label());
        canvas.text(MARGIN + 110, y + 1, &check.id, crate::chart::TEXT, 1);
        text_right(
            canvas,
            MARGIN + 588,
            y + 1,
            &opt_money(check.value),
            crate::chart::MUTED,
        );
    }
}

fn draw_funnel(canvas: &mut Canvas, diagnosis: &Diagnosis) {
    panel(canvas, 648, 78, 600, 332, "Funnel");
    let funnel = &diagnosis.funnel;
    let rows = [
        ("strategy bars", funnel.bars),
        ("decisions", funnel.decisions),
        ("entry attempts", funnel.entry_attempts),
        ("exit attempts", funnel.exit_attempts),
        ("orders", funnel.orders),
        ("filled", funnel.filled),
        ("rejected", funnel.rejected),
        ("trades", funnel.trades),
    ];
    for (index, (label, value)) in rows.iter().enumerate() {
        let y = 106 + index as i32 * 22;
        canvas.text(660, y, label, crate::chart::MUTED, 1);
        text_right(canvas, 1_236, y, &value.to_string(), crate::chart::TEXT);
    }
    let post_halt = funnel.post_halt_rejections;
    canvas.text(
        660,
        292,
        &format!(
            "post-halt rejections {post_halt} | entry fills {}",
            funnel.entry_fills
        ),
        if post_halt > 0 { FAIL } else { PASS },
        1,
    );
    canvas.text(
        660,
        314,
        &format!(
            "max drawdown {} | limit {}",
            short_share(Some(diagnosis.portfolio.max_drawdown)),
            short_share(diagnosis.portfolio.max_drawdown_limit)
        ),
        crate::chart::MUTED,
        1,
    );
    canvas.text(
        660,
        336,
        &format!(
            "net {} | costs {}",
            money(diagnosis.portfolio.net_pnl),
            money(
                diagnosis.portfolio.commission
                    + diagnosis.portfolio.execution_fees
                    + diagnosis.portfolio.funding_net
            )
        ),
        crate::chart::MUTED,
        1,
    );
    canvas.text(
        660,
        358,
        &format!(
            "entry fill rate {}",
            short_share(if diagnosis.strategy.entry_attempts > 0 {
                Some(
                    diagnosis.strategy.executed_entries as f64
                        / diagnosis.strategy.entry_attempts as f64,
                )
            } else {
                None
            })
        ),
        crate::chart::MUTED,
        1,
    );
    canvas.text(
        660,
        380,
        &format!(
            "gross exp {} | net exp {}",
            opt_money(diagnosis.strategy.gross_expectancy),
            opt_money(diagnosis.strategy.net_expectancy)
        ),
        crate::chart::MUTED,
        1,
    );
}

fn draw_waterfall(canvas: &mut Canvas, diagnosis: &Diagnosis) {
    panel(canvas, MARGIN, 430, 600, 230, "Cost waterfall");
    let steps = &diagnosis.portfolio.cost_waterfall;
    let max = steps
        .iter()
        .map(|step| step.amount.abs())
        .fold(0.0_f64, f64::max);
    if steps.is_empty() || max == 0.0 {
        empty(canvas, MARGIN + 12, 460, "no cost step");
        return;
    }
    for (index, step) in steps.iter().enumerate() {
        let y = 462 + index as i32 * 34;
        canvas.text(MARGIN + 12, y + 4, step.label, crate::chart::TEXT, 1);
        let width = ((step.amount.abs() / max) * 300.0).round() as i32;
        let color = if step.amount < 0.0 { FAIL } else { PASS };
        canvas.fill_rect(MARGIN + 170, y, width, 16, color);
        text_right(
            canvas,
            MARGIN + 588,
            y + 4,
            &money(step.amount),
            crate::chart::TEXT,
        );
    }
}

fn draw_triage(canvas: &mut Canvas, diagnosis: &Diagnosis) {
    panel(canvas, 648, 430, 600, 230, "Triage");
    if diagnosis.triage.is_empty() {
        empty(canvas, 660, 460, "no action");
        return;
    }
    let mut y = 458;
    for action in diagnosis.triage.iter().take(5) {
        let lines = wrap(action, 57);
        for line in lines.into_iter().take(3) {
            canvas.text(660, y, &line, crate::chart::TEXT, 1);
            y += 14;
        }
        y += 4;
        if y > 638 {
            break;
        }
    }
    if diagnosis.triage.len() > 5 {
        canvas.text(
            660,
            642,
            &format!("+{} more in summary.txt", diagnosis.triage.len() - 5),
            crate::chart::MUTED,
            1,
        );
    }
}

fn draw_footer(canvas: &mut Canvas, diagnosis: &Diagnosis) {
    canvas.text(
        MARGIN,
        680,
        &format!(
            "period {} | trades {} | wins {} | losses {} | exposure {}",
            diagnosis.identity.actual_period.as_deref().unwrap_or("-"),
            diagnosis.portfolio.trades,
            diagnosis.portfolio.wins,
            diagnosis.portfolio.losses,
            short_share(diagnosis.portfolio.exposure)
        ),
        crate::chart::MUTED,
        1,
    );
}
