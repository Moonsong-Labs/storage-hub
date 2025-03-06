#![cfg_attr(not(feature = "std"), no_std)]

//! Functions to calculate the percentage amount of tokens from the baseline that
//! will be transferred to the treasury from the charged payment streams.

use core::marker::PhantomData;
use shp_traits::{NumericalParam, TreasuryCutCalculator};
use sp_arithmetic::{
    biguint::BigUint,
    traits::{SaturatedConversion, UniqueSaturatedInto, Zero},
    PerThing, Perquintill,
};
use sp_core::Get;

/// A struct that implements the `TreasuryCutCalculator` trait, where the cut is 0%
pub struct NoCutTreasuryCutCalculator<P: NumericalParam, S: NumericalParam + Into<u64>>(
    PhantomData<(P, S)>,
);

impl<P: NumericalParam, S: NumericalParam + Into<u64>> TreasuryCutCalculator
    for NoCutTreasuryCutCalculator<P, S>
{
    type Balance = P;
    type ProvidedUnit = S;
    fn calculate_treasury_cut(
        _provided_amount: Self::ProvidedUnit,
        _used_amount: Self::ProvidedUnit,
        _amount_to_charge: Self::Balance,
    ) -> Self::Balance {
        P::zero()
    }
}

/// A struct that implements the `TreasuryCutCalculator` trait, where the cut is a fixed percentage
pub struct FixedCutTreasuryCutCalculator<T: FixedCutTreasuryCutCalculatorConfig<P>, P: PerThing>(
    PhantomData<(T, P)>,
);

pub trait FixedCutTreasuryCutCalculatorConfig<P: PerThing> {
    /// The numerical type which represents the price of a storage request.
    type Balance: NumericalParam + UniqueSaturatedInto<P::Inner> + From<P::Inner>;
    /// The numerical type used to represent a ProvidedUnit.
    type ProvidedUnit: NumericalParam + Into<u64>;
    /// The fixed cut for the treasury. It's a PerThing since it should be a percentage.
    type TreasuryCut: Get<P>;
}

impl<T, P> TreasuryCutCalculator for FixedCutTreasuryCutCalculator<T, P>
where
    T: FixedCutTreasuryCutCalculatorConfig<P>,
    P: PerThing,
{
    type Balance = T::Balance;
    type ProvidedUnit = T::ProvidedUnit;
    fn calculate_treasury_cut(
        _provided_amount: Self::ProvidedUnit,
        _used_amount: Self::ProvidedUnit,
        amount_to_charge: Self::Balance,
    ) -> Self::Balance {
        let treasury_cut = T::TreasuryCut::get();
        treasury_cut.mul_floor::<Self::Balance>(amount_to_charge)
    }
}

/// A struct that implements the `TreasuryCutCalculator` trait, where the cut is determined by the
/// `compute_adjustment_over_minimum_cut` function.
///
/// The treasury cut is calculated using the following formula:
///
/// ```ignore
/// treasury_cut = minimum_cut + (maximum_cut - minimum_cut) * adjustment
/// ```
///
/// where `adjustment` is calculated using the following formula:
///
/// ```ignore
/// adjustment(x) = {
/// 	for x between 0 and x_ideal: 1 - x / x_ideal,
/// 	for x between x_ideal and 1: 1 - 2^((x_ideal - x) / d)
/// }
/// ```
///
/// where:
/// - `x` is the system utilisation rate, i.e. fraction of the total storage being provided in the system
/// that is currently being used.
/// - `d` is the falloff or `decay_rate`. A co-efficient dictating the strength of the global incentivisation to get the `ideal_system_utilisation`. A higher number results in less
/// typical funds to the treasury at the cost of greater volatility for providers, since the treasury cut will be more sensitive to changes in the system utilisation rate.
/// `d` then should be bigger than 0.01, as if the falloff is smaller than 1%, the treasury cut will get to its maximum value with a really small change in the system utilisation rate over the ideal,
/// as the exponential that calculates the adjustment will be really close to 1.
/// `d` should also be smaller than `(1 - x_ideal) / 3`, as if the falloff is bigger than that, the maximum value of the exponential that calculates the adjustment will be always less than 90% even at 100% utilisation,
/// making it so the treasury cut will max out at barely over 90% of the maximum treasury cut instead of the full 100%.
/// - `x_ideal` is the ideal system utilisation rate.
///
/// The parameters utilized in the calculation are provided by the configuration trait `LinearThenPowerOfTwoTreasuryCutCalculatorConfig`.
pub struct LinearThenPowerOfTwoTreasuryCutCalculator<
    T: LinearThenPowerOfTwoTreasuryCutCalculatorConfig<P>,
    P: PerThing,
>(PhantomData<(T, P)>);

/// The configuration trait for the [`LinearThenPowerOfTwoTreasuryCutCalculator`].
pub trait LinearThenPowerOfTwoTreasuryCutCalculatorConfig<P: PerThing> {
    /// The numerical type which represents the price of a storage request.
    type Balance: NumericalParam + UniqueSaturatedInto<P::Inner> + From<P::Inner>;
    /// The numerical type used to represent a ProvidedUnit.
    type ProvidedUnit: NumericalParam + Into<u64>;
    /// The ideal system utilisation rate. It's a PerThing since it should be a percentage.
    type IdealUtilisationRate: Get<P>;
    /// The falloff or decay rate. It's a PerThing since it should be a percentage, and for
    /// the calculation to work, it should be more than 0.01.
    type DecayRate: Get<P>;
    /// The minimum cut for the treasury. It's a PerThing since it should be a percentage.
    type MinimumCut: Get<P>;
    /// The maximum cut for the treasury. It's a PerThing since it should be a percentage.
    type MaximumCut: Get<P>;
}

impl<T, P> TreasuryCutCalculator for LinearThenPowerOfTwoTreasuryCutCalculator<T, P>
where
    T: LinearThenPowerOfTwoTreasuryCutCalculatorConfig<P>,
    P: PerThing,
{
    type Balance = T::Balance;
    type ProvidedUnit = T::ProvidedUnit;
    fn calculate_treasury_cut(
        provided_amount: Self::ProvidedUnit,
        used_amount: Self::ProvidedUnit,
        amount_to_charge: Self::Balance,
    ) -> Self::Balance {
        // Get the system utilisation rate, ideal system utilisation rate and falloff from the configuration.
        let system_utilisation =
            P::from_rational(used_amount.into().into(), provided_amount.into().into());
        let ideal_system_utilisation = T::IdealUtilisationRate::get();
        let falloff = T::DecayRate::get();

        // Calculate the adjustment to be used to calculate the treasury cut.
        let adjustment = compute_adjustment_over_minimum_cut(
            system_utilisation,
            ideal_system_utilisation,
            falloff,
        );

        // Get the minimum and maximum cut from the configuration and calculate the difference between them.
        let minimum_cut = T::MinimumCut::get();
        let maximum_cut = T::MaximumCut::get();
        let delta_cut = maximum_cut.saturating_sub(minimum_cut);

        // Calculate the final treasury cut percentage by adjusting the minimum cut with the adjustment times the delta cut.
        let treasury_cut = minimum_cut.saturating_add(delta_cut * adjustment);

        // Calculate the amount to transfer to the treasury by using the equation `amount_to_charge * treasury_cut`,
        // where `treasury_cut` is the percentage of funds that should go to the treasury. `mul_floor` is used to
        // round down the result to the nearest integer.
        treasury_cut.mul_floor::<Self::Balance>(amount_to_charge)
    }
}

/// Compute the adjustment percentage of the treasury cut, which is the percentage of the delta between the minimum
/// and maximum treasury cut that should be added to the minimum cut. The adjustment is calculated using the following formula:
///
/// ```ignore
/// I(x) = for x between 0 and x_ideal: 1 - x / x_ideal,
/// for x between x_ideal and 1: 1 - 2^((x_ideal - x) / d)
/// ```
///
/// where:
/// - x is the system utilisation rate, i.e. fraction of the total storage being provided in the system
/// that is currently being used.
/// - d is the falloff or `decay_rate`
/// - x_ideal: the ideal system utilisation rate.
///
/// The result is meant to be scaled with the minimum percentage amount and maximum percentage amount
///  that goes to the treasury.
///
/// Arguments are:
/// - `system_utilisation`: The fraction of the total storage being provided in the system that is
/// currently being used. Known as `x` in the literature. Must be between 0 and 1.
/// - `ideal_system_utilisation`: The fraction of total storage being provided in the system that should
/// be being actively used. Known as `x_ideal` in the literature. Must be between 0 and 1.
/// - `falloff`: Known as `decay_rate` in the literature. A co-efficient dictating the strength of
///   the global incentivisation to get the `ideal_system_utilisation`. A higher number results in less typical
///   funds to the treasury at the cost of greater volatility for providers. Must be more than 0.01.
///
/// The calculations done here are heavily based on Polkadot's inflation model, which can be found in [Polkadot's
/// documentation](https://wiki.polkadot.network/docs/learn-inflation).
pub fn compute_adjustment_over_minimum_cut<P: PerThing>(
    system_utilisation: P,
    ideal_system_utilisation: P,
    falloff: P,
) -> P {
    // If the system utilisation is less than the ideal system utilisation, we return the result of the
    // formula `1 - x / x_ideal` for the adjustment.
    if system_utilisation < ideal_system_utilisation {
        // ideal_system_utilisation is more than 0 because it is strictly more than system_utilisation
        return (system_utilisation / ideal_system_utilisation).left_from_one();
    }

    // Else, if the system utilisation is equal to the ideal system utilisation, we return no adjustment.
    if system_utilisation == ideal_system_utilisation {
        return P::zero();
    }

    // Else, if the system utilisation is greater than the ideal system utilisation, we return the result of the
    // formula `1 - 2^((x_ideal - x) / d)` for the adjustment.
    // To do this, we first make sure that the falloff is more than the minimum acceptable value of 1%.
    if falloff < P::from_percent(1.into()) {
        log::error!("Invalid treasury cut calculation: falloff less than 1% is not supported");
        return PerThing::zero();
    }
    // Then we get the accuracy of the PerThing type currently being used.
    let accuracy = {
        let mut a = BigUint::from(Into::<u128>::into(P::ACCURACY));
        a.lstrip();
        a
    };

    // We get the `falloff` as a BigUint, and strip it to remove leading zeros.
    let mut falloff = BigUint::from(falloff.deconstruct().into());
    falloff.lstrip();

    // We get the logarithm of 2 as precisely as possible according to our PerThing accuracy,
    // which we'll need for the calculation of the approximation of the adjustment.
    let ln2 = {
        /// `ln(2)` expressed in as perquintillionth.
        const LN2: u64 = 0_693_147_180_559_945_309;
        let ln2 = P::from_rational(LN2.into(), Perquintill::ACCURACY.into());
        BigUint::from(ln2.deconstruct().into())
    };

    // Since we already stripped the falloff, we can now divide `ln2 * accuracy` by `falloff` to get `ln2 / d`.
    let ln2_div_d = div_by_stripped(ln2.mul(&accuracy), &falloff);

    // We set up the parameters to calculate the approximation of the adjustment.
    let ftt_param = FTTParam {
        x_ideal: BigUint::from(ideal_system_utilisation.deconstruct().into()),
        x: BigUint::from(system_utilisation.deconstruct().into()),
        accuracy,
        ln2_div_d,
    };

    // We compute the taylor series approximation of the adjustment.
    let res = compute_taylor_series_part(&ftt_param);

    // We convert the result to a PerThing and return 1 minus the result.
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

/// Internal struct holding parameter info alongside other cached values.
/// FTT means Funds To Treasury.
///
/// All expressed in part from `accuracy`
struct FTTParam {
    ln2_div_d: BigUint,
    x_ideal: BigUint,
    x: BigUint,
    /// Must be stripped and have no leading zeros.
    accuracy: BigUint,
}

/// Compute `2^((x_ideal - x) / d)` using a taylor series.
///
/// x must be strictly more than x_ideal.
///
/// result is expressed with accuracy `FTTParam.accuracy`
fn compute_taylor_series_part(p: &FTTParam) -> BigUint {
    // The last computed taylor term.
    let mut last_taylor_term = p.accuracy.clone();

    // Whereas the taylor sum is positive.
    let mut taylor_sum_positive = true;

    // The sum of all taylor terms.
    let mut taylor_sum = last_taylor_term.clone();

    // Iterate through the taylor series, computing the terms and adding them to the sum.
    // Note: the amount of iterations is currently hardcoded but could be made dynamic to
    // increase precision if needed.
    for k in 1..300 {
        // Compute the k-th taylor term.
        last_taylor_term = compute_taylor_term(k, &last_taylor_term, p);

        // If the last term is zero, break out of the loop.
        if last_taylor_term.is_zero() {
            break;
        }

        let last_taylor_term_positive = k % 2 == 0;

        // If the last term is positive and the sum is positive, add the term to the sum.
        if taylor_sum_positive == last_taylor_term_positive {
            taylor_sum = taylor_sum.add(&last_taylor_term);
        } else if taylor_sum >= last_taylor_term {
            // Else, if the sum is greater than the term, subtract the term from the sum.
            taylor_sum = taylor_sum
                .sub(&last_taylor_term)
                // NOTE: Should never happen as checked above
                .unwrap_or_else(|e| e);
        } else {
            // Else, if the sum is less than the term, subtract the sum from the term and change the sign of the sum.
            taylor_sum_positive = !taylor_sum_positive;
            taylor_sum = last_taylor_term
                .clone()
                .sub(&taylor_sum)
                // NOTE: Should never happen as checked above
                .unwrap_or_else(|e| e);
        }
    }

    // If the sum is negative, return 0.
    if !taylor_sum_positive {
        return BigUint::zero();
    }

    // Else, return the sum stripped of leading zeros.
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
    // Get the difference between x and x_ideal.
    let x_minus_x_ideal =
        p.x.clone()
            .sub(&p.x_ideal)
            // NOTE: Should never happen, as x must be more than x_ideal
            .unwrap_or_else(|_| BigUint::zero());

    // Compute the k-th taylor term using the (k-1)-th term.
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

    // If b is a single limb, use the `div_unit` function.
    if b.len() == 1 {
        return a.div_unit(b.checked_get(0).unwrap_or(1));
    }

    // If a is less than b, return 0 (integer division).
    if b.len() > a.len() {
        return BigUint::zero();
    }

    // If they have the same number of limbs, use the `div` function but add an extra limb to `a` first,
    // since `div` requires `a` to have more limbs than `b`. Remove the extra limb from the result.
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

    // If `a` has more limbs than `b`, use the `div` function.
    a.div(b, false)
        .map(|res| res.0)
        .unwrap_or_else(BigUint::zero)
}
