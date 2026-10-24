//! As-of alignment to the latest closed higher timeframe.
//!
//! The research output contract, §7.2, fixes the rule: a base bar opening at
//! `t_b` with duration `d_b` closes at `c_b = t_b + d_b`. A higher bar opening
//! at `T` with duration `d_h` is eligible when `T + d_h <= c_b`, and the latest
//! eligible bar is used. An open containing bar is never used, so appending a
//! later higher bar cannot change an earlier alignment.
//!
//! The module is pure and reads only its arguments. Staleness is published in
//! seconds, as the contract's `staleness_secs` column fixes it.

/// The index of the latest higher-timeframe bar that closed at or before the
/// base bar's close.
///
/// `higher_open_times` must be sorted ascending. The function binary-searches
/// it, so an unsorted slice would produce an unspecified index. Returns `None`
/// when no higher bar has closed by the base close, when the base close
/// overflows, or when a duration does not fit an `i64`.
///
/// # Examples
///
/// ```
/// use quantrule_analytics::alignment::latest_closed_index;
///
/// // The M1 bar at 09:03 closes at 09:04; the 09:00-09:05 M5 bar is open.
/// let m5 = [31_800, 32_100, 32_400];
/// assert_eq!(latest_closed_index(32_580, 60, &m5, 300), Some(1));
/// ```
#[must_use]
pub fn latest_closed_index(
    base_open_time: i64,
    base_secs: u64,
    higher_open_times: &[i64],
    higher_secs: u64,
) -> Option<usize> {
    let base_close = close_time(base_open_time, base_secs)?;
    let higher_secs = i64::try_from(higher_secs).ok()?;
    let position = higher_open_times.partition_point(|open| {
        open.checked_add(higher_secs)
            .is_some_and(|close| close <= base_close)
    });
    position.checked_sub(1)
}

/// The seconds between the higher bar's close and the base bar's close.
///
/// Returns `None` when the higher bar is not closed at or before the base
/// close. The contract's staleness is non-negative, so a still-open higher bar
/// has no staleness rather than a negative one.
///
/// # Examples
///
/// ```
/// use quantrule_analytics::alignment::staleness_secs;
///
/// assert_eq!(staleness_secs(32_580, 60, 32_100, 300), Some(240));
/// assert_eq!(staleness_secs(32_580, 60, 32_400, 300), None);
/// ```
#[must_use]
pub fn staleness_secs(
    base_open_time: i64,
    base_secs: u64,
    higher_open_time: i64,
    higher_secs: u64,
) -> Option<u64> {
    let base_close = close_time(base_open_time, base_secs)?;
    let higher_close = close_time(higher_open_time, higher_secs)?;
    u64::try_from(base_close.checked_sub(higher_close)?).ok()
}

fn close_time(open_time: i64, secs: u64) -> Option<i64> {
    open_time.checked_add(i64::try_from(secs).ok()?)
}

#[cfg(test)]
mod tests {
    use super::{latest_closed_index, staleness_secs};

    const M1: u64 = 60;
    const M5: u64 = 300;
    const H1: u64 = 3_600;

    // A round hour is 32_400 seconds from the epoch, so these are 08:50, 08:55,
    // 09:00, and 09:05 UTC on an arbitrary day.
    const M5_0850: i64 = 31_800;
    const M5_0855: i64 = 32_100;
    const M5_0900: i64 = 32_400;
    const M5_0905: i64 = 32_700;

    fn m5_opens() -> Vec<i64> {
        vec![M5_0850, M5_0855, M5_0900, M5_0905]
    }

    #[test]
    fn an_open_containing_bar_is_not_used() {
        // 09:03 closes at 09:04; the 09:00-09:05 M5 bar is still open.
        let index =
            latest_closed_index(32_580, M1, &m5_opens(), M5).expect("the 08:55 bar is closed");
        assert_eq!(index, 1);
        assert_eq!(staleness_secs(32_580, M1, M5_0855, M5), Some(240));
    }

    #[test]
    fn a_bar_closed_at_the_base_close_is_used() {
        // 09:04 closes at 09:05, when the 09:00-09:05 M5 bar closes.
        let index =
            latest_closed_index(32_640, M1, &m5_opens(), M5).expect("the 09:00 bar is closed");
        assert_eq!(index, 2);
        assert_eq!(staleness_secs(32_640, M1, M5_0900, M5), Some(0));
    }

    #[test]
    fn a_base_bar_that_closes_with_the_higher_bar_uses_it() {
        // 08:59 closes at 09:00, exactly when the 08:55 bar closes.
        let index =
            latest_closed_index(32_340, M1, &m5_opens(), M5).expect("the 08:55 bar is closed");
        assert_eq!(index, 1);
        assert_eq!(staleness_secs(32_340, M1, M5_0855, M5), Some(0));
    }

    #[test]
    fn a_base_bar_after_the_higher_close_is_one_minute_stale() {
        // 09:00 closes at 09:01; the latest M5 close was 09:00.
        let index =
            latest_closed_index(32_400, M1, &m5_opens(), M5).expect("the 08:55 bar is closed");
        assert_eq!(index, 1);
        assert_eq!(staleness_secs(32_400, M1, M5_0855, M5), Some(60));
    }

    #[test]
    fn the_first_higher_bar_is_used_once_it_closes() {
        // 08:54 closes at 08:55, exactly when the 08:50 bar closes.
        let index =
            latest_closed_index(32_040, M1, &m5_opens(), M5).expect("the 08:50 bar is closed");
        assert_eq!(index, 0);
        assert_eq!(staleness_secs(32_040, M1, M5_0850, M5), Some(0));
    }

    #[test]
    fn a_base_bar_before_the_first_higher_close_has_no_block() {
        // 08:49 closes at 08:50, before the first M5 bar closes at 08:55.
        assert_eq!(latest_closed_index(31_740, M1, &m5_opens(), M5), None);
    }

    #[test]
    fn an_empty_higher_series_has_no_block() {
        assert_eq!(latest_closed_index(32_580, M1, &[], M5), None);
    }

    #[test]
    fn an_unaligned_boundary_uses_the_previous_higher_close() {
        // An M5 base bar at 09:05 closes at 09:10; the 09:00 H1 bar is open.
        let h1_opens = [28_800, 32_400]; // 08:00 and 09:00
        let index =
            latest_closed_index(32_700, M5, &h1_opens, H1).expect("the 08:00 H1 bar is closed");
        assert_eq!(index, 0);
        assert_eq!(staleness_secs(32_700, M5, 28_800, H1), Some(600));
    }

    #[test]
    fn a_future_higher_bar_has_no_staleness() {
        // The 09:00-09:05 bar closes at 09:05, after the 09:04 base close.
        assert_eq!(staleness_secs(32_640, M1, M5_0900, M5), Some(0));
        assert_eq!(staleness_secs(32_580, M1, M5_0900, M5), None);
        assert_eq!(staleness_secs(32_580, M1, M5_0905, M5), None);
    }

    #[test]
    fn later_bars_do_not_change_an_earlier_alignment() {
        let mut opens = m5_opens();
        let before = latest_closed_index(32_580, M1, &opens, M5).map(|index| opens[index]);

        opens.push(33_000); // 09:10
        opens.push(33_300); // 09:15

        let after = latest_closed_index(32_580, M1, &opens, M5).map(|index| opens[index]);
        assert_eq!(before, after);
        assert_eq!(after, Some(M5_0855));
    }
}
