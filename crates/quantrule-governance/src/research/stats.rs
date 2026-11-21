//! Statistical helpers shared by the research evaluators.

/// The inverse standard-normal CDF.
///
/// Reproduces CPython's Wichura AS241 implementation, which the recorded
/// evaluators used for their Bonferroni and validation z-scores.
#[must_use]
#[allow(clippy::excessive_precision)] // The published rational coefficients are kept verbatim.
pub fn inverse_normal_cdf(probability: f64) -> f64 {
    if probability <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if probability >= 1.0 {
        return f64::INFINITY;
    }
    let q = probability - 0.5;
    if q.abs() <= 0.425 {
        let r = 0.180_625 - q * q;
        let num = poly(
            &[
                2.509_080_928_730_122_672_7e+3,
                3.343_057_558_358_812_810_5e+4,
                6.726_577_092_700_870_085_3e+4,
                4.592_195_393_154_987_145_7e+4,
                1.373_169_376_550_946_112_5e+4,
                1.971_590_950_306_551_442_7e+3,
                1.331_416_678_917_843_774_5e+2,
                3.387_132_872_796_366_608_0e+0,
            ],
            r,
        ) * q;
        let den = poly(
            &[
                5.226_495_278_852_854_561_0e+3,
                2.872_908_573_572_194_267_4e+4,
                3.930_789_580_009_271_061_0e+4,
                2.121_379_430_158_659_586_7e+4,
                5.394_196_021_424_751_107_7e+3,
                6.871_870_074_920_579_083_0e+2,
                4.231_333_070_160_091_125_2e+1,
                1.0,
            ],
            r,
        );
        return num / den;
    }
    let tail = if q <= 0.0 {
        probability
    } else {
        1.0 - probability
    };
    let mut r = (-tail.ln()).sqrt();
    let (num, den) = if r <= 5.0 {
        r -= 1.6;
        (
            poly(
                &[
                    7.745_450_142_783_414_076_4e-4,
                    2.272_384_498_926_918_458_3e-2,
                    2.417_807_251_774_506_117_7e-1,
                    1.270_458_252_452_368_382_6e+0,
                    3.647_848_324_763_204_605_0e+0,
                    5.769_497_221_460_691_405_5e+0,
                    4.630_337_846_156_545_295_9e+0,
                    1.423_437_110_749_683_577_3e+0,
                ],
                r,
            ),
            poly(
                &[
                    1.050_750_071_644_416_843_2e-9,
                    5.475_938_084_995_344_946_0e-4,
                    1.519_866_656_361_645_719_7e-2,
                    1.481_039_764_274_800_745_9e-1,
                    6.897_673_349_851_000_045_5e-1,
                    1.676_384_830_183_803_849_4e+0,
                    2.053_191_626_637_758_821_9e+0,
                    1.0,
                ],
                r,
            ),
        )
    } else {
        r -= 5.0;
        (
            poly(
                &[
                    2.010_334_399_292_288_132_7e-7,
                    2.711_555_568_743_487_578_2e-5,
                    1.242_660_947_388_078_438_6e-3,
                    2.653_218_952_657_612_309_3e-2,
                    2.965_605_718_285_048_912_3e-1,
                    1.784_826_539_917_291_335_8e+0,
                    5.463_784_911_164_114_369_9e+0,
                    6.657_904_643_501_103_777_2e+0,
                ],
                r,
            ),
            poly(
                &[
                    2.044_263_103_389_939_785_6e-15,
                    1.421_511_758_316_445_888_7e-7,
                    1.846_318_317_510_054_681_8e-5,
                    7.868_691_311_456_132_591_0e-4,
                    1.487_536_129_085_061_485_2e-2,
                    1.369_298_809_227_358_053_1e-1,
                    5.998_322_065_558_879_376_9e-1,
                    1.0,
                ],
                r,
            ),
        )
    };
    let value = num / den;
    if q < 0.0 { -value } else { value }
}

fn poly(coefficients: &[f64], value: f64) -> f64 {
    coefficients.iter().fold(0.0, |accumulator, coefficient| {
        accumulator * value + coefficient
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverse_normal_matches_cpython() {
        let z = inverse_normal_cdf(0.95);
        assert!((z - 1.644_853_626_951_471_5).abs() < 1e-15, "{z}");
        let z = inverse_normal_cdf(0.975);
        assert!((z - 1.959_963_984_540_053_6).abs() < 1e-15, "{z}");
        let z = inverse_normal_cdf(1.0 - 0.05 / 24.0);
        assert!((z - 2.865_260_238_532_133_3).abs() < 1e-14, "{z}");
        let z = inverse_normal_cdf(1.0 - 0.05 / 48.0);
        assert!((z - 3.078_088_072_842_161_3).abs() < 1e-14, "{z}");
    }
}
