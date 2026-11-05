//! Stable numeric codes for every registered event.
//!
//! The registry assigns one code to each event identifier. The code is the
//! runtime identity of the event: two event identifiers may share a type,
//! subtype, and source, but they never share a code. The model publishes the
//! code in every [`EventRecord`](crate::events::EventRecord), and the spec
//! filters and votes by it. A code is never reused after retirement.

/// A legacy event that predates the typed registry. The model still publishes
/// it for backward compatibility, but no spec can select it and no new event
/// may use this code.
pub const LEGACY_UNREGISTERED: u16 = 0;

/// `level.test`.
pub const LEVEL_TEST: u16 = 1;
/// `level.cross`.
pub const LEVEL_CROSS: u16 = 2;
/// `level.break`.
pub const LEVEL_BREAK: u16 = 3;
/// `divergence.rsi_regular_bullish`.
pub const DIVERGENCE_RSI_REGULAR_BULLISH: u16 = 4;
/// `divergence.rsi_regular_bearish`.
pub const DIVERGENCE_RSI_REGULAR_BEARISH: u16 = 5;
/// `divergence.rsi_hidden_bullish`.
pub const DIVERGENCE_RSI_HIDDEN_BULLISH: u16 = 6;
/// `divergence.rsi_hidden_bearish`.
pub const DIVERGENCE_RSI_HIDDEN_BEARISH: u16 = 7;
/// `signal.rsi_threshold`.
pub const SIGNAL_RSI_THRESHOLD: u16 = 8;
/// `signal.ema_pair_cross`.
pub const SIGNAL_EMA_PAIR_CROSS: u16 = 9;
/// `signal.ema_stack_change`.
pub const SIGNAL_EMA_STACK_CHANGE: u16 = 10;
/// `signal.rsi_zone_change`.
pub const SIGNAL_RSI_ZONE_CHANGE: u16 = 11;
/// `signal.macd_line_signal_cross`.
pub const SIGNAL_MACD_LINE_SIGNAL_CROSS: u16 = 12;
/// `signal.macd_histogram_slope_flip`.
pub const SIGNAL_MACD_HISTOGRAM_SLOPE_FLIP: u16 = 13;
/// `signal.macd_zero_cross`.
pub const SIGNAL_MACD_ZERO_CROSS: u16 = 14;
/// `signal.di_cross`.
pub const SIGNAL_DI_CROSS: u16 = 15;
/// `signal.trend_continuation`.
pub const SIGNAL_TREND_CONTINUATION: u16 = 16;
/// `signal.compression_breakout`.
pub const SIGNAL_COMPRESSION_BREAKOUT: u16 = 17;
/// `signal.compression_range`.
pub const SIGNAL_COMPRESSION_RANGE: u16 = 18;
/// `signal.compression_range_onset`.
pub const SIGNAL_COMPRESSION_RANGE_ONSET: u16 = 19;
/// `level.vwap_test`.
pub const LEVEL_VWAP_TEST: u16 = 20;
/// `level.vwap_cross`.
pub const LEVEL_VWAP_CROSS: u16 = 21;
/// `level.vwap_break`.
pub const LEVEL_VWAP_BREAK: u16 = 22;
/// `divergence.vwap_regular_bullish`.
pub const DIVERGENCE_VWAP_REGULAR_BULLISH: u16 = 23;
/// `divergence.vwap_regular_bearish`.
pub const DIVERGENCE_VWAP_REGULAR_BEARISH: u16 = 24;
/// `divergence.vwap_hidden_bullish`.
pub const DIVERGENCE_VWAP_HIDDEN_BULLISH: u16 = 25;
/// `divergence.vwap_hidden_bearish`.
pub const DIVERGENCE_VWAP_HIDDEN_BEARISH: u16 = 26;
/// `signal.vwap_reclaim`.
pub const SIGNAL_VWAP_RECLAIM: u16 = 27;
/// `signal.vwap_rejection`.
pub const SIGNAL_VWAP_REJECTION: u16 = 28;
/// `signal.vwap_slope`.
pub const SIGNAL_VWAP_SLOPE: u16 = 29;
/// `level.profile_poc_test`.
pub const LEVEL_PROFILE_POC_TEST: u16 = 30;
/// `level.profile_vah_test`.
pub const LEVEL_PROFILE_VAH_TEST: u16 = 31;
/// `level.profile_val_test`.
pub const LEVEL_PROFILE_VAL_TEST: u16 = 32;
/// `level.profile_va_break`.
pub const LEVEL_PROFILE_VA_BREAK: u16 = 33;
/// `level.profile_va_reentry`.
pub const LEVEL_PROFILE_VA_REENTRY: u16 = 34;
/// `divergence.profile_poc_bullish`.
pub const DIVERGENCE_PROFILE_POC_BULLISH: u16 = 35;
/// `divergence.profile_poc_bearish`.
pub const DIVERGENCE_PROFILE_POC_BEARISH: u16 = 36;
/// `signal.profile_va_breakout`.
pub const SIGNAL_PROFILE_VA_BREAKOUT: u16 = 37;
/// `signal.profile_poc_cross`.
pub const SIGNAL_PROFILE_POC_CROSS: u16 = 38;
/// `signal.profile_skew`.
pub const SIGNAL_PROFILE_SKEW: u16 = 39;
/// `signal.profile_concentration`.
pub const SIGNAL_PROFILE_CONCENTRATION: u16 = 40;
/// `divergence.rvol_bullish`.
pub const DIVERGENCE_RVOL_BULLISH: u16 = 41;
/// `divergence.rvol_bearish`.
pub const DIVERGENCE_RVOL_BEARISH: u16 = 42;
/// `signal.rvol_expansion`.
pub const SIGNAL_RVOL_EXPANSION: u16 = 43;
/// `signal.rvol_contraction`.
pub const SIGNAL_RVOL_CONTRACTION: u16 = 44;
/// `signal.rvol_ratio`.
pub const SIGNAL_RVOL_RATIO: u16 = 45;
/// `divergence.climax_bullish`.
pub const DIVERGENCE_CLIMAX_BULLISH: u16 = 46;
/// `divergence.climax_bearish`.
pub const DIVERGENCE_CLIMAX_BEARISH: u16 = 47;
/// `signal.climax_reversal`.
pub const SIGNAL_CLIMAX_REVERSAL: u16 = 48;
/// `signal.climax_continuation`.
pub const SIGNAL_CLIMAX_CONTINUATION: u16 = 49;
/// `level.absorption_test`.
pub const LEVEL_ABSORPTION_TEST: u16 = 50;
/// `signal.absorption_break`.
pub const SIGNAL_ABSORPTION_BREAK: u16 = 51;
/// `signal.absorption_rejection`.
pub const SIGNAL_ABSORPTION_REJECTION: u16 = 52;
/// `signal.volume_oscillator_cross`.
pub const SIGNAL_VOLUME_OSCILLATOR_CROSS: u16 = 53;
/// `divergence.obv_regular_bullish`.
pub const DIVERGENCE_OBV_REGULAR_BULLISH: u16 = 54;
/// `divergence.obv_regular_bearish`.
pub const DIVERGENCE_OBV_REGULAR_BEARISH: u16 = 55;
/// `divergence.obv_hidden_bullish`.
pub const DIVERGENCE_OBV_HIDDEN_BULLISH: u16 = 56;
/// `divergence.obv_hidden_bearish`.
pub const DIVERGENCE_OBV_HIDDEN_BEARISH: u16 = 57;
/// `signal.obv_cross`.
pub const SIGNAL_OBV_CROSS: u16 = 58;
/// `signal.obv_slope`.
pub const SIGNAL_OBV_SLOPE: u16 = 59;
/// `divergence.cmf_regular_bullish`.
pub const DIVERGENCE_CMF_REGULAR_BULLISH: u16 = 60;
/// `divergence.cmf_regular_bearish`.
pub const DIVERGENCE_CMF_REGULAR_BEARISH: u16 = 61;
/// `divergence.cmf_hidden_bullish`.
pub const DIVERGENCE_CMF_HIDDEN_BULLISH: u16 = 62;
/// `divergence.cmf_hidden_bearish`.
pub const DIVERGENCE_CMF_HIDDEN_BEARISH: u16 = 63;
/// `signal.cmf_cross`.
pub const SIGNAL_CMF_CROSS: u16 = 64;
/// `signal.cmf_threshold`.
pub const SIGNAL_CMF_THRESHOLD: u16 = 65;
/// `divergence.mfi_regular_bullish`.
pub const DIVERGENCE_MFI_REGULAR_BULLISH: u16 = 66;
/// `divergence.mfi_regular_bearish`.
pub const DIVERGENCE_MFI_REGULAR_BEARISH: u16 = 67;
/// `divergence.mfi_hidden_bullish`.
pub const DIVERGENCE_MFI_HIDDEN_BULLISH: u16 = 68;
/// `divergence.mfi_hidden_bearish`.
pub const DIVERGENCE_MFI_HIDDEN_BEARISH: u16 = 69;
/// `signal.mfi_threshold`.
pub const SIGNAL_MFI_THRESHOLD: u16 = 70;
/// `signal.mfi_zone`.
pub const SIGNAL_MFI_ZONE: u16 = 71;
/// `divergence.ad_regular_bullish`.
pub const DIVERGENCE_AD_REGULAR_BULLISH: u16 = 72;
/// `divergence.ad_regular_bearish`.
pub const DIVERGENCE_AD_REGULAR_BEARISH: u16 = 73;
/// `signal.ad_cross`.
pub const SIGNAL_AD_CROSS: u16 = 74;
/// `level.keltner_test`.
pub const LEVEL_KELTNER_TEST: u16 = 75;
/// `level.keltner_break`.
pub const LEVEL_KELTNER_BREAK: u16 = 76;
/// `level.donchian_test`.
pub const LEVEL_DONCHIAN_TEST: u16 = 77;
/// `level.donchian_break`.
pub const LEVEL_DONCHIAN_BREAK: u16 = 78;
/// `level.structure_break`.
pub const LEVEL_STRUCTURE_BREAK: u16 = 79;
/// `level.strength_test`.
pub const LEVEL_STRENGTH_TEST: u16 = 80;
/// `level.wick_test`.
pub const LEVEL_WICK_TEST: u16 = 81;
/// `level.gap_fill`.
pub const LEVEL_GAP_FILL: u16 = 82;
/// `level.gap_test`.
pub const LEVEL_GAP_TEST: u16 = 83;
/// `divergence.atr_bullish`.
pub const DIVERGENCE_ATR_BULLISH: u16 = 84;
/// `divergence.atr_bearish`.
pub const DIVERGENCE_ATR_BEARISH: u16 = 85;
/// `divergence.rs_regular_bullish`.
pub const DIVERGENCE_RS_REGULAR_BULLISH: u16 = 86;
/// `divergence.rs_regular_bearish`.
pub const DIVERGENCE_RS_REGULAR_BEARISH: u16 = 87;
/// `divergence.lead_lag_bullish`.
pub const DIVERGENCE_LEAD_LAG_BULLISH: u16 = 88;
/// `divergence.lead_lag_bearish`.
pub const DIVERGENCE_LEAD_LAG_BEARISH: u16 = 89;
/// `divergence.breadth_regular_bullish`.
pub const DIVERGENCE_BREADTH_REGULAR_BULLISH: u16 = 90;
/// `divergence.breadth_regular_bearish`.
pub const DIVERGENCE_BREADTH_REGULAR_BEARISH: u16 = 91;
/// `signal.atr_compression`.
pub const SIGNAL_ATR_COMPRESSION: u16 = 92;
/// `signal.atr_expansion`.
pub const SIGNAL_ATR_EXPANSION: u16 = 93;
/// `signal.atr_regime`.
pub const SIGNAL_ATR_REGIME: u16 = 94;
/// `signal.range_expansion`.
pub const SIGNAL_RANGE_EXPANSION: u16 = 95;
/// `signal.range_contraction`.
pub const SIGNAL_RANGE_CONTRACTION: u16 = 96;
/// `signal.mar_regime`.
pub const SIGNAL_MAR_REGIME: u16 = 97;
/// `signal.keltner_breakout`.
pub const SIGNAL_KELTNER_BREAKOUT: u16 = 98;
/// `signal.keltner_position`.
pub const SIGNAL_KELTNER_POSITION: u16 = 99;
/// `signal.donchian_breakout`.
pub const SIGNAL_DONCHIAN_BREAKOUT: u16 = 100;
/// `signal.donchian_position`.
pub const SIGNAL_DONCHIAN_POSITION: u16 = 101;
/// `signal.structure_change`.
pub const SIGNAL_STRUCTURE_CHANGE: u16 = 102;
/// `signal.strength_change`.
pub const SIGNAL_STRENGTH_CHANGE: u16 = 103;
/// `signal.aroon_cross`.
pub const SIGNAL_AROON_CROSS: u16 = 104;
/// `signal.aroon_osc`.
pub const SIGNAL_AROON_OSC: u16 = 105;
/// `signal.candle_momentum`.
pub const SIGNAL_CANDLE_MOMENTUM: u16 = 106;
/// `signal.candle_rejection`.
pub const SIGNAL_CANDLE_REJECTION: u16 = 107;
/// `signal.candle_doji`.
pub const SIGNAL_CANDLE_DOJI: u16 = 108;
/// `signal.candle_marubozu`.
pub const SIGNAL_CANDLE_MARUBOZU: u16 = 109;
/// `signal.pattern_engulfing`.
pub const SIGNAL_PATTERN_ENGULFING: u16 = 110;
/// `signal.pattern_inside`.
pub const SIGNAL_PATTERN_INSIDE: u16 = 111;
/// `signal.pattern_outside`.
pub const SIGNAL_PATTERN_OUTSIDE: u16 = 112;
/// `signal.pattern_hammer`.
pub const SIGNAL_PATTERN_HAMMER: u16 = 113;
/// `signal.pattern_star`.
pub const SIGNAL_PATTERN_STAR: u16 = 114;
/// `signal.pattern_three_soldiers`.
pub const SIGNAL_PATTERN_THREE_SOLDIERS: u16 = 115;
/// `signal.pattern_three_crows`.
pub const SIGNAL_PATTERN_THREE_CROWS: u16 = 116;
/// `signal.gap_continuation`.
pub const SIGNAL_GAP_CONTINUATION: u16 = 117;
/// `signal.gap_fade`.
pub const SIGNAL_GAP_FADE: u16 = 118;
/// `signal.path_efficiency`.
pub const SIGNAL_PATH_EFFICIENCY: u16 = 119;
/// `signal.direction_changes`.
pub const SIGNAL_DIRECTION_CHANGES: u16 = 120;
/// `signal.extreme_new`.
pub const SIGNAL_EXTREME_NEW: u16 = 121;
/// `signal.extreme_stale`.
pub const SIGNAL_EXTREME_STALE: u16 = 122;
/// `signal.drawdown_threshold`.
pub const SIGNAL_DRAWDOWN_THRESHOLD: u16 = 123;
/// `signal.drawdown_recovery`.
pub const SIGNAL_DRAWDOWN_RECOVERY: u16 = 124;
/// `signal.range_position`.
pub const SIGNAL_RANGE_POSITION: u16 = 125;
/// `signal.relative_strength_cross`.
pub const SIGNAL_RELATIVE_STRENGTH_CROSS: u16 = 126;
/// `signal.lead_lag_cross`.
pub const SIGNAL_LEAD_LAG_CROSS: u16 = 127;
/// `signal.breadth_cross`.
pub const SIGNAL_BREADTH_CROSS: u16 = 128;
/// `signal.co_movement_cross`.
pub const SIGNAL_CO_MOVEMENT_CROSS: u16 = 129;
/// `signal.dispersion_cross`.
pub const SIGNAL_DISPERSION_CROSS: u16 = 130;
/// `signal.session_open`.
pub const SIGNAL_SESSION_OPEN: u16 = 131;
/// `signal.session_close`.
pub const SIGNAL_SESSION_CLOSE: u16 = 132;
/// `signal.opening_range_breakout`.
pub const SIGNAL_OPENING_RANGE_BREAKOUT: u16 = 133;
/// `signal.opening_range_fade`.
pub const SIGNAL_OPENING_RANGE_FADE: u16 = 134;
/// `signal.overnight_continuation`.
pub const SIGNAL_OVERNIGHT_CONTINUATION: u16 = 135;
/// `signal.overnight_fade`.
pub const SIGNAL_OVERNIGHT_FADE: u16 = 136;
/// `signal.session_range_expansion`.
pub const SIGNAL_SESSION_RANGE_EXPANSION: u16 = 137;
/// `signal.session_range_contraction`.
pub const SIGNAL_SESSION_RANGE_CONTRACTION: u16 = 138;
/// `level.bos`.
pub const LEVEL_BOS: u16 = 139;
/// `level.choch`.
pub const LEVEL_CHOCH: u16 = 140;
/// `level.session_vwap_test`.
pub const LEVEL_SESSION_VWAP_TEST: u16 = 141;
/// `level.opening_range_test`.
pub const LEVEL_OPENING_RANGE_TEST: u16 = 142;
/// `level.opening_range_break`.
pub const LEVEL_OPENING_RANGE_BREAK: u16 = 143;

/// The number of registered event codes.
pub const COUNT: usize = 143;

/// Every registered event code.
pub const ALL: [u16; 143] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50,
    51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74,
    75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98,
    99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117,
    118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135, 136,
    137, 138, 139, 140, 141, 142, 143,
];

/// The registered family for each event code: `(code, type, subtype, sources)`.
pub const FAMILIES: &[(u16, u8, u8, &[u16])] = &[
    (1, 1, 1, &[1, 2, 3, 7, 8]),
    (2, 1, 2, &[1, 2, 3, 7, 8]),
    (3, 1, 3, &[1, 2, 3, 7, 8]),
    (4, 2, 1, &[4]),
    (5, 2, 2, &[4]),
    (6, 2, 3, &[4]),
    (7, 2, 4, &[4]),
    (8, 3, 1, &[4]),
    (9, 3, 2, &[1, 2, 3]),
    (10, 3, 3, &[1]),
    (11, 3, 3, &[4]),
    (12, 3, 2, &[13, 14]),
    (13, 3, 4, &[15]),
    (14, 3, 5, &[13]),
    (15, 3, 2, &[17, 18]),
    (16, 3, 6, &[16]),
    (17, 3, 7, &[5]),
    (18, 3, 8, &[5]),
    (19, 3, 9, &[5]),
    (20, 1, 1, &[19]),
    (21, 1, 2, &[19]),
    (22, 1, 3, &[19]),
    (23, 2, 1, &[19]),
    (24, 2, 2, &[19]),
    (25, 2, 3, &[19]),
    (26, 2, 4, &[19]),
    (27, 3, 14, &[19]),
    (28, 3, 14, &[19]),
    (29, 3, 14, &[19]),
    (30, 1, 1, &[20]),
    (31, 1, 1, &[20]),
    (32, 1, 1, &[20]),
    (33, 1, 3, &[20]),
    (34, 1, 2, &[20]),
    (35, 2, 1, &[20]),
    (36, 2, 2, &[20]),
    (37, 3, 13, &[20]),
    (38, 3, 13, &[20]),
    (39, 3, 13, &[20]),
    (40, 3, 13, &[20]),
    (41, 2, 1, &[6]),
    (42, 2, 2, &[6]),
    (43, 3, 11, &[6]),
    (44, 3, 11, &[6]),
    (45, 3, 11, &[6]),
    (46, 2, 1, &[21]),
    (47, 2, 2, &[21]),
    (48, 3, 11, &[21]),
    (49, 3, 11, &[21]),
    (50, 1, 1, &[22]),
    (51, 3, 12, &[22]),
    (52, 3, 12, &[22]),
    (53, 3, 11, &[23]),
    (54, 2, 1, &[24]),
    (55, 2, 2, &[24]),
    (56, 2, 3, &[24]),
    (57, 2, 4, &[24]),
    (58, 3, 15, &[24]),
    (59, 3, 15, &[24]),
    (60, 2, 1, &[25]),
    (61, 2, 2, &[25]),
    (62, 2, 3, &[25]),
    (63, 2, 4, &[25]),
    (64, 3, 15, &[25]),
    (65, 3, 15, &[25]),
    (66, 2, 1, &[26]),
    (67, 2, 2, &[26]),
    (68, 2, 3, &[26]),
    (69, 2, 4, &[26]),
    (70, 3, 15, &[26]),
    (71, 3, 15, &[26]),
    (72, 2, 1, &[27]),
    (73, 2, 2, &[27]),
    (74, 3, 15, &[27]),
    (75, 1, 1, &[31]),
    (76, 1, 3, &[31]),
    (77, 1, 1, &[32]),
    (78, 1, 3, &[32]),
    (79, 1, 3, &[33]),
    (80, 1, 1, &[34]),
    (81, 1, 1, &[37]),
    (82, 1, 1, &[39]),
    (83, 1, 1, &[39]),
    (84, 2, 1, &[28]),
    (85, 2, 2, &[28]),
    (86, 2, 1, &[45]),
    (87, 2, 2, &[45]),
    (88, 2, 1, &[46]),
    (89, 2, 2, &[46]),
    (90, 2, 1, &[47]),
    (91, 2, 2, &[47]),
    (92, 3, 16, &[28]),
    (93, 3, 16, &[28]),
    (94, 3, 16, &[28]),
    (95, 3, 17, &[29]),
    (96, 3, 17, &[29]),
    (97, 3, 16, &[30]),
    (98, 3, 18, &[31]),
    (99, 3, 18, &[31]),
    (100, 3, 18, &[32]),
    (101, 3, 18, &[32]),
    (102, 3, 19, &[33]),
    (103, 3, 19, &[34]),
    (104, 3, 19, &[35]),
    (105, 3, 19, &[35]),
    (106, 3, 10, &[37]),
    (107, 3, 10, &[37]),
    (108, 3, 10, &[37]),
    (109, 3, 10, &[37]),
    (110, 3, 10, &[38]),
    (111, 3, 10, &[38]),
    (112, 3, 10, &[38]),
    (113, 3, 10, &[38]),
    (114, 3, 10, &[38]),
    (115, 3, 10, &[38]),
    (116, 3, 10, &[38]),
    (117, 3, 20, &[39]),
    (118, 3, 20, &[39]),
    (119, 3, 21, &[40]),
    (120, 3, 21, &[41]),
    (121, 3, 21, &[42]),
    (122, 3, 21, &[42]),
    (123, 3, 21, &[43]),
    (124, 3, 21, &[43]),
    (125, 3, 21, &[44]),
    (126, 3, 22, &[45]),
    (127, 3, 22, &[46]),
    (128, 3, 22, &[47]),
    (129, 3, 22, &[48]),
    (130, 3, 22, &[49]),
    (131, 3, 23, &[50]),
    (132, 3, 23, &[50]),
    (133, 3, 23, &[51]),
    (134, 3, 23, &[51]),
    (135, 3, 23, &[52]),
    (136, 3, 23, &[52]),
    (137, 3, 23, &[53]),
    (138, 3, 23, &[53]),
    (139, 1, 4, &[36]),
    (140, 1, 5, &[36]),
    (141, 1, 1, &[50]),
    (142, 1, 1, &[51]),
    (143, 1, 3, &[51]),
];

#[cfg(test)]
mod tests {
    use super::{ALL, COUNT, FAMILIES, LEGACY_UNREGISTERED};

    #[test]
    fn every_registered_event_code_is_unique_and_nonzero() {
        assert_eq!(ALL.len(), COUNT);
        assert_eq!(FAMILIES.len(), COUNT);
        let mut seen = std::collections::BTreeSet::new();
        for code in ALL {
            assert_ne!(code, LEGACY_UNREGISTERED);
            assert!(seen.insert(code), "duplicate event code {code}");
        }
        for (code, _, _, _) in FAMILIES {
            assert!(seen.contains(code), "family table misses code {code}");
        }
    }
}
