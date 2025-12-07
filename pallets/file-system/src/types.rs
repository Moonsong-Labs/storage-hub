use core::cmp::max;

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
    traits::{fungible::Inspect, nonfungibles_v2::Inspect as NonFungiblesInspect, Get},
    BoundedVec,
};
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_nfts::CollectionConfig;
use scale_info::TypeInfo;
use shp_traits::{MutateBucketsInterface, ReadProvidersInterface};
use sp_runtime::{traits::CheckedAdd, DispatchError, SaturatedConversion};
use sp_std::{fmt::Debug, vec::Vec};

use crate::{
    Config, Error, MoveBucketRequestExpirations, NextAvailableMoveBucketRequestExpirationTick,
    NextAvailableStorageRequestExpirationTick, StorageRequestBsps, StorageRequestExpirations,
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

    /// Tick number at which the storage request will expire.
    ///
    /// Used to track what storage elements to clean when a storage request gets fulfilled.
    pub expires_at: TickNumber<T>,

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
    pub size: StorageDataUnit<T>,

    /// MSP who is requested to store the data, and if it has already confirmed that it is storing it.
    ///
    /// This is optional in the event when a storage request is created solely to replicate data to other BSPs and an MSP is already storing the data.
    pub msp: Option<(ProviderIdFor<T>, bool)>,

    /// Whether the MSP confirmed this storage request with an inclusion proof (file already existed in bucket).
    ///
    /// This is used to determine whether `pending_bucket_removal` should be set on incomplete storage requests:
    /// - `true`: MSP confirmed with inclusion proof → file already existed → `pending_bucket_removal = false` on incomplete
    /// - `false`: MSP confirmed with non-inclusion proof → file was newly added → `pending_bucket_removal = true` on incomplete
    /// - Default is `false` (for new requests or when MSP hasn't confirmed yet)
    pub msp_confirmed_with_inclusion_proof: bool,

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

    /// Deposit paid by the user to open this storage request.
    ///
    /// This is used to pay for the cost of the BSPs volunteering for the storage request in case it either expires
    /// or gets revoked by the user. If the storage request is fulfilled, the deposit will be refunded to the user.
    pub deposit_paid: BalanceOf<T>,
}

impl<T: Config> StorageRequestMetadata<T> {
    pub fn to_file_metadata(self) -> Result<FileMetadata, DispatchError> {
        FileMetadata::new(
            self.owner.encode(),
            self.bucket_id.as_ref().to_vec(),
            self.location.to_vec(),
            self.size.saturated_into(),
            self.fingerprint.as_ref().into(),
        )
        .map_err(|_| Error::<T>::FailedToCreateFileMetadata.into())
    }
}

/// The enum which holds different options for the replication target of a storage request.
///
/// When a user wants to issue a storage request, it can select between any of these options as
/// the replication target for it. There's a tradeoff between the security level of the data and
/// both the time it takes for the storage request to be fulfilled and the price paid per byte
/// during the file's lifetime in StorageHub.
/// Each option has a different security level, which represents the resiliency that the data will
/// have against a malicious actor controlling 1/3 of the total BSPs of the network.
/// All the following percentages assume that all the BSPs of the network have the same reputation
/// weight, which on average is a realistic scenario since both good and bad BSPs are expected to
/// have low and high reputations.
///
/// The options are:
/// - Basic: the data will be stored by enough BSPs so the probability that a malicious
/// actor can hold the file hostage by controlling all its BSPs is ~1%.
/// - Standard: the data will be stored by enough BSPs so the probability that a malicious
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
    Basic,
    Standard,
    HighSecurity,
    SuperHighSecurity,
    UltraHighSecurity,
    Custom(ReplicationTargetType<T>),
}

impl<T: Config> Debug for ReplicationTarget<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ReplicationTarget::Basic => write!(f, "ReplicationTarget::Basic"),
            ReplicationTarget::Standard => write!(f, "ReplicationTarget::Standard"),
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
    pub file_size: StorageDataUnit<T>,
    pub deposit_paid_for_creation: BalanceOf<T>,
    /// Flag to indicate if a priority challenge should be queued for this file deletion request.
    pub queue_priority_challenge: bool,
}

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct PendingStopStoringRequest<T: Config> {
    pub tick_when_requested: TickNumber<T>,
    pub file_owner: T::AccountId,
    pub file_size: StorageDataUnit<T>,
}

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub enum ExpirationItem<T: Config> {
    StorageRequest(MerkleHash<T>),
    MoveBucketRequest(BucketIdFor<T>),
}

impl<T: Config> ExpirationItem<T> {
    pub(crate) fn get_ttl(&self) -> TickNumber<T> {
        match self {
            ExpirationItem::StorageRequest(_) => T::StorageRequestTtl::get().into(),
            ExpirationItem::MoveBucketRequest(_) => T::MoveBucketRequestTtl::get().into(),
        }
    }

    pub(crate) fn get_next_expiration_tick(&self) -> TickNumber<T> {
        // The expiration tick is the maximum between the next available tick and the current tick number plus the TTL.
        let current_tick_plus_ttl =
            <T::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick()
                + self.get_ttl();
        let next_available_tick = match self {
            ExpirationItem::StorageRequest(_) => {
                NextAvailableStorageRequestExpirationTick::<T>::get()
            }
            ExpirationItem::MoveBucketRequest(_) => {
                NextAvailableMoveBucketRequestExpirationTick::<T>::get()
            }
        };

        max(next_available_tick, current_tick_plus_ttl)
    }

    pub(crate) fn try_append(
        &self,
        expiration_tick: TickNumber<T>,
    ) -> Result<TickNumber<T>, DispatchError> {
        let mut next_expiration_tick = expiration_tick;
        while let Err(_) = match self {
            ExpirationItem::StorageRequest(storage_request) => {
                <StorageRequestExpirations<T>>::try_append(next_expiration_tick, *storage_request)
            }
            ExpirationItem::MoveBucketRequest(msp_bucket_id) => {
                <MoveBucketRequestExpirations<T>>::try_append(next_expiration_tick, *msp_bucket_id)
            }
        } {
            next_expiration_tick = next_expiration_tick
                .checked_add(&1u8.into())
                .ok_or(Error::<T>::MaxTickNumberReached)?;
        }

        Ok(next_expiration_tick)
    }

    pub(crate) fn set_next_expiration_tick(&self, next_expiration_tick: TickNumber<T>) {
        match self {
            ExpirationItem::StorageRequest(_) => {
                NextAvailableStorageRequestExpirationTick::<T>::set(next_expiration_tick);
            }
            ExpirationItem::MoveBucketRequest(_) => {
                NextAvailableMoveBucketRequestExpirationTick::<T>::set(next_expiration_tick);
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
    /// The MSP ID of the new MSP that the user requested to store the bucket.
    pub new_msp_id: ProviderIdFor<T>,
    /// The new value proposition that this bucket will have after it has been moved.
    /// It must be a valid value proposition that the new MSP supports.
    pub new_value_prop_id: ValuePropId<T>,
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

impl<T: Config> EitherAccountIdOrMspId<T> {
    pub fn is_account_id(&self) -> bool {
        match self {
            EitherAccountIdOrMspId::AccountId(_) => true,
            EitherAccountIdOrMspId::MspId(_) => false,
        }
    }

    pub fn is_msp_id(&self) -> bool {
        match self {
            EitherAccountIdOrMspId::AccountId(_) => false,
            EitherAccountIdOrMspId::MspId(_) => true,
        }
    }
}

/// Enum representing the different file operations that can be used.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq, Clone)]
pub enum FileOperation {
    /// Delete operation for a file.
    Delete,
}

/// File operation intention. This, when signed by the file owner,
/// allows an actor to execute the operation on the file owner's behalf.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct FileOperationIntention<T: Config> {
    /// The file key to act upon.
    pub file_key: MerkleHash<T>,
    /// The operation to be performed on the file.
    pub operation: FileOperation,
}

impl<T: Config> Debug for FileOperationIntention<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "FileOperationIntention(file_key: {:?}, operation: {:?})",
            self.file_key, self.operation
        )
    }
}

/// A single file deletion request containing all metadata and signatures needed.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct FileDeletionRequest<T: Config> {
    /// Owner account of the file
    pub file_owner: T::AccountId,
    /// Signed intention containing the file key and delete operation
    pub signed_intention: FileOperationIntention<T>,
    /// Signature from the file owner authorizing the deletion
    pub signature: T::OffchainSignature,
    /// Bucket containing the file
    pub bucket_id: BucketIdFor<T>,
    /// File location/path
    pub location: FileLocation<T>,
    /// File size in storage units
    pub size: StorageDataUnit<T>,
    /// File fingerprint for verification
    pub fingerprint: Fingerprint<T>,
}

impl<T: Config> core::fmt::Debug for FileDeletionRequest<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FileDeletionRequest")
            .field("file_owner", &self.file_owner)
            .field("signed_intention", &self.signed_intention)
            .field("signature", &"<signature>")
            .field("bucket_id", &self.bucket_id)
            .field("location", &self.location)
            .field("size", &self.size)
            .field("fingerprint", &self.fingerprint)
            .finish()
    }
}

/// Ephemeral metadata for incomplete storage requests.
/// This is used to track which providers still need to remove their files.
/// Once all providers have removed their files, the entry is  cleaned up.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct IncompleteStorageRequestMetadata<T: Config> {
    /// File owner for validation
    pub owner: T::AccountId,
    /// Bucket containing the file
    pub bucket_id: BucketIdFor<T>,
    /// File location/path
    pub location: FileLocation<T>,
    /// File size
    pub file_size: StorageDataUnit<T>,
    /// File fingerprint
    pub fingerprint: Fingerprint<T>,
    /// BSPs that still need to remove the file (bounded by max number of BSPs that can have confirmed)
    pub pending_bsp_removals: BoundedVec<ProviderIdFor<T>, MaxReplicationTarget<T>>,
    /// Whether the file still needs to be removed from the bucket
    pub pending_bucket_removal: bool,
}

impl<T: Config> IncompleteStorageRequestMetadata<T> {
    /// Check if all providers have removed their files
    pub fn is_fully_cleaned(&self) -> bool {
        self.pending_bsp_removals.is_empty() && !self.pending_bucket_removal
    }

    /// Remove a provider from pending lists
    pub fn remove_provider(&mut self, provider_id: Option<ProviderIdFor<T>>) {
        match provider_id {
            None => {
                // Bucket removal complete
                self.pending_bucket_removal = false;
            }
            Some(id) => {
                // Remove BSP from the pending list
                self.pending_bsp_removals.retain(|&bsp_id| bsp_id != id);
            }
        }
    }
}

impl<T: Config> From<(&StorageRequestMetadata<T>, &MerkleHash<T>)>
    for IncompleteStorageRequestMetadata<T>
{
    fn from((storage_request, file_key): (&StorageRequestMetadata<T>, &MerkleHash<T>)) -> Self {
        // Collect all confirmed BSPs
        let mut confirmed_bsps = sp_std::vec::Vec::new();
        for (bsp_id, metadata) in StorageRequestBsps::<T>::iter_prefix(file_key) {
            if metadata.confirmed {
                confirmed_bsps.push(bsp_id);
            }
        }

        // Check if MSP has accepted the storage request and confirmed it with a non-inclusion proof.
        // This is because if the MSP confirmed it with an inclusion proof, the file already existed in the bucket from a previous
        // storage request, so we should not mark it for bucket removal.
        let pending_bucket_removal = matches!(storage_request.msp, Some((_, true)))
            && !storage_request.msp_confirmed_with_inclusion_proof;

        let bounded_bsps = BoundedVec::truncate_from(confirmed_bsps);

        Self {
            owner: storage_request.owner.clone(),
            bucket_id: storage_request.bucket_id,
            location: storage_request.location.clone(),
            file_size: storage_request.size,
            fingerprint: storage_request.fingerprint,
            pending_bsp_removals: bounded_bsps,
            pending_bucket_removal,
        }
    }
}

/// Alias for FileMetadata with the concrete constants used in StorageHub.
pub type FileMetadata = shp_file_metadata::FileMetadata<
    { shp_constants::H_LENGTH },
    { shp_constants::FILE_CHUNK_SIZE },
    { shp_constants::FILE_SIZE_TO_CHALLENGES },
>;

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

/// Alias for the `StorageDataUnit` type used in the MutateProvidersInterface.
pub type StorageDataUnit<T> =
    <<T as crate::Config>::Providers as shp_traits::MutateStorageProvidersInterface>::StorageDataUnit;

/// Alias for the `ReplicationTargetType` type used in the FileSystem pallet.
pub type ReplicationTargetType<T> = <T as crate::Config>::ReplicationTargetType;

/// Alias for the `MaxReplicationTarget` type used in the FileSystem pallet.
pub type MaxReplicationTarget<T> = <T as crate::Config>::MaxReplicationTarget;

/// Alias for the `StorageRequestTtl` type used in the FileSystem pallet.
pub type StorageRequestTtl<T> = <T as crate::Config>::StorageRequestTtl;

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

/// Alias for the `OffchainSignature` type used in the FileSystem pallet.
pub type OffchainSignatureFor<T> = <T as crate::Config>::OffchainSignature;
