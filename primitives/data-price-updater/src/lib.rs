#![cfg_attr(not(feature = "std"), no_std)]

use core::{
    cmp::{max, min},
    marker::PhantomData,
};

use shp_traits::{NumericalParam, UpdateStoragePrice};
use sp_core::Get;
use sp_runtime::{
    traits::{One, Saturating},
    FixedPointNumber, FixedU128, Perbill,
};

/// A struct that implements the `UpdateStoragePrice` trait, where the price is not updated.
///
/// The current price is returned as is.
pub struct NoUpdatePriceIndexUpdater<P: NumericalParam, S: NumericalParam>(PhantomData<(P, S)>);

impl<P: NumericalParam, S: NumericalParam> UpdateStoragePrice for NoUpdatePriceIndexUpdater<P, S> {
    type Price = P;
    type StorageDataUnit = S;

    fn update_storage_price(
        current_price: Self::Price,
        _used_capacity: Self::StorageDataUnit,
        _total_capacity: Self::StorageDataUnit,
    ) -> Self::Price {
        current_price
    }
}

/// A struct that implements the `UpdateStoragePrice` trait, where the price is updated based on the
/// system utilisation, but keeps a mostly stable price.
///
/// This struct is configured through a type that implements the [`MostlyStablePriceIndexUpdaterConfig`] trait.
///
/// The price is only changed if the system utilisation is getting close to 0% or 100%.
/// This struct offers configurable parameters for how close to 0% or 100% the system utilisation needs to be
/// before the price is updated.
/// If the system utilisation is above the upper threshold, the price increases exponentially, saturating
/// to the maximum price.
/// If the system utilisation is below the lower threshold, the price decreases exponentially, saturating
/// to the minimum price.
///
/// The price is updated based on the following formula:
///
/// ```ignore
/// if system_utilisation < LowerThreshold:
///     price = stable_price - e ^ (system_utilisation) * LowerExponentFactor
///     price = max(price, MinPrice)
/// else if system_utilisation > UpperThreshold:
///     price = stable_price + e ^ (1 - system_utilisation) * UpperExponentFactor
///     price = min(price, MaxPrice)
/// else:
///     price = stable_price
/// ```
///
/// The exponential functions (like `e ^ (system_utilisation)`) are approximated using a second-order
/// Taylor series, which holds:
///
/// ```ignore
/// e^x ≈ 1 + x + x^2 / 2
/// ```
///
/// As long as `|x| < 0.5`. In our case, `x` would be `1 - system_utilisation` or just `system_utilisation`.
/// Given that `system_utilisation` is between 0 and 1, and considering that we only use the formulas when
/// `system_utilisation` is below `LowerThreshold` or `UpperThreshold`, we can safely assume that `x` is
/// indeed within the bounds of `0 < x < 0.5`.
pub struct MostlyStablePriceIndexUpdater<T: MostlyStablePriceIndexUpdaterConfig>(PhantomData<T>);

/// The configuration trait for the [`MostlyStablePriceIndexUpdater`].
pub trait MostlyStablePriceIndexUpdaterConfig {
    /// The numerical type used to represent the price.
    type Price: NumericalParam + From<u128>;
    /// The numerical type used to represent a storage data unit.
    type StorageDataUnit: NumericalParam;
    /// The price that is used when the system utilisation is between `LowerThreshold` and `UpperThreshold`.
    type MostlyStablePrice: Get<Self::Price>;
    /// With a system utilisation below this threshold, the price decreases exponentially.
    ///
    /// It's a [`Perbill`] because we want it between 0 and 1.
    type LowerThreshold: Get<Perbill>;
    /// With a system utilisation above this threshold, the price increases exponentially.
    ///
    /// It's a [`Perbill`] because we want it between 0 and 1.
    type UpperThreshold: Get<Perbill>;
    /// The maximum price that can be set.
    ///
    /// Even with system utilisation above `UpperThreshold`, the price saturates to this value.
    type MaxPrice: Get<Self::Price>;
    /// The minimum price that can be set.
    ///
    /// Even with system utilisation below `LowerThreshold`, the price saturates to this value.
    type MinPrice: Get<Self::Price>;
    /// The factor that multiplies `e ^ (1 - sys_util)` for calculating the price when the
    /// system utilisation is above `UpperThreshold`.
    type UpperExponentFactor: Get<u32>;
    /// The factor that multiplies `e ^ (sys_util - 1)` for calculating the price when the
    /// system utilisation is below `LowerThreshold`.
    type LowerExponentFactor: Get<u32>;
}

impl<T> UpdateStoragePrice for MostlyStablePriceIndexUpdater<T>
where
    T: MostlyStablePriceIndexUpdaterConfig,
{
    type Price = <T as MostlyStablePriceIndexUpdaterConfig>::Price;
    type StorageDataUnit = <T as MostlyStablePriceIndexUpdaterConfig>::StorageDataUnit;

    fn update_storage_price(
        _current_price: Self::Price,
        used_capacity: Self::StorageDataUnit,
        total_capacity: Self::StorageDataUnit,
    ) -> Self::Price {
        let system_utilisation = Perbill::from_rational(used_capacity, total_capacity);
        let stable_price = T::MostlyStablePrice::get();

        if system_utilisation < T::LowerThreshold::get() {
            // Calculate our `x` for the exponential function approximation.
            // Using [`Perbill`] ensures that `x` is between 0 and 1.
            let x = system_utilisation;

            // Approximate the exponential function using the second-degree Taylor series.
            let exp_taylor_2 = Self::exp_approx_taylor_2(x);

            // Calculate the price based on the formula:
            // `price = stable_price - e ^ (system_utilisation) * LowerExponentFactor`
            let lower_exponent_factor = T::LowerExponentFactor::get();
            let addition_term = exp_taylor_2
                .saturating_mul(FixedU128::from_u32(lower_exponent_factor))
                .into_inner()
                .saturating_div(FixedU128::DIV);
            let calculated_price = stable_price.saturating_add(addition_term.into());

            // Saturate the price to the minimum price.
            min(calculated_price, T::MinPrice::get())
        } else if system_utilisation > T::UpperThreshold::get() {
            // Calculate our `x` for the exponential function approximation.
            // Using [`Perbill`] ensures that `x` is between 0 and 1.
            let x = Perbill::one() - system_utilisation;

            // Approximate the exponential function using the second-degree Taylor series.
            let exp_taylor_2 = Self::exp_approx_taylor_2(x);

            // Calculate the price based on the formula:
            // `price = stable_price + e ^ (1 - system_utilisation) * UpperExponentFactor`
            let upper_exponent_factor = T::UpperExponentFactor::get();
            let addition_term = exp_taylor_2
                .saturating_mul(FixedU128::from_u32(upper_exponent_factor))
                .into_inner()
                .saturating_div(FixedU128::DIV);
            let calculated_price = stable_price.saturating_add(addition_term.into());

            // Saturate the price to the maximum price.
            max(calculated_price, T::MaxPrice::get())
        } else {
            return stable_price;
        }
    }
}

impl<T> MostlyStablePriceIndexUpdater<T>
where
    T: MostlyStablePriceIndexUpdaterConfig,
{
    fn exp_approx_taylor_2(x: Perbill) -> FixedU128 {
        // Calculate the quadratic term of the Taylor series (i.e. the `x^2/2` term).
        // Given that `x` is between 0 and 1, we can safely assume that `x^2/2` is also
        // between 0 and 1. Which is why using Perbill is safe. We convert to FixedU128
        // in the end, for the next steps of the approximation.
        let degree_2_term: FixedU128 = Perbill::from_rational(1u32, 2u32)
            .saturating_mul(x)
            .saturating_mul(x)
            .into();

        // Calculate the second-degreeTaylor series approximation of `e^x`
        // (i.e. `1 + x + x^2/2`).
        let exp_taylor_2 = degree_2_term
            .saturating_add(x.into())
            .saturating_add(One::one());

        exp_taylor_2
    }
}
