//! Various types to use in the Storage Providers pallet.

use super::*;
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use core::cmp::max;
use frame_support::{pallet_prelude::*, traits::fungible::Inspect};
use frame_system::pallet_prelude::BlockNumberFor;
use scale_info::TypeInfo;
use shp_traits::{PaymentStreamsInterface, StorageHubTickGetter};
use sp_runtime::{traits::CheckedAdd, ArithmeticError, BoundedVec};

pub type Multiaddresses<T> = BoundedVec<MultiAddress<T>, MaxMultiAddressAmount<T>>;

pub type ValuePropId<T> = HashId<T>;

/// Awaited top up metadata for a provider.
#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebugNoBound,
    PartialEq,
    Eq,
    Clone,
)]
#[scale_info(skip_type_params(T))]
pub struct TopUpMetadata<T: Config> {
    /// The payment streams tick number at which the provider started awaiting a top up.
    ///
    /// This is used for payment streams to determine when the provider should not be able to charge the user anymore starting
    /// from this tick number.
    pub started_at: PaymentStreamsTickNumber<T>,
    /// The Storage Hub tick number at which the provider will be marked as insolvent.
    ///
    /// It is the tick number at which the provider will be marked as insolvent after processing it from the [`ProviderTopUpExpirations`](crate::ProviderTopUpExpirations) storage.
    pub end_tick_grace_period: StorageHubTickNumber<T>,
}

#[derive(
    Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq, Clone,
)]
#[scale_info(skip_type_params(T))]
pub enum ExpirationItem<T: Config> {
    ProviderTopUp(StorageProviderId<T>),
}

impl<T: Config> ExpirationItem<T> {
    pub(crate) fn get_ttl(&self) -> StorageHubTickNumber<T> {
        match self {
            ExpirationItem::ProviderTopUp(_) => T::ProviderTopUpTtl::get(),
        }
    }

    pub(crate) fn get_next_expiration_tick(
        &self,
    ) -> Result<StorageHubTickNumber<T>, DispatchError> {
        // The expiration block is the maximum between the next available block and the current block number plus the TTL.
        let current_global_tick_with_ttl = ShTickGetter::<T>::get_current_tick()
            .checked_add(&self.get_ttl())
            .ok_or(ArithmeticError::Overflow)?;

        let next_available_tick: StorageHubTickNumber<T> = match self {
            ExpirationItem::ProviderTopUp(_) => {
                NextAvailableProviderTopUpExpirationShTick::<T>::get()
            }
        };

        Ok(max(next_available_tick, current_global_tick_with_ttl))
    }

    pub(crate) fn try_append(
        &self,
        at_tick: StorageHubTickNumber<T>,
    ) -> Result<StorageHubTickNumber<T>, DispatchError> {
        let mut appended_at_tick = at_tick;
        while let Err(_) = match self {
            ExpirationItem::ProviderTopUp(provider_id) => {
                <ProviderTopUpExpirations<T>>::try_append(appended_at_tick, provider_id.clone())
            }
        } {
            appended_at_tick = appended_at_tick
                .checked_add(&1u8.into())
                .ok_or(Error::<T>::MaxBlockNumberReached)?;
        }

        Ok(appended_at_tick)
    }

    pub(crate) fn set_next_expiration_tick(
        &self,
        next_expiration_tick: StorageHubTickNumber<T>,
    ) -> DispatchResult {
        match self {
            ExpirationItem::ProviderTopUp(_) => {
                NextAvailableProviderTopUpExpirationShTick::<T>::set(next_expiration_tick);

                Ok(())
            }
        }
    }
}
#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebugNoBound,
    PartialEq,
    Eq,
    Clone,
)]
#[scale_info(skip_type_params(T))]
pub struct ValuePropositionWithId<T: Config> {
    pub id: ValuePropIdFor<T>,
    pub value_prop: ValueProposition<T>,
}

impl<T: Config> ValuePropositionWithId<T> {
    pub fn new(id: ValuePropIdFor<T>, value_prop: ValueProposition<T>) -> Self {
        Self { id, value_prop }
    }

    pub fn build(
        price_per_unit_of_data_per_block: BalanceOf<T>,
        commitment: Commitment<T>,
        bucket_data_limit: StorageDataUnit<T>,
    ) -> Self {
        let value_prop = ValueProposition::<T>::new(
            price_per_unit_of_data_per_block,
            commitment,
            bucket_data_limit,
        );
        Self {
            id: value_prop.derive_id(),
            value_prop,
        }
    }
}

#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebugNoBound,
    PartialEq,
    Eq,
    Clone,
)]
#[scale_info(skip_type_params(T))]
pub struct ValueProposition<T: Config> {
    pub price_per_giga_unit_of_data_per_block: BalanceOf<T>,
    pub commitment: Commitment<T>,
    /// Maximum [`StorageDataUnit`]s that can be stored in a bucket.
    pub bucket_data_limit: StorageDataUnit<T>,
    /// Newly created buckets can only specify available value propositions.
    /// Any existing bucket with an unavailable value proposition are not affected.
    pub available: bool,
}

impl<T: Config> ValueProposition<T> {
    pub fn new(
        price_per_giga_unit_of_data_per_block: BalanceOf<T>,
        commitment: Commitment<T>,
        bucket_data_limit: StorageDataUnit<T>,
    ) -> Self {
        Self {
            price_per_giga_unit_of_data_per_block,
            commitment,
            bucket_data_limit,
            available: true,
        }
    }

    /// Produce the ID of the ValueProposition not including the `available` field.
    pub fn derive_id(&self) -> ValuePropIdFor<T> {
        let mut concat = self.price_per_giga_unit_of_data_per_block.encode();
        concat.extend_from_slice(&self.commitment.encode());
        concat.extend_from_slice(&self.bucket_data_limit.encode());
        <<T as crate::Config>::ValuePropIdHashing as sp_runtime::traits::Hash>::hash(&concat)
    }
}

pub type Commitment<T> = BoundedVec<u8, <T as crate::Config>::MaxCommitmentSize>;

/// Structure that represents a Main Storage Provider. It holds the buckets that the MSP has, the total data that the MSP is able to store,
/// the amount of data that it is storing, and its libp2p multiaddresses.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebugNoBound, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct MainStorageProvider<T: Config> {
    /// The total capacity that this MSP can store.
    pub capacity: StorageDataUnit<T>,
    /// The amount of data that this MSP is currently storing.
    pub capacity_used: StorageDataUnit<T>,
    /// The multiaddresses that can be used to connect to this MSP.
    pub multiaddresses: Multiaddresses<T>,
    /// The number of buckets that this MSP currently has. This is used for stats and to know how many Buckets have to moved
    /// if the MSP ever gets deleted because of slashing.
    pub amount_of_buckets: BucketCount<T>,
    /// The number of value propositions that this MSP has registered over its lifetime. This is used to know how many storage
    /// elements have to be deleted if this MSP signs off or gets deleted.
    pub amount_of_value_props: u32,
    /// The block at which this MSP last changed its total capacity.
    pub last_capacity_change: BlockNumberFor<T>,
    /// The account ID of the owner of this MSP.
    pub owner_account: T::AccountId,
    /// The account ID that this MSP uses to receive payments.
    pub payment_account: T::AccountId,
    /// The block at which this MSP signed up.
    pub sign_up_block: BlockNumberFor<T>,
}

/// Structure that represents a Backup Storage Provider. It holds the total data that the BSP is able to store, the amount of data that it is storing,
/// its libp2p multiaddresses, and the root of the Merkle Patricia Trie that it stores.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebugNoBound, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct BackupStorageProvider<T: Config> {
    pub capacity: StorageDataUnit<T>,
    pub capacity_used: StorageDataUnit<T>,
    pub multiaddresses: Multiaddresses<T>,
    pub root: MerklePatriciaRoot<T>,
    pub last_capacity_change: BlockNumberFor<T>,
    pub owner_account: T::AccountId,
    pub payment_account: T::AccountId,
    pub reputation_weight: ReputationWeightType<T>,
    pub sign_up_block: BlockNumberFor<T>,
}

/// Structure that represents a Bucket. It holds the root of the Merkle Patricia Trie, the User ID that owns the bucket,
/// and the MainStorageProviderId that the bucket belongs to.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebugNoBound, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct Bucket<T: Config> {
    /// The current root of the bucket.
    pub root: MerklePatriciaRoot<T>,
    /// The user that owns the bucket.
    pub user_id: T::AccountId,
    /// The MSP ID of the MSP that is currently storing the bucket. It's an Option because the
    /// bucket can be in transit between MSPs after the first MSP stops storing it.
    pub msp_id: Option<MainStorageProviderId<T>>,
    /// Whether the bucket is private or not.
    pub private: bool,
    /// If the bucket is private, it has to have a collection ID that holds the items that allow a user to access the bucket's data.
    /// This is not enforced by the runtime but by the MSP storing that bucket.
    pub read_access_group_id: Option<T::ReadAccessGroupId>,
    /// The current size of the bucket.
    pub size: StorageDataUnit<T>,
    /// The value proposition that the bucket is associated with. It's only valid if the bucket is associated with a MSP.
    pub value_prop_id: ValuePropIdFor<T>,
}

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebugNoBound, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct SignUpRequest<T: Config> {
    pub sp_sign_up_request: SignUpRequestSpParams<T>,
    pub at: BlockNumberFor<T>,
}

/// Enum that represents a Storage Provider sign up request parameters. It holds either a BackupStorageProvider or a MainStorageProvider,
/// allowing to operate generically with both types.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebugNoBound, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub enum SignUpRequestSpParams<T: Config> {
    BackupStorageProvider(BackupStorageProvider<T>),
    MainStorageProvider(MainStorageProviderSignUpRequest<T>),
}

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebugNoBound, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct MainStorageProviderSignUpRequest<T: Config> {
    pub msp_info: MainStorageProvider<T>,
    pub value_prop: ValueProposition<T>,
}

/// Enum that represents a Storage Provider ID. It holds either a BackupStorageProviderId or a MainStorageProviderId,
/// allowing to operate generically with both types.
#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebugNoBound,
    PartialEq,
    Eq,
    Clone,
    Copy,
)]
#[scale_info(skip_type_params(T))]
pub enum StorageProviderId<T: Config> {
    BackupStorageProvider(BackupStorageProviderId<T>),
    MainStorageProvider(MainStorageProviderId<T>),
}

impl<T: Config> StorageProviderId<T> {
    /// Returns the inner value of the enum variant.
    pub fn inner(&self) -> &ProviderIdFor<T> {
        match self {
            StorageProviderId::BackupStorageProvider(id) => id,
            StorageProviderId::MainStorageProvider(id) => id,
        }
    }
}

/// The delta applied to a fixed rate payment stream via [`Pallet::compute_new_rate_delta`].
pub enum RateDeltaParam<T: Config> {
    /// Variant should be used when a new bucket is associated to an MSP.
    /// The bucket can be of any size, including zero since this variant can be selected when a bucket is being *moved* from one
    /// MSP to another.
    NewBucket,
    /// Variant should be used when a bucket is removed from an MSP.
    RemoveBucket,
    /// Variant should be used when a bucket size has increased by some amount.
    Increase(StorageDataUnit<T>),
    /// Variant should be used when a bucket size has decreased by some amount.
    Decrease(StorageDataUnit<T>),
}

// Type aliases:

/// BalanceOf is the balance type of the runtime.
pub type BalanceOf<T> =
    <<T as Config>::NativeBalance as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

pub type ProviderIdFor<T> = <T as crate::Config>::ProviderId;
/// BackupStorageProviderId is the type that represents an ID of a Backup Storage Provider, uniquely linked with an AccountId
pub type BackupStorageProviderId<T> = ProviderIdFor<T>;
/// MainStorageProviderId is the type that represents an ID of a Main Storage Provider, uniquely linked with an AccountId
pub type MainStorageProviderId<T> = ProviderIdFor<T>;
/// BucketId is the type that identifies the different buckets that a Main Storage Provider can have.
pub type BucketId<T> = ProviderIdFor<T>;

/// MaxMultiAddressSize is the maximum size of the libp2p multiaddress of a Storage Provider in bytes.
pub type MaxMultiAddressSize<T> = <T as crate::Config>::MaxMultiAddressSize;
/// MaxMultiAddressAmount is the maximum amount of MultiAddresses that a Storage Provider can have.
pub type MaxMultiAddressAmount<T> = <T as crate::Config>::MaxMultiAddressAmount;
/// MultiAddress is a byte array that represents the libp2p multiaddress of a Storage Provider.
/// Its maximum size is defined in the runtime configuration, as MaxMultiAddressSize.
pub type MultiAddress<T> = BoundedVec<u8, MaxMultiAddressSize<T>>;

/// MerklePatriciaRoot is the type of the root of a Merkle Patricia Trie, either the root of a BSP or a bucket from an MSP.
pub type MerklePatriciaRoot<T> = <T as crate::Config>::MerklePatriciaRoot;

/// HashId is the type that uniquely identifies either a Storage Provider (MSP or BSP) or a Bucket.
pub type HashId<T> = <T as frame_system::Config>::Hash;
/// Hashing is the hashing algorithm used to get the HashId.
pub type Hashing<T> = <T as frame_system::Config>::Hashing;

/// StorageData is the type of the unit in which we measure data size. We define its required traits in the
/// pallet configuration so the runtime can use any type that implements them.
pub type StorageDataUnit<T> = <T as crate::Config>::StorageDataUnit;
/// BucketCount is the type that is used to count the amount of buckets that a Main Storage Provider can have.
pub type BucketCount<T> = <T as crate::Config>::BucketCount;

/// Protocols is a vector of the protocols that (the runtime is aware of and) the Main Storage Provider supports.
/// Its maximum size is defined in the runtime configuration, as MaxProtocols.
pub type MaxProtocols<T> = <T as crate::Config>::MaxProtocols;
pub type Protocols<T> = BoundedVec<u8, MaxProtocols<T>>; // todo!("Define a type for protocols")

/// Type alias for the `ValuePropId` type used in the Storage Providers pallet.
pub type ValuePropIdFor<T> = <T as crate::Config>::ValuePropId;

/// Type alias for the `ReputationWeightType` type used in the Storage Providers pallet.
pub type ReputationWeightType<T> = <T as crate::Config>::ReputationWeightType;

/// Type alias for the `StartingReputationWeight` type used in the Storage Providers pallet.
pub type StartingReputationWeight<T> = <T as crate::Config>::StartingReputationWeight;

/// Type alias for the `StorageHubTickGetter` type used in the Storage Providers pallet.
pub type ShTickGetter<T> = <T as crate::Config>::StorageHubTickGetter;

/// Type alias for the `BlockNumber` type used by `StorageHubTickGetter`.
pub type StorageHubTickNumber<T> =
    <<T as crate::Config>::StorageHubTickGetter as shp_traits::StorageHubTickGetter>::TickNumber;

/// Type alias for the `StorageDataUnitAndBalanceConvert` type used in the Storage Providers pallet.
pub type StorageDataUnitAndBalanceConverter<T> =
    <T as crate::Config>::StorageDataUnitAndBalanceConvert;

/// Type alias for the `ProviderTopUpTtl` type used in the Storage Providers pallet.
pub type ProviderTopUpTtl<T> = <T as crate::Config>::ProviderTopUpTtl;

/// Type alias for the `TickNumber` type used in the Storage Providers pallet.
pub type PaymentStreamsTickNumber<T> =
    <<T as crate::Config>::PaymentStreams as PaymentStreamsInterface>::TickNumber;
