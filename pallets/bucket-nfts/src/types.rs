use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::{traits::StaticLookup, BoundedVec};

use crate::Config;

#[derive(MaxEncodedLen, TypeInfo, Encode, Decode, PartialEq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct ItemMetadata<T: Config> {
    pub read_access_regex: Option<ReadAccessRegex<T>>,
}

impl<T: Config> ItemMetadata<T> {
    pub fn new(read_access_regex: Option<ReadAccessRegex<T>>) -> Self {
        Self { read_access_regex }
    }
}

/// Implement Debug for Proof. Cannot derive Debug directly because of compiler issues
/// with the generic type.
impl<T: Config> core::fmt::Debug for ItemMetadata<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "ItemMetadata {{ read_access_regex: {:?} }}",
            self.read_access_regex
        )
    }
}

/// Type alias representing the type of `BucketId` used in `ProvidersInterface`.
pub(crate) type BucketIdFor<T> =
    <<T as crate::Config>::Providers as storage_hub_traits::ProvidersConfig>::BucketId;

/// Type alias for the account ID source type.
pub(crate) type AccountIdLookupSourceOf<T> =
    <<T as frame_system::Config>::Lookup as StaticLookup>::Source;

/// Type alias for the account ID target type.
pub(crate) type AccountIdLookupTargetOf<T> =
    <<T as frame_system::Config>::Lookup as StaticLookup>::Target;

/// Type alias for the string limit of a read access regex.
pub(crate) type ReadAccessRegex<T> = BoundedVec<u8, <T as pallet_nfts::Config>::StringLimit>;

#[cfg(test)]
/// Type alias for the `ProviderId` type used in `ProvidersInterface`.
pub(crate) type ProviderIdFor<T> =
    <<T as crate::Config>::Providers as storage_hub_traits::ProvidersInterface>::Provider;