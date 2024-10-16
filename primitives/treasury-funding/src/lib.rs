#![cfg_attr(not(feature = "std"), no_std)]

//! Function to calculate the percentage amount of tokens from the baseline that
//! will be transfered to the treasury from the charged payment streams.

use core::marker::PhantomData;
use shp_traits::{NumericalParam, TreasuryCutCalculator};
use sp_arithmetic::{
    biguint::BigUint,
    traits::{SaturatedConversion, Zero},
    PerThing, Perquintill,
};
use sp_core::Get;

/// A struct that implements the `TreasuryCutCalculator` trait, where the cut is 0%
pub struct NoCutTreasuryCutCalculator<P: PerThing, S: NumericalParam + Into<u64>>(
    PhantomData<(P, S)>,
);

impl<P: PerThing, S: NumericalParam + Into<u64>> TreasuryCutCalculator
    for NoCutTreasuryCutCalculator<P, S>
{
    type PercentageType = P;
    type ProvidedUnit = S;
    fn calculate_treasury_cut(
        _provided_amount: Self::ProvidedUnit,
        _used_amount: Self::ProvidedUnit,
    ) -> Self::PercentageType {
        P::zero()
    }
}

/// A struct that implements the `TreasuryCutCalculator` trait, where the cut is determined by the
/// `compute_percentage_to_treasury` function.
pub struct LinearThenPowerOfTwoTreasuryCutCalculator<
    T: LinearThenPowerOfTwoTreasuryCutCalculatorConfig,
>(PhantomData<T>);

/// The configuration trait for the [`LinearThenPowerOfTwoTreasuryCutCalculator`].
pub trait LinearThenPowerOfTwoTreasuryCutCalculatorConfig {
    /// The PerThing type used to represent the percentage of funds that should go to the treasury.
    type PercentageType: PerThing;
    /// The numerical type used to represent a ProvidedUnit.
    type ProvidedUnit: NumericalParam + Into<u64>;
    /// The ideal system utilization rate. It's a PerThing since it should be a percentage.
    type IdealUtilizationRate: Get<Self::PercentageType>;
    /// The falloff or decay rate. It's a PerThing since it should be a percentage.
    type DecayRate: Get<Self::PercentageType>;
}

impl<T> TreasuryCutCalculator for LinearThenPowerOfTwoTreasuryCutCalculator<T>
where
    T: LinearThenPowerOfTwoTreasuryCutCalculatorConfig,
{
    type PercentageType = T::PercentageType;
    type ProvidedUnit = T::ProvidedUnit;
    fn calculate_treasury_cut(
        provided_amount: Self::ProvidedUnit,
        used_amount: Self::ProvidedUnit,
    ) -> Self::PercentageType {
        let system_utilization = Self::PercentageType::from_rational(
            used_amount.into().into(),
            provided_amount.into().into(),
        );
        let ideal_system_utilization = T::IdealUtilizationRate::get();
        let falloff = T::DecayRate::get();
        compute_percentage_to_treasury(system_utilization, ideal_system_utilization, falloff)
    }
}

/// Compute the fraction of charged tokens that go to the treasury using function
///
/// ```ignore
/// I(x) = for x between 0 and x_ideal: 1 - x / x_ideal,
/// for x between x_ideal and 1: 1 - 2^((x_ideal - x) / d)
/// ```
///
/// where:
/// * x is the system utilization rate, i.e. fraction of the total storage being provided in the system
/// that is currently being used.
/// * d is the falloff or `decay_rate`
/// * x_ideal: the ideal system utilization rate.
///
/// The result is meant to be scaled with the minimum percentage amount and maximum percentage amount
///  that goes to the treasury.
///
/// Arguments are:
/// * `system_utilization`: The fraction of the total storage being provided in the system that is
/// currently being used. Known as `x` in the literature. Must be between 0 and 1.
/// * `ideal_system_utilization`: The fraction of total storage being provided in the system that should
/// be being actively used. Known as `x_ideal` in the literature. Must be between 0 and 1.
/// * `falloff`: Known as `decay_rate` in the literature. A co-efficient dictating the strength of
///   the global incentivization to get the `ideal_system_utilization`. A higher number results in less typical
///   funds to the treasury at the cost of greater volatility for providers. Must be more than 0.01.
pub fn compute_percentage_to_treasury<P: PerThing>(
    system_utilization: P,
    ideal_system_utilization: P,
    falloff: P,
) -> P {
    if system_utilization < ideal_system_utilization {
        // ideal_system_utilization is more than 0 because it is strictly more than system_utilization
        return (system_utilization / ideal_system_utilization).left_from_one();
    }

    if falloff < P::from_percent(1.into()) {
        log::error!("Invalid inflation computation: falloff less than 1% is not supported");
        return PerThing::zero();
    }

    let accuracy = {
        let mut a = BigUint::from(Into::<u128>::into(P::ACCURACY));
        a.lstrip();
        a
    };

    let mut falloff = BigUint::from(falloff.deconstruct().into());
    falloff.lstrip();

    let ln2 = {
        /// `ln(2)` expressed in as perquintillionth.
        const LN2: u64 = 0_693_147_180_559_945_309;
        let ln2 = P::from_rational(LN2.into(), Perquintill::ACCURACY.into());
        BigUint::from(ln2.deconstruct().into())
    };

    // falloff is stripped above.
    let ln2_div_d = div_by_stripped(ln2.mul(&accuracy), &falloff);

    let ftt_param = FTTParam {
        x_ideal: BigUint::from(ideal_system_utilization.deconstruct().into()),
        x: BigUint::from(system_utilization.deconstruct().into()),
        accuracy,
        ln2_div_d,
    };

    let res = compute_taylor_serie_part(&ftt_param);

    match u128::try_from(res.clone()) {
        Ok(res) if res <= Into::<u128>::into(P::ACCURACY) => {
            P::from_parts(res.saturated_into()).left_from_one()
        }
        // If result is beyond bounds there is nothing we can do
        _ => {
            log::error!(
                "Invalid funds to treasury computation: unexpected result {:?}",
                res
            );
            P::zero()
        }
    }
}

/// Internal struct holding parameter info alongside other cached value.
/// Funds To Treasury params.
///
/// All expressed in part from `accuracy`
struct FTTParam {
    ln2_div_d: BigUint,
    x_ideal: BigUint,
    x: BigUint,
    /// Must be stripped and have no leading zeros.
    accuracy: BigUint,
}

/// Compute `2^((x_ideal - x) / d)` using a taylor serie.
///
/// x must be strictly more than x_ideal.
///
/// result is expressed with accuracy `FTTParam.accuracy`
fn compute_taylor_serie_part(p: &FTTParam) -> BigUint {
    // The last computed taylor term.
    let mut last_taylor_term = p.accuracy.clone();

    // Whereas taylor sum is positive.
    let mut taylor_sum_positive = true;

    // The sum of all taylor term.
    let mut taylor_sum = last_taylor_term.clone();

    for k in 1..300 {
        last_taylor_term = compute_taylor_term(k, &last_taylor_term, p);

        if last_taylor_term.is_zero() {
            break;
        }

        let last_taylor_term_positive = k % 2 == 0;

        if taylor_sum_positive == last_taylor_term_positive {
            taylor_sum = taylor_sum.add(&last_taylor_term);
        } else if taylor_sum >= last_taylor_term {
            taylor_sum = taylor_sum
                .sub(&last_taylor_term)
                // NOTE: Should never happen as checked above
                .unwrap_or_else(|e| e);
        } else {
            taylor_sum_positive = !taylor_sum_positive;
            taylor_sum = last_taylor_term
                .clone()
                .sub(&taylor_sum)
                // NOTE: Should never happen as checked above
                .unwrap_or_else(|e| e);
        }
    }

    if !taylor_sum_positive {
        return BigUint::zero();
    }

    taylor_sum.lstrip();
    taylor_sum
}

/// Return the absolute value of k-th taylor term of `2^((x_ideal - x))/d` i.e.
/// `((x - x_ideal) * ln(2) / d)^k / k!`
///
/// x must be strictly more x_ideal.
///
/// We compute the term from the last term using this formula:
///
/// `((x - x_ideal) * ln(2) / d)^k / k! == previous_term * (x - x_ideal) * ln(2) / d / k`
///
/// `previous_taylor_term` and result are expressed with accuracy `FTTParam.accuracy`
fn compute_taylor_term(k: u32, previous_taylor_term: &BigUint, p: &FTTParam) -> BigUint {
    let x_minus_x_ideal =
        p.x.clone()
            .sub(&p.x_ideal)
            // NOTE: Should never happen, as x must be more than x_ideal
            .unwrap_or_else(|_| BigUint::zero());

    let res = previous_taylor_term
        .clone()
        .mul(&x_minus_x_ideal)
        .mul(&p.ln2_div_d)
        .div_unit(k);

    // p.accuracy is stripped by definition.
    let res = div_by_stripped(res, &p.accuracy);
    let mut res = div_by_stripped(res, &p.accuracy);

    res.lstrip();
    res
}

/// Compute a div b.
///
/// requires `b` to be stripped and have no leading zeros.
fn div_by_stripped(mut a: BigUint, b: &BigUint) -> BigUint {
    a.lstrip();

    if b.len() == 0 {
        log::error!("Computation error: Invalid division");
        return BigUint::zero();
    }

    if b.len() == 1 {
        return a.div_unit(b.checked_get(0).unwrap_or(1));
    }

    if b.len() > a.len() {
        return BigUint::zero();
    }

    if b.len() == a.len() {
        // 100_000^2 is more than 2^32-1, thus `new_a` has more limbs than `b`.
        let mut new_a = a.mul(&BigUint::from(100_000u64.pow(2)));
        new_a.lstrip();

        debug_assert!(new_a.len() > b.len());
        return new_a
            .div(b, false)
            .map(|res| res.0)
            .unwrap_or_else(BigUint::zero)
            .div_unit(100_000)
            .div_unit(100_000);
    }

    a.div(b, false)
        .map(|res| res.0)
        .unwrap_or_else(BigUint::zero)
}
