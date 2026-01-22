//! Contains the various runtime APIs that the backend accesses
//!
//! This module provides a type-safe way to interact with runtime APIs through the
//! `RuntimeApiCalls` enum, which encodes the API method names, parameter types, and
//! return types.
//!
//! TODO: consider refactoring this to use subxt or similar
//! and auto-import/generate the bindings from the runtime

use codec::Codec;

use crate::runtime::{Balance, ProviderId};

// Type Aliases for Runtime API Types
pub type CurrentPrice = Balance;

/// Enumeration of all runtime API calls that the backend can make.
///
/// Each variant represents a specific runtime API method and carries
/// phantom data for its parameter and return types, allowing for
/// compile-time type safety.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeApiCalls {
    /// Get the current price per giga unit per tick from the PaymentStreams pallet.
    ///
    /// - **Parameters**: `()` (none)
    /// - **Returns**: `u128`
    GetCurrentPricePerGigaUnitPerTick,

    /// Get the count of users with an active payment stream for a given provider.
    ///
    /// - **Parameters**: `H256` (provider ID as H256)
    /// - **Returns**: `u32`
    GetNumberOfActiveUsersOfProvider,
}

impl RuntimeApiCalls {
    /// Returns the runtime API method name as a string.
    ///
    /// The method name follows the format: `<TraitName>_<method_name>`.
    pub const fn method_name(&self) -> &'static str {
        match self {
            Self::GetCurrentPricePerGigaUnitPerTick => {
                "PaymentStreamsApi_get_current_price_per_giga_unit_per_tick"
            }
            Self::GetNumberOfActiveUsersOfProvider => {
                "PaymentStreamsApi_get_number_of_active_users_of_provider"
            }
        }
    }
}

/// Trait to associate parameter and return types with each runtime API call.
///
/// This trait allows you to access the concrete types for parameters and return
/// values at compile time.
pub trait RuntimeApiCallTypes {
    /// The type of parameters expected by this API call.
    type Params: Codec;

    /// The type of the return value from this API call.
    type ReturnType: Codec;

    /// Returns the runtime API call variant associated with this type.
    fn runtime_api_call() -> RuntimeApiCalls;
}

pub struct GetCurrentPricePerGigaUnitPerTick;
impl RuntimeApiCallTypes for GetCurrentPricePerGigaUnitPerTick {
    type Params = ();
    type ReturnType = CurrentPrice;

    fn runtime_api_call() -> RuntimeApiCalls {
        RuntimeApiCalls::GetCurrentPricePerGigaUnitPerTick
    }
}

pub struct GetNumberOfActiveUsersOfProvider;
impl RuntimeApiCallTypes for GetNumberOfActiveUsersOfProvider {
    type Params = ProviderId;
    type ReturnType = u32;

    fn runtime_api_call() -> RuntimeApiCalls {
        RuntimeApiCalls::GetNumberOfActiveUsersOfProvider
    }
}
