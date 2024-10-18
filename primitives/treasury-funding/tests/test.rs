use shp_traits::TreasuryCutCalculator;
use sp_arithmetic::{PerThing, PerU16, Perbill, Percent, Perquintill};

/// This test the precision and panics if error too big error.
///
/// error is asserted to be less or equal to 8/accuracy or 8*f64::EPSILON
fn test_precision<P: PerThing>(system_utilization: P, ideal_system_utilization: P, falloff: P) {
    let accuracy_f64 = Into::<u128>::into(P::ACCURACY) as f64;
    let res = shp_treasury_funding::compute_percentage_to_treasury(
        system_utilization,
        ideal_system_utilization,
        falloff,
    );
    let res = Into::<u128>::into(res.deconstruct()) as f64 / accuracy_f64;

    let expect = float_ftt(system_utilization, ideal_system_utilization, falloff);

    let error = (res - expect).abs();

    if error > 8f64 / accuracy_f64 && error > 8.0 * f64::EPSILON {
        panic!(
            "system_utilization: {:?}, ideal_system_utilization: {:?}, falloff: {:?}, res: {}, expect: {}",
            system_utilization, ideal_system_utilization, falloff, res, expect
        );
    }
}

/// compute the percentage of funds to treasury using floats
fn float_ftt<P: PerThing>(system_utilization: P, ideal_system_utilization: P, falloff: P) -> f64 {
    let accuracy_f64 = Into::<u128>::into(P::ACCURACY) as f64;

    let ideal_system_utilization =
        Into::<u128>::into(ideal_system_utilization.deconstruct()) as f64 / accuracy_f64;
    let system_utilization =
        Into::<u128>::into(system_utilization.deconstruct()) as f64 / accuracy_f64;
    let falloff = Into::<u128>::into(falloff.deconstruct()) as f64 / accuracy_f64;

    let x_ideal = ideal_system_utilization;
    let x = system_utilization;
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
        for system_utilization in 0..1_000 {
            let system_utilization = P::from_rational(system_utilization, 1_000);
            let ideal_system_utilization = P::zero();
            let falloff = P::from_rational(1, 100);
            test_precision(system_utilization, ideal_system_utilization, falloff);
        }
    }

    test_falloff_precision_for_minimum_falloff::<Perquintill>();

    test_falloff_precision_for_minimum_falloff::<PerU16>();

    test_falloff_precision_for_minimum_falloff::<Perbill>();

    test_falloff_precision_for_minimum_falloff::<Percent>();
}

#[test]
fn compute_percentage_to_treasury_works() {
    fn compute_percentage_to_treasury_works<P: PerThing>() {
        for system_utilization in 0..100 {
            for ideal_system_utilization in 0..10 {
                for falloff in 1..10 {
                    let system_utilization = P::from_rational(system_utilization, 100);
                    let ideal_system_utilization = P::from_rational(ideal_system_utilization, 10);
                    let falloff = P::from_rational(falloff, 100);
                    test_precision(system_utilization, ideal_system_utilization, falloff);
                }
            }
        }
    }

    compute_percentage_to_treasury_works::<Perquintill>();

    compute_percentage_to_treasury_works::<PerU16>();

    compute_percentage_to_treasury_works::<Perbill>();

    compute_percentage_to_treasury_works::<Percent>();
}

mod no_treasury_cut {
    use super::*;

    #[test]
    fn correctly_returns_0_for_any_system_utilization() {
        fn correctly_returns_0_for_any_system_utilization<P: PerThing>() {
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

        correctly_returns_0_for_any_system_utilization::<Perquintill>();

        correctly_returns_0_for_any_system_utilization::<PerU16>();

        correctly_returns_0_for_any_system_utilization::<Perbill>();

        correctly_returns_0_for_any_system_utilization::<Percent>();
    }
}

mod linear_then_power_of_two_cut {
    use shp_treasury_funding::{
        LinearThenPowerOfTwoTreasuryCutCalculator, LinearThenPowerOfTwoTreasuryCutCalculatorConfig,
    };
    use sp_arithmetic::{FixedPointNumber, FixedU128};
    use sp_core::Get;

    use super::*;

    // Mock implementation of LinearThenPowerOfTwoTreasuryCutCalculator
    struct IdealUtilizationRate<P: PerThing>(core::marker::PhantomData<P>);
    impl<P: PerThing> Get<P> for IdealUtilizationRate<P> {
        fn get() -> P {
            P::from_rational(60, 100)
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
    impl<P: PerThing> LinearThenPowerOfTwoTreasuryCutCalculatorConfig<P> for MockConfig {
        type Balance = u128;
        type ProvidedUnit = u64;
        type IdealUtilizationRate = IdealUtilizationRate<P>;
        type DecayRate = DecayRate<P>;
        type MinimumCut = MinimumCut<P>;
        type MaximumCut = MaximumCut<P>;
    }
    type TestTreasuryCutCalculator<P> = LinearThenPowerOfTwoTreasuryCutCalculator<MockConfig, P>;

    #[test]
    fn correctly_returns_lineal_cut_until_ideal_utilization_rate() {
        fn correctly_returns_lineal_cut_until_ideal_utilization_rate<P: PerThing>() {
            // We calculate what the linear decayment of the treasury cut should be
            let minimum_cut = MinimumCut::<P>::get();
            let maximum_cut = MaximumCut::<P>::get();
            let delta_cut = maximum_cut.saturating_sub(minimum_cut);

            // Then for each utilization rate between 0 and the ideal rate we calculate the treasury cut
            let ideal_utilization_rate: P = IdealUtilizationRate::<P>::get();
            let ideal_utilization_rate_as_percentage: u128 =
                Into::<u128>::into(ideal_utilization_rate.deconstruct()) * 100 / P::ACCURACY.into();
            for used_amount in 0..ideal_utilization_rate_as_percentage {
                let provided_amount = 100;
                let amount_to_charge = 100000;
                let res: u128 =
                    <TestTreasuryCutCalculator<P> as TreasuryCutCalculator>::calculate_treasury_cut(
                        provided_amount,
                        used_amount.try_into().expect(
                            "Used amount is at most 100 so it should comfortably fit into u64",
                        ),
                        amount_to_charge,
                    );

                // We manually calculate the treasury cut with the parameters calculated before for the linear formula
                let adjustment = (P::from_rational(used_amount, provided_amount.into())
                    / ideal_utilization_rate)
                    .left_from_one();
                let treasury_cut: FixedU128 =
                    minimum_cut.saturating_add(delta_cut * adjustment).into();

                // And then we check that both match
                assert_eq!(
                    res,
                    amount_to_charge * treasury_cut.into_inner() / FixedU128::DIV
                );
            }
        }

        correctly_returns_lineal_cut_until_ideal_utilization_rate::<Perquintill>();

        correctly_returns_lineal_cut_until_ideal_utilization_rate::<PerU16>();

        correctly_returns_lineal_cut_until_ideal_utilization_rate::<Perbill>();

        correctly_returns_lineal_cut_until_ideal_utilization_rate::<Percent>();
    }

    #[test]
    fn correctly_decays_with_power_of_2_after_ideal_utilization_rate() {
        fn correctly_decays_with_power_of_2_after_ideal_utilization_rate<P: PerThing>() {
            // We calculate what the linear decayment of the treasury cut should be
            let minimum_cut = MinimumCut::<P>::get();
            let maximum_cut = MaximumCut::<P>::get();
            let delta_cut = maximum_cut.saturating_sub(minimum_cut);

            // Then for each utilization rate between the ideal rate and 100 we calculate the treasury cut
            let ideal_utilization_rate: P = IdealUtilizationRate::<P>::get();
            let ideal_utilization_rate_as_percentage: u128 =
                Into::<u128>::into(ideal_utilization_rate.deconstruct()) * 100 / P::ACCURACY.into();
            for used_amount in ideal_utilization_rate_as_percentage..100 {
                let provided_amount = 100;
                let amount_to_charge = 100000;
                let res: u128 =
                    <TestTreasuryCutCalculator<P> as TreasuryCutCalculator>::calculate_treasury_cut(
                        provided_amount,
                        used_amount.try_into().expect(
                            "Used amount is at most 100 so it should comfortably fit into u64",
                        ),
                        amount_to_charge,
                    );

                // We manually calculate the treasury cut with the parameters calculated before for the linear formula
                let adjustment = (ideal_utilization_rate
                    / P::from_rational(used_amount, provided_amount.into()))
                .left_from_one();
                let treasury_cut: FixedU128 =
                    minimum_cut.saturating_add(delta_cut * adjustment).into();

                // And then we check that the treasury cut increases faster than linearly
                if treasury_cut != minimum_cut.into() {
                    assert!(res > amount_to_charge * treasury_cut.into_inner() / FixedU128::DIV);
                }
            }
        }

        correctly_decays_with_power_of_2_after_ideal_utilization_rate::<Perquintill>();

        correctly_decays_with_power_of_2_after_ideal_utilization_rate::<PerU16>();

        correctly_decays_with_power_of_2_after_ideal_utilization_rate::<Perbill>();

        correctly_decays_with_power_of_2_after_ideal_utilization_rate::<Percent>();
    }
}
