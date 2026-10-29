"""Unit tests for the independent Progression replay.

The tests exercise the contract §8.4/§9 state machine directly and through
``replay`` with synthetic readings. They do not import the Rust implementation
and do not read a fixture, so a failure points at the contract semantics rather
than at a golden file.
"""

from __future__ import annotations

import unittest

import progression_model as model
import progression_replay as cli


def candle(bar: int, close: float) -> model.Candle:
    """One closed candle with a symmetric range around ``close``."""
    return model.Candle(
        bar=bar,
        timestamp=1_700_000_000 + 60 * bar,
        open=close,
        high=close + 0.2,
        low=close - 0.2,
        close=close,
        volume=1.0,
    )


def empty_series(length: int) -> list[None]:
    return [None] * length


_DEFAULT_TARGET = object()


def make_leg(
    leg_id: int = 1,
    scale: int = model.MINOR,
    direction: int = 1,
    origin: float = 100.0,
    atr: float | None = 1.0,
    target: model.Target | None | object = _DEFAULT_TARGET,
) -> model.Leg:
    """A leg with an ATR-fallback target unless a target is supplied.

    Passing ``target=None`` explicitly creates a leg with no target; omitting
    the argument uses the ATR fallback.
    """
    if target is _DEFAULT_TARGET:
        target = (
            model.Target(
                origin + direction * 2.0 * atr, model.ATR_FALLBACK_BASIS
            )
            if atr is not None
            else None
        )
    return model.Leg.create(
        leg_id=leg_id,
        scale=scale,
        direction=direction,
        origin_price=origin,
        origin_occurred_at=leg_id,
        atr_at_origin=atr,
        target=target,
    )


def confirmed_leg(
    leg_id: int = 1,
    scale: int = model.MINOR,
    direction: int = 1,
    lifecycle: int = model.ACTIVE,
) -> model.Leg:
    """A confirmed leg in the requested lifecycle, for publication tests."""
    leg = make_leg(leg_id=leg_id, scale=scale, direction=direction)
    leg.confirm(leg_id + 1)
    leg.lifecycle = lifecycle
    return leg


def bar_with_stage(stage: int | None) -> model.Bar:
    return model.Bar(
        progress=None,
        stage_code=stage,
        leg_direction=None,
        leg_scale_code=None,
        size_percentile=None,
        readiness_code=0,
        unavailable_reason_code=None,
        state_age_bars=0,
        state_changes_50=0,
        published_leg=None,
        minor_leg=None,
        major_leg=None,
    )


class StageTests(unittest.TestCase):
    def test_bucket_boundaries(self) -> None:
        cases = [
            (-0.01, 0),
            (0.0, 1),
            (0.2499, 1),
            (0.25, 2),
            (0.4999, 2),
            (0.5, 3),
            (0.7499, 3),
            (0.75, 4),
            (5.0, 4),
        ]
        for progress, expected in cases:
            with self.subTest(progress=progress):
                self.assertEqual(model.stage_for(progress), expected)


class PercentileTests(unittest.TestCase):
    def test_missing_value(self) -> None:
        self.assertIsNone(model.empirical_percentile([1.0] * 50, None))
        self.assertIsNone(model.empirical_percentile([1.0] * 50, float("nan")))

    def test_requires_fifty_samples(self) -> None:
        self.assertIsNone(model.empirical_percentile([1.0] * 49, 1.0))
        self.assertEqual(
            model.empirical_percentile([1.0] * 50, 1.0), 100.0
        )

    def test_comparison_is_inclusive(self) -> None:
        samples = [float(value) for value in range(50)]
        self.assertEqual(model.empirical_percentile(samples, 24.0), 50.0)
        self.assertEqual(model.empirical_percentile(samples, 49.0), 100.0)
        self.assertEqual(model.empirical_percentile(samples, -1.0), 0.0)

    def test_uses_only_the_most_recent_window(self) -> None:
        samples = [1000.0] * 48 + [1.0] * 252
        self.assertEqual(model.empirical_percentile(samples, 1.0), 100.0)

    def test_skips_non_finite_samples(self) -> None:
        samples = [1.0] * 49 + [float("nan"), float("inf")]
        self.assertIsNone(model.empirical_percentile(samples, 1.0))
        samples = [1.0] * 50 + [float("nan"), float("inf")]
        self.assertEqual(model.empirical_percentile(samples, 1.0), 100.0)


class CounterTests(unittest.TestCase):
    def test_first_code_and_repeats(self) -> None:
        counters = model.StateCounters()
        self.assertEqual(counters.observe(2), (1, 0))
        self.assertEqual(counters.observe(2), (2, 0))
        self.assertEqual(counters.observe(2), (3, 0))

    def test_null_freezes_both_counters(self) -> None:
        counters = model.StateCounters()
        counters.observe(2)
        counters.observe(3)
        self.assertEqual(counters.observe(None), (1, 1))
        self.assertEqual(counters.observe(3), (2, 1))

    def test_change_resets_age(self) -> None:
        counters = model.StateCounters()
        counters.observe(2)
        counters.observe(2)
        self.assertEqual(counters.observe(3), (1, 1))
        self.assertEqual(counters.observe(3), (2, 1))

    def test_window_holds_fifty_changes(self) -> None:
        counters = model.StateCounters()
        counters.observe(0)
        for index in range(50):
            counters.observe(1 if index % 2 == 0 else 0)
        self.assertEqual(counters.observe(1)[1], 50)
        # A code-bearing bar that does not change still advances the window,
        # so it can push an older change out.
        self.assertEqual(counters.observe(1)[1], 49)

    def test_window_expires_after_fifty_code_bars(self) -> None:
        counters = model.StateCounters()
        counters.observe(0)
        for index in range(50):
            counters.observe(1 if index % 2 == 0 else 0)
        counters.observe(1)
        self.assertEqual(counters.changes_50, 50)
        for _ in range(50):
            counters.observe(1)
        self.assertEqual(counters.changes_50, 0)


class LegTests(unittest.TestCase):
    def test_creation_from_candidate(self) -> None:
        leg = make_leg(leg_id=7, scale=model.MAJOR, origin=99.5)
        record = leg.record()
        self.assertEqual(record.leg_id, 7)
        self.assertEqual(record.scale_code, model.MAJOR)
        self.assertEqual(record.lifecycle_code, model.FORMING)
        self.assertEqual(record.origin_confirmed_code, 0)
        self.assertIsNone(record.origin_available_at)
        self.assertEqual(record.target_stable_code, 1)
        self.assertIsNone(record.stage_code)

    def test_creation_without_a_target_is_unstable(self) -> None:
        leg = make_leg(atr=1.0, target=None)
        self.assertEqual(leg.record().target_stable_code, 0)
        self.assertIsNone(leg.record().target_price)

    def test_confirmation_promotes_a_forming_leg(self) -> None:
        leg = make_leg()
        leg.confirm(5)
        self.assertEqual(leg.lifecycle, model.ACTIVE)
        self.assertTrue(leg.origin_confirmed)
        self.assertEqual(leg.origin_available_at, 5)
        self.assertEqual(leg.record().origin_confirmed_code, 1)

    def test_confirmation_of_a_terminal_leg_records_the_origin(self) -> None:
        leg = make_leg()
        leg.terminate(model.INVALIDATED)
        leg.confirm(5)
        self.assertEqual(leg.lifecycle, model.INVALIDATED)
        self.assertTrue(leg.origin_confirmed)
        self.assertEqual(leg.origin_available_at, 5)

    def test_terminate_accepts_only_terminal_codes(self) -> None:
        leg = make_leg()
        leg.confirm(1)
        for code in (model.FORMING, model.ACTIVE, 5, 99):
            leg.terminate(code)
            self.assertEqual(leg.lifecycle, model.ACTIVE)
        leg.terminate(model.COMPLETED)
        self.assertEqual(leg.lifecycle, model.COMPLETED)

    def test_breach_is_strict(self) -> None:
        up = make_leg(direction=1, origin=100.0, atr=10.0)
        self.assertFalse(up.breach(98.0))
        self.assertTrue(up.breach(97.99))

        down = make_leg(direction=-1, origin=100.0, atr=10.0)
        self.assertFalse(down.breach(102.0))
        self.assertTrue(down.breach(102.01))

    def test_a_leg_without_atr_never_breaches(self) -> None:
        leg = make_leg(direction=1, origin=100.0, atr=None)
        self.assertFalse(leg.breach(0.0))

    def test_measurement_and_stage(self) -> None:
        leg = make_leg(origin=100.0, atr=10.0)
        leg.target = model.Target(110.0, model.STRUCTURAL_BASIS)
        leg.confirm(0)

        leg.update(105.0, 0)
        self.assertAlmostEqual(leg.progress, 0.5)
        self.assertAlmostEqual(leg.displacement_price, 5.0)
        self.assertAlmostEqual(leg.displacement_atr, 0.5)
        self.assertEqual(leg.target_reached, 0)
        self.assertEqual(leg.stage, 3)

        leg.update(110.0, 1)
        self.assertAlmostEqual(leg.progress, 1.0)
        self.assertEqual(leg.target_reached, 1)
        self.assertEqual(leg.stage, 4)

        leg.update(90.0, 2)
        self.assertAlmostEqual(leg.progress, -1.0)
        self.assertEqual(leg.target_reached, 0)
        self.assertEqual(leg.stage, 0)

    def test_missing_close_clears_the_measurements(self) -> None:
        leg = make_leg(origin=100.0, atr=10.0)
        leg.target = model.Target(110.0, model.STRUCTURAL_BASIS)
        leg.confirm(0)
        leg.update(105.0, 0)
        leg.update(None, 1)
        self.assertIsNone(leg.progress)
        self.assertIsNone(leg.displacement_price)
        self.assertIsNone(leg.displacement_atr)
        self.assertIsNone(leg.target_reached)
        self.assertIsNone(leg.stage)
        self.assertTrue(leg.missing_close)

    def test_terminal_measurements_freeze(self) -> None:
        leg = make_leg(origin=100.0, atr=10.0)
        leg.target = model.Target(110.0, model.STRUCTURAL_BASIS)
        leg.confirm(0)
        leg.update(105.0, 0)
        leg.terminate(model.COMPLETED)
        leg.update(90.0, 1)
        self.assertAlmostEqual(leg.progress, -1.0)
        self.assertEqual(leg.stage, 5)
        leg.update(120.0, 2)
        self.assertAlmostEqual(leg.progress, -1.0)
        self.assertAlmostEqual(leg.displacement_price, -10.0)
        self.assertEqual(leg.stage, 5)

    def test_unavailable_reasons(self) -> None:
        forming = make_leg()
        self.assertEqual(forming.unavailable_reason(), model.ORIGIN_UNCONFIRMED)

        no_target = make_leg(atr=1.0, target=None)
        no_target.confirm(0)
        no_target.update(100.0, 0)
        self.assertEqual(no_target.unavailable_reason(), model.NO_TARGET)

        degenerate = make_leg(origin=100.0, atr=1.0)
        degenerate.target = model.Target(100.0, model.STRUCTURAL_BASIS)
        degenerate.confirm(0)
        degenerate.update(100.0, 0)
        self.assertEqual(
            degenerate.unavailable_reason(), model.INVALID_DENOMINATOR
        )

        near_degenerate = make_leg(origin=100.0, atr=1.0)
        near_degenerate.target = model.Target(100.1, model.STRUCTURAL_BASIS)
        near_degenerate.confirm(0)
        near_degenerate.update(100.0, 0)
        self.assertEqual(
            near_degenerate.unavailable_reason(), model.INVALID_DENOMINATOR
        )

        missing = make_leg(origin=100.0, atr=1.0)
        missing.target = model.Target(110.0, model.STRUCTURAL_BASIS)
        missing.confirm(0)
        missing.update(None, 0)
        self.assertEqual(
            missing.unavailable_reason(), model.INSUFFICIENT_BARS
        )

        available = make_leg(origin=100.0, atr=1.0)
        available.target = model.Target(110.0, model.STRUCTURAL_BASIS)
        available.confirm(0)
        available.update(105.0, 0)
        self.assertIsNone(available.unavailable_reason())

    def test_degenerate_target_rule(self) -> None:
        leg = make_leg(origin=100.0, atr=1.0)
        leg.target = model.Target(100.0, model.STRUCTURAL_BASIS)
        self.assertTrue(leg._target_is_degenerate())
        leg.target = model.Target(100.19, model.STRUCTURAL_BASIS)
        self.assertTrue(leg._target_is_degenerate())
        leg.target = model.Target(100.2, model.STRUCTURAL_BASIS)
        self.assertFalse(leg._target_is_degenerate())
        leg.atr_at_origin = None
        leg.target = model.Target(100.1, model.STRUCTURAL_BASIS)
        self.assertFalse(leg._target_is_degenerate())

    def test_revising_to_the_same_target_is_a_noop(self) -> None:
        leg = make_leg(origin=100.0, atr=10.0)
        leg.target = model.Target(110.0, model.STRUCTURAL_BASIS)
        leg.revise(model.Target(110.0, model.STRUCTURAL_BASIS), 5)
        self.assertEqual(leg.target_version, 0)
        self.assertTrue(leg.target_stable)
        self.assertIsNone(leg.revision_bar)

    def test_revision_delays_stability_by_three_bars(self) -> None:
        leg = make_leg(origin=100.0, atr=10.0)
        leg.target = model.Target(110.0, model.STRUCTURAL_BASIS)
        leg.confirm(0)
        leg.revise(model.Target(108.0, model.STRUCTURAL_BASIS), 5)
        self.assertEqual(leg.target_version, 1)
        self.assertFalse(leg.target_stable)
        self.assertEqual(leg.revision_bar, 5)

        for bar in (6, 7):
            leg.update(105.0, bar)
            self.assertFalse(leg.target_stable)
            self.assertIsNone(leg.stage)

        leg.update(105.0, 8)
        self.assertTrue(leg.target_stable)
        self.assertIsNone(leg.revision_bar)
        self.assertEqual(leg.stage, 3)


class TargetTests(unittest.TestCase):
    def setUp(self) -> None:
        self.index = model.LevelIndex(
            [
                model.Pivot(model.MINOR, "high", 102.0, 1, 1),
                model.Pivot(model.MINOR, "high", 105.0, 2, 2),
                model.Pivot(model.MINOR, "high", 101.0, 3, 3),
                model.Pivot(model.MINOR, "low", 98.0, 4, 4),
                model.Pivot(model.MINOR, "low", 95.0, 5, 5),
                model.Pivot(model.MAJOR, "high", 103.0, 6, 6),
            ]
        )
        self.index.advance(10)

    def test_nearest_structural_high_above_the_origin(self) -> None:
        target = model.select_target(self.index, model.MINOR, 1, 100.0, 1.0)
        self.assertEqual(target, model.Target(101.0, model.STRUCTURAL_BASIS))

    def test_an_exact_origin_is_not_a_target(self) -> None:
        target = model.select_target(self.index, model.MINOR, 1, 101.0, 1.0)
        self.assertEqual(target, model.Target(102.0, model.STRUCTURAL_BASIS))

    def test_nearest_structural_low_below_the_origin(self) -> None:
        target = model.select_target(self.index, model.MINOR, -1, 100.0, 1.0)
        self.assertEqual(target, model.Target(98.0, model.STRUCTURAL_BASIS))
        target = model.select_target(self.index, model.MINOR, -1, 98.0, 1.0)
        self.assertEqual(target, model.Target(95.0, model.STRUCTURAL_BASIS))

    def test_atr_fallback(self) -> None:
        empty = model.LevelIndex([])
        empty.advance(10)
        self.assertEqual(
            model.select_target(empty, model.MINOR, 1, 100.0, 2.0),
            model.Target(104.0, model.ATR_FALLBACK_BASIS),
        )
        self.assertEqual(
            model.select_target(empty, model.MINOR, -1, 100.0, 2.0),
            model.Target(96.0, model.ATR_FALLBACK_BASIS),
        )

    def test_no_target_without_a_level_or_atr(self) -> None:
        empty = model.LevelIndex([])
        empty.advance(10)
        self.assertIsNone(
            model.select_target(empty, model.MINOR, 1, 100.0, None)
        )

    def test_a_level_is_not_visible_before_it_is_available(self) -> None:
        index = model.LevelIndex(
            [model.Pivot(model.MINOR, "high", 102.0, 1, 5)]
        )
        index.advance(4)
        self.assertIsNone(
            model.select_target(index, model.MINOR, 1, 100.0, None)
        )
        index.advance(5)
        self.assertEqual(
            model.select_target(index, model.MINOR, 1, 100.0, None),
            model.Target(102.0, model.STRUCTURAL_BASIS),
        )

    def test_scales_are_isolated(self) -> None:
        target = model.select_target(self.index, model.MAJOR, 1, 100.0, 1.0)
        self.assertEqual(target, model.Target(103.0, model.STRUCTURAL_BASIS))
        target = model.select_target(self.index, model.MAJOR, 1, 104.0, None)
        self.assertIsNone(target)


class TransitionTests(unittest.TestCase):
    def test_an_opposing_confirmation_completes_the_active_leg(self) -> None:
        legs = [
            confirmed_leg(leg_id=1, direction=1),
            confirmed_leg(leg_id=2, direction=-1),
        ]
        model.transition_prior(legs, 1)
        self.assertEqual(legs[0].lifecycle, model.COMPLETED)
        self.assertEqual(legs[1].lifecycle, model.ACTIVE)

    def test_a_same_direction_confirmation_supersedes_the_active_leg(
        self,
    ) -> None:
        legs = [
            confirmed_leg(leg_id=1, direction=1),
            confirmed_leg(leg_id=2, direction=1),
        ]
        model.transition_prior(legs, 1)
        self.assertEqual(legs[0].lifecycle, model.SUPERSEDED)

    def test_forming_and_terminal_legs_are_skipped(self) -> None:
        legs = [
            confirmed_leg(leg_id=1, direction=1),
            confirmed_leg(leg_id=2, direction=-1, lifecycle=model.COMPLETED),
            confirmed_leg(leg_id=3, direction=-1, lifecycle=model.FORMING),
            confirmed_leg(leg_id=4, direction=-1),
        ]
        model.transition_prior(legs, 3)
        self.assertEqual(legs[0].lifecycle, model.COMPLETED)
        self.assertEqual(legs[1].lifecycle, model.COMPLETED)
        self.assertEqual(legs[2].lifecycle, model.FORMING)

    def test_no_active_leg_is_untouched(self) -> None:
        legs = [
            confirmed_leg(leg_id=1, direction=1, lifecycle=model.FORMING),
            confirmed_leg(leg_id=2, direction=-1),
        ]
        model.transition_prior(legs, 1)
        self.assertEqual(legs[0].lifecycle, model.FORMING)


class PublicationTests(unittest.TestCase):
    def test_empty(self) -> None:
        self.assertIsNone(model.published_leg([], []))

    def test_before_the_first_confirmation_the_newest_forming_leg(self) -> None:
        minor = [make_leg(leg_id=1, scale=model.MINOR)]
        major = [make_leg(leg_id=2, scale=model.MAJOR)]
        self.assertEqual(model.published_leg(minor, major).leg_id, 2)

    def test_a_confirmed_leg_beats_a_newer_forming_leg(self) -> None:
        minor = [confirmed_leg(leg_id=1, scale=model.MINOR)]
        major = [make_leg(leg_id=2, scale=model.MAJOR)]
        self.assertEqual(model.published_leg(minor, major).leg_id, 1)

    def test_the_newest_confirmed_leg_wins_across_scales(self) -> None:
        minor = [confirmed_leg(leg_id=1, scale=model.MINOR)]
        major = [confirmed_leg(leg_id=2, scale=model.MAJOR)]
        self.assertEqual(model.published_leg(minor, major).leg_id, 2)
        minor.append(confirmed_leg(leg_id=3, scale=model.MINOR))
        self.assertEqual(model.published_leg(minor, major).leg_id, 3)


class ReadinessTests(unittest.TestCase):
    def test_missing_publication_or_progress(self) -> None:
        leg = confirmed_leg()
        self.assertEqual(model.readiness(None, None, None, None), 0)
        self.assertEqual(model.readiness(leg, None, None, None), 0)

    def test_missing_stage_or_unstable_target(self) -> None:
        leg = confirmed_leg()
        self.assertEqual(model.readiness(leg, 0.5, None, None), 1)
        leg.target_stable = False
        self.assertEqual(model.readiness(leg, 0.5, 2, None), 1)

    def test_percentile_completes_readiness(self) -> None:
        leg = confirmed_leg()
        self.assertEqual(model.readiness(leg, 0.5, 2, None), 1)
        self.assertEqual(model.readiness(leg, 0.5, 2, 50.0), 2)


class ReplayTests(unittest.TestCase):
    def test_confirmation_publication_and_completion(self) -> None:
        length = 10
        closes = [100.0, 100.2, 100.4, 100.5, 100.5, 100.5, 100.5, 100.5, 100.5, 100.5]
        candles = [candle(index, close) for index, close in enumerate(closes)]
        candidates = {
            model.MINOR: empty_series(length),
            model.MAJOR: empty_series(length),
        }
        confirmed = {
            model.MINOR: empty_series(length),
            model.MAJOR: empty_series(length),
        }
        candidates[model.MAJOR][0] = model.Pivot(
            model.MAJOR, "high", 101.0, 0, 0
        )
        confirmed[model.MAJOR][5] = model.Pivot(
            model.MAJOR, "high", 101.0, 0, 5
        )
        candidates[model.MAJOR][1] = model.Pivot(
            model.MAJOR, "low", 99.5, 1, 1
        )
        confirmed[model.MAJOR][6] = model.Pivot(
            model.MAJOR, "low", 99.5, 1, 6
        )

        result = model.replay(candles, candidates, confirmed, [1.0] * length)

        self.assertEqual(result.bars[0].published_leg.leg_id, 1)
        self.assertEqual(result.bars[0].published_leg.lifecycle_code, 0)
        self.assertAlmostEqual(result.bars[0].progress, 0.5)
        self.assertIsNone(result.bars[0].stage_code)
        self.assertIsNone(result.bars[0].unavailable_reason_code)
        self.assertEqual(result.bars[0].readiness_code, 1)
        self.assertEqual(result.bars[1].published_leg.leg_id, 2)
        self.assertEqual(result.bars[5].published_leg.leg_id, 1)
        self.assertEqual(result.bars[5].published_leg.lifecycle_code, 1)
        self.assertAlmostEqual(result.bars[5].progress, 0.25)
        self.assertEqual(result.bars[5].stage_code, 2)

        published = result.bars[6].published_leg
        self.assertEqual(published.leg_id, 2)
        self.assertEqual(published.target_price, 101.0)
        self.assertEqual(published.target_basis_code, 1)
        self.assertEqual(published.target_version, 1)
        self.assertEqual(published.target_stable_code, 0)
        self.assertIsNone(result.bars[6].stage_code)
        self.assertIsNone(result.bars[7].stage_code)
        self.assertEqual(result.bars[8].published_leg.target_stable_code, 1)
        self.assertEqual(result.bars[8].stage_code, 3)

        ledger = {leg.leg_id: leg for leg in result.legs}
        self.assertEqual(ledger[1].lifecycle_code, model.COMPLETED)
        self.assertEqual(ledger[2].lifecycle_code, model.ACTIVE)

    def test_target_revision_changes_progress_without_price_movement(
        self,
    ) -> None:
        length = 9
        closes = [100.0, 100.2, 100.4, 100.5, 100.5, 100.5, 100.5, 100.5, 100.5]
        candles = [candle(index, close) for index, close in enumerate(closes)]
        candidates = {
            model.MINOR: empty_series(length),
            model.MAJOR: empty_series(length),
        }
        confirmed = {
            model.MINOR: empty_series(length),
            model.MAJOR: empty_series(length),
        }
        candidates[model.MAJOR][0] = model.Pivot(
            model.MAJOR, "high", 101.0, 0, 0
        )
        confirmed[model.MAJOR][5] = model.Pivot(
            model.MAJOR, "high", 101.0, 0, 5
        )
        candidates[model.MAJOR][1] = model.Pivot(
            model.MAJOR, "low", 99.5, 1, 1
        )

        result = model.replay(candles, candidates, confirmed, [1.0] * length)

        before = result.bars[4].major_leg
        after = result.bars[5].major_leg
        self.assertEqual(before.target_price, 101.5)
        self.assertEqual(before.target_basis_code, 2)
        self.assertAlmostEqual(before.progress, 0.5)
        self.assertEqual(after.target_price, 101.0)
        self.assertEqual(after.target_basis_code, 1)
        self.assertEqual(after.target_version, 1)
        self.assertAlmostEqual(after.progress, 2.0 / 3.0)
        self.assertEqual(closes[4], closes[5])

    def test_invalidated_forming_candidate_is_retained(self) -> None:
        length = 3
        closes = [100.0, 99.0, 99.0]
        candles = [candle(index, close) for index, close in enumerate(closes)]
        candidates = {
            model.MINOR: empty_series(length),
            model.MAJOR: empty_series(length),
        }
        confirmed = {
            model.MINOR: empty_series(length),
            model.MAJOR: empty_series(length),
        }
        candidates[model.MINOR][0] = model.Pivot(
            model.MINOR, "low", 99.5, 0, 0
        )

        result = model.replay(candles, candidates, confirmed, [1.0] * length)

        self.assertEqual(len(result.legs), 1)
        self.assertEqual(result.legs[0].lifecycle_code, model.INVALIDATED)
        self.assertEqual(result.legs[0].origin_confirmed_code, 0)
        self.assertEqual(result.bars[2].stage_code, 5)
        self.assertEqual(result.bars[2].published_leg.leg_id, 1)

    def test_failed_candidate_without_a_breach_is_discarded(self) -> None:
        length = 3
        closes = [100.0, 100.5, 100.5]
        candles = [candle(index, close) for index, close in enumerate(closes)]
        candidates = {
            model.MINOR: empty_series(length),
            model.MAJOR: empty_series(length),
        }
        confirmed = {
            model.MINOR: empty_series(length),
            model.MAJOR: empty_series(length),
        }
        candidates[model.MINOR][0] = model.Pivot(
            model.MINOR, "low", 99.5, 0, 0
        )

        result = model.replay(candles, candidates, confirmed, [1.0] * length)

        self.assertEqual(result.legs, ())
        self.assertIsNone(result.bars[2].published_leg)
        self.assertEqual(result.bars[2].unavailable_reason_code, 6)

    def test_same_direction_confirmation_supersedes(self) -> None:
        length = 6
        closes = [100.0, 100.5, 100.5, 100.5, 100.5, 100.5]
        candles = [candle(index, close) for index, close in enumerate(closes)]
        candidates = {
            model.MINOR: empty_series(length),
            model.MAJOR: empty_series(length),
        }
        confirmed = {
            model.MINOR: empty_series(length),
            model.MAJOR: empty_series(length),
        }
        candidates[model.MINOR][0] = model.Pivot(
            model.MINOR, "low", 99.5, 0, 0
        )
        confirmed[model.MINOR][2] = model.Pivot(
            model.MINOR, "low", 99.5, 0, 2
        )
        candidates[model.MINOR][3] = model.Pivot(
            model.MINOR, "low", 100.5, 3, 3
        )
        confirmed[model.MINOR][5] = model.Pivot(
            model.MINOR, "low", 100.5, 3, 5
        )

        result = model.replay(candles, candidates, confirmed, [1.0] * length)

        ledger = {leg.leg_id: leg for leg in result.legs}
        self.assertEqual(ledger[1].lifecycle_code, model.SUPERSEDED)
        self.assertEqual(ledger[2].lifecycle_code, model.ACTIVE)
        self.assertEqual(result.bars[5].published_leg.leg_id, 2)

    def test_active_leg_invalidation_freezes_its_measurements(self) -> None:
        length = 5
        closes = [100.0, 100.5, 100.5, 99.0, 101.0]
        candles = [candle(index, close) for index, close in enumerate(closes)]
        candidates = {
            model.MINOR: empty_series(length),
            model.MAJOR: empty_series(length),
        }
        confirmed = {
            model.MINOR: empty_series(length),
            model.MAJOR: empty_series(length),
        }
        candidates[model.MINOR][0] = model.Pivot(
            model.MINOR, "low", 99.5, 0, 0
        )
        confirmed[model.MINOR][2] = model.Pivot(
            model.MINOR, "low", 99.5, 0, 2
        )

        result = model.replay(candles, candidates, confirmed, [1.0] * length)

        self.assertEqual(result.legs[0].lifecycle_code, model.INVALIDATED)
        self.assertEqual(result.bars[3].stage_code, 5)
        self.assertAlmostEqual(result.bars[3].progress, -0.25)
        self.assertEqual(
            result.bars[4].published_leg.progress,
            result.bars[3].published_leg.progress,
        )
        self.assertEqual(
            result.bars[4].published_leg.lifecycle_code, model.INVALIDATED
        )

    def test_percentile_becomes_available_after_fifty_samples(self) -> None:
        length = 60
        closes = [100.0] + [100.5 + 0.01 * index for index in range(length - 1)]
        candles = [candle(index, close) for index, close in enumerate(closes)]
        candidates = {
            model.MINOR: empty_series(length),
            model.MAJOR: empty_series(length),
        }
        confirmed = {
            model.MINOR: empty_series(length),
            model.MAJOR: empty_series(length),
        }
        candidates[model.MINOR][0] = model.Pivot(
            model.MINOR, "low", 100.0, 0, 0
        )
        confirmed[model.MINOR][2] = model.Pivot(
            model.MINOR, "low", 100.0, 0, 2
        )

        result = model.replay(candles, candidates, confirmed, [1.0] * length)

        self.assertIsNone(result.bars[49].size_percentile)
        self.assertIsNotNone(result.bars[50].size_percentile)
        self.assertEqual(result.bars[50].readiness_code, 2)

    def test_stage_counters_follow_the_published_stage(self) -> None:
        length = 7
        closes = [100.0, 100.5, 100.5, 101.0, 101.0, 99.9, 102.5]
        candles = [candle(index, close) for index, close in enumerate(closes)]
        candidates = {
            model.MINOR: empty_series(length),
            model.MAJOR: empty_series(length),
        }
        confirmed = {
            model.MINOR: empty_series(length),
            model.MAJOR: empty_series(length),
        }
        candidates[model.MINOR][0] = model.Pivot(
            model.MINOR, "low", 100.0, 0, 0
        )
        confirmed[model.MINOR][2] = model.Pivot(
            model.MINOR, "low", 100.0, 0, 2
        )

        result = model.replay(candles, candidates, confirmed, [1.0] * length)

        stages = [bar.stage_code for bar in result.bars]
        self.assertEqual(stages, [None, None, 2, 3, 3, 0, 4])
        counters = [
            (bar.state_age_bars, bar.state_changes_50) for bar in result.bars
        ]
        self.assertEqual(
            counters,
            [(0, 0), (0, 0), (1, 0), (1, 1), (2, 1), (1, 2), (1, 3)],
        )
        self.assertEqual(result.bars[6].published_leg.target_reached_code, 1)

    def test_replay_is_deterministic_across_dict_order(self) -> None:
        length = 3
        closes = [100.0, 99.0, 99.0]
        candles = [candle(index, close) for index, close in enumerate(closes)]
        candidates = {
            model.MINOR: empty_series(length),
            model.MAJOR: empty_series(length),
        }
        confirmed = {
            model.MINOR: empty_series(length),
            model.MAJOR: empty_series(length),
        }
        candidates[model.MINOR][0] = model.Pivot(
            model.MINOR, "low", 99.5, 0, 0
        )
        atr = [1.0] * length

        first = model.replay(candles, candidates, confirmed, atr)
        second = model.replay(
            candles,
            {
                model.MAJOR: candidates[model.MAJOR],
                model.MINOR: candidates[model.MINOR],
            },
            {
                model.MAJOR: confirmed[model.MAJOR],
                model.MINOR: confirmed[model.MINOR],
            },
            atr,
        )
        self.assertEqual(first, second)


class FormattingTests(unittest.TestCase):
    def test_published_float_drops_an_integral_suffix(self) -> None:
        self.assertEqual(cli.format_published_float(1.0), "1")
        self.assertEqual(cli.format_published_float(100.0), "100")
        self.assertEqual(cli.format_published_float(0.5), "0.5")
        self.assertEqual(cli.format_published_float(None), "")

    def test_leg_float_keeps_an_integral_suffix(self) -> None:
        self.assertEqual(cli.format_float(98.0), "98.0")
        self.assertEqual(cli.format_float(None), "")

    def test_report_optional_values_use_a_dash(self) -> None:
        self.assertEqual(cli.format_optional_int(None), "-")
        self.assertEqual(cli.format_optional_int(4), "4")
        self.assertEqual(cli.format_float9(None), "-")
        self.assertEqual(cli.format_float9(1.0), "1.000000000")

    def test_stage_transitions_ignore_the_first_bar(self) -> None:
        bars = tuple(
            bar_with_stage(stage)
            for stage in (None, 1, 1, 2, None, 2)
        )
        self.assertEqual(
            cli.progression_transitions(bars, 5),
            [(1, None, 1), (3, 1, 2), (4, 2, None), (5, None, 2)],
        )

    def test_stage_transitions_keep_the_last_fifty(self) -> None:
        stages = [0]
        for _ in range(60):
            stages.append(1 if stages[-1] == 0 else 0)
        bars = tuple(bar_with_stage(stage) for stage in stages)
        transitions = cli.progression_transitions(bars, len(stages) - 1)
        self.assertEqual(len(transitions), 50)
        self.assertEqual(transitions[-1][0], len(stages) - 1)


if __name__ == "__main__":
    unittest.main()
