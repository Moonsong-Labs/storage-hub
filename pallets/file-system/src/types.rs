use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
    traits::{nonfungibles_v2::Inspect as NonFungiblesInspect, Currency},
    BoundedVec,
};
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_nfts::CollectionConfig;
use scale_info::TypeInfo;
use shp_file_metadata::FileMetadata;
use shp_traits::ReadProvidersInterface;

use crate::Config;

/// Ephemeral metadata of a storage request.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct StorageRequestMetadata<T: Config> {
    /// Block number at which the storage request was made.
    ///
    /// Used primarily for tracking the age of the request which is useful for
    /// cleaning up old requests.
    pub requested_at: BlockNumberFor<T>,
    /// AccountId of the user who owns the data being stored.
    pub owner: T::AccountId,
    /// Bucket id where this file is stored.
    pub bucket_id: BucketIdFor<T>,
    /// User defined name of the file being stored.
    pub location: FileLocation<T>,
    /// Identifier of the data being stored.
    pub fingerprint: Fingerprint<T>,
    /// Size of the data being stored.
    ///
    /// SPs will use this to determine if they have enough space to store the data.
    /// This is also used to verify that the data sent by the user matches the size specified here.
    pub size: StorageData<T>,
    /// MSP who is requested to store the data.
    ///
    /// This is optional in the event when a storage request is created solely to replicate data to other BSPs and an MSP is already storing the data.
    pub msp: Option<ProviderIdFor<T>>,
    /// Peer Ids of the user who requested the storage.
    ///
    /// SPs will expect a connection request to be initiated by the user with this Peer Id.
    pub user_peer_ids: PeerIds<T>,
    /// List of storage providers that can serve the data that is requested to be stored.
    ///
    /// This is useful when a BSP stops serving data and automatically creates a new storage request with no user multiaddresses, since
    /// SPs can prove and serve the data to be replicated to other BSPs without the user having this stored on their local machine.
    pub data_server_sps: BoundedVec<ProviderIdFor<T>, MaxBspsPerStorageRequest<T>>, // TODO: Change the Maximum data servers to be the maximum SPs allowed
    /// Number of BSPs requested to store the data.
    ///
    /// The storage request will be dropped/complete once all the minimum required BSPs have
    /// submitted a proof of storage after volunteering to store the data.
    pub bsps_required: T::StorageRequestBspsRequiredType,
    /// Number of BSPs that have successfully volunteered AND confirmed that they are storing the data.
    ///
    /// This starts at 0 and increases up to `bsps_required`. Once this reaches `bsps_required`, the
    /// storage request is considered complete and will be deleted..
    pub bsps_confirmed: T::StorageRequestBspsRequiredType,
    /// Number of BSPs that have volunteered to store the data.
    ///
    /// There can be more than `bsps_required` volunteers, but it is essentially a race for BSPs to confirm that they are storing the data.
    pub bsps_volunteered: T::StorageRequestBspsRequiredType,
}

impl<T: Config> StorageRequestMetadata<T> {
    pub fn to_file_metadata(
        self,
    ) -> FileMetadata<
        { shp_constants::H_LENGTH },
        { shp_constants::FILE_CHUNK_SIZE },
        { shp_constants::FILE_SIZE_TO_CHALLENGES },
    > {
        FileMetadata {
            owner: self.owner.encode(),
            bucket_id: self.bucket_id.as_ref().to_vec(),
            location: self.location.to_vec(),
            file_size: self.size.into() as u64,
            fingerprint: self.fingerprint.as_ref().into(),
        }
    }
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

/// Bucket privacy settings.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq, Clone)]
pub enum BucketPrivacy {
    Public,
    Private,
}

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub enum ExpiredItems<T: Config> {
    StorageRequest(MerkleHash<T>),
    PendingFileDeletionRequests((T::AccountId, MerkleHash<T>)),
}

/// Alias for the `MerkleHash` type used in the ProofsDealerInterface representing file keys.
pub type MerkleHash<T> =
    <<T as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::MerkleHash;

/// Alias for the `ForestProof` type used in the ProofsDealerInterface.
pub type ForestProof<T> =
    <<T as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::ForestProof;

/// Alias for the `KeyProof` type used in the ProofsDealerInterface.
pub type KeyProof<T> =
    <<T as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::KeyProof;

/// Alias for the `MerkleHashing` type used in the ProofsDealerInterface.
pub type FileKeyHasher<T> =
    <<T as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::MerkleHashing;

/// Alias for the `MaxBspsPerStorageRequest` type used in the FileSystem pallet.
pub type MaxBspsPerStorageRequest<T> = <T as crate::Config>::MaxBspsPerStorageRequest;

/// Alias for the `MaxFilePathSize` type used in the FileSystem pallet.
pub type MaxFilePathSize<T> = <T as crate::Config>::MaxFilePathSize;

/// Alias for the `Fingerprint` type used in the FileSystem pallet.
pub type Fingerprint<T> = <T as crate::Config>::Fingerprint;

/// Alias for the `StorageData` type used in the MutateProvidersInterface.
pub type StorageData<T> =
    <<T as crate::Config>::Providers as shp_traits::MutateStorageProvidersInterface>::StorageDataUnit;

/// Alias for the `TargetBspsRequired` type used in the FileSystem pallet.
pub type TargetBspsRequired<T> = <T as crate::Config>::TargetBspsRequired;

/// Alias for the `StorageRequestTtl` type used in the FileSystem pallet.
pub type StorageRequestTtl<T> = <T as crate::Config>::StorageRequestTtl;

/// Alias for the `PendingFileDeletionRequestTtl` type used in the FileSystem pallet.
pub type PendingFileDeletionRequestTtl<T> = <T as crate::Config>::PendingFileDeletionRequestTtl;

/// Byte array representing the file path.
pub type FileLocation<T> = BoundedVec<u8, MaxFilePathSize<T>>;

/// Alias for the `MaxPeerIdSize` type used in the FileSystem pallet.
pub type MaxPeerIdSize<T> = <T as crate::Config>::MaxPeerIdSize;

/// Byte array representing the libp2p peer Id.
pub type PeerId<T> = BoundedVec<u8, MaxPeerIdSize<T>>;

/// Alias for the `MaxNumberOfPeerIds` type used in the FileSystem pallet.
pub type MaxNumberOfPeerIds<T> = <T as crate::Config>::MaxNumberOfPeerIds;

/// Alias for a bounded vector of [`PeerId`].
pub type PeerIds<T> = BoundedVec<PeerId<T>, MaxNumberOfPeerIds<T>>;

/// Alias for the `MultiAddress` type used in the ReadProvidersInterface.
pub type MultiAddress<T> =
    <<T as crate::Config>::Providers as shp_traits::ReadStorageProvidersInterface>::MultiAddress;

/// Alias for the `MaxMultiAddresses` type used in the ReadProvidersInterface.
pub type MaxMultiAddresses<T> =
    <<T as crate::Config>::Providers as shp_traits::ReadStorageProvidersInterface>::MaxNumberOfMultiAddresses;

/// Alias for a bounded vector of [`MultiAddress`].
pub type MultiAddresses<T> = BoundedVec<MultiAddress<T>, MaxMultiAddresses<T>>;

/// Alias for the `Balance` type used in the FileSystem pallet.
type BalanceOf<T> =
    <<T as crate::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

/// Alias for the `CollectionId` type used in the Nfts pallet.
pub(super) type CollectionIdFor<T> = <<T as crate::Config>::Nfts as NonFungiblesInspect<
    <T as frame_system::Config>::AccountId,
>>::CollectionId;

/// Alias for the `CollectionConfig` type used in the FileSystem pallet.
pub(super) type CollectionConfigFor<T> =
    CollectionConfig<BalanceOf<T>, BlockNumberFor<T>, CollectionIdFor<T>>;

/// Alias for the `BucketNameLimit` type used in the ReadProvidersInterface.
pub(super) type BucketNameLimitFor<T> =
    <<T as crate::Config>::Providers as shp_traits::ReadBucketsInterface>::BucketNameLimit;

/// Type alias representing the type of `BucketId` used in `ProvidersInterface`.
pub(crate) type BucketIdFor<T> =
    <<T as crate::Config>::Providers as shp_traits::ReadBucketsInterface>::BucketId;

/// Alias for the `ProviderId` type used in the ProvidersInterface.
pub type ProviderIdFor<T> = <<T as crate::Config>::Providers as ReadProvidersInterface>::ProviderId;

/// Alias for the bucket name.
pub type BucketNameFor<T> = BoundedVec<u8, BucketNameLimitFor<T>>;
