#![cfg_attr(not(feature = "std"), no_std)]

use core::marker::PhantomData;

use shp_traits::{NumericalParam, UpdateStoragePrice};

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
/// The price is only changed if the system utilisation is getting close to 0% or 100%.
/// This struct offers configurable parameters for how close to 0% or 100% the system utilisation needs to be
/// before the price is updated.
pub struct MostlyStablePriceIndexUpdater<const LOWER_THRESHOLD: u32, const UPPER_THRESHOLD: u32>;
