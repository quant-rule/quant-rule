//! The data-quality page.

use quantrule_analytics::diagnosis::types::Diagnosis;

use super::{
    ACCENT, FAIL, MARGIN, PASS, Rect, base, draw_bar_h, draw_bar_v, empty, money, panel, percent,
    text_right, time_label,
};
use crate::chart::{Canvas, ChartError};

/// Renders `10-data-quality.png`.
pub(super) fn render(diagnosis: &Diagnosis) -> Result<Vec<u8>, ChartError> {
    let subtitle = format!(
        "{} | {} to {}",
        diagnosis.identity.data_version,
        diagnosis
            .data_quality
            .timeframes
            .first()
            .and_then(|timeframe| timeframe.first_time)
            .map_or_else(|| String::from("-"), time_label),
        diagnosis
            .data_quality
            .timeframes
            .last()
            .and_then(|timeframe| timeframe.last_time)
            .map_or_else(|| String::from("-"), time_label)
    );
    let mut canvas = base("QuantRule data quality", &subtitle);
    draw_coverage(&mut canvas, diagnosis);
    draw_gap_histogram(&mut canvas, diagnosis);
    draw_gaps(&mut canvas, diagnosis);
    draw_summary(&mut canvas, diagnosis);
    canvas.encode_png().map_err(ChartError::Encode)
}

fn draw_coverage(canvas: &mut Canvas, diagnosis: &Diagnosis) {
    panel(canvas, MARGIN, 78, 1_216, 216, "Timeframe coverage");
    let timeframes = &diagnosis.data_quality.timeframes;
    if timeframes.is_empty() {
        empty(canvas, MARGIN + 12, 110, "no timeframe");
        return;
    }
    for (index, timeframe) in timeframes.iter().enumerate() {
        let y = 108 + index as i32 * 38;
        canvas.text(
            MARGIN + 12,
            y + 5,
            &label(timeframe.timeframe_secs),
            crate::chart::TEXT,
            1,
        );
        draw_bar_h(
            canvas,
            Rect::new(MARGIN + 70, y, 770, 16),
            timeframe.coverage,
            1.0,
            if timeframe.coverage < 0.99 {
                FAIL
            } else {
                PASS
            },
        );
        text_right(
            canvas,
            MARGIN + 920,
            y + 5,
            &percent(timeframe.coverage),
            crate::chart::TEXT,
        );
        canvas.text(
            MARGIN + 950,
            y + 5,
            &format!(
                "{}/{} (-{})",
                timeframe.bars, timeframe.expected, timeframe.missing
            ),
            crate::chart::MUTED,
            1,
        );
    }
}

fn draw_gap_histogram(canvas: &mut Canvas, diagnosis: &Diagnosis) {
    panel(canvas, MARGIN, 306, 600, 206, "1m gap histogram (minutes)");
    let buckets = &diagnosis.data_quality.gap_histogram;
    let max = buckets.iter().map(|bucket| bucket.count).max().unwrap_or(0);
    if buckets.is_empty() || max == 0 {
        empty(canvas, MARGIN + 12, 340, "no 1m gap");
        return;
    }
    let baseline = 484;
    let width = 56;
    for (index, bucket) in buckets.iter().enumerate() {
        let x = MARGIN + 24 + index as i32 * 92;
        draw_bar_v(
            canvas,
            baseline,
            Rect::new(x, baseline, width, 120),
            bucket.count as f64,
            max as f64,
            ACCENT,
        );
        canvas.text(x, baseline + 8, &bucket.label, crate::chart::MUTED, 1);
        canvas.text(
            x,
            baseline + 24,
            &bucket.count.to_string(),
            crate::chart::TEXT,
            1,
        );
    }
}

fn draw_gaps(canvas: &mut Canvas, diagnosis: &Diagnosis) {
    panel(canvas, 648, 306, 600, 206, "Longest 1m gaps");
    let mut gaps = diagnosis.data_quality.raw_gaps.iter().collect::<Vec<_>>();
    gaps.sort_by(|left, right| {
        right
            .missing_bars
            .cmp(&left.missing_bars)
            .then_with(|| left.start.cmp(&right.start))
    });
    if gaps.is_empty() {
        empty(canvas, 660, 340, "no 1m gap");
        return;
    }
    for (index, gap) in gaps.iter().take(7).enumerate() {
        let y = 338 + index as i32 * 21;
        canvas.text(
            660,
            y,
            &format!("{} .. {}", time_label(gap.start), time_label(gap.end)),
            crate::chart::MUTED,
            1,
        );
        text_right(
            canvas,
            1_236,
            y,
            &format!("{} min", gap.missing_bars),
            if gap.missing_bars >= 60 {
                FAIL
            } else {
                crate::chart::TEXT
            },
        );
    }
}

fn draw_summary(canvas: &mut Canvas, diagnosis: &Diagnosis) {
    panel(canvas, MARGIN, 524, 1_216, 138, "Summary");
    let longest = diagnosis
        .data_quality
        .raw_gaps
        .iter()
        .map(|gap| gap.missing_bars)
        .max()
        .unwrap_or(0);
    canvas.text(
        MARGIN + 12,
        552,
        &format!(
            "raw 1m gaps {} | longest {} minutes | timeframes {}",
            diagnosis.data_quality.raw_gaps.len(),
            longest,
            diagnosis.data_quality.timeframes.len()
        ),
        crate::chart::TEXT,
        1,
    );
    let worst = diagnosis
        .data_quality
        .timeframes
        .iter()
        .min_by(|left, right| {
            left.coverage
                .partial_cmp(&right.coverage)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    if let Some(timeframe) = worst {
        canvas.text(
            MARGIN + 12,
            578,
            &format!(
                "worst coverage {} at {} ({} missing bars)",
                percent(timeframe.coverage),
                label(timeframe.timeframe_secs),
                timeframe.missing
            ),
            if timeframe.coverage < 0.99 {
                FAIL
            } else {
                PASS
            },
            1,
        );
    }
    canvas.text(
        MARGIN + 12,
        604,
        &format!(
            "funding {} | commission {} | execution fees {}",
            money(diagnosis.portfolio.funding_net),
            money(diagnosis.portfolio.commission),
            money(diagnosis.portfolio.execution_fees)
        ),
        crate::chart::MUTED,
        1,
    );
    canvas.text(
        MARGIN + 12,
        630,
        "A gap is absent data, never a zero; the diagnosis does not fill it.",
        crate::chart::MUTED,
        1,
    );
}

fn label(seconds: u64) -> String {
    match seconds {
        60 => String::from("1m"),
        300 => String::from("5m"),
        900 => String::from("15m"),
        1_800 => String::from("30m"),
        3_600 => String::from("1h"),
        14_400 => String::from("4h"),
        86_400 => String::from("1d"),
        other => format!("{other}s"),
    }
}
