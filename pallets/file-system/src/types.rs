use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::BoundedVec;
use frame_system::pallet_prelude::BlockNumberFor;
use scale_info::TypeInfo;

use crate::Config;

/// Metadata for a storage request.
///
/// This is used to track the status of a storage request
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct StorageRequestMetadata<T: Config> {
    /// Block number at which the storage request was made.
    ///
    /// Used primarily for tracking the age of the request which is useful for
    /// cleaning up old requests.
    pub requested_at: BlockNumberFor<T>,
    /// Identifier of the data being stored.
    pub fingerprint: Fingerprint<T>,
    /// Size of the data being stored.
    ///
    /// SPs will use this to determine if they have enough space to store the data.
    /// This is also used to verify that the data sent by the user matches the size specified here.
    pub size: StorageUnit<T>,
    /// Multiaddress of the user who requested the storage.
    ///
    /// SPs will expect a connection request to be initiated by the user with this multiaddress.
    pub user_multiaddr: MultiAddress<T>,
    /// List of BSPs that have volunteered to store the data.
    pub bsps_volunteered: BoundedVec<StorageProviderId<T>, MaxBsps<T>>,
    /// List of BSPs that have proven they are storing the data.
    ///
    /// The storage request will be dropped/complete once all the minimum required BSPs have
    /// submitted a proof of storage after volunteering to store the data.
    pub bsps_confirmed: BoundedVec<StorageProviderId<T>, MaxBsps<T>>,
    /// Overwrite data if it already exists.
    ///
    /// SPs should overwrite any data at the given location if this is set to `true`.
    pub overwrite: bool,
}

/// Alias for the `AccoundId` type used in the FileSystem pallet.
pub type StorageProviderId<T> = <T as frame_system::Config>::AccountId;

/// Alias for the `MaxBsps` type used in the FileSystem pallet.
pub type MaxBsps<T> = <T as crate::Config>::MaxBspsPerStorageRequest;

/// Alias for the `MaxFilePathSize` type used in the FileSystem pallet.
pub type MaxFilePathSize<T> = <T as crate::Config>::MaxFilePathSize;

/// Alias for the `MaxMultiAddressSize` type used in the FileSystem pallet.
pub type MaxMultiAddressSize<T> = <T as crate::Config>::MaxMultiAddressSize;

/// Alias for the `Fingerprint` type used in the FileSystem pallet.
pub type Fingerprint<T> = <T as crate::Config>::Fingerprint;

/// Alias for the `StorageCount` type used in the FileSystem pallet.
pub type StorageUnit<T> = <T as crate::Config>::StorageUnit;

/// Byte array representing the file path.
pub type FileLocation<T> = BoundedVec<u8, MaxFilePathSize<T>>;

/// Byte array representing the libp2p multiaddress.
pub type MultiAddress<T> = BoundedVec<u8, MaxMultiAddressSize<T>>;
