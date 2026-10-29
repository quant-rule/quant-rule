"""Independent replay of the Progression axis.

This module implements contract §8.4 and §9 from the checked-in readings: the
minor and major leg state machines, target selection and revision, target
stability, progress, stage, the confirmed-leg publication rule, readiness,
unavailable reasons, the empirical percentile, and the state counters.

It is a pure function of its inputs. It does not import, call, or read the Rust
implementation, the filesystem, the clock, or a random source. The replay CLI
in ``progression_replay.py`` owns all file I/O.

The semantics follow the contract and the approved D1-D10 decisions from the
2026-10-24 Progression plan. Where the contract leaves a choice open, the
choice is stated in a comment next to the code so a reviewer can reject it
without reading the Rust code.
"""

from __future__ import annotations

from bisect import bisect_left, bisect_right, insort
from dataclasses import dataclass, replace
import math
from typing import Iterable, Optional, Sequence

MINOR = 1
MAJOR = 2
SCALES = (MINOR, MAJOR)
LOOKBACK = {MINOR: 2, MAJOR: 5}
KINDS = ("high", "low")

PERCENTILE_WINDOW = 252
PERCENTILE_MIN_SAMPLES = 50
STATE_CHANGE_WINDOW = 50
MAX_RETAINED_LEGS = 1024

FORMING = 0
ACTIVE = 1
COMPLETED = 2
INVALIDATED = 3
SUPERSEDED = 4
TERMINAL = (COMPLETED, INVALIDATED, SUPERSEDED)

STRUCTURAL_BASIS = 1
ATR_FALLBACK_BASIS = 2

INSUFFICIENT_BARS = 1
INVALID_DENOMINATOR = 3
NO_TARGET = 4
ORIGIN_UNCONFIRMED = 6


@dataclass(frozen=True)
class Candle:
    """One closed OHLCV candle, as supplied by the readings fixture."""

    bar: int
    timestamp: int
    open: float
    high: float
    low: float
    close: float
    volume: Optional[float]


@dataclass(frozen=True)
class Pivot:
    """A candidate or confirmed swing pivot.

    A candidate has ``available_at == occurred_at``; a confirmed pivot is
    stored at the bar its right-hand lookback closed.
    """

    scale: int
    kind: str
    price: float
    occurred_at: int
    available_at: int


@dataclass(frozen=True)
class Target:
    """A selected target price and the contract's basis code."""

    price: float
    basis_code: int


@dataclass(frozen=True)
class LegRecord:
    """The contract's §8.4 leg record, with numeric fields only."""

    leg_id: int
    timeframe_secs: Optional[int]
    scale_code: int
    origin_price: float
    origin_occurred_at: int
    origin_available_at: Optional[int]
    origin_confirmed_code: int
    direction_code: int
    target_price: Optional[float]
    target_basis_code: Optional[int]
    target_version: int
    target_stable_code: int
    progress: Optional[float]
    displacement_price: Optional[float]
    displacement_atr: Optional[float]
    lifecycle_code: int
    stage_code: Optional[int]
    target_reached_code: Optional[int]


@dataclass(frozen=True)
class Bar:
    """The published Progression values for one closed bar."""

    progress: Optional[float]
    stage_code: Optional[int]
    leg_direction: Optional[int]
    leg_scale_code: Optional[int]
    size_percentile: Optional[float]
    readiness_code: int
    unavailable_reason_code: Optional[int]
    state_age_bars: int
    state_changes_50: int
    published_leg: Optional[LegRecord]
    minor_leg: Optional[LegRecord]
    major_leg: Optional[LegRecord]


@dataclass(frozen=True)
class ReplayResult:
    """The per-bar series and the retained-leg ledger."""

    bars: tuple[Bar, ...]
    legs: tuple[LegRecord, ...]


@dataclass
class Leg:
    """One mutable leg state."""

    leg_id: int
    scale: int
    direction: int
    origin_price: float
    origin_occurred_at: int
    atr_at_origin: Optional[float]
    target: Optional[Target]
    origin_available_at: Optional[int] = None
    origin_confirmed: bool = False
    target_version: int = 0
    target_stable: bool = False
    revision_bar: Optional[int] = None
    lifecycle: int = FORMING
    progress: Optional[float] = None
    displacement_price: Optional[float] = None
    displacement_atr: Optional[float] = None
    stage: Optional[int] = None
    missing_close: bool = False
    target_reached: Optional[int] = None
    breached: bool = False
    frozen: bool = False

    @classmethod
    def create(
        cls,
        leg_id: int,
        scale: int,
        direction: int,
        origin_price: float,
        origin_occurred_at: int,
        atr_at_origin: Optional[float],
        target: Optional[Target],
    ) -> "Leg":
        """Creates a forming leg from a pivot candidate.

        A target that exists at creation is stable. A leg with no target has
        ``target_stable_code`` 0, so its stage stays null until a target is
        selected.
        """
        return cls(
            leg_id=leg_id,
            scale=scale,
            direction=direction,
            origin_price=origin_price,
            origin_occurred_at=origin_occurred_at,
            atr_at_origin=atr_at_origin,
            target=target,
            target_stable=target is not None,
        )

    def confirm(self, bar: int) -> None:
        """Promotes a forming leg at its confirmation bar.

        A terminal leg keeps its terminal lifecycle but still records that its
        origin confirmed, because the origin fact is independent of the leg's
        final state. A breach recorded while forming is spent: the active rule
        applies from this bar on.
        """
        self.origin_confirmed = True
        self.origin_available_at = bar
        self.breached = False
        if self.lifecycle == FORMING:
            self.lifecycle = ACTIVE

    def terminate(self, lifecycle: int) -> None:
        """Moves the leg to a terminal lifecycle, if the code is terminal."""
        if lifecycle not in TERMINAL:
            return
        self.lifecycle = lifecycle
        self.breached = False

    def revise(self, target: Target, bar: int) -> None:
        """Applies a newly selected target.

        The version and stability change only when the selected price or basis
        actually changes. A confirmation that leaves the nearest target in
        place therefore does not manufacture a revision.
        """
        if (
            self.target is not None
            and self.target.price == target.price
            and self.target.basis_code == target.basis_code
        ):
            return
        self.target = target
        self.target_version += 1
        self.target_stable = False
        self.revision_bar = bar

    def breach(self, close: float) -> bool:
        """Whether ``close`` is beyond the origin against the leg.

        The threshold is strictly greater than ``0.2 * ATR_at_origin``. A leg
        without a usable ATR can never be invalidated on this rule.
        """
        if self.atr_at_origin is None:
            return False
        beyond = (
            self.origin_price - close
            if self.direction > 0
            else close - self.origin_price
        )
        return beyond > 0.2 * self.atr_at_origin

    def update(self, close: Optional[float], bar: int) -> None:
        """Observes one closed bar.

        The target's stability clock advances even after the measurements
        freeze, because stability is a property of the unchanged reference.
        A frozen leg keeps its progress, displacement, target, stage, and
        target-reached fields. A missing candle clears the measurements but
        still advances the stability clock, because the bar itself closed.
        """
        self._update_stability(bar)
        if self.frozen:
            return

        if close is not None:
            self.missing_close = False
            self._measure(close)
        else:
            self.missing_close = True
            self.progress = None
            self.displacement_price = None
            self.displacement_atr = None
            self.target_reached = None

        self._update_stage()
        if self.lifecycle in TERMINAL:
            self.frozen = True

    def unavailable_reason(self) -> Optional[int]:
        """Why this leg's progress is null, or ``None`` when it is available."""
        if self.progress is not None:
            return None
        if self.lifecycle == FORMING:
            return ORIGIN_UNCONFIRMED
        if self.target is None:
            return NO_TARGET
        if self._target_is_degenerate():
            return INVALID_DENOMINATOR
        if self.missing_close:
            return INSUFFICIENT_BARS
        return INVALID_DENOMINATOR

    def record(self, timeframe_secs: Optional[int] = None) -> LegRecord:
        """Builds the published numeric record."""
        return LegRecord(
            leg_id=self.leg_id,
            timeframe_secs=timeframe_secs,
            scale_code=self.scale,
            origin_price=self.origin_price,
            origin_occurred_at=self.origin_occurred_at,
            origin_available_at=self.origin_available_at,
            origin_confirmed_code=1 if self.origin_confirmed else 0,
            direction_code=self.direction,
            target_price=self.target.price if self.target is not None else None,
            target_basis_code=(
                self.target.basis_code if self.target is not None else None
            ),
            target_version=self.target_version,
            target_stable_code=1 if self.target_stable else 0,
            progress=self.progress,
            displacement_price=self.displacement_price,
            displacement_atr=self.displacement_atr,
            lifecycle_code=self.lifecycle,
            stage_code=self.stage,
            target_reached_code=self.target_reached,
        )

    def _update_stability(self, bar: int) -> None:
        if self.revision_bar is not None:
            if bar >= self.revision_bar + 3:
                self.target_stable = True
                self.revision_bar = None
        else:
            self.target_stable = self.target is not None

    def _measure(self, close: float) -> None:
        self.displacement_price = close - self.origin_price
        self.displacement_atr = (
            (close - self.origin_price) / self.atr_at_origin
            if self.atr_at_origin is not None
            else None
        )

        if self.target is None or self._target_is_degenerate():
            self.progress = None
            self.target_reached = None
            return

        progress = (close - self.origin_price) / (
            self.target.price - self.origin_price
        )
        if not math.isfinite(progress):
            self.progress = None
            self.target_reached = None
            return

        self.progress = progress
        self.target_reached = 1 if progress >= 1.0 else 0

    def _target_is_degenerate(self) -> bool:
        if self.target is None:
            return False
        distance = abs(self.target.price - self.origin_price)
        return self.target.price == self.origin_price or (
            self.atr_at_origin is not None
            and distance < 0.2 * self.atr_at_origin
        )

    def _update_stage(self) -> None:
        if self.lifecycle in TERMINAL:
            self.stage = 5
            return
        if self.lifecycle == FORMING or not self.target_stable:
            self.stage = None
            return
        if self.progress is None:
            self.stage = None
            return
        self.stage = stage_for(self.progress)


class LevelIndex:
    """A price-sorted index of the confirmed levels available so far.

    The registry is append-only. ``advance`` inserts every level whose
    availability bar is at or before the current bar, so a target selection
    never reads a level that was not knowable yet. The readings fixture has no
    level-invalidation input, so a level is never removed; revision is
    exercised on newly confirmed levels only.
    """

    def __init__(self, pivots: Iterable[Pivot]):
        self._pivots = sorted(
            pivots,
            key=lambda pivot: (
                pivot.available_at,
                pivot.occurred_at,
                pivot.scale,
                pivot.kind,
            ),
        )
        self._prices = {
            (scale, kind): [] for scale in SCALES for kind in KINDS
        }
        self._next = 0

    def advance(self, bar: int) -> None:
        """Inserts every level whose availability bar is at or before ``bar``."""
        while self._next < len(self._pivots):
            pivot = self._pivots[self._next]
            if pivot.available_at > bar:
                break
            insort(self._prices[(pivot.scale, pivot.kind)], pivot.price)
            self._next += 1

    def nearest(
        self,
        scale: int,
        kind: str,
        direction: int,
        origin_price: float,
    ) -> Optional[float]:
        """The nearest level beyond the origin in the leg's direction.

        For an upward leg the target is the lowest high strictly above the
        origin; for a downward leg the highest low strictly below it. An exact
        price tie is represented by the same price, so the result is
        unambiguous.
        """
        prices = self._prices[(scale, kind)]
        if direction > 0:
            position = bisect_right(prices, origin_price)
            return prices[position] if position < len(prices) else None
        position = bisect_left(prices, origin_price)
        return prices[position - 1] if position > 0 else None


def select_target(
    index: LevelIndex,
    scale: int,
    direction: int,
    origin_price: float,
    atr_at_origin: Optional[float],
) -> Optional[Target]:
    """Selects the target for a leg at the current bar.

    A confirmed structural level always takes precedence over the ATR
    fallback. The fallback requires a usable ATR; with neither, the leg has no
    target.
    """
    kind = "high" if direction > 0 else "low"
    price = index.nearest(scale, kind, direction, origin_price)
    if price is not None:
        return Target(price=price, basis_code=STRUCTURAL_BASIS)

    if atr_at_origin is None:
        return None
    fallback = origin_price + direction * 2.0 * atr_at_origin
    if not math.isfinite(fallback):
        return None
    return Target(price=fallback, basis_code=ATR_FALLBACK_BASIS)


def stage_for(progress: float) -> int:
    """The contract's progress bucket."""
    if progress < 0.0:
        return 0
    if progress < 0.25:
        return 1
    if progress < 0.5:
        return 2
    if progress < 0.75:
        return 3
    return 4


def empirical_percentile(
    samples: Sequence[float], value: Optional[float]
) -> Optional[float]:
    """The empirical percentile of ``value`` against the last 252 samples.

    The window keeps the most recent finite samples, requires at least 50, and
    compares inclusively. The current bar is never in ``samples``.
    """
    if value is None or not math.isfinite(value):
        return None

    window: list[float] = []
    for sample in reversed(samples):
        if sample is not None and math.isfinite(sample):
            window.append(sample)
            if len(window) == PERCENTILE_WINDOW:
                break
    if len(window) < PERCENTILE_MIN_SAMPLES:
        return None

    window.reverse()
    window.sort()
    count = sum(1 for sample in window if sample <= value)
    return 100.0 * count / len(window)


class StateCounters:
    """The contract's compact-state age and 50-bar change counter."""

    def __init__(self) -> None:
        self.last_code: Optional[int] = None
        self.age_bars = 0
        self.changes_50 = 0
        self._window = [False] * STATE_CHANGE_WINDOW
        self._next_slot = 0

    def observe(self, code: Optional[int]) -> tuple[int, int]:
        """Observes one bar and returns ``(age_bars, changes_50)``.

        A null code freezes both counters. The first code-bearing bar has age
        1; each later bar with the same code increments it, and a change resets
        it to 1.
        """
        if code is None:
            return self.age_bars, self.changes_50

        if self.last_code is not None and self.last_code == code:
            self.age_bars += 1
            self._advance_window(False)
        elif self.last_code is not None:
            self.age_bars = 1
            self._advance_window(True)
        else:
            self.age_bars = 1
            self._advance_window(False)
        self.last_code = code
        return self.age_bars, self.changes_50

    def _advance_window(self, changed: bool) -> None:
        slot = self._next_slot
        if self._window[slot]:
            self.changes_50 -= 1
        self._window[slot] = changed
        if changed:
            self.changes_50 += 1
        self._next_slot = (slot + 1) % STATE_CHANGE_WINDOW


def transition_prior(legs: list[Leg], index: int) -> None:
    """Ends the most recent active leg before a confirmation.

    The prior leg is completed when it points the other way and superseded
    when it points the same way. Only an active leg is transitioned: a forming
    leg has not confirmed its origin, and a terminal leg is final. Skipping the
    failed candidates in between keeps an older active leg reachable.
    """
    direction = legs[index].direction
    prior_index = None
    for position in range(index - 1, -1, -1):
        if legs[position].lifecycle == ACTIVE:
            prior_index = position
            break
    if prior_index is None:
        return

    prior = legs[prior_index]
    prior.terminate(
        SUPERSEDED if prior.direction == direction else COMPLETED
    )


def published_leg(minor: Sequence[Leg], major: Sequence[Leg]) -> Optional[Leg]:
    """The newest leg whose origin is confirmed, or the newest forming leg.

    A newer unconfirmed candidate never displaces a confirmed leg, because its
    origin may never exist. Before the first confirmation, the newest retained
    leg is published so the axis still reports the forming candidate.
    """
    legs = list(minor) + list(major)
    confirmed = [leg for leg in legs if leg.origin_confirmed]
    if confirmed:
        return max(confirmed, key=lambda leg: leg.leg_id)
    return max(legs, key=lambda leg: leg.leg_id) if legs else None


def readiness(
    published: Optional[Leg],
    progress: Optional[float],
    stage_code: Optional[int],
    size_percentile: Optional[float],
) -> int:
    """The contract's mechanical completeness code."""
    if published is None or progress is None:
        return 0
    if stage_code is None or not published.target_stable:
        return 1
    if size_percentile is not None:
        return 2
    return 1


def _usable_atr(values: Sequence[Optional[float]], bar: int) -> Optional[float]:
    if bar < 0 or bar >= len(values):
        return None
    value = values[bar]
    if value is None or not math.isfinite(value) or value <= 0.0:
        return None
    return value


def _at(values: Sequence[Optional[Pivot]], bar: int) -> Optional[Pivot]:
    if bar < 0 or bar >= len(values):
        return None
    return values[bar]


def _direction_for(kind: str) -> int:
    return 1 if kind == "low" else -1


def replay(
    candles: Sequence[Candle],
    candidates: dict[int, Sequence[Optional[Pivot]]],
    confirmed: dict[int, Sequence[Optional[Pivot]]],
    atr14: Sequence[Optional[float]],
) -> ReplayResult:
    """Replays the Progression axis over the readings.

    ``candidates`` and ``confirmed`` are indexed by bar for each scale. The
    confirmed series stores a pivot at its availability bar. The result has one
    ``Bar`` per candle and the retained-leg ledger in creation order.
    """
    length = len(candles)
    for scale in SCALES:
        if len(candidates[scale]) != length:
            raise ValueError(f"{scale} candidate series has the wrong length")
        if len(confirmed[scale]) != length:
            raise ValueError(f"{scale} confirmed series has the wrong length")
    if len(atr14) != length:
        raise ValueError("atr14 series has the wrong length")

    index = LevelIndex(
        pivot
        for scale in SCALES
        for pivot in confirmed[scale]
        if pivot is not None
    )
    scale_legs = {MINOR: [], MAJOR: []}  # type: dict[int, list[Leg]]
    bars: list[Bar] = []
    counters = StateCounters()
    magnitudes: list[float] = []
    next_leg_id = 1

    for bar in range(length):
        index.advance(bar)
        close = candles[bar].close

        # A confirmation promotes its own leg, transitions the most recent
        # active leg, and revises the targets that the newly confirmed level
        # can improve. The prior leg freezes before the revision pass, so it is
        # not revised on the bar it ends.
        for scale in SCALES:
            pivot = _at(confirmed[scale], bar)
            if pivot is None:
                continue
            direction = _direction_for(pivot.kind)
            legs = scale_legs[scale]
            position = next(
                (
                    position
                    for position, leg in enumerate(legs)
                    if leg.origin_occurred_at == pivot.occurred_at
                    and leg.direction == direction
                ),
                None,
            )
            if position is None:
                continue

            transition_prior(legs, position)
            legs[position].confirm(bar)

            # A high can only target an upward leg; a low a downward leg.
            target_direction = -direction
            for leg in legs:
                if leg.lifecycle in TERMINAL or leg.direction != target_direction:
                    continue
                target = select_target(
                    index,
                    scale,
                    leg.direction,
                    leg.origin_price,
                    leg.atr_at_origin,
                )
                if target is not None:
                    leg.revise(target, bar)

        # A candidate is knowable at the close of its own bar, so the leg and
        # its target selection never use a later level.
        for scale in SCALES:
            candidate = _at(candidates[scale], bar)
            if candidate is None:
                continue
            atr_at_origin = _usable_atr(atr14, candidate.occurred_at)
            direction = _direction_for(candidate.kind)
            target = select_target(
                index,
                scale,
                direction,
                candidate.price,
                atr_at_origin,
            )
            legs = scale_legs[scale]
            legs.append(
                Leg.create(
                    leg_id=next_leg_id,
                    scale=scale,
                    direction=direction,
                    origin_price=candidate.price,
                    origin_occurred_at=candidate.occurred_at,
                    atr_at_origin=atr_at_origin,
                    target=target,
                )
            )
            next_leg_id += 1
            while len(legs) > MAX_RETAINED_LEGS:
                terminal = next(
                    (
                        position
                        for position, leg in enumerate(legs)
                        if leg.lifecycle in TERMINAL
                    ),
                    None,
                )
                if terminal is None:
                    break
                legs.pop(terminal)

        # Record a forming leg's breach, then resolve the candidates whose
        # lookback window closes on this bar. A confirmed candidate is no
        # longer forming, so it is not resolved here.
        if close is not None:
            for scale in SCALES:
                for leg in scale_legs[scale]:
                    if leg.lifecycle == FORMING and leg.breach(close):
                        leg.breached = True

        for scale in SCALES:
            lookback = LOOKBACK[scale]
            legs = scale_legs[scale]
            position = 0
            while position < len(legs):
                leg = legs[position]
                fails = (
                    leg.lifecycle == FORMING
                    and not leg.origin_confirmed
                    and leg.origin_occurred_at + lookback == bar
                )
                if not fails:
                    position += 1
                    continue

                if leg.breached and leg.atr_at_origin is not None:
                    leg.terminate(INVALIDATED)
                    position += 1
                else:
                    # The candidate did not confirm and never moved far enough
                    # against the leg to invalidate it.
                    legs.pop(position)

        if close is not None:
            for scale in SCALES:
                for leg in scale_legs[scale]:
                    if leg.lifecycle == ACTIVE and leg.breach(close):
                        leg.terminate(INVALIDATED)

        # Every retained leg advances its stability clock and its
        # measurements. A leg terminated on this bar measures this bar and then
        # freezes; a leg terminated earlier is unchanged.
        for scale in SCALES:
            for leg in scale_legs[scale]:
                leg.update(close, bar)

        published = published_leg(scale_legs[MINOR], scale_legs[MAJOR])
        progress = published.progress if published is not None else None
        stage_code = published.stage if published is not None else None
        magnitude = None
        if (
            published is not None
            and published.displacement_atr is not None
            and math.isfinite(published.displacement_atr)
        ):
            magnitude = abs(published.displacement_atr)
        size_percentile = (
            empirical_percentile(magnitudes, magnitude)
            if magnitude is not None
            else None
        )

        if published is None:
            reason = ORIGIN_UNCONFIRMED
        elif progress is None and close is None:
            reason = INSUFFICIENT_BARS
        elif progress is None:
            reason = published.unavailable_reason()
        else:
            reason = None

        age_bars, changes_50 = counters.observe(stage_code)
        bars.append(
            Bar(
                progress=progress,
                stage_code=stage_code,
                leg_direction=(
                    published.direction if published is not None else None
                ),
                leg_scale_code=(
                    published.scale if published is not None else None
                ),
                size_percentile=size_percentile,
                readiness_code=readiness(
                    published, progress, stage_code, size_percentile
                ),
                unavailable_reason_code=reason,
                state_age_bars=age_bars,
                state_changes_50=changes_50,
                published_leg=(
                    published.record() if published is not None else None
                ),
                minor_leg=(
                    scale_legs[MINOR][-1].record()
                    if scale_legs[MINOR]
                    else None
                ),
                major_leg=(
                    scale_legs[MAJOR][-1].record()
                    if scale_legs[MAJOR]
                    else None
                ),
            )
        )

        if magnitude is not None:
            if len(magnitudes) == PERCENTILE_WINDOW:
                magnitudes.pop(0)
            magnitudes.append(magnitude)

    ledger = sorted(
        scale_legs[MINOR] + scale_legs[MAJOR], key=lambda leg: leg.leg_id
    )
    return ReplayResult(
        bars=tuple(bars),
        legs=tuple(leg.record() for leg in ledger),
    )


def with_timeframe(record: Optional[LegRecord], timeframe_secs: int) -> Optional[LegRecord]:
    """Returns a copy of a leg record stamped with the report's timeframe."""
    if record is None:
        return None
    return replace(record, timeframe_secs=timeframe_secs)
