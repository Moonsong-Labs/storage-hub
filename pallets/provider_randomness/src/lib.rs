//! # Randomness Pallet
//!
//! This pallet provides access to randomness using as source the relay chain BABE one epoch ago randomness,
//! produced by the relay chain per relay chain epoch
//!
//! There are no extrinsics for this pallet. Instead, an inherent updates the pseudo-random word obtained from
//! the relay chain when an epoch changes, and that word can be then used by other pallets as a source of randomness
//! as this pallet implements the Randomness trait
//!
//! ## Babe Epoch Randomness
//! Babe epoch randomness is retrieved once every relay chain epoch.
//!
//! The `set_babe_randomness` mandatory inherent reads the Babe epoch randomness from the
//! relay chain state proof and updates the latest pseudo-random word with this epoch randomness.
//!
//! `Config::BabeDataGetter` is responsible for reading the epoch index and epoch randomness
//! from the relay chain state proof
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet;
pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod queue;

pub trait RandomSeedMixer<Seed> {
    /// Mix randomeness seed 1 and seed 2 to generate new seed.
    /// Optionally takes a context seed to further randomize the mixing.
    fn mix_randomness_seed(seed_1: &Seed, seed_2: &Seed, context: Option<impl Into<Seed>>) -> Seed;
}

pub trait VerifiableSeed {
    type SeedCommitment;
    /// Verifies if the seed commitment matches the seed
    fn verify(&self, seed_commitment: Self::SeedCommitment) -> bool;
}

#[pallet]
pub mod pallet {
    use codec::FullCodec;
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_support::traits::Len;
    use frame_system::pallet_prelude::{BlockNumberFor, *};
    use frame_system::WeightInfo;
    use shp_session_keys::{InherentError, INHERENT_IDENTIFIER};
    use sp_runtime::traits::Saturating;
    use sp_std::{
        collections::{btree_set::BTreeSet},
        prelude::*,
    };

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(PhantomData<T>);

    /// Configuration trait of this pallet.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Overarching event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        type StorageProviderId: FullCodec + TypeInfo + Ord + Clone;

        /// Id of the seed
        type SeedId: FullCodec + TypeInfo;

        /// Commitment of a seed
        type SeedCommitment: FullCodec + TypeInfo;

        /// Randomness seed type
        type Seed: VerifiableSeed<SeedCommitment=Self::SeedCommitment> + FullCodec + TypeInfo + Clone;

        /// Get the BABE data from the runtime
        type RandomSeedMixer: RandomSeedMixer<<Self as Config>::Seed>;

        /// Size of the queue must be greater than or equal to tolerance size
        type QueueSize: Get<u32>;

        /// Weight info
        type WeightInfo: WeightInfo;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Event emitted when a new random seed is available from the relay chain
        NewRandomnessSeedGenerated {
            randomness_seed: String,
        },
    }

    /// End block (where the seed will be in effect) to set of seed commitment
    #[pallet::storage]
    type EndBlocks<T: Config> = StorageMap<_, Identity, BlockNumberFor<T>, Vec<T::StorageProviderId>, ValueQuery>;

    #[pallet::storage]
    type ReceivedCommitments<T: Config> = StorageMap<_, Identity, BlockNumberFor<T>, Vec<T::StorageProviderId>, ValueQuery>;

    #[pallet::storage]
    type ExpiredCommitments<T: Config> = StorageMap<_, Identity, BlockNumberFor<T>, Vec<T::StorageProviderId>, ValueQuery>;

    /// Seed commitment to end block number (when the seed must be submitted and at which seed must take effect)
    #[pallet::storage]
    type SeedCommitmentToEndBlockMapping<T: Config> = StorageMap<_, Identity, T::SeedCommitment, BlockNumberFor<T>, OptionQuery>;

    /// Commitments that we need to have seed for before next tick
    #[pallet::storage]
    type PendingCommitments<T: Config> = StorageMap<_, Identity, T::SeedCommitment, T::StorageProviderId, OptionQuery>;

    /// New seed commitments for storage provider ids for whom new tick is not yet started
    #[pallet::storage]
    type NewCommitments<T: Config> = StorageMap<_, Identity, T::StorageProviderId, T::SeedCommitment, OptionQuery>;

    /// Internal storage to manage queue elements
    #[pallet::storage]
    type RandomnessSeedsQueue<T: Config> = StorageValue<_, BoundedVec<T::Seed, T::QueueSize>, ValueQuery>;

    /// Internal storage to manage current head and tail of queue
    #[pallet::storage]
    type QueueParameters<T: Config> = StorageValue<_, (u32, u32), ValueQuery>;

    /// Bounded Queue which uses two storage value underneath to maintain logical queue.
    pub type BoundedQueue<T: Config> = queue::BoundedQueue<<T as frame_system::Config>::DbWeight, T::Seed, T::QueueSize, RandomnessSeedsQueue<T>, QueueParameters<T>>;


    impl<T: Config> Pallet<T> {
        fn add_randomness(sp_id: T::StorageProviderId, seed: T::Seed, corresponding_seed_commitment: T::SeedCommitment, new_seed_commitment: T::SeedCommitment) -> Weight {
            let is_valid_seed = seed.verify(corresponding_seed_commitment);
            if !is_valid_seed {
                // TODO: Handle the error
            }

            let maybe_end_block_number = SeedCommitmentToEndBlockMapping::<T>::take(corresponding_seed_commitment);
            let end_block_number = if let Some(block_number) = maybe_end_block_number {
                block_number
            } else {
                // Storage provider does not have new seed commitment (likely because they were slashed)
                // TODO: Have proper error handling in place
                return Weight::zero();
            };

            // Storage provider is late
            if end_block_number <= frame_system::Pallet::<T>::block_number() {
                // TODO: Return an error
            }

            PendingCommitments::<T>::remove(corresponding_seed_commitment);
            PendingCommitments::<T>::insert(new_seed_commitment, sp_id);

            ReceivedCommitments::<T>::append(end_block_number, sp_id);

            // Calculating new end block for new seed commitment
            let queue_size_in_block = BlockNumberFor::<T>::from(T::QueueSize::get());
            let new_end_block = frame_system::Pallet::<T>::block_number().saturating_add(tolerance_in_block);
            SeedCommitmentToEndBlockMapping::<T>::insert(new_seed_commitment, new_end_block);

            let distance_from_head = end_block_number.saturating_sub(frame_system::Pallet::<T>::block_number());
            BoundedQueue::<T>::overwrite_queue(|element| {
                *element = T::RandomSeedMixer::mix_randomness_seed(&element, &seed, None);
                // TODO: Put better weight
                (true, Weight::zero())
            }, distance_from_head)
        }

        fn get_head_randomness() -> (T::Seed, Weight) {
            BoundedQueue::<T>::head()
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// This hook is called on block initialization and returns the Weight of the `on_finalize` hook to
        /// let block builders know how much weight to reserve for it
        /// TODO: Benchmark on_finalize to get its weight and replace the placeholder weight for that
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            let queue_shift_weight = BoundedQueue::<T>::shift_queue();

            let block_to_target = now;
            let storage_provider_ids =  BTreeSet::from_iter(EndBlocks::<T>::take(block_to_target).into_iter());
            let received_commitments = BTreeSet::from_iter(ReceivedCommitments::<T>::take(block_to_target).into_iter());

            // Minus the set in order to find storage providers who did not submit
            let expired_commitments = storage_provider_ids.difference(&received_commitments).into_iter().cloned().collect();
            // TODO: Expired commitments should have reasonable bound to prevent bloating
            ExpiredCommitments::<T>::set(block_to_target, expired_commitments);

            queue_shift_weight
        }
    }
}
