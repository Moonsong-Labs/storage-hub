use shp_traits::TreasuryCutCalculator;
use sp_arithmetic::{PerThing, PerU16, Perbill, Percent, Perquintill};

/// This tests the precision and panics if the error is too big.
///
/// The error is asserted to be less or equal to 8/accuracy or 8*f64::EPSILON
fn test_precision<P: PerThing>(system_utilisation: P, ideal_system_utilisation: P, falloff: P) {
    let accuracy_f64 = Into::<u128>::into(P::ACCURACY) as f64;
    let res = shp_treasury_funding::compute_adjustment_over_minimum_cut(
        system_utilisation,
        ideal_system_utilisation,
        falloff,
    );
    let res = Into::<u128>::into(res.deconstruct()) as f64 / accuracy_f64;

    let expect = float_ftt(system_utilisation, ideal_system_utilisation, falloff);

    let error = (res - expect).abs();

    if error > 8f64 / accuracy_f64 && error > 8.0 * f64::EPSILON {
        panic!(
            "system_utilisation: {:?}, ideal_system_utilisation: {:?}, falloff: {:?}, res: {}, expect: {}",
            system_utilisation, ideal_system_utilisation, falloff, res, expect
        );
    }
}

/// Compute the percentage of the adjustment of the treasury cut using floats instead of PerThings
fn float_ftt<P: PerThing>(system_utilisation: P, ideal_system_utilisation: P, falloff: P) -> f64 {
    let accuracy_f64 = Into::<u128>::into(P::ACCURACY) as f64;

    let ideal_system_utilisation =
        Into::<u128>::into(ideal_system_utilisation.deconstruct()) as f64 / accuracy_f64;
    let system_utilisation =
        Into::<u128>::into(system_utilisation.deconstruct()) as f64 / accuracy_f64;
    let falloff = Into::<u128>::into(falloff.deconstruct()) as f64 / accuracy_f64;

    let x_ideal = ideal_system_utilisation;
    let x = system_utilisation;
    let d = falloff;

    if x < x_ideal {
        1f64 - x / x_ideal
    } else {
        1f64 - 2_f64.powf((x_ideal - x) / d)
    }
}

#[test]
fn test_precision_for_minimum_falloff() {
    fn test_falloff_precision_for_minimum_falloff<P: PerThing>() {
        for system_utilisation in 0..1_000 {
            let system_utilisation = P::from_rational(system_utilisation, 1_000);
            let ideal_system_utilisation = P::zero();
            let falloff = P::from_rational(1, 100);
            test_precision(system_utilisation, ideal_system_utilisation, falloff);
        }
    }

    test_falloff_precision_for_minimum_falloff::<Perquintill>();

    test_falloff_precision_for_minimum_falloff::<PerU16>();

    test_falloff_precision_for_minimum_falloff::<Perbill>();

    test_falloff_precision_for_minimum_falloff::<Percent>();
}

#[test]
fn compute_adjustment_over_minimum_cut_works() {
    fn compute_adjustment_over_minimum_cut_works<P: PerThing>() {
        for system_utilisation in 0..100 {
            for ideal_system_utilisation in 0..10 {
                for falloff in 1..10 {
                    let system_utilisation = P::from_rational(system_utilisation, 100);
                    let ideal_system_utilisation = P::from_rational(ideal_system_utilisation, 10);
                    let falloff = P::from_rational(falloff, 100);
                    test_precision(system_utilisation, ideal_system_utilisation, falloff);
                }
            }
        }
    }

    compute_adjustment_over_minimum_cut_works::<Perquintill>();

    compute_adjustment_over_minimum_cut_works::<PerU16>();

    compute_adjustment_over_minimum_cut_works::<Perbill>();

    compute_adjustment_over_minimum_cut_works::<Percent>();
}

mod no_treasury_cut {
    use super::*;

    #[test]
    fn correctly_returns_0_for_any_system_utilisation() {
        fn correctly_returns_0_for_any_system_utilisation<P: PerThing>() {
            for used_amount in 0..100 {
                let provided_amount = 100;
                let amount_to_charge = 100000;
                let res = <shp_treasury_funding::NoCutTreasuryCutCalculator<u64, u64> as TreasuryCutCalculator>::calculate_treasury_cut(
                            provided_amount,
                            used_amount,
                            amount_to_charge,
                        );
                assert_eq!(res, 0);
            }
        }

        correctly_returns_0_for_any_system_utilisation::<Perquintill>();

        correctly_returns_0_for_any_system_utilisation::<PerU16>();

        correctly_returns_0_for_any_system_utilisation::<Perbill>();

        correctly_returns_0_for_any_system_utilisation::<Percent>();
    }
}

mod linear_then_power_of_two_cut {
    use shp_treasury_funding::{
        LinearThenPowerOfTwoTreasuryCutCalculator, LinearThenPowerOfTwoTreasuryCutCalculatorConfig,
    };
    use sp_arithmetic::{traits::Saturating, FixedPointNumber, FixedU128};
    use sp_core::Get;

    use super::*;

    // Mock implementation of LinearThenPowerOfTwoTreasuryCutCalculator
    struct IdealUtilisationRate<P: PerThing>(core::marker::PhantomData<P>);
    impl<P: PerThing> Get<P> for IdealUtilisationRate<P> {
        fn get() -> P {
            P::from_rational(85, 100)
        }
    }

    struct MinimumCut<P: PerThing>(core::marker::PhantomData<P>);
    impl<P: PerThing> Get<P> for MinimumCut<P> {
        fn get() -> P {
            P::from_rational(1, 100)
        }
    }

    struct MaximumCut<P: PerThing>(core::marker::PhantomData<P>);
    impl<P: PerThing> Get<P> for MaximumCut<P> {
        fn get() -> P {
            P::from_rational(5, 100)
        }
    }

    struct DecayRate<P: PerThing>(core::marker::PhantomData<P>);
    impl<P: PerThing> Get<P> for DecayRate<P> {
        fn get() -> P {
            P::from_rational(5, 100)
        }
    }

    struct MockConfig;
    impl LinearThenPowerOfTwoTreasuryCutCalculatorConfig<Perquintill> for MockConfig {
        type Balance = u128;
        type ProvidedUnit = u64;
        type IdealUtilisationRate = IdealUtilisationRate<Perquintill>;
        type DecayRate = DecayRate<Perquintill>;
        type MinimumCut = MinimumCut<Perquintill>;
        type MaximumCut = MaximumCut<Perquintill>;
    }
    impl LinearThenPowerOfTwoTreasuryCutCalculatorConfig<Perbill> for MockConfig {
        type Balance = u128;
        type ProvidedUnit = u64;
        type IdealUtilisationRate = IdealUtilisationRate<Perbill>;
        type DecayRate = DecayRate<Perbill>;
        type MinimumCut = MinimumCut<Perbill>;
        type MaximumCut = MaximumCut<Perbill>;
    }
    impl LinearThenPowerOfTwoTreasuryCutCalculatorConfig<PerU16> for MockConfig {
        type Balance = u128;
        type ProvidedUnit = u64;
        type IdealUtilisationRate = IdealUtilisationRate<PerU16>;
        type DecayRate = DecayRate<PerU16>;
        type MinimumCut = MinimumCut<PerU16>;
        type MaximumCut = MaximumCut<PerU16>;
    }
    impl LinearThenPowerOfTwoTreasuryCutCalculatorConfig<Percent> for MockConfig {
        type Balance = u128;
        type ProvidedUnit = u64;
        type IdealUtilisationRate = IdealUtilisationRate<Percent>;
        type DecayRate = DecayRate<Percent>;
        type MinimumCut = MinimumCut<Percent>;
        type MaximumCut = MaximumCut<Percent>;
    }

    type TestTreasuryCutCalculatorPerquintill =
        LinearThenPowerOfTwoTreasuryCutCalculator<MockConfig, Perquintill>;
    type TestTreasuryCutCalculatorPerbill =
        LinearThenPowerOfTwoTreasuryCutCalculator<MockConfig, Perbill>;
    type TestTreasuryCutCalculatorPerU16 =
        LinearThenPowerOfTwoTreasuryCutCalculator<MockConfig, PerU16>;
    type TestTreasuryCutCalculatorPercent =
        LinearThenPowerOfTwoTreasuryCutCalculator<MockConfig, Percent>;

    #[test]
    fn correctly_returns_lineal_cut_until_ideal_utilisation_rate_perquintill() {
        // We calculate what the linear decayment of the treasury cut should be
        let minimum_cut = MinimumCut::<Perquintill>::get();
        let maximum_cut = MaximumCut::<Perquintill>::get();
        let delta_cut = maximum_cut.saturating_sub(minimum_cut);

        // Then for each utilisation rate between 0 and the ideal rate we calculate the treasury cut
        let ideal_utilisation_rate = IdealUtilisationRate::<Perquintill>::get();
        let ideal_utilisation_rate_as_percentage: u128 =
            Into::<u128>::into(ideal_utilisation_rate.deconstruct()) * 100
                / Perquintill::ACCURACY as u128;
        for used_amount in 0..ideal_utilisation_rate_as_percentage {
            let provided_amount = 100;
            let amount_to_charge = 100000;
            let res =
                    <TestTreasuryCutCalculatorPerquintill as TreasuryCutCalculator>::calculate_treasury_cut(
                        provided_amount,
                        used_amount.try_into().expect(
                            "Used amount is at most 100 so it should comfortably fit into u64",
                        ),
                        amount_to_charge,
                    );

            // We manually calculate the treasury cut with the parameters calculated before for the linear formula
            let adjustment = (Perquintill::from_rational(used_amount, provided_amount.into())
                / ideal_utilisation_rate)
                .left_from_one();
            let treasury_cut: FixedU128 = minimum_cut.saturating_add(delta_cut * adjustment).into();

            // And then we check that both match
            assert_eq!(
                res,
                amount_to_charge * treasury_cut.into_inner() / FixedU128::DIV
            );
        }
    }

    #[test]
    fn correctly_returns_lineal_cut_until_ideal_utilisation_rate_perbill() {
        // We calculate what the linear decayment of the treasury cut should be
        let minimum_cut = MinimumCut::<Perbill>::get();
        let maximum_cut = MaximumCut::<Perbill>::get();
        let delta_cut = maximum_cut.saturating_sub(minimum_cut);

        // Then for each utilisation rate between 0 and the ideal rate we calculate the treasury cut
        let ideal_utilisation_rate = IdealUtilisationRate::<Perbill>::get();
        let ideal_utilisation_rate_as_percentage: u128 =
            Into::<u128>::into(ideal_utilisation_rate.deconstruct()) * 100
                / Perbill::ACCURACY as u128;
        for used_amount in 0..ideal_utilisation_rate_as_percentage {
            let provided_amount = 100;
            let amount_to_charge = 100000;
            let res =
                <TestTreasuryCutCalculatorPerbill as TreasuryCutCalculator>::calculate_treasury_cut(
                    provided_amount,
                    used_amount
                        .try_into()
                        .expect("Used amount is at most 100 so it should comfortably fit into u64"),
                    amount_to_charge,
                );

            // We manually calculate the treasury cut with the parameters calculated before for the linear formula
            let adjustment = (Perbill::from_rational(used_amount, provided_amount.into())
                / ideal_utilisation_rate)
                .left_from_one();
            let treasury_cut: FixedU128 = minimum_cut.saturating_add(delta_cut * adjustment).into();

            // And then we check that both match
            assert_eq!(
                res,
                amount_to_charge * treasury_cut.into_inner() / FixedU128::DIV
            );
        }
    }

    #[test]
    fn correctly_returns_lineal_cut_until_ideal_utilisation_rate_per_u16() {
        // We calculate what the linear decayment of the treasury cut should be
        let minimum_cut = MinimumCut::<PerU16>::get();
        let maximum_cut = MaximumCut::<PerU16>::get();
        let delta_cut = maximum_cut.saturating_sub(minimum_cut);

        // Then for each utilisation rate between 0 and the ideal rate we calculate the treasury cut
        let ideal_utilisation_rate = IdealUtilisationRate::<PerU16>::get();
        let ideal_utilisation_rate_as_percentage: u128 =
            Into::<u128>::into(ideal_utilisation_rate.deconstruct()) * 100
                / PerU16::ACCURACY as u128;
        for used_amount in 0..ideal_utilisation_rate_as_percentage {
            let provided_amount = 100;
            let amount_to_charge = 100000;
            let res =
                <TestTreasuryCutCalculatorPerU16 as TreasuryCutCalculator>::calculate_treasury_cut(
                    provided_amount,
                    used_amount
                        .try_into()
                        .expect("Used amount is at most 100 so it should comfortably fit into u64"),
                    amount_to_charge,
                );

            // We manually calculate the treasury cut with the parameters calculated before for the linear formula
            let adjustment = (PerU16::from_rational(used_amount, provided_amount.into())
                / ideal_utilisation_rate)
                .left_from_one();
            let treasury_cut: FixedU128 = minimum_cut.saturating_add(delta_cut * adjustment).into();

            // And then we check that both match
            assert_eq!(
                res,
                amount_to_charge * treasury_cut.into_inner() / FixedU128::DIV
            );
        }
    }

    #[test]
    fn correctly_returns_lineal_cut_until_ideal_utilisation_rate_percent() {
        // We calculate what the linear decayment of the treasury cut should be
        let minimum_cut = MinimumCut::<Percent>::get();
        let maximum_cut = MaximumCut::<Percent>::get();
        let delta_cut = maximum_cut.saturating_sub(minimum_cut);

        // Then for each utilisation rate between 0 and the ideal rate we calculate the treasury cut
        let ideal_utilisation_rate = IdealUtilisationRate::<Percent>::get();
        let ideal_utilisation_rate_as_percentage: u128 =
            Into::<u128>::into(ideal_utilisation_rate.deconstruct()) * 100
                / Percent::ACCURACY as u128;
        for used_amount in 0..ideal_utilisation_rate_as_percentage {
            let provided_amount = 100;
            let amount_to_charge = 100000;
            let res =
                <TestTreasuryCutCalculatorPercent as TreasuryCutCalculator>::calculate_treasury_cut(
                    provided_amount,
                    used_amount
                        .try_into()
                        .expect("Used amount is at most 100 so it should comfortably fit into u64"),
                    amount_to_charge,
                );

            // We manually calculate the treasury cut with the parameters calculated before for the linear formula
            let adjustment = (Percent::from_rational(used_amount, provided_amount.into())
                / ideal_utilisation_rate)
                .left_from_one();
            let treasury_cut: FixedU128 = minimum_cut.saturating_add(delta_cut * adjustment).into();

            // And then we check that both match
            assert_eq!(
                res,
                amount_to_charge * treasury_cut.into_inner() / FixedU128::DIV
            );
        }
    }

    #[test]
    fn correctly_decays_with_power_of_2_after_ideal_utilisation_rate_perquintill() {
        // We calculate what the linear decayment of the treasury cut should be
        let minimum_cut = MinimumCut::<Perquintill>::get();
        let maximum_cut = MaximumCut::<Perquintill>::get();
        let delta_cut = maximum_cut.saturating_sub(minimum_cut);

        // Then for each utilisation rate between the ideal rate and 100 we calculate the treasury cut
        let ideal_utilisation_rate = IdealUtilisationRate::<Perquintill>::get();
        let ideal_utilisation_rate_as_percentage: u128 =
            Into::<u128>::into(ideal_utilisation_rate.deconstruct()) * 100
                / Perquintill::ACCURACY as u128;
        for used_amount in ideal_utilisation_rate_as_percentage..100 {
            let provided_amount = 100;
            let amount_to_charge = 100000;
            let res: u128 =
                <TestTreasuryCutCalculatorPerquintill as TreasuryCutCalculator>::calculate_treasury_cut(
                    provided_amount,
                    used_amount
                        .try_into()
                        .expect("Used amount is at most 100 so it should comfortably fit into u64"),
                    amount_to_charge,
                );

            // We manually calculate the treasury cut with the parameters calculated before for the linear formula
            let adjustment = (ideal_utilisation_rate
                / Perquintill::from_rational(used_amount, provided_amount.into()))
            .left_from_one();
            let treasury_cut: FixedU128 = minimum_cut.saturating_add(delta_cut * adjustment).into();

            // And then we check that the treasury cut increases faster than linearly
            if treasury_cut != minimum_cut.into() {
                assert!(res > amount_to_charge * treasury_cut.into_inner() / FixedU128::DIV);
            }
        }
    }

    #[test]
    fn correctly_decays_with_power_of_2_after_ideal_utilisation_rate_perbill() {
        // We calculate what the linear decayment of the treasury cut should be
        let minimum_cut = MinimumCut::<Perbill>::get();
        let maximum_cut = MaximumCut::<Perbill>::get();
        let delta_cut = maximum_cut.saturating_sub(minimum_cut);

        // Then for each utilisation rate between the ideal rate and 100 we calculate the treasury cut
        let ideal_utilisation_rate = IdealUtilisationRate::<Perbill>::get();
        let ideal_utilisation_rate_as_percentage: u128 =
            Into::<u128>::into(ideal_utilisation_rate.deconstruct()) * 100
                / Perbill::ACCURACY as u128;
        for used_amount in ideal_utilisation_rate_as_percentage..100 {
            let provided_amount = 100;
            let amount_to_charge = 100000;
            let res: u128 =
                <TestTreasuryCutCalculatorPerbill as TreasuryCutCalculator>::calculate_treasury_cut(
                    provided_amount,
                    used_amount
                        .try_into()
                        .expect("Used amount is at most 100 so it should comfortably fit into u64"),
                    amount_to_charge,
                );

            // We manually calculate the treasury cut with the parameters calculated before for the linear formula
            let adjustment = (ideal_utilisation_rate
                / Perbill::from_rational(used_amount, provided_amount.into()))
            .left_from_one();
            let treasury_cut: FixedU128 = minimum_cut.saturating_add(delta_cut * adjustment).into();

            // And then we check that the treasury cut increases faster than linearly
            if treasury_cut != minimum_cut.into() {
                assert!(res > amount_to_charge * treasury_cut.into_inner() / FixedU128::DIV);
            }
        }
    }

    #[test]
    fn correctly_decays_with_power_of_2_after_ideal_utilisation_rate_per_u16() {
        // We calculate what the linear decayment of the treasury cut should be
        let minimum_cut = MinimumCut::<PerU16>::get();
        let maximum_cut = MaximumCut::<PerU16>::get();
        let delta_cut = maximum_cut.saturating_sub(minimum_cut);

        // Then for each utilisation rate between the ideal rate and 100 we calculate the treasury cut
        let ideal_utilisation_rate = IdealUtilisationRate::<PerU16>::get();
        let ideal_utilisation_rate_as_percentage: u128 =
            Into::<u128>::into(ideal_utilisation_rate.deconstruct()) * 100
                / PerU16::ACCURACY as u128;
        for used_amount in ideal_utilisation_rate_as_percentage..100 {
            let provided_amount = 100;
            let amount_to_charge = 100000;
            let res: u128 =
                <TestTreasuryCutCalculatorPerU16 as TreasuryCutCalculator>::calculate_treasury_cut(
                    provided_amount,
                    used_amount
                        .try_into()
                        .expect("Used amount is at most 100 so it should comfortably fit into u64"),
                    amount_to_charge,
                );

            // We manually calculate the treasury cut with the parameters calculated before for the linear formula
            let adjustment = (ideal_utilisation_rate
                / PerU16::from_rational(used_amount, provided_amount.into()))
            .left_from_one();
            let treasury_cut: FixedU128 = minimum_cut.saturating_add(delta_cut * adjustment).into();

            // And then we check that the treasury cut increases faster than linearly
            if treasury_cut != minimum_cut.into() {
                assert!(res > amount_to_charge * treasury_cut.into_inner() / FixedU128::DIV);
            }
        }
    }

    #[test]
    fn correctly_decays_with_power_of_2_after_ideal_utilisation_rate_percent() {
        // We calculate what the linear decayment of the treasury cut should be
        let minimum_cut = MinimumCut::<Percent>::get();
        let maximum_cut = MaximumCut::<Percent>::get();
        let delta_cut = maximum_cut.saturating_sub(minimum_cut);

        // Then for each utilisation rate between the ideal rate and 100 we calculate the treasury cut
        let ideal_utilisation_rate = IdealUtilisationRate::<Percent>::get();
        let ideal_utilisation_rate_as_percentage: u128 =
            Into::<u128>::into(ideal_utilisation_rate.deconstruct()) * 100
                / Percent::ACCURACY as u128;
        for used_amount in ideal_utilisation_rate_as_percentage..100 {
            let provided_amount = 100;
            let amount_to_charge = 100000;
            let res: u128 =
                <TestTreasuryCutCalculatorPercent as TreasuryCutCalculator>::calculate_treasury_cut(
                    provided_amount,
                    used_amount
                        .try_into()
                        .expect("Used amount is at most 100 so it should comfortably fit into u64"),
                    amount_to_charge,
                );

            // We manually calculate the treasury cut with the parameters calculated before for the linear formula
            let adjustment = (ideal_utilisation_rate
                / Percent::from_rational(used_amount, provided_amount.into()))
            .left_from_one();
            let treasury_cut: FixedU128 = minimum_cut.saturating_add(delta_cut * adjustment).into();

            // And then we check that the treasury cut increases faster than linearly
            if treasury_cut != minimum_cut.into() {
                assert!(res > amount_to_charge * treasury_cut.into_inner() / FixedU128::DIV);
            }
        }
    }
}
