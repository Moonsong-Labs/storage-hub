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
