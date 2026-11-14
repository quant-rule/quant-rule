//! Deterministic window selection.
//!
//! The selector reads a recorded bundle and the run's event codes. It builds a
//! window around every candidate bar, keeps the event's available bar inside the
//! candidate zone, removes heavily overlapping windows, and takes a seeded,
//! year-stratified sample. The same bundle, event codes, and seed produce the
//! same windows.
//!
//! The sampling reproduces the reference's `random.Random(seed + year).shuffle`
//! exactly, which means a bit-exact Mersenne Twister with the reference's
//! integer seeding. Using any other generator would select different windows
//! from the same seed and silently break every historical run.
//!
//! `port-parity: PORT-001` names the golden fixture this module is compared
//! against.

// port-parity: PORT-001

use serde::{Deserialize, Serialize};

use crate::bundle::{Bundle, Catalog, Event, utc_from_seconds, utc_year};

/// Why a selection cannot be produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowError {
    /// The bundle has no bars.
    EmptyBundle,
    /// The spec names no event code.
    NoEventCodes,
    /// A window would hold fewer than twenty bars.
    WindowTooShort,
    /// The candidate zone is not strictly inside the window.
    ZoneOutsideWindow,
    /// The maximum window count is zero.
    NoWindows,
    /// The overlap allowance is not smaller than the window.
    OverlapTooLarge,
}

impl std::fmt::Display for WindowError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::EmptyBundle => "the bundle has no bars",
            Self::NoEventCodes => "event_codes must not be empty",
            Self::WindowTooShort => "window_bars must be at least 20",
            Self::ZoneOutsideWindow => "candidate_zone must be strictly inside the window",
            Self::NoWindows => "max_windows must be positive",
            Self::OverlapTooLarge => "overlap_bars must be smaller than window_bars",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for WindowError {}

/// The pre-registered selection parameters for one run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowSpec {
    /// The event codes a candidate bar must publish.
    pub event_codes: Vec<u32>,
    /// Bars in one window.
    pub window_bars: usize,
    /// The inclusive lower and exclusive upper candidate position in the window.
    pub candidate_zone: (usize, usize),
    /// The most windows to select.
    pub max_windows: usize,
    /// The fewest windows a run needs before it is not underpowered.
    pub min_windows: usize,
    /// The seed of the year-stratified sample.
    pub seed: u64,
    /// How much neighbouring windows may overlap.
    pub overlap_bars: usize,
    /// The first candidate bar timestamp, when the run restricts the window.
    pub from_ts: Option<i64>,
    /// The last candidate bar timestamp, when the run restricts the window.
    pub to_ts: Option<i64>,
}

impl WindowSpec {
    /// Reject a spec that cannot produce a useful window.
    ///
    /// # Errors
    ///
    /// Returns [`WindowError`] when the codes are empty, the window is too
    /// short, the zone is outside it, the count is zero, or the overlap is not
    /// smaller than the window.
    pub fn validate(&self) -> Result<(), WindowError> {
        if self.event_codes.is_empty() {
            return Err(WindowError::NoEventCodes);
        }
        if self.window_bars < 20 {
            return Err(WindowError::WindowTooShort);
        }
        if !(self.candidate_zone.0 < self.candidate_zone.1
            && self.candidate_zone.1 < self.window_bars)
        {
            return Err(WindowError::ZoneOutsideWindow);
        }
        if self.max_windows < 1 {
            return Err(WindowError::NoWindows);
        }
        if self.overlap_bars >= self.window_bars {
            return Err(WindowError::OverlapTooLarge);
        }
        Ok(())
    }
}

/// One event as it appears in a selected window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowEvent {
    /// The event identity.
    pub event_index: u64,
    /// The event code.
    pub code: u32,
    /// The registry name.
    pub name: String,
    /// The type code.
    pub type_code: u32,
    /// The subtype code.
    pub subtype_code: u32,
    /// The source code.
    pub source_code: u32,
    /// The bar on which the event occurred.
    pub occurred_bar: u64,
    /// The first bar on which it became knowable.
    pub available_bar: u64,
    /// The direction code.
    pub direction: i32,
    /// The measured primary value.
    pub primary: Option<f64>,
    /// The measured reference value.
    pub reference: Option<f64>,
}

impl WindowEvent {
    fn from_event(event: &Event, catalog: &Catalog) -> Self {
        Self {
            event_index: event.event_index,
            code: event.code,
            name: event.label(catalog),
            type_code: event.type_code,
            subtype_code: event.subtype_code,
            source_code: event.source_code,
            occurred_bar: event.occurred_bar,
            available_bar: event.available_bar,
            direction: event.direction,
            primary: event.primary,
            reference: event.reference,
        }
    }
}

/// One selected window around one candidate bar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowRecord {
    /// The stable window name, `w001` and so on.
    pub window_id: String,
    /// The inclusive first bar index.
    pub start_index: usize,
    /// The exclusive last bar index.
    pub end_index: usize,
    /// The first bar identity in the window.
    pub start_bar: u64,
    /// The last bar identity in the window.
    pub end_bar: u64,
    /// The first bar time.
    pub start_time: String,
    /// The last bar time.
    pub end_time: String,
    /// The candidate position inside the window.
    pub candidate_index: usize,
    /// The candidate bar identity.
    pub candidate_bar: u64,
    /// The candidate bar time.
    pub candidate_time: String,
    /// The events available at the candidate bar.
    pub events: Vec<WindowEvent>,
}

/// The complete `windows.json` record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Selection {
    /// The record schema.
    pub schema: String,
    /// The bar timeframe, in seconds.
    pub timeframe_secs: u64,
    /// Bars in one window.
    pub window_bars: usize,
    /// The candidate zone.
    pub candidate_zone: [usize; 2],
    /// The most windows to select.
    pub max_windows: usize,
    /// The fewest windows a run needs.
    pub min_windows: usize,
    /// The sample seed.
    pub seed: u64,
    /// The overlap allowance.
    pub overlap_bars: usize,
    /// The event codes the run reads.
    pub event_codes: Vec<u32>,
    /// The registry names of those codes.
    pub event_names: Vec<String>,
    /// The first timestamp the selection covers.
    #[serde(rename = "from")]
    pub from_time: String,
    /// The last timestamp the selection covers.
    #[serde(rename = "to")]
    pub to_time: String,
    /// Candidate bars before sampling.
    pub candidate_count: usize,
    /// Windows selected.
    pub selected_count: usize,
    /// Whether the sample is below the run's minimum.
    pub underpowered: bool,
    /// The selected windows.
    pub windows: Vec<WindowRecord>,
}

type Group = (usize, Vec<Event>);

/// Select deterministic windows for one run.
///
/// # Errors
///
/// Returns [`WindowError`] when the bundle is empty or the spec cannot produce
/// a window.
pub fn select(
    bundle: &Bundle,
    spec: &WindowSpec,
    catalog: &Catalog,
) -> Result<Selection, WindowError> {
    spec.validate()?;
    if bundle.bars.is_empty() {
        return Err(WindowError::EmptyBundle);
    }
    let groups = candidate_groups(bundle, spec);
    let mut candidates: Vec<Group> = Vec::new();
    let mut last_end: i64 = -1;
    for (candidate_index, events) in groups {
        let Some((start, end)) = window_around(bundle, candidate_index, spec) else {
            continue;
        };
        if (start as i64) < last_end - spec.overlap_bars as i64 {
            continue;
        }
        candidates.push((candidate_index, events));
        last_end = end as i64;
    }
    let selected = stratified_sample(&candidates, bundle, spec);
    let mut windows = Vec::with_capacity(selected.len());
    for (number, (candidate_index, events)) in selected.iter().enumerate() {
        let Some((start, end)) = window_around(bundle, *candidate_index, spec) else {
            continue;
        };
        let first = &bundle.bars[start];
        let last = &bundle.bars[end - 1];
        let candidate = &bundle.bars[*candidate_index];
        windows.push(WindowRecord {
            window_id: format!("w{:03}", number + 1),
            start_index: start,
            end_index: end,
            start_bar: first.bar,
            end_bar: last.bar,
            start_time: utc_from_seconds(first.timestamp),
            end_time: utc_from_seconds(last.timestamp),
            candidate_index: *candidate_index,
            candidate_bar: candidate.bar,
            candidate_time: utc_from_seconds(candidate.timestamp),
            events: events
                .iter()
                .map(|event| WindowEvent::from_event(event, catalog))
                .collect(),
        });
    }
    let first = &bundle.bars[0];
    let last = &bundle.bars[bundle.bars.len() - 1];
    Ok(Selection {
        schema: "quantrule.simulation-windows/v1".to_owned(),
        timeframe_secs: bundle.timeframe_secs,
        window_bars: spec.window_bars,
        candidate_zone: [spec.candidate_zone.0, spec.candidate_zone.1],
        max_windows: spec.max_windows,
        min_windows: spec.min_windows,
        seed: spec.seed,
        overlap_bars: spec.overlap_bars,
        event_codes: spec.event_codes.clone(),
        event_names: spec
            .event_codes
            .iter()
            .map(|code| {
                catalog
                    .get(code)
                    .cloned()
                    .unwrap_or_else(|| format!("event.{code}"))
            })
            .collect(),
        from_time: utc_from_seconds(spec.from_ts.unwrap_or(first.timestamp)),
        to_time: utc_from_seconds(spec.to_ts.unwrap_or(last.timestamp)),
        candidate_count: candidates.len(),
        selected_count: windows.len(),
        underpowered: windows.len() < spec.min_windows,
        windows,
    })
}

fn candidate_groups(bundle: &Bundle, spec: &WindowSpec) -> Vec<Group> {
    let mut by_bar: std::collections::BTreeMap<u64, Vec<Event>> = std::collections::BTreeMap::new();
    for event in &bundle.events {
        if !spec.event_codes.contains(&event.code) {
            continue;
        }
        let Some(index) = bundle.index_of(event.available_bar) else {
            continue;
        };
        let timestamp = bundle.bars[index].timestamp;
        if spec.from_ts.is_some_and(|from| timestamp < from) {
            continue;
        }
        if spec.to_ts.is_some_and(|to| timestamp > to) {
            continue;
        }
        by_bar.entry(event.available_bar).or_default().push(*event);
    }
    let mut groups: Vec<Group> = by_bar
        .into_iter()
        .filter_map(|(bar, mut events)| {
            let index = bundle.index_of(bar)?;
            events.sort_by_key(|event| event.event_index);
            Some((index, events))
        })
        .collect();
    groups.sort_by_key(|(index, events)| (bundle.bars[*index].timestamp, events[0].event_index));
    groups
}

fn window_around(bundle: &Bundle, candidate: usize, spec: &WindowSpec) -> Option<(usize, usize)> {
    let mut start = candidate.saturating_sub(spec.candidate_zone.0);
    let mut end = start + spec.window_bars;
    if end > bundle.bars.len() {
        end = bundle.bars.len();
        start = end.saturating_sub(spec.window_bars);
        if end < spec.window_bars {
            return None;
        }
    }
    let position = candidate - start;
    if !(spec.candidate_zone.0..spec.candidate_zone.1).contains(&position) {
        return None;
    }
    Some((start, end))
}

fn stratified_sample(candidates: &[Group], bundle: &Bundle, spec: &WindowSpec) -> Vec<Group> {
    let mut by_year: std::collections::BTreeMap<i64, Vec<Group>> =
        std::collections::BTreeMap::new();
    for candidate in candidates {
        let year = utc_year(bundle.bars[candidate.0].timestamp);
        by_year.entry(year).or_default().push(candidate.clone());
    }
    if by_year.is_empty() {
        return Vec::new();
    }
    let per_year = (spec.max_windows / by_year.len()).max(1);
    let mut selected: Vec<Group> = Vec::new();
    let mut remaining: Vec<Group> = Vec::new();
    for (year, mut pool) in by_year {
        let mut generator = Mt19937::from_seed(spec.seed.wrapping_add(year as u64));
        generator.shuffle(&mut pool);
        let take = per_year.min(pool.len());
        selected.extend(pool.iter().take(take).cloned());
        remaining.extend(pool.into_iter().skip(take));
    }
    if selected.len() < spec.max_windows {
        remaining
            .sort_by_key(|(index, events)| (bundle.bars[*index].timestamp, events[0].event_index));
        selected.extend(
            remaining
                .into_iter()
                .take(spec.max_windows - selected.len()),
        );
    }
    selected.sort_by_key(|(index, events)| (bundle.bars[*index].timestamp, events[0].event_index));
    selected
}

/// The Mersenne Twister the reference's `random.Random` uses.
///
/// The generator is seeded from the integer the reference would supply, with
/// the reference's little-endian word expansion and `init_by_array` mixing. The
/// constants are the published MT19937 constants; changing one changes every
/// historical selection.
struct Mt19937 {
    state: [u32; 624],
    index: usize,
}

impl Mt19937 {
    fn from_seed(seed: u64) -> Self {
        let mut key = Vec::with_capacity(2);
        let mut value = seed;
        loop {
            key.push((value & 0xFFFF_FFFF) as u32);
            value >>= 32;
            if value == 0 {
                break;
            }
        }
        Self::from_key(&key)
    }

    fn from_key(key: &[u32]) -> Self {
        let mut state = [0_u32; 624];
        state[0] = 19_650_218;
        for index in 1..624 {
            state[index] = 1_812_433_253_u32
                .wrapping_mul(state[index - 1] ^ (state[index - 1] >> 30))
                .wrapping_add(index as u32);
        }
        let mut index = 1;
        let mut key_index = 0;
        let mut remaining = 624.max(key.len());
        while remaining > 0 {
            state[index] = (state[index]
                ^ (state[index - 1] ^ (state[index - 1] >> 30)).wrapping_mul(1_664_525))
            .wrapping_add(key[key_index])
            .wrapping_add(key_index as u32);
            index += 1;
            key_index += 1;
            if index >= 624 {
                state[0] = state[623];
                index = 1;
            }
            if key_index >= key.len() {
                key_index = 0;
            }
            remaining -= 1;
        }
        let mut remaining = 623;
        while remaining > 0 {
            state[index] = (state[index]
                ^ (state[index - 1] ^ (state[index - 1] >> 30)).wrapping_mul(1_566_083_941))
            .wrapping_sub(index as u32);
            index += 1;
            if index >= 624 {
                state[0] = state[623];
                index = 1;
            }
            remaining -= 1;
        }
        state[0] = 0x8000_0000;
        Self { state, index: 624 }
    }

    fn next_u32(&mut self) -> u32 {
        if self.index >= 624 {
            for position in 0..624 {
                let value = (self.state[position] & 0x8000_0000)
                    | (self.state[(position + 1) % 624] & 0x7FFF_FFFF);
                let mut next = value >> 1;
                if value & 1 != 0 {
                    next ^= 0x9908_B0DF;
                }
                self.state[position] = self.state[(position + 397) % 624] ^ next;
            }
            self.index = 0;
        }
        let mut value = self.state[self.index];
        self.index += 1;
        value ^= value >> 11;
        value ^= (value << 7) & 0x9D2C_5680;
        value ^= (value << 15) & 0xEFC6_0000;
        value ^= value >> 18;
        value
    }

    fn getrandbits(&mut self, bits: u32) -> u32 {
        self.next_u32() >> (32 - bits)
    }

    fn randbelow(&mut self, bound: usize) -> usize {
        let bits = 64 - (bound as u64).leading_zeros();
        loop {
            let value = self.getrandbits(bits);
            if (value as usize) < bound {
                return value as usize;
            }
        }
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        for position in (1..items.len()).rev() {
            let other = self.randbelow(position + 1);
            items.swap(position, other);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::Bar;

    fn bar(index: u64, timestamp: i64) -> Bar {
        Bar::from_prices(index, timestamp, 1.0, 1.0, 1.0, 1.0)
    }

    fn event(index: u64, code: u32, bar: u64) -> Event {
        Event {
            event_index: index,
            code,
            type_code: 1,
            subtype_code: 1,
            source_code: 1,
            occurred_bar: bar,
            available_bar: bar,
            direction: 0,
            reference_code: None,
            primary: None,
            reference: None,
            completeness: None,
        }
    }

    fn catalog() -> Catalog {
        let mut catalog = Catalog::new();
        catalog.insert(149, "level.fib_test".to_owned());
        catalog
    }

    fn spec() -> WindowSpec {
        WindowSpec {
            event_codes: vec![149],
            window_bars: 200,
            candidate_zone: (60, 100),
            max_windows: 4,
            min_windows: 1,
            seed: 0,
            overlap_bars: 120,
            from_ts: None,
            to_ts: None,
        }
    }

    fn sample_bundle() -> Bundle {
        let bars = (0..400)
            .map(|index| bar(index, 1_514_764_800 + index as i64 * 900))
            .collect();
        let events = vec![event(1, 149, 60), event(2, 149, 160), event(3, 149, 260)];
        Bundle::new(900, bars, events)
    }

    #[test]
    fn the_reference_shuffle_is_reproduced() {
        let mut values: Vec<u32> = (0..10).collect();
        Mt19937::from_seed(2018).shuffle(&mut values);
        assert_eq!(values, vec![6, 1, 7, 9, 4, 5, 3, 0, 2, 8]);
        let mut values: Vec<u32> = (0..10).collect();
        Mt19937::from_seed(2019).shuffle(&mut values);
        assert_eq!(values, vec![0, 4, 8, 9, 5, 6, 1, 7, 3, 2]);
        let mut generator = Mt19937::from_seed(2018);
        let bits: Vec<u32> = (0..4).map(|_| generator.getrandbits(8)).collect();
        assert_eq!(bits, vec![136, 32, 7, 103]);
    }

    #[test]
    fn the_three_fib_bars_are_selected() {
        let record = select(&sample_bundle(), &spec(), &catalog()).expect("select");
        assert_eq!(record.candidate_count, 3);
        assert_eq!(record.selected_count, 3);
        assert!(!record.underpowered);
        let bars: Vec<u64> = record
            .windows
            .iter()
            .map(|window| window.candidate_bar)
            .collect();
        assert_eq!(bars, vec![60, 160, 260]);
        assert_eq!(record.windows[0].events[0].name, "level.fib_test");
    }

    #[test]
    fn the_same_seed_selects_the_same_windows() {
        let first = select(&sample_bundle(), &spec(), &catalog()).expect("first");
        let second = select(&sample_bundle(), &spec(), &catalog()).expect("second");
        assert_eq!(first, second);
    }

    #[test]
    fn the_maximum_is_respected() {
        let mut spec = spec();
        spec.max_windows = 2;
        let record = select(&sample_bundle(), &spec, &catalog()).expect("select");
        assert_eq!(record.selected_count, 2);
    }

    #[test]
    fn a_thin_sample_is_underpowered() {
        let mut spec = spec();
        spec.min_windows = 10;
        let record = select(&sample_bundle(), &spec, &catalog()).expect("select");
        assert!(record.underpowered);
    }

    #[test]
    fn the_date_filter_reduces_candidates() {
        let mut spec = spec();
        spec.from_ts = Some(1_514_764_800 + 900 * 100);
        let record = select(&sample_bundle(), &spec, &catalog()).expect("select");
        assert_eq!(record.candidate_count, 2);
        let bars: Vec<u64> = record
            .windows
            .iter()
            .map(|window| window.candidate_bar)
            .collect();
        assert_eq!(bars, vec![160, 260]);
    }

    #[test]
    fn the_window_bounds_stay_inside_the_series() {
        let record = select(&sample_bundle(), &spec(), &catalog()).expect("select");
        for window in &record.windows {
            assert!(window.start_index < window.end_index);
            assert!(window.end_index <= 400);
            assert!(window.start_index <= window.candidate_index);
            assert!(window.candidate_index < window.end_index);
        }
    }

    #[test]
    fn an_empty_code_list_is_refused() {
        let mut spec = spec();
        spec.event_codes.clear();
        assert_eq!(
            select(&sample_bundle(), &spec, &catalog()),
            Err(WindowError::NoEventCodes)
        );
    }

    #[test]
    fn an_empty_bundle_is_refused() {
        let bundle = Bundle::new(900, Vec::new(), Vec::new());
        assert_eq!(
            select(&bundle, &spec(), &catalog()),
            Err(WindowError::EmptyBundle)
        );
    }
}
