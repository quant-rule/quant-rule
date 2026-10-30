//! Unit tests for the portfolio type layer.

use super::*;
use crate::side::Side;

fn commission() -> CommissionConfig {
    CommissionConfig::new(0.001, CommissionBase::Equity, CommissionPer::Operation)
        .expect("the test commission is valid")
}

fn funding() -> FundingConfig {
    FundingConfig::new(0.0001, FundingInterval::H8).expect("the test funding is valid")
}

fn drawdown() -> MaxDrawdownConfig {
    MaxDrawdownConfig::new(Some(0.2), DrawdownAction::Halt).expect("the test drawdown is valid")
}

fn config() -> PortfolioConfig {
    PortfolioConfig {
        initial_cash: 10_000.0,
        currency: Currency::Usd,
        sizing: SizingMode::FixedNotional {
            notional: 10_000.0,
            quantity_step: 0.0001,
        },
        leverage: 1.0,
        max_open_positions: 1,
        allow_short: true,
        commission: commission(),
        funding: funding(),
        max_drawdown: drawdown(),
        mark_to_market: MarkToMarket::BarClose,
    }
}

#[test]
fn valid_config_validates_and_exposes_its_values() {
    let config = config();
    assert_eq!(config.validate(), Ok(()));
    assert_eq!(config.initial_cash, 10_000.0);
    assert_eq!(config.currency, Currency::Usd);
    assert_eq!(config.leverage, 1.0);
    assert_eq!(config.max_open_positions, 1);
    assert!(config.allow_short);
    assert_eq!(config.commission.rate(), 0.001);
    assert_eq!(config.commission.base(), CommissionBase::Equity);
    assert_eq!(config.commission.per(), CommissionPer::Operation);
    assert_eq!(config.funding.rate(), 0.0001);
    assert_eq!(config.funding.interval(), FundingInterval::H8);
    assert_eq!(config.max_drawdown.limit(), Some(0.2));
    assert_eq!(config.max_drawdown.action(), DrawdownAction::Halt);
    assert_eq!(config.mark_to_market, MarkToMarket::BarClose);
}

#[test]
fn zero_initial_cash_is_rejected() {
    let mut config = config();
    config.initial_cash = 0.0;
    assert_eq!(
        config.validate(),
        Err(PortfolioConfigError::NonPositiveInitialCash)
    );
}

#[test]
fn negative_initial_cash_is_rejected() {
    let mut config = config();
    config.initial_cash = -1.0;
    assert_eq!(
        config.validate(),
        Err(PortfolioConfigError::NonPositiveInitialCash)
    );
}

#[test]
fn non_finite_initial_cash_is_rejected() {
    let mut config = config();
    config.initial_cash = f64::NAN;
    assert_eq!(
        config.validate(),
        Err(PortfolioConfigError::NonFiniteInitialCash)
    );
}

#[test]
fn leverage_above_one_is_rejected() {
    let mut config = config();
    config.leverage = 2.0;
    assert_eq!(
        config.validate(),
        Err(PortfolioConfigError::UnsupportedLeverage)
    );
}

#[test]
fn non_finite_leverage_is_rejected() {
    let mut config = config();
    config.leverage = f64::INFINITY;
    assert_eq!(
        config.validate(),
        Err(PortfolioConfigError::NonFiniteLeverage)
    );
}

#[test]
fn position_limit_above_one_is_rejected() {
    let mut config = config();
    config.max_open_positions = 2;
    assert_eq!(
        config.validate(),
        Err(PortfolioConfigError::UnsupportedPositionLimit)
    );
}

#[test]
fn zero_notional_is_rejected() {
    let mut config = config();
    config.sizing = SizingMode::FixedNotional {
        notional: 0.0,
        quantity_step: 0.0001,
    };
    assert_eq!(
        config.validate(),
        Err(PortfolioConfigError::NonPositiveNotional)
    );
}

#[test]
fn non_finite_notional_is_rejected() {
    let mut config = config();
    config.sizing = SizingMode::FixedNotional {
        notional: f64::NAN,
        quantity_step: 0.0001,
    };
    assert_eq!(
        config.validate(),
        Err(PortfolioConfigError::NonFiniteNotional)
    );
}

#[test]
fn zero_quantity_step_is_rejected() {
    let mut config = config();
    config.sizing = SizingMode::FixedNotional {
        notional: 10_000.0,
        quantity_step: 0.0,
    };
    assert_eq!(
        config.validate(),
        Err(PortfolioConfigError::NonPositiveQuantityStep)
    );
}

#[test]
fn fraction_above_one_is_rejected() {
    let mut config = config();
    config.sizing = SizingMode::EquityFraction {
        fraction: 1.1,
        quantity_step: 0.0001,
    };
    assert_eq!(
        config.validate(),
        Err(PortfolioConfigError::FractionOutOfRange)
    );
}

#[test]
fn zero_fraction_is_rejected() {
    let mut config = config();
    config.sizing = SizingMode::EquityFraction {
        fraction: 0.0,
        quantity_step: 0.0001,
    };
    assert_eq!(
        config.validate(),
        Err(PortfolioConfigError::FractionOutOfRange)
    );
}

#[test]
fn negative_commission_is_rejected() {
    assert_eq!(
        CommissionConfig::new(-0.001, CommissionBase::Equity, CommissionPer::Operation),
        Err(PortfolioConfigError::NegativeCommissionRate)
    );
}

#[test]
fn non_finite_commission_is_rejected() {
    assert_eq!(
        CommissionConfig::new(f64::NAN, CommissionBase::Equity, CommissionPer::Operation),
        Err(PortfolioConfigError::NonFiniteCommissionRate)
    );
}

#[test]
fn commission_base_and_period_accept_only_the_contract_values() {
    assert_eq!(CommissionBase::parse("equity"), Ok(CommissionBase::Equity));
    assert_eq!(
        CommissionBase::parse("notional"),
        Err(PortfolioConfigError::UnsupportedCommissionBase)
    );
    assert_eq!(
        CommissionPer::parse("operation"),
        Ok(CommissionPer::Operation)
    );
    assert_eq!(
        CommissionPer::parse("bar"),
        Err(PortfolioConfigError::UnsupportedCommissionPer)
    );
    assert_eq!(CommissionBase::Equity.code(), "equity");
    assert_eq!(CommissionPer::Operation.code(), "operation");
}

#[test]
fn funding_interval_accepts_only_the_contract_values() {
    for (hours, expected) in [
        (1, FundingInterval::H1),
        (2, FundingInterval::H2),
        (4, FundingInterval::H4),
        (8, FundingInterval::H8),
        (12, FundingInterval::H12),
        (24, FundingInterval::H24),
    ] {
        let interval = FundingInterval::new(hours).expect("a contract interval is valid");
        assert_eq!(interval, expected);
        assert_eq!(interval.hours(), hours);
        assert_eq!(interval.seconds(), i64::from(hours) * 3_600);
    }
}

#[test]
fn invalid_funding_interval_is_rejected() {
    for hours in [0, 3, 6, 25, 48] {
        assert_eq!(
            FundingInterval::new(hours),
            Err(PortfolioConfigError::UnsupportedFundingInterval)
        );
    }
}

#[test]
fn negative_funding_rate_is_accepted() {
    let funding = FundingConfig::new(-0.0001, FundingInterval::H8)
        .expect("a negative rate reverses the payer");
    assert_eq!(funding.rate(), -0.0001);
}

#[test]
fn non_finite_funding_rate_is_rejected() {
    assert_eq!(
        FundingConfig::new(f64::INFINITY, FundingInterval::H8),
        Err(PortfolioConfigError::NonFiniteFundingRate)
    );
}

#[test]
fn drawdown_limit_must_be_in_range() {
    for limit in [0.0, -0.1, 1.1, f64::NAN, f64::INFINITY] {
        assert!(
            MaxDrawdownConfig::new(Some(limit), DrawdownAction::Halt).is_err(),
            "limit {limit} must be rejected"
        );
    }
    assert_eq!(
        MaxDrawdownConfig::new(Some(0.0), DrawdownAction::Halt),
        Err(PortfolioConfigError::DrawdownLimitOutOfRange)
    );
    assert_eq!(
        MaxDrawdownConfig::new(Some(1.1), DrawdownAction::Halt),
        Err(PortfolioConfigError::DrawdownLimitOutOfRange)
    );
    assert_eq!(
        MaxDrawdownConfig::new(Some(f64::NAN), DrawdownAction::Halt),
        Err(PortfolioConfigError::NonFiniteDrawdownLimit)
    );
}

#[test]
fn drawdown_limit_accepts_one() {
    let config = MaxDrawdownConfig::new(Some(1.0), DrawdownAction::Halt)
        .expect("a limit of one is in range");
    assert_eq!(config.limit(), Some(1.0));
}

#[test]
fn halt_without_a_limit_is_rejected() {
    assert_eq!(
        MaxDrawdownConfig::new(None, DrawdownAction::Halt),
        Err(PortfolioConfigError::DrawdownWithoutLimit)
    );
}

#[test]
fn no_limit_with_no_action_is_allowed() {
    let config =
        MaxDrawdownConfig::new(None, DrawdownAction::None).expect("no limit and no action agree");
    assert_eq!(config.limit(), None);
    assert_eq!(config.action(), DrawdownAction::None);
}

#[test]
fn drawdown_action_parses_only_the_contract_values() {
    assert_eq!(DrawdownAction::parse("halt"), Ok(DrawdownAction::Halt));
    assert_eq!(DrawdownAction::parse("none"), Ok(DrawdownAction::None));
    assert_eq!(
        DrawdownAction::parse("liquidate"),
        Err(PortfolioConfigError::UnsupportedDrawdownAction)
    );
    assert_eq!(DrawdownAction::Halt.code(), "halt");
    assert_eq!(DrawdownAction::None.code(), "none");
}

#[test]
fn currency_accepts_only_usd() {
    assert_eq!(Currency::parse("USD"), Ok(Currency::Usd));
    assert_eq!(
        Currency::parse("EUR"),
        Err(PortfolioConfigError::UnsupportedCurrency)
    );
    assert_eq!(Currency::Usd.code(), "USD");
}

#[test]
fn mark_to_market_accepts_only_bar_close() {
    assert_eq!(MarkToMarket::parse("bar_close"), Ok(MarkToMarket::BarClose));
    assert_eq!(
        MarkToMarket::parse("bar_open"),
        Err(PortfolioConfigError::UnsupportedMarkToMarket)
    );
    assert_eq!(MarkToMarket::BarClose.code(), "bar_close");
}

#[test]
fn fixed_notional_sizes_a_long() {
    let config = config();
    let quantity = config
        .size_entry(Side::Long, 10_000.0, 200.0)
        .expect("the entry is sizeable");
    assert_eq!(quantity, 50.0);
}

#[test]
fn fixed_notional_sizes_a_short() {
    let config = config();
    let quantity = config
        .size_entry(Side::Short, 10_000.0, 200.0)
        .expect("the short is sizeable");
    assert_eq!(quantity, 50.0);
}

#[test]
fn fixed_notional_ignores_the_equity() {
    let config = config();
    let low = config
        .size_entry(Side::Long, 1_000.0, 200.0)
        .expect("the entry is sizeable");
    let high = config
        .size_entry(Side::Long, 100_000.0, 200.0)
        .expect("the entry is sizeable");
    assert_eq!(low, high);
}

#[test]
fn equity_fraction_scales_with_equity() {
    let mut config = config();
    config.sizing = SizingMode::EquityFraction {
        fraction: 0.5,
        quantity_step: 1.0,
    };
    let quantity = config
        .size_entry(Side::Long, 10_000.0, 200.0)
        .expect("the entry is sizeable");
    assert_eq!(quantity, 25.0);
}

#[test]
fn equity_fraction_with_zero_equity_is_below_one_step() {
    let mut config = config();
    config.sizing = SizingMode::EquityFraction {
        fraction: 0.5,
        quantity_step: 1.0,
    };
    assert_eq!(
        config.size_entry(Side::Long, 0.0, 200.0),
        Err(SizingRejection::BelowOneStep {
            raw: 0.0,
            step: 1.0
        })
    );
}

#[test]
fn quantity_floors_to_the_step() {
    let mut config = config();
    config.sizing = SizingMode::FixedNotional {
        notional: 1_000.0,
        quantity_step: 3.0,
    };
    let quantity = config
        .size_entry(Side::Long, 1_000.0, 100.0)
        .expect("the entry is sizeable");
    assert_eq!(quantity, 9.0);
}

#[test]
fn exact_step_boundaries_are_not_lost_to_binary_rounding() {
    let mut config = config();
    config.sizing = SizingMode::FixedNotional {
        notional: 3.0,
        quantity_step: 0.1,
    };
    let quantity = config
        .size_entry(Side::Long, 1_000.0, 5.0)
        .expect("the exact boundary is sizeable");
    assert_eq!(quantity, 0.6);

    config.sizing = SizingMode::FixedNotional {
        notional: 0.3,
        quantity_step: 0.1,
    };
    let quantity = config
        .size_entry(Side::Long, 1_000.0, 3.0)
        .expect("the exact boundary is sizeable");
    assert_eq!(quantity, 0.1);
}

#[test]
fn exact_step_boundaries_at_small_and_non_grid_steps() {
    for (notional, step, expected) in [
        (5e-13, 1e-13, 5e-13),
        (4e-13, 1e-13, 4e-13),
        (5e-13, 2.5e-13, 5e-13),
        (1.5e-12, 1.5e-12, 1.5e-12),
        (1.51e-12, 1.5e-12, 1.5e-12),
        (5e-15, 1e-15, 5e-15),
        (5e-18, 1e-18, 5e-18),
        (5e-19, 1e-19, 5e-19),
        (0.001, 1.0 / 3000.0, 3.0 * (1.0 / 3000.0)),
    ] {
        let mut config = config();
        config.sizing = SizingMode::FixedNotional {
            notional,
            quantity_step: step,
        };
        let quantity = config
            .size_entry(Side::Long, 1_000.0, 1.0)
            .expect("the exact boundary is sizeable");
        assert_eq!(quantity, expected, "notional={notional}, step={step}");
    }

    let mut config = config();
    config.sizing = SizingMode::FixedNotional {
        notional: 1.49e-12,
        quantity_step: 1.5e-12,
    };
    assert!(
        matches!(
            config.size_entry(Side::Long, 1_000.0, 1.0),
            Err(SizingRejection::BelowOneStep { .. })
        ),
        "a quantity below one step must reject"
    );
}

#[test]
fn quantity_below_one_step_is_rejected() {
    let mut config = config();
    config.sizing = SizingMode::FixedNotional {
        notional: 10.0,
        quantity_step: 1.0,
    };
    assert_eq!(
        config.size_entry(Side::Long, 10.0, 100.0),
        Err(SizingRejection::BelowOneStep {
            raw: 0.1,
            step: 1.0
        })
    );
}

#[test]
fn zero_price_is_rejected() {
    let config = config();
    assert_eq!(
        config.size_entry(Side::Long, 10_000.0, 0.0),
        Err(SizingRejection::NonPositivePrice)
    );
}

#[test]
fn negative_price_is_rejected() {
    let config = config();
    assert_eq!(
        config.size_entry(Side::Long, 10_000.0, -1.0),
        Err(SizingRejection::NonPositivePrice)
    );
}

#[test]
fn non_finite_price_is_rejected() {
    let config = config();
    assert_eq!(
        config.size_entry(Side::Long, 10_000.0, f64::NAN),
        Err(SizingRejection::NonFinitePrice)
    );
}

#[test]
fn negative_equity_is_rejected() {
    let config = config();
    assert_eq!(
        config.size_entry(Side::Long, -1.0, 100.0),
        Err(SizingRejection::NegativeEquity)
    );
}

#[test]
fn non_finite_equity_is_rejected() {
    let config = config();
    assert_eq!(
        config.size_entry(Side::Long, f64::NAN, 100.0),
        Err(SizingRejection::NonFiniteEquity)
    );
}

#[test]
fn short_is_rejected_when_not_allowed() {
    let mut config = config();
    config.allow_short = false;
    assert_eq!(
        config.size_entry(Side::Short, 10_000.0, 200.0),
        Err(SizingRejection::ShortNotAllowed)
    );
    assert!(config.size_entry(Side::Long, 10_000.0, 200.0).is_ok());
}

#[test]
fn sizing_is_deterministic() {
    let config = config();
    let first = config
        .size_entry(Side::Long, 10_000.0, 123.45)
        .expect("the entry is sizeable");
    for _ in 0..100 {
        assert_eq!(
            config
                .size_entry(Side::Long, 10_000.0, 123.45)
                .expect("the entry is sizeable"),
            first
        );
    }
}

#[test]
fn state_starts_flat_at_the_initial_cash() {
    let state = PortfolioState::new(10_000.0).expect("the state is valid");
    assert!(state.is_flat());
    assert!(!state.is_long());
    assert!(!state.is_short());
    assert_eq!(state.cash, 10_000.0);
    assert_eq!(state.equity, 10_000.0);
    assert_eq!(state.peak_equity, 10_000.0);
    assert_eq!(state.drawdown, 0.0);
    assert!(!state.halted);
}

#[test]
fn state_rejects_zero_and_negative_cash() {
    assert_eq!(
        PortfolioState::new(0.0),
        Err(PortfolioStateError::NonPositiveInitialCash)
    );
    assert_eq!(
        PortfolioState::new(-1.0),
        Err(PortfolioStateError::NonPositiveInitialCash)
    );
    assert_eq!(
        PortfolioState::new(f64::NAN),
        Err(PortfolioStateError::NonFiniteInitialCash)
    );
}

#[test]
fn state_reports_the_open_position_side() {
    let position = OpenPosition::new(
        Side::Short,
        2.0,
        100.0,
        10,
        1_700_000_000,
        10_000.0,
        1.0,
        10.0,
    )
    .expect("the position is valid");
    let state = PortfolioState {
        cash: 9_000.0,
        position: Some(position),
        equity: 9_800.0,
        peak_equity: 10_000.0,
        drawdown: 0.02,
        halted: false,
    };
    assert!(!state.is_flat());
    assert!(state.is_short());
    assert!(!state.is_long());
}

#[test]
fn open_position_rejects_invalid_values() {
    assert_eq!(
        OpenPosition::new(Side::Long, 0.0, 100.0, 0, 0, 10_000.0, 0.0, 0.0),
        Err(OpenPositionError::NonPositiveQuantity)
    );
    assert_eq!(
        OpenPosition::new(Side::Long, f64::NAN, 100.0, 0, 0, 10_000.0, 0.0, 0.0),
        Err(OpenPositionError::NonFiniteQuantity)
    );
    assert_eq!(
        OpenPosition::new(Side::Long, 1.0, 0.0, 0, 0, 10_000.0, 0.0, 0.0),
        Err(OpenPositionError::NonPositiveEntryPrice)
    );
    assert_eq!(
        OpenPosition::new(Side::Long, 1.0, 100.0, 0, 0, -1.0, 0.0, 0.0),
        Err(OpenPositionError::NegativeEntryEquity)
    );
    assert_eq!(
        OpenPosition::new(Side::Long, 1.0, 100.0, 0, 0, 10_000.0, -1.0, 0.0),
        Err(OpenPositionError::NegativeCost)
    );
    assert_eq!(
        OpenPosition::new(Side::Long, 1.0, 100.0, 0, 0, 10_000.0, 0.0, f64::NAN),
        Err(OpenPositionError::NonFiniteCost)
    );
}

#[test]
fn open_position_starts_with_no_funding() {
    let position = OpenPosition::new(
        Side::Long,
        2.0,
        100.0,
        10,
        1_700_000_000,
        10_000.0,
        1.0,
        10.0,
    )
    .expect("the position is valid");
    assert_eq!(position.funding_paid, 0.0);
    assert_eq!(position.funding_received, 0.0);
    assert!(position.is_long());
    assert!(!position.is_short());
}

#[test]
fn config_errors_display_a_deterministic_message() {
    assert_eq!(
        PortfolioConfigError::UnsupportedLeverage.to_string(),
        "the leverage must be exactly 1.0"
    );
    assert_eq!(
        PortfolioConfigError::UnsupportedFundingInterval.to_string(),
        "the funding interval must be one of 1, 2, 4, 8, 12, 24 hours"
    );
    assert_eq!(
        PortfolioConfigError::DrawdownWithoutLimit.to_string(),
        "a halt action requires a drawdown limit"
    );
}

#[test]
fn sizing_rejections_display_a_deterministic_message() {
    assert_eq!(
        SizingRejection::ShortNotAllowed.to_string(),
        "a short entry is not allowed"
    );
    assert_eq!(
        SizingRejection::BelowOneStep {
            raw: 0.5,
            step: 1.0
        }
        .to_string(),
        "the quantity 0.5 is below one step of 1"
    );
}

#[test]
fn exit_reasons_have_the_contract_codes() {
    assert_eq!(ExitReason::AgreementLost.code(), "agreement_lost");
    assert_eq!(ExitReason::Stop.code(), "stop");
    assert_eq!(ExitReason::Target.code(), "target");
}
