use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::BoundedVec;
use frame_system::pallet_prelude::BlockNumberFor;
use scale_info::TypeInfo;

use crate::Config;

/// Ephemeral metadata of a storage request.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct StorageRequestMetadata<T: Config> {
    // TODO: Add MSP
    /// Block number at which the storage request was made.
    ///
    /// Used primarily for tracking the age of the request which is useful for
    /// cleaning up old requests.
    pub requested_at: BlockNumberFor<T>,
    /// AccountId of the user who owns the data being stored.
    pub owner: T::AccountId,
    /// Identifier of the data being stored.
    pub fingerprint: Fingerprint<T>,
    /// Size of the data being stored.
    ///
    /// SPs will use this to determine if they have enough space to store the data.
    /// This is also used to verify that the data sent by the user matches the size specified here.
    pub size: StorageData<T>,
    /// Multiaddress of the user who requested the storage.
    ///
    /// SPs will expect a connection request to be initiated by the user with this multiaddress.
    pub user_multiaddresses: BoundedVec<MultiAddress<T>, MaxMultiAddresses<T>>,
    /// List of storage providers that can serve the data that is requested to be stored.
    ///
    /// This is useful when a BSP stops serving data and automatically creates a new storage request with no user multiaddresses, since
    /// SPs can prove and serve the data to be replicated to other BSPs without the user having this stored on their local machine.
    pub data_server_sps: BoundedVec<T::AccountId, MaxBspsPerStorageRequest<T>>, // TODO: Change the Maximum data servers to be the maximum SPs allowed
    /// Number of BSPs requested to store the data.
    ///
    ///
    /// The storage request will be dropped/complete once all the minimum required BSPs have
    /// submitted a proof of storage after volunteering to store the data.
    pub bsps_required: T::StorageRequestBspsRequiredType,
    /// Number of BSPs that have successfully volunteered AND confirmed that they are storing the data.
    ///
    /// This starts at 0 and increases up to `bsps_required`. Once this reaches `bsps_required`, the
    /// storage request is considered complete and will be deleted..
    pub bsps_confirmed: T::StorageRequestBspsRequiredType,
}

/// Ephemeral BSP storage request tracking metadata.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct StorageRequestBspsMetadata<T: Config> {
    /// Confirmed that the data is being stored.
    ///
    /// This is normally when the BSP submits a proof of storage to the `pallet-proofs-dealer-trie`.
    pub confirmed: bool,
    pub _phantom: core::marker::PhantomData<T>,
}

/// Alias for the `AccountId` type used in the FileSystem pallet.
pub type StorageProviderId<T> = <T as frame_system::Config>::AccountId;

/// Alias for the `MerkleHash` type used in the ProofsDealerInterface.
pub type FileKey<T> =
    <<T as crate::Config>::ProofDealer as storage_hub_traits::ProofsDealerInterface>::MerkleHash;

/// Alias for the `Proof` type used in the ProofsDealerInterface.
pub type Proof<T> =
    <<T as crate::Config>::ProofDealer as storage_hub_traits::ProofsDealerInterface>::Proof;

/// Alias for the `MaxBsps` type used in the FileSystem pallet.
pub type MaxBspsPerStorageRequest<T> = <T as crate::Config>::MaxBspsPerStorageRequest;

/// Alias for the `MaxFilePathSize` type used in the FileSystem pallet.
pub type MaxFilePathSize<T> = <T as crate::Config>::MaxFilePathSize;

/// Alias for the `Fingerprint` type used in the FileSystem pallet.
pub type Fingerprint<T> = <T as crate::Config>::Fingerprint;

/// Alias for the `StorageData` type used in the MutateProvidersInterface.
pub type StorageData<T> =
    <<T as crate::Config>::Providers as storage_hub_traits::MutateProvidersInterface>::StorageData;

/// Alias for the `TargetBspsRequired` type used in the FileSystem pallet.
pub type TargetBspsRequired<T> = <T as crate::Config>::TargetBspsRequired;

/// Byte array representing the file path.
pub type FileLocation<T> = BoundedVec<u8, MaxFilePathSize<T>>;

/// Alias for the `MaxMultiAddressSize` type used in the FileSystem pallet.
pub type MaxMultiAddressSize<T> = <T as crate::Config>::MaxMultiAddressSize;

/// Byte array representing the libp2p multiaddress.
pub type MultiAddress<T> = BoundedVec<u8, MaxMultiAddressSize<T>>;

/// Alias for the `MaxMultiAddresses` type used in the FileSystem pallet.
pub type MaxMultiAddresses<T> = <T as crate::Config>::MaxMultiAddresses;

/// Alias for a bounded vector of [`MultiAddress`].
pub type MultiAddresses<T> = BoundedVec<MultiAddress<T>, MaxMultiAddresses<T>>;
