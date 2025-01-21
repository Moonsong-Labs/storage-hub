use core::cmp::max;

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
    traits::{fungible::Inspect, nonfungibles_v2::Inspect as NonFungiblesInspect, Get},
    BoundedVec,
};
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_nfts::CollectionConfig;
use scale_info::TypeInfo;
use shp_file_metadata::FileMetadata;
use shp_traits::{MutateBucketsInterface, ReadProvidersInterface};
use sp_runtime::{traits::CheckedAdd, DispatchError};
use sp_std::{fmt::Debug, vec::Vec};

use crate::{
    Config, Error, FileDeletionRequestExpirations, MoveBucketRequestExpirations,
    NextAvailableFileDeletionRequestExpirationBlock, NextAvailableMoveBucketRequestExpirationBlock,
    NextAvailableStorageRequestExpirationBlock, StorageRequestExpirations,
};

/// Ephemeral metadata of a storage request.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct StorageRequestMetadata<T: Config> {
    /// Tick number at which the storage request was made.
    ///
    /// Used primarily for tracking the age of the request which is useful for
    /// cleaning up old requests.
    pub requested_at: TickNumber<T>,

    /// Block number at which the storage request will expire.
    ///
    /// Used to track what storage elements to clean when a storage request gets fulfilled.
    /// Note: we use block numbers for expiration items instead of ticks. Maybe we should unify this.
    pub expires_at: BlockNumberFor<T>,

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

    /// MSP who is requested to store the data, and if it has already confirmed that it is storing it.
    ///
    /// This is optional in the event when a storage request is created solely to replicate data to other BSPs and an MSP is already storing the data.
    pub msp: Option<(ProviderIdFor<T>, bool)>,

    /// Peer Ids of the user who requested the storage.
    ///
    /// SPs will expect a connection request to be initiated by the user with this Peer Id.
    pub user_peer_ids: PeerIds<T>,

    /// Number of BSPs requested to store the data.
    ///
    /// The storage request will be dropped/complete once all the minimum required BSPs have
    /// submitted a proof of storage after volunteering to store the data.
    pub bsps_required: ReplicationTargetType<T>,

    /// Number of BSPs that have successfully volunteered AND confirmed that they are storing the data.
    ///
    /// This starts at 0 and increases up to `bsps_required`. Once this reaches `bsps_required`, the
    /// storage request is considered complete and will be deleted..
    pub bsps_confirmed: ReplicationTargetType<T>,

    /// Number of BSPs that have volunteered to store the data.
    ///
    /// There can be more than `bsps_required` volunteers, but it is essentially a race for BSPs to confirm that they are storing the data.
    pub bsps_volunteered: ReplicationTargetType<T>,
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

/// The enum which holds different options for the replication target of a storage request.
///
/// When a user wants to issue a storage request, it can select between any of these options as
/// the replication target for it. There's a tradeoff between the security level of the data and
/// both the time it takes for the storage request to be fulfilled and the price paid per byte
/// during the file's lifetime in StorageHub.
/// Each option has a different security level, which represents the resiliency that the data will
/// have against a malicious actor controlling 1/3 of the BSPs of the network.
/// All the following percentages assume that all the BSPs of the network have the same reputation
/// weight, which on average is a realistic scenario since both good and bad BSPs are expected to
/// have low and high reputations.
///
/// The options are:
/// - LowSecurity: the data will be stored by enough BSPs so the probability that a malicious
/// actor can hold the file hostage by controlling all its BSPs is ~1%.
/// - MediumSecurity: the data will be stored by enough BSPs so the probability that a malicious
/// actor can hold the file hostage by controlling all its BSPs is ~0.1%.
/// - HighSecurity: the data will be stored by enough BSPs so the probability that a malicious
/// actor can hold the file hostage by controlling all its BSPs is ~0.01%.
/// - SuperHighSecurity: the data will be stored by enough BSPs so the probability that a malicious
/// actor can hold the file hostage by controlling all its BSPs is ~0.001%.
/// - UltraHighSecurity: the data will be stored by enough BSPs so the probability that a malicious
/// actor can hold the file hostage by controlling all its BSPs is ~0.0001%.
/// - Custom: the user can select the number of BSPs that will store the data. This allows the user to
/// select the security level of the data manually.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub enum ReplicationTarget<T: Config> {
    LowSecurity,
    MediumSecurity,
    HighSecurity,
    SuperHighSecurity,
    UltraHighSecurity,
    Custom(ReplicationTargetType<T>),
}

impl<T: Config> Debug for ReplicationTarget<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ReplicationTarget::LowSecurity => write!(f, "ReplicationTarget::LowSecurity"),
            ReplicationTarget::MediumSecurity => write!(f, "ReplicationTarget::MediumSecurity"),
            ReplicationTarget::HighSecurity => write!(f, "ReplicationTarget::HighSecurity"),
            ReplicationTarget::SuperHighSecurity => {
                write!(f, "ReplicationTarget::SuperHighSecurity")
            }
            ReplicationTarget::UltraHighSecurity => {
                write!(f, "ReplicationTarget::UltraHighSecurity")
            }
            ReplicationTarget::Custom(target) => {
                write!(
                    f,
                    "ReplicationTarget::Custom({})",
                    <<T as crate::Config>::ReplicationTargetType as Into<u64>>::into(*target)
                )
            }
        }
    }
}

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct FileKeyWithProof<T: Config> {
    pub file_key: MerkleHash<T>,
    pub proof: KeyProof<T>,
}

impl<T: Config> Debug for FileKeyWithProof<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "FileKeyWithProof(file_key: {:?}, proof: {:?})",
            self.file_key, self.proof
        )
    }
}

/// A bundle of file keys that have been accepted by an MSP, alongside the proofs required to  
/// add these file keys into the corresponding bucket.  
///  
/// This struct includes a list of file keys and their corresponding key proofs (i.e. the  
/// proofs for the file chunks) and a non-inclusion forest proof. The latter is required to  
/// verify that the file keys were not part of the bucket's Merkle Patricia Forest before,  
/// and add them now. One single non-inclusion forest proof for all the file keys is sufficient.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct StorageRequestMspAcceptedFileKeys<T: Config> {
    pub file_keys_and_proofs: Vec<FileKeyWithProof<T>>,
    /// File keys which have already been accepted by the MSP in a previous storage request should be included
    /// in the proof.
    pub forest_proof: ForestProof<T>,
}

impl<T: Config> Debug for StorageRequestMspAcceptedFileKeys<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "StorageRequestMspAcceptedFileKeys(file_keys_and_proofs: {:?}, forest_proof: {:?})",
            self.file_keys_and_proofs, self.forest_proof
        )
    }
}

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq, Clone)]
pub enum RejectedStorageRequestReason {
    ReachedMaximumCapacity,
    ReceivedInvalidProof,
    FileKeyAlreadyStored,
    RequestExpired,
    InternalError,
}

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct RejectedStorageRequest<T: Config> {
    pub file_key: MerkleHash<T>,
    pub reason: RejectedStorageRequestReason,
}

impl<T: Config> Debug for RejectedStorageRequest<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "RejectedStorageRequest(file_key: {:?}, reason: {:?})",
            self.file_key, self.reason
        )
    }
}

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct StorageRequestMspBucketResponse<T: Config> {
    pub bucket_id: BucketIdFor<T>,
    pub accept: Option<StorageRequestMspAcceptedFileKeys<T>>,
    pub reject: Vec<RejectedStorageRequest<T>>,
}

impl<T: Config> Debug for StorageRequestMspBucketResponse<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "StorageRequestMspBucketResponse(bucket_id: {:?}, accept: {:?}, reject: {:?})",
            self.bucket_id, self.accept, self.reject
        )
    }
}

/// Unbounded input for MSPs to respond to storage request(s).
///
/// The input is a list of bucket responses, where each response contains:
/// - The bucket ID
/// - Optional accepted file keys and proof for the whole list
/// - List of rejected file keys and rejection reasons
pub type StorageRequestMspResponse<T> = Vec<StorageRequestMspBucketResponse<T>>;

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
pub struct PendingFileDeletionRequest<T: Config> {
    pub user: T::AccountId,
    pub file_key: MerkleHash<T>,
    pub bucket_id: BucketIdFor<T>,
    pub file_size: StorageData<T>,
}

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct PendingStopStoringRequest<T: Config> {
    pub tick_when_requested: TickNumber<T>,
    pub file_owner: T::AccountId,
    pub file_size: StorageData<T>,
}

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub enum ExpirationItem<T: Config> {
    StorageRequest(MerkleHash<T>),
    PendingFileDeletionRequests(PendingFileDeletionRequest<T>),
    MoveBucketRequest((ProviderIdFor<T>, BucketIdFor<T>)),
}

impl<T: Config> ExpirationItem<T> {
    pub(crate) fn get_ttl(&self) -> BlockNumberFor<T> {
        match self {
            ExpirationItem::StorageRequest(_) => T::StorageRequestTtl::get().into(),
            ExpirationItem::PendingFileDeletionRequests(_) => {
                T::PendingFileDeletionRequestTtl::get().into()
            }
            ExpirationItem::MoveBucketRequest(_) => T::MoveBucketRequestTtl::get().into(),
        }
    }

    pub(crate) fn get_next_expiration_block(&self) -> BlockNumberFor<T> {
        // The expiration block is the maximum between the next available block and the current block number plus the TTL.
        let current_block_plus_ttl = frame_system::Pallet::<T>::block_number() + self.get_ttl();
        let next_available_block = match self {
            ExpirationItem::StorageRequest(_) => {
                NextAvailableStorageRequestExpirationBlock::<T>::get()
            }
            ExpirationItem::PendingFileDeletionRequests(_) => {
                NextAvailableFileDeletionRequestExpirationBlock::<T>::get()
            }
            ExpirationItem::MoveBucketRequest(_) => {
                NextAvailableMoveBucketRequestExpirationBlock::<T>::get()
            }
        };

        max(next_available_block, current_block_plus_ttl)
    }

    pub(crate) fn try_append(
        &self,
        expiration_block: BlockNumberFor<T>,
    ) -> Result<BlockNumberFor<T>, DispatchError> {
        let mut next_expiration_block = expiration_block;
        while let Err(_) = match self {
            ExpirationItem::StorageRequest(storage_request) => {
                <StorageRequestExpirations<T>>::try_append(next_expiration_block, *storage_request)
            }
            ExpirationItem::PendingFileDeletionRequests(pending_file_deletion_requests) => {
                <FileDeletionRequestExpirations<T>>::try_append(
                    next_expiration_block,
                    pending_file_deletion_requests.clone(),
                )
            }
            ExpirationItem::MoveBucketRequest(msp_bucket_id) => {
                <MoveBucketRequestExpirations<T>>::try_append(next_expiration_block, *msp_bucket_id)
            }
        } {
            next_expiration_block = next_expiration_block
                .checked_add(&1u8.into())
                .ok_or(Error::<T>::MaxBlockNumberReached)?;
        }

        Ok(next_expiration_block)
    }

    pub(crate) fn set_next_expiration_block(&self, next_expiration_block: BlockNumberFor<T>) {
        match self {
            ExpirationItem::StorageRequest(_) => {
                NextAvailableStorageRequestExpirationBlock::<T>::set(next_expiration_block);
            }
            ExpirationItem::PendingFileDeletionRequests(_) => {
                NextAvailableFileDeletionRequestExpirationBlock::<T>::set(next_expiration_block);
            }
            ExpirationItem::MoveBucketRequest(_) => {
                NextAvailableMoveBucketRequestExpirationBlock::<T>::set(next_expiration_block);
            }
        }
    }
}

/// Possible responses to a move bucket request.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq, Clone)]
pub enum BucketMoveRequestResponse {
    Accepted,
    Rejected,
}

/// Move bucket request metadata
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct MoveBucketRequestMetadata<T: Config> {
    /// The user who requested to move the bucket.
    pub requester: T::AccountId,
}

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub enum EitherAccountIdOrMspId<T: Config> {
    AccountId(T::AccountId),
    MspId(ProviderIdFor<T>),
}

impl<T: Config> Debug for EitherAccountIdOrMspId<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EitherAccountIdOrMspId::AccountId(account_id) => {
                write!(f, "AccountId({:?})", account_id)
            }
            EitherAccountIdOrMspId::MspId(provider_id) => {
                write!(f, "MspId({:?})", provider_id)
            }
        }
    }
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

/// Alias for the `MaxBatchConfirmStorageRequests` type used in the FileSystem pallet.
pub type MaxBatchConfirmStorageRequests<T> = <T as crate::Config>::MaxBatchConfirmStorageRequests;

/// Alias for the `MaxFilePathSize` type used in the FileSystem pallet.
pub type MaxFilePathSize<T> = <T as crate::Config>::MaxFilePathSize;

/// Alias for the `Fingerprint` type used in the FileSystem pallet.
pub type Fingerprint<T> = <T as crate::Config>::Fingerprint;

/// Alias for the `StorageData` type used in the MutateProvidersInterface.
pub type StorageData<T> =
    <<T as crate::Config>::Providers as shp_traits::MutateStorageProvidersInterface>::StorageDataUnit;

/// Alias for the `ReplicationTargetType` type used in the FileSystem pallet.
pub type ReplicationTargetType<T> = <T as crate::Config>::ReplicationTargetType;

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

/// Alias for the `ValuePropId` type used in the MutateBucketsInterface.
pub type ValuePropId<T> = <<T as crate::Config>::Providers as MutateBucketsInterface>::ValuePropId;

/// Alias for a bounded vector of [`MultiAddress`].
pub type MultiAddresses<T> = BoundedVec<MultiAddress<T>, MaxMultiAddresses<T>>;

/// Alias for the `Balance` type used in the FileSystem pallet.
pub type BalanceOf<T> =
    <<T as Config>::Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

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

/// Alias for the type of the storage request expiration item.
pub type StorageRequestExpirationItem<T> = MerkleHash<T>;

/// Alias for the type of the file deletion request expiration item.
pub type FileDeletionRequestExpirationItem<T> = PendingFileDeletionRequest<T>;

/// Alias for the `ThresholdType` used in the FileSystem pallet.
pub type ThresholdType<T> = <T as crate::Config>::ThresholdType;

/// Alias for the `TickNumber` used in the ProofsDealer pallet.
pub type TickNumber<T> =
    <<T as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::TickNumber;
