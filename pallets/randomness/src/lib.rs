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

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

/// Read babe randomness info from the relay chain state proof
pub trait GetBabeData<EpochIndex, Randomness> {
    fn get_epoch_index() -> EpochIndex;
    fn get_epoch_randomness() -> Randomness;
    fn get_parent_randomness() -> Randomness;
}

#[pallet]
pub mod pallet {
    use super::{weights::WeightInfo, *};
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::{BlockNumberFor, *};
    use shp_session_keys::{InherentError, INHERENT_IDENTIFIER};
    use sp_runtime::traits::{BlockNumberProvider, Saturating};

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    /// Configuration trait of this pallet.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Overarching event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Get the BABE data from the runtime
        type BabeDataGetter: GetBabeData<u64, Self::Hash>;

        /// Interface to get the relay (or other) chain block number which was used as an anchor for the last block in the parachain.
        type BabeBlockGetter: BlockNumberProvider<BlockNumber = Self::BabeDataGetterBlockNumber>;
        // CurrentBlockNumber

        /// Weight info
        type WeightInfo: crate::weights::WeightInfo;

        /// Get chain block number (relay or other)
        type BabeDataGetterBlockNumber: sp_runtime::traits::BlockNumber;
    }

    /// The events that can be emitted by this pallet.
    ///
    /// # Event Encoding Stability
    ///
    /// All event variants use explicit `#[codec(index = N)]` to ensure stable SCALE encoding/decoding
    /// across runtime upgrades.
    ///
    /// These indices must NEVER be changed or reused. Any breaking changes to errors must be
    /// introduced as new variants (append-only) to ensure backward and forward compatibility.
    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Event emitted when a new random seed is available from the relay chain
        #[codec(index = 0)]
        NewOneEpochAgoRandomnessAvailable {
            randomness_seed: T::Hash,
            from_epoch: u64,
            valid_until_block: BlockNumberFor<T>,
        },
    }

    /// Latest random seed obtained from the one epoch ago randomness from BABE, and the latest block that it can process randomness requests from
    #[pallet::storage]
    pub type LatestOneEpochAgoRandomness<T: Config> = StorageValue<_, (T::Hash, BlockNumberFor<T>)>;

    /// Latest random seed obtained from the parent block randomness from BABE, and the latest block that it can process randomness requests from
    #[pallet::storage]
    pub type LatestParentBlockRandomness<T: Config> = StorageValue<_, (T::Hash, BlockNumberFor<T>)>;

    /// Current relay epoch
    #[pallet::storage]
    pub(crate) type RelayEpoch<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// The relay chain block (and anchored parachain block) to use when epoch changes
    #[pallet::storage]
    pub(crate) type LastRelayBlockAndParaBlockValidForNextEpoch<T: Config> =
        StorageValue<_, (T::BabeDataGetterBlockNumber, BlockNumberFor<T>), ValueQuery>;

    /// Ensures the mandatory inherent was included in the block
    #[pallet::storage]
    pub type InherentIncluded<T: Config> = StorageValue<_, ()>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// This inherent that must be included (DispatchClass::Mandatory) at each block saves the latest randomness available from the
        /// relay chain into a variable that can then be used as a seed for commitments that happened during
        /// the previous relay chain epoch
        #[pallet::call_index(0)]
        #[pallet::weight((
			T::WeightInfo::set_babe_randomness(),
			DispatchClass::Mandatory
		))]
        pub fn set_babe_randomness(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            // Make sure this is included in the block as an inherent, unsigned
            ensure_none(origin)?;

            // Update the randomness from the parent block
            let parent_randomness = T::BabeDataGetter::get_parent_randomness();
            let two_blocks: BlockNumberFor<T> = BlockNumberFor::<T>::default()
                + sp_runtime::traits::One::one()
                + sp_runtime::traits::One::one();
            let latest_valid_block =
                frame_system::Pallet::<T>::block_number().saturating_sub(two_blocks);
            LatestParentBlockRandomness::<T>::put((parent_randomness, latest_valid_block));

            // Get the last relay epoch index for which the randomness has been processed
            let last_relay_epoch_index = <RelayEpoch<T>>::get();

            // Get the current epoch of the relay chain
            let relay_epoch_index = T::BabeDataGetter::get_epoch_index();

            // If the current epoch is greater than the one for which the randomness was last processed for
            if relay_epoch_index > last_relay_epoch_index {
                // Get the new randomness of this new epoch
                let epoch_randomness = T::BabeDataGetter::get_epoch_randomness();
                // The latest BABE randomness is predictable during the current epoch and this inherent
                // must be executed and included in every block, which means that iff this logic is being
                // executed, the epoch JUST changed, so the obtained randomness is valid for every block of the
                // parachain that was anchored to a relay chain block that's not the current one nor the last one.
                let (_second_to_last_relay_anchor, latest_valid_block_for_randomness) =
                    LastRelayBlockAndParaBlockValidForNextEpoch::<T>::get();

                // Save it to be readily available for use
                LatestOneEpochAgoRandomness::<T>::put((
                    epoch_randomness,
                    latest_valid_block_for_randomness,
                ));

                // Update storage with the latest epoch for which randomness was processed for
                <RelayEpoch<T>>::put(relay_epoch_index);

                // Emit an event detailing that a new randomness is available
                Self::deposit_event(Event::NewOneEpochAgoRandomnessAvailable {
                    randomness_seed: epoch_randomness,
                    from_epoch: relay_epoch_index,
                    valid_until_block: latest_valid_block_for_randomness,
                });
            }

            // Update the last relay block and parachain block anchored to it to have ready for next block in case epoch changes
            let previous_relay_block = T::BabeBlockGetter::current_block_number(); // This returns the relay chain block anchor for the PREVIOUS parachain block
            let previous_parachain_block = frame_system::Pallet::<T>::block_number()
                .saturating_sub(sp_runtime::traits::One::one());
            LastRelayBlockAndParaBlockValidForNextEpoch::<T>::put((
                previous_relay_block,
                previous_parachain_block,
            ));

            // Update storage to reflect that this inherent was included in the block (so the block is valid)
            <InherentIncluded<T>>::put(());

            // Inherents do not pay for execution
            Ok(Pays::No.into())
        }
    }

    /// Implement the required trait to provide an inherent to the runtime
    #[pallet::inherent]
    impl<T: Config> ProvideInherent for Pallet<T> {
        type Call = Call<T>;
        type Error = InherentError;
        const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

        // This function returns if the inherent should be added to the current block or not
        fn is_inherent_required(_: &InherentData) -> Result<Option<Self::Error>, Self::Error> {
            // Return Ok(Some(_)) unconditionally because this inherent is required in every block
            // If it is not found, throw a InherentRequired error.
            Ok(Some(InherentError::Other(
                "Inherent required to set babe randomness".into(),
            )))
        }

        // The empty-payload inherent extrinsic.
        fn create_inherent(_data: &InherentData) -> Option<Self::Call> {
            Some(Call::set_babe_randomness {})
        }

        fn is_inherent(call: &Self::Call) -> bool {
            matches!(call, Call::set_babe_randomness { .. })
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// This hook is called on block initialization and returns the Weight of the `on_finalize` hook to
        /// let block builders know how much weight to reserve for it
        fn on_initialize(_now: BlockNumberFor<T>) -> Weight {
            T::WeightInfo::on_finalize_hook()
        }
        /// This hook checks, on block finalization, that the required inherent was included and clears
        /// storage to make it necessary to include it in future blocks as well
        fn on_finalize(_now: BlockNumberFor<T>) {
            // Ensure the mandatory inherent was included in the block or the block is invalid
            // We use take() to make sure this is storage is not set for the next block
            assert!(
				<InherentIncluded<T>>::take().is_some(),
				"Mandatory randomness inherent not included; InherentIncluded storage item is empty"
			);
        }
    }

    // Read-only functions
    impl<T: Config> Pallet<T> {
        /// Get the latest BABE randomness seed from one epoch ago and the latest block for which it's valid
        pub fn latest_babe_randomness() -> Option<(T::Hash, BlockNumberFor<T>)> {
            LatestOneEpochAgoRandomness::<T>::get()
        }

        /// Get the latest parent block randomness seed and the latest block for which it's valid
        pub fn latest_parent_block_randomness() -> Option<(T::Hash, BlockNumberFor<T>)> {
            LatestParentBlockRandomness::<T>::get()
        }

        /// Get the latest relay epoch processed
        pub fn relay_epoch() -> u64 {
            RelayEpoch::<T>::get()
        }

        /// Get the variable that's used to check if the mandatory BABE inherent was included in the block
        pub fn inherent_included() -> Option<()> {
            InherentIncluded::<T>::get()
        }
    }
}

use frame_support::traits::Randomness as RandomnessT;
use frame_system::pallet_prelude::BlockNumberFor;
use scale_info::prelude::vec::Vec;
use sp_runtime::traits::Hash;
pub struct RandomnessFromOneEpochAgo<T>(core::marker::PhantomData<T>);

impl<T: Config> RandomnessT<T::Hash, BlockNumberFor<T>> for RandomnessFromOneEpochAgo<T> {
    /// Uses the BABE randomness of this epoch to generate a random seed that can be used
    /// for commitments from the last epoch. The provided `subject` MUST have been committed
    /// AT LEAST during the last epoch for the result of this function to not be predictable
    ///
    /// The subject is a byte array that is hashed (to make it a fixed size) and then concatenated with
    /// the latest BABE randomness. The result is then hashed again to provide the final randomness.
    fn random(subject: &[u8]) -> (T::Hash, BlockNumberFor<T>) {
        // If there's randomness available
        if let Some((babe_randomness, latest_valid_block)) = LatestOneEpochAgoRandomness::<T>::get()
        {
            let hashed_subject = T::Hashing::hash(subject);
            let mut digest = Vec::new();
            // Concatenate the latest randomness with the hashed subject
            digest.extend_from_slice(babe_randomness.as_ref());
            digest.extend_from_slice(hashed_subject.as_ref());
            // Hash it
            let randomness = T::Hashing::hash(digest.as_slice());
            // Return the randomness for this subject and the latest block for which this randomness is useful
            // `subject` commitments done after `latest_valid_block` are predictable, and as such MUST be discarded
            (randomness, latest_valid_block)
        } else {
            // If there's no randomness available, return an empty randomness that's invalid for every block
            let randomness = T::Hash::default();
            let latest_valid_block: BlockNumberFor<T> = sp_runtime::traits::Zero::zero();
            (randomness, latest_valid_block)
        }
    }
}
pub struct ParentBlockRandomness<T>(core::marker::PhantomData<T>);

impl<T: Config> RandomnessT<T::Hash, BlockNumberFor<T>> for ParentBlockRandomness<T> {
    /// Uses the BABE randomness of two epochs ago in combination with the parent's block randomness
    /// to generate a random seed that can be used for commitments from previous blocks. Take extreme
    /// care, as the block producer can predict this randomness.
    ///
    /// The subject is a byte array that is hashed (to make it a fixed size) and then concatenated with
    /// the latest parent block randomness. The result is then hashed again to provide the final randomness.
    fn random(subject: &[u8]) -> (T::Hash, BlockNumberFor<T>) {
        // If there's randomness available
        if let Some((parent_block_randomness, latest_valid_block)) =
            LatestParentBlockRandomness::<T>::get()
        {
            let hashed_subject = T::Hashing::hash(subject);
            let mut digest = Vec::new();
            // Concatenate the latest randomness with the hashed subject
            digest.extend_from_slice(parent_block_randomness.as_ref());
            digest.extend_from_slice(hashed_subject.as_ref());
            // Hash it
            let randomness = T::Hashing::hash(digest.as_slice());
            // Return the randomness for this subject and the latest block for which this randomness is useful
            // `subject` commitments done after `latest_valid_block` are predictable, and as such MUST be discarded
            (randomness, latest_valid_block)
        } else {
            // If there's no randomness available, return an empty randomness that's invalid for every block
            let randomness = T::Hash::default();
            let latest_valid_block: BlockNumberFor<T> = sp_runtime::traits::Zero::zero();
            (randomness, latest_valid_block)
        }
    }
}
