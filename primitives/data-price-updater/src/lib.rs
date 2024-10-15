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
///     price = stable_price - e ^ (LowerThreshold - system_utilisation) * LowerExponentFactor
///     price = max(price, MinPrice)
/// else if system_utilisation > UpperThreshold:
///     price = stable_price + e ^ (system_utilisation - UpperThreshold) * UpperExponentFactor
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
    /// The factor that multiplies `e ^ (sys_util - UpperThreshold)` for calculating the price when the
    /// system utilisation is above `UpperThreshold`.
    type UpperExponentFactor: Get<u32>;
    /// The factor that multiplies `e ^ (LowerThreshold - sys_util)` for calculating the price when the
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
            let x = T::LowerThreshold::get() - system_utilisation;

            // Approximate the exponential function using the second-degree Taylor series.
            let exp_taylor_2 = Self::exp_approx_taylor_2(x);

            // Calculate the price based on the formula:
            // `price = stable_price - e ^ (LowerThreshold - system_utilisation) * LowerExponentFactor`
            let lower_exponent_factor = T::LowerExponentFactor::get();
            let addition_term =
                exp_taylor_2.saturating_mul(FixedU128::from_u32(lower_exponent_factor));
            // Round the addition term and downcast it to a uint type like `u128`.
            let addition_term = addition_term
                .round()
                .into_inner()
                .saturating_div(FixedU128::DIV);
            let calculated_price = stable_price.saturating_sub(addition_term.into());

            // Saturate the price to the minimum price.
            max(calculated_price, T::MinPrice::get())
        } else if system_utilisation > T::UpperThreshold::get() {
            // Calculate our `x` for the exponential function approximation.
            // Using [`Perbill`] ensures that `x` is between 0 and 1.
            let x = system_utilisation - T::UpperThreshold::get();

            // Approximate the exponential function using the second-degree Taylor series.
            let exp_taylor_2 = Self::exp_approx_taylor_2(x);

            // Calculate the price based on the formula:
            // `price = stable_price + e ^ (system_utilisation - UpperThreshold) * UpperExponentFactor`
            let upper_exponent_factor = T::UpperExponentFactor::get();
            let addition_term =
                exp_taylor_2.saturating_mul(FixedU128::from_u32(upper_exponent_factor));
            // Round the addition term and downcast it to a uint type like `u128`.
            let addition_term = addition_term
                .round()
                .into_inner()
                .saturating_div(FixedU128::DIV);
            let calculated_price = stable_price.saturating_add(addition_term.into());

            // Saturate the price to the maximum price.
            min(calculated_price, T::MaxPrice::get())
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

#[cfg(test)]
mod tests {
    use sp_core::{ConstU128, ConstU32};

    use super::*;

    struct LowerThreshold;
    impl Get<Perbill> for LowerThreshold {
        fn get() -> Perbill {
            Perbill::from_percent(30)
        }
    }

    struct UpperThreshold;
    impl Get<Perbill> for UpperThreshold {
        fn get() -> Perbill {
            Perbill::from_percent(95)
        }
    }

    // Mock implementation of MostlyStablePriceIndexUpdaterConfig
    struct MockConfig;
    impl MostlyStablePriceIndexUpdaterConfig for MockConfig {
        type Price = u128;
        type StorageDataUnit = u64;
        type MostlyStablePrice = ConstU128<50>;
        type LowerThreshold = LowerThreshold; // 30%
        type UpperThreshold = UpperThreshold; // 95%
        type MaxPrice = ConstU128<469>; // 50 + 400 * e ^ ( 1 - 0.95 ) ≈ 470, we set this to be slightly lower, to saturate.
        type MinPrice = ConstU128<10>; // 50 - 30 * e ^ ( 0.3 - 0 ) ≈ 9.5, we set this to be slightly higher, to saturate.
        type UpperExponentFactor = ConstU32<400>;
        type LowerExponentFactor = ConstU32<30>;
    }

    type TestPriceUpdater = MostlyStablePriceIndexUpdater<MockConfig>;

    #[test]
    fn test_stable_price_region() {
        let current_price = 50u128;
        let used_capacity = 6000u64;
        let total_capacity = 10000u64;

        let new_price =
            TestPriceUpdater::update_storage_price(current_price, used_capacity, total_capacity);
        assert_eq!(
            new_price,
            <MockConfig as MostlyStablePriceIndexUpdaterConfig>::MostlyStablePrice::get(),
            "Price should remain stable in the middle region"
        );
    }

    #[test]
    fn test_upper_threshold() {
        let current_price = 50u128;
        let used_capacity = 9600u64;
        let total_capacity = 10000u64;

        let new_price =
            TestPriceUpdater::update_storage_price(current_price, used_capacity, total_capacity);
        assert!(
            new_price
                > <MockConfig as MostlyStablePriceIndexUpdaterConfig>::MostlyStablePrice::get(),
            "Price should increase above upper threshold"
        );
        assert!(new_price <= 500u128, "Price should not exceed max price");
    }

    #[test]
    fn test_lower_threshold() {
        let current_price = 50u128;
        let used_capacity = 2900u64;
        let total_capacity = 10000u64;

        let new_price =
            TestPriceUpdater::update_storage_price(current_price, used_capacity, total_capacity);
        assert!(
            new_price
                < <MockConfig as MostlyStablePriceIndexUpdaterConfig>::MostlyStablePrice::get(),
            "Price should decrease below lower threshold"
        );
        assert!(new_price >= 5u128, "Price should not go below min price");
    }

    #[test]
    fn test_zero_utilization() {
        let current_price = 50u128;
        let used_capacity = 0u64;
        let total_capacity = 10000u64;

        let new_price =
            TestPriceUpdater::update_storage_price(current_price, used_capacity, total_capacity);
        assert_eq!(
            new_price,
            <MockConfig as MostlyStablePriceIndexUpdaterConfig>::MinPrice::get(),
            "Price should be at minimum for zero utilization"
        );
    }

    #[test]
    fn test_full_utilization() {
        let current_price = 50u128;
        let used_capacity = 10000u64;
        let total_capacity = 10000u64;

        let new_price =
            TestPriceUpdater::update_storage_price(current_price, used_capacity, total_capacity);
        assert_eq!(
            new_price,
            <MockConfig as MostlyStablePriceIndexUpdaterConfig>::MaxPrice::get(),
            "Price should be at maximum for full utilization"
        );
    }

    #[test]
    fn test_just_below_upper_threshold() {
        let current_price = 50u128;
        let used_capacity = 9499u64;
        let total_capacity = 10000u64;

        let new_price =
            TestPriceUpdater::update_storage_price(current_price, used_capacity, total_capacity);
        assert_eq!(
            new_price,
            <MockConfig as MostlyStablePriceIndexUpdaterConfig>::MostlyStablePrice::get(),
            "Price should remain stable just below upper threshold"
        );
    }

    #[test]
    fn test_just_above_lower_threshold() {
        let current_price = 50u128;
        let used_capacity = 3001u64;
        let total_capacity = 10000u64;

        let new_price =
            TestPriceUpdater::update_storage_price(current_price, used_capacity, total_capacity);
        assert_eq!(
            new_price,
            <MockConfig as MostlyStablePriceIndexUpdaterConfig>::MostlyStablePrice::get(),
            "Price should remain stable just above lower threshold"
        );
    }

    #[test]
    fn test_exp_approx_taylor_2() {
        // Expected value: e^0.1 ≈ 1.1051709
        let epsilon = FixedU128::from_inner(1_000_000_000_000_000); // Small value for comparison (0.001)
        let x = Perbill::from_percent(10);
        let result = TestPriceUpdater::exp_approx_taylor_2(x);
        let expected = FixedU128::from_inner(1_105_170_900_000_000_000);
        let error = if expected > result {
            expected - result
        } else {
            result - expected
        };

        assert!(
            error < epsilon,
            "Approximation should be close to actual value"
        );

        // Expected value: e^0.3 ≈ 1.3498588
        let epsilon = FixedU128::from_inner(5_000_000_000_000_000); // Small value for comparison (0.005), here the approximation is worse
        let x = Perbill::from_percent(30);
        let result = TestPriceUpdater::exp_approx_taylor_2(x);
        let expected = FixedU128::from_inner(1_349_858_800_000_000_000);
        let error = if expected > result {
            expected - result
        } else {
            result - expected
        };

        assert!(
            error < epsilon,
            "Approximation should be close to actual value"
        );

        // Expected value: e^0.5 ≈ 1.64872127
        let epsilon = FixedU128::from_inner(30_000_000_000_000_000); // Small value for comparison (0.03), here the approximation is even worse
        let x = Perbill::from_percent(50);
        let result = TestPriceUpdater::exp_approx_taylor_2(x);
        let expected = FixedU128::from_inner(1_648_721_270_000_000_000);
        let error = if expected > result {
            expected - result
        } else {
            result - expected
        };

        assert!(
            error < epsilon,
            "Approximation should be close to actual value"
        );
    }

    #[test]
    fn test_price_increase_rate() {
        let current_price = 50u128;
        let used_capacity_1 = 9600u64;
        let used_capacity_2 = 9800u64;
        let total_capacity = 10000u64;

        let new_price_1 =
            TestPriceUpdater::update_storage_price(current_price, used_capacity_1, total_capacity);
        let new_price_2 =
            TestPriceUpdater::update_storage_price(current_price, used_capacity_2, total_capacity);

        assert!(
            new_price_2 >= new_price_1,
            "Price should increase more as utilization increases"
        );

        let used_capacity_1 = 9900u64;
        let used_capacity_2 = 10000u64;

        let new_price_1 =
            TestPriceUpdater::update_storage_price(current_price, used_capacity_1, total_capacity);
        let new_price_2 =
            TestPriceUpdater::update_storage_price(current_price, used_capacity_2, total_capacity);

        assert!(
            new_price_2 >= new_price_1,
            "Price should increase more as utilization increases"
        );
    }

    #[test]
    fn test_price_decrease_rate() {
        let current_price = 50u128;
        let used_capacity_1 = 2900u64;
        let used_capacity_2 = 2700u64;
        let total_capacity = 10000u64;

        let new_price_1 =
            TestPriceUpdater::update_storage_price(current_price, used_capacity_1, total_capacity);
        let new_price_2 =
            TestPriceUpdater::update_storage_price(current_price, used_capacity_2, total_capacity);

        assert!(
            new_price_2 <= new_price_1,
            "Price should decrease more as utilization decreases"
        );

        let used_capacity_1 = 2000u64;
        let used_capacity_2 = 1500u64;

        let new_price_1 =
            TestPriceUpdater::update_storage_price(current_price, used_capacity_1, total_capacity);
        let new_price_2 =
            TestPriceUpdater::update_storage_price(current_price, used_capacity_2, total_capacity);

        assert!(
            new_price_2 <= new_price_1,
            "Price should decrease more as utilization decreases"
        );

        let used_capacity_1 = 10000u64;
        let used_capacity_2 = 1000u64;

        let new_price_1 =
            TestPriceUpdater::update_storage_price(current_price, used_capacity_1, total_capacity);
        let new_price_2 =
            TestPriceUpdater::update_storage_price(current_price, used_capacity_2, total_capacity);

        assert!(
            new_price_2 <= new_price_1,
            "Price should decrease more as utilization decreases"
        );
    }
}
