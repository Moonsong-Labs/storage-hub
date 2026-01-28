#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use frame_support::{pallet, traits::Randomness};
pub use pallet::*;

mod queue;
mod types;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use crate::types::ProviderIdFor;
use shp_traits::CommitRevealRandomnessInterface;

pub trait RandomSeedMixer<Seed> {
    /// Mix randomness seed 1 and seed 2 to generate new seed.
    /// Optionally takes a context seed to further randomize the mixing.
    fn mix_randomness_seed(seed_1: &Seed, seed_2: &Seed, context: Option<impl Into<Seed>>) -> Seed;
}

pub trait SeedVerifier {
    type Seed;
    type SeedCommitment;
    /// Verifies if the seed commitment matches the seed
    fn verify(seed: &Self::Seed, seed_commitment: &Self::SeedCommitment) -> bool;
}

pub trait SeedGenerator {
    type Seed;
    /// Generates a new seed from a slice of u8s.
    fn generate_seed(generator: &[u8]) -> Self::Seed;
}

#[pallet]
pub mod pallet {
    use super::*;
    use alloc::{collections::BTreeSet, vec, vec::Vec};
    use codec::FullCodec;
    use frame_support::pallet_prelude::*;
    use frame_support::weights::WeightMeter;
    use frame_system::pallet_prelude::{BlockNumberFor, OriginFor};
    use frame_system::{ensure_root, ensure_signed, WeightInfo};
    use pallet_proofs_dealer::types::BalanceFor;
    use shp_traits::ReadChallengeableProvidersInterface;
    use sp_runtime::traits::{
        CheckEqual, CheckedDiv, Convert, Debug, MaybeDisplay, One, Saturating, SimpleBitOps, Zero,
    };
    use types::{
        CommitmentWithSeed, ProviderIdFor, ProvidersPalletFor, SeedCommitmentFor,
        StakeToBlockNumberFor,
    };

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(PhantomData<T>);

    /// Configuration trait of this pallet.
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_proofs_dealer::Config {
        /// Overarching event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Commitment of a seed
        type SeedCommitment: Parameter
            + Member
            + MaybeSerializeDeserialize
            + Debug
            + MaybeDisplay
            + SimpleBitOps
            + Ord
            + Default
            + Copy
            + CheckEqual
            + AsRef<[u8]>
            + AsMut<[u8]>
            + MaxEncodedLen
            + FullCodec;

        /// Randomness seed type
        type Seed: Parameter
            + Member
            + MaybeSerializeDeserialize
            + Debug
            + MaybeDisplay
            + SimpleBitOps
            + Ord
            + Default
            + Copy
            + CheckEqual
            + AsRef<[u8]>
            + AsMut<[u8]>
            + MaxEncodedLen
            + FullCodec;

        /// The verifier of seed commitments
        type SeedVerifier: SeedVerifier<Seed = Self::Seed, SeedCommitment = Self::SeedCommitment>;

        /// The generator of seeds
        type SeedGenerator: SeedGenerator<Seed = Self::Seed>;

        /// The seed mixer used to get fresh randomness
        type RandomSeedMixer: RandomSeedMixer<<Self as Config>::Seed>;

        /// Seed tolerance window
        #[pallet::constant]
        type MaxSeedTolerance: Get<u32>;

        /// The ratio to convert staked balance to the seed period.
        /// This is used to determine the period in which a Provider should reveal their previous randomness and commit a new seed, based on
        /// their stake. The period is calculated as `StakeToSeedPeriod / stake`, saturating at [`Config::MinSeedPeriod`].
        #[pallet::constant]
        type StakeToSeedPeriod: Get<BalanceFor<Self>>;

        /// The minimum period in which a Provider can be asked to reveal and commit seeds, regardless of their stake.
        #[pallet::constant]
        type MinSeedPeriod: Get<BlockNumberFor<Self>>;

        /// Weight info
        type WeightInfo: WeightInfo;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Event emitted when a Provider's CR cycle has been initialised. It has the information about its first deadline for a seed commitment.
        ProviderCycleInitialised {
            provider_id: ProviderIdFor<T>,
            first_seed_commitment_deadline_tick: BlockNumberFor<T>,
        },

        /// Event emitted when a Provider's CR cycle has been stopped. It has the information about the Provider ID.
        ProviderCycleStopped { provider_id: ProviderIdFor<T> },

        /// Event emitted when a Provider submits their first seed commitment.
        ProviderInitialisedRandomness {
            first_seed_commitment: T::SeedCommitment,
            next_deadline_tick: BlockNumberFor<T>,
        },

        /// Event emitted when a Provider gets marked as slashable for not submitting their seed reveal and new commitment in time.
        ProviderMarkedAsSlashable {
            provider_id: ProviderIdFor<T>,
            next_deadline: BlockNumberFor<T>,
        },

        /// Event emitted when a Provider correctly reveals their previous randomness seed and commits a new one.
        RandomnessCommitted {
            previous_randomness_revealed: T::Seed,
            valid_from_tick: BlockNumberFor<T>,
            new_seed_commitment: T::SeedCommitment,
            next_deadline_tick: BlockNumberFor<T>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Provider ID provided by the caller is not valid
        ProviderIdNotValid,
        /// Caller not owner of the Provider ID
        CallerNotOwner,
        /// Seed provided by the storage provider is not valid
        NotAValidSeed,
        /// We cannot find corresponding end tick for provided seed commitment
        NoEndTickForSeedCommitment,
        /// Provider is early in submitting the seed commitment
        EarlySubmissionOfSeed,
        /// Storage provider is late in submitting the seed commitment
        LateSubmissionOfSeed,
        /// Seed reveal is missing
        MissingSeedReveal,
        /// Seed commitment is already in the list of pending commitments
        NewCommitmentAlreadyPending,
        /// We are not able to convert tick number to u32 for arithmetic
        UnableToConvertTickNumberForArithmetic,
        /// We encountered an error while modifying seed queue
        QueueError(queue::QueueError),
    }

    /// A map that holds whether a Provider is active in the randomness system and as such should send seed commitments and reveals and
    /// can be slashed if they don't. It holds the next seed commitment that the Provider has to answer, if any.
    #[pallet::storage]
    pub type ActiveProviders<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        ProviderIdFor<T>,
        Option<SeedCommitmentFor<T>>,
        OptionQuery,
    >;

    /// A map from each deadline tick to a vector of the Providers that need to reveal their previous seed commitment and commit a new one in that tick
    #[pallet::storage]
    pub type DeadlineTickToProviders<T: Config> =
        StorageMap<_, Blake2_128Concat, BlockNumberFor<T>, Vec<ProviderIdFor<T>>, ValueQuery>;

    /// Commitments that are pending to be revealed by the Providers
    #[pallet::storage]
    pub type PendingCommitments<T: Config> =
        StorageMap<_, Blake2_128Concat, T::SeedCommitment, ProviderIdFor<T>, OptionQuery>;

    /// A map from each deadline tick to the Providers that have revealed their seed commitments for it
    #[pallet::storage]
    pub type ReceivedCommitments<T: Config> =
        StorageMap<_, Blake2_128Concat, BlockNumberFor<T>, Vec<ProviderIdFor<T>>, ValueQuery>;

    /// A map from each tick to the Providers that have to be marked as slashable in that tick. This will be processed by the `on_idle` hook
    #[pallet::storage]
    pub type ProvidersToMarkAsSlashable<T: Config> =
        StorageMap<_, Blake2_128Concat, BlockNumberFor<T>, Vec<ProviderIdFor<T>>, OptionQuery>;

    /// Seed commitment to its deadline tick
    #[pallet::storage]
    pub type SeedCommitmentToDeadline<T: Config> =
        StorageMap<_, Blake2_128Concat, T::SeedCommitment, BlockNumberFor<T>, OptionQuery>;

    /// A map from Providers which shouldn't send a reveal next tick, only a seed commitment be it because they
    /// have just registered or because they have been slashed, to their deadline tick
    #[pallet::storage]
    pub type ProvidersWithoutCommitment<T: Config> =
        StorageMap<_, Blake2_128Concat, ProviderIdFor<T>, BlockNumberFor<T>, OptionQuery>;

    /// The tick from which we should start checking for slashable Providers in the next `on_idle` execution
    #[pallet::storage]
    pub type TickToCheckForSlashableProviders<T: Config> =
        StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    /// Internal storage to manage queue elements
    #[pallet::storage]
    pub type RandomnessSeedsQueue<T: Config> =
        StorageValue<_, BoundedVec<T::Seed, T::MaxSeedTolerance>, ValueQuery>;

    /// Internal storage to manage current head and tail of queue
    #[pallet::storage]
    pub type QueueParameters<T: Config> = StorageValue<_, (u32, u32), ValueQuery>;

    /// Bounded Queue which uses two storage value underneath to maintain logical queue.
    pub type BoundedQueue<T> = queue::BoundedQueue<
        <T as frame_system::Config>::DbWeight,
        <T as crate::Config>::Seed,
        <T as crate::Config>::MaxSeedTolerance,
        RandomnessSeedsQueue<T>,
        QueueParameters<T>,
    >;

    // Genesis config:
    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub tick_to_start_checking_for_slashable_providers: BlockNumberFor<T>,
        pub initial_elements_for_randomness: BoundedVec<T::Seed, T::MaxSeedTolerance>,
    }

    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            let tick_to_start_checking_for_slashable_providers = Zero::zero();
            let queue_size: usize = T::MaxSeedTolerance::get() as usize; // `MaxSeedTolerance` has to fit into an usize
            let initial_elements_for_randomness =
                BoundedVec::truncate_from(vec![Default::default(); queue_size]);

            Self {
                tick_to_start_checking_for_slashable_providers,
                initial_elements_for_randomness,
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            TickToCheckForSlashableProviders::<T>::put(
                self.tick_to_start_checking_for_slashable_providers,
            );

            BoundedQueue::<T>::init(self.initial_elements_for_randomness.clone());
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn add_randomness(
            origin: OriginFor<T>,
            provider_id: ProviderIdFor<T>,
            commitment_with_seed_to_reveal: Option<CommitmentWithSeed<T>>,
            new_seed_commitment: T::SeedCommitment,
        ) -> DispatchResultWithPostInfo {
            // Check if the origin is signed and get the signer
            let who = ensure_signed(origin)?;

            // Check that the received Provider ID belongs to a Provider owned by the signer
            let owner_account =
                <ProvidersPalletFor<T> as ReadChallengeableProvidersInterface>::get_owner_account(
                    provider_id,
                )
                .ok_or(Error::<T>::ProviderIdNotValid)?;
            ensure!(owner_account == who, Error::<T>::CallerNotOwner);

            // Check that the Provider is active in the system
            ensure!(
                ActiveProviders::<T>::contains_key(provider_id),
                Error::<T>::ProviderIdNotValid
            );

            // Check that the new commitment is not on the pending commitments storage
            ensure!(
                !PendingCommitments::<T>::contains_key(new_seed_commitment),
                Error::<T>::NewCommitmentAlreadyPending
            );

            // Get the deadline tick for the seed commitment
            let (deadline, provider_without_previous_commitment) =
                match ProvidersWithoutCommitment::<T>::take(provider_id) {
                    // If the Provider is a first-time submitter or has been slashed, the deadline is the one stored in the ProvidersWithoutCommitment storage
                    Some(deadline) => (deadline, true),
                    None => {
                        // If not, the deadline is the one stored in the SeedCommitmentToDeadline storage
                        let seed_commitment_to_reveal = commitment_with_seed_to_reveal
                            .as_ref()
                            .ok_or(Error::<T>::MissingSeedReveal)?
                            .commitment;
                        let deadline =
                            SeedCommitmentToDeadline::<T>::take(seed_commitment_to_reveal)
                                .ok_or(Error::<T>::NoEndTickForSeedCommitment)?;
                        (deadline, false)
                    }
                };

            // Get the current tick
            let current_tick = pallet_proofs_dealer::ChallengesTicker::<T>::get();

            // Set up the next seed commitment
            let new_deadline = Self::set_up_next_seed(
                &provider_id,
                &deadline,
                &current_tick,
                &new_seed_commitment,
            )?;

            // If this Provider did not have a commitment from before
            if provider_without_previous_commitment {
                // Emit the ProviderInitialisedRandomness event
                Self::deposit_event(Event::ProviderInitialisedRandomness {
                    first_seed_commitment: new_seed_commitment,
                    next_deadline_tick: new_deadline,
                });
            } else {
                // If this Provider did have a pending commitment to reveal, the seed to reveal must have been sent
                let CommitmentWithSeed {
                    commitment: seed_commitment_to_reveal,
                    seed: seed_to_reveal,
                } = commitment_with_seed_to_reveal.ok_or(Error::<T>::MissingSeedReveal)?;

                // Verify that the received seed to be revealed matches the seed commitment
                ensure!(
                    <T as Config>::SeedVerifier::verify(
                        &seed_to_reveal,
                        &seed_commitment_to_reveal
                    ),
                    Error::<T>::NotAValidSeed
                );

                // If verification passed, remove the seed commitment from the pending commitments
                PendingCommitments::<T>::remove(seed_commitment_to_reveal);

                // Calculate the distance from the head of the queue that the deadline is, to know from where to start mixing the revealed seed
                // The head of the queue is the current tick, and the mixing should start from the deadline tick
                // Since at maximum the distance from the head is the max seed tolerance (which is a u32), we can safely convert it to u32
                let distance_from_head: u32 = deadline
                    .saturating_sub(current_tick)
                    .try_into()
                    .map_err(|_e| Error::<T>::UnableToConvertTickNumberForArithmetic)?;

                // Now, for each element in the queue from the distance from the head up to the tail, mix the seed with the revealed seed
                BoundedQueue::<T>::overwrite_queue(
                    &|element: &mut <T as Config>::Seed| {
                        *element = T::RandomSeedMixer::mix_randomness_seed(
                            element,
                            &seed_to_reveal,
                            None::<T::Seed>,
                        );
                        // TODO: Put better weight after benchmarking
                        (true, Weight::zero())
                    },
                    distance_from_head,
                )
                .map_err(|queue_error| Error::<T>::QueueError(queue_error))?;

                // Finally, emit the RandomnessCommitted event
                Self::deposit_event(Event::RandomnessCommitted {
                    previous_randomness_revealed: seed_to_reveal,
                    valid_from_tick: deadline,
                    new_seed_commitment,
                    next_deadline_tick: new_deadline,
                });
            }

            // If the extrinsic has succeeded, it shouldn't pay any fee. This is to incentivise
            // being a Provider in the network, since it diminishes the costs to be one substantially.
            Ok(Pays::No.into())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads(1))]
        pub fn force_initialise_provider_cycle(
            origin: OriginFor<T>,
            provider_id: ProviderIdFor<T>,
        ) -> DispatchResult {
            // Check that the origin is the Root
            ensure_root(origin)?;

            // Initialise the Provider cycle
            Self::initialise_provider_cycle(&provider_id)?;

            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads(1))]
        pub fn force_stop_provider_cycle(
            origin: OriginFor<T>,
            provider_id: ProviderIdFor<T>,
        ) -> DispatchResult {
            // Check that the origin is the Root
            ensure_root(origin)?;

            // Stop the Provider cycle
            Self::stop_provider_cycle(&provider_id)?;

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn get_head_randomness() -> (T::Seed, Weight) {
            BoundedQueue::<T>::head()
        }

        /// This function should be called when a new Provider is registered in the system, to initialise their seed commitment cycle
        pub fn initialise_provider_cycle(provider_id: &ProviderIdFor<T>) -> DispatchResult {
            // Get the current tick
            let current_tick = pallet_proofs_dealer::ChallengesTicker::<T>::get();

            // Calculate the seed period for the Provider
            let min_seed_period = T::MinSeedPeriod::get();
            let stake = <ProvidersPalletFor<T> as ReadChallengeableProvidersInterface>::get_stake(
                *provider_id,
            )
            .ok_or(Error::<T>::ProviderIdNotValid)?;
            let seed_period = match T::StakeToSeedPeriod::get().checked_div(&stake) {
                Some(period) => {
                    let seed_period = StakeToBlockNumberFor::<T>::convert(period);
                    min_seed_period.max(seed_period)
                }
                None => min_seed_period,
            };

            // Calculate the deadline tick for the seed commitment
            let seed_submission_tolerance = T::MaxSeedTolerance::get();
            let deadline = current_tick
                .saturating_add(seed_period)
                .saturating_add(seed_submission_tolerance.into());

            // Store the Provider in the ProvidersWithoutCommitment storage, since it does not have a commitment to reveal next cycle
            ProvidersWithoutCommitment::<T>::insert(provider_id, deadline);

            // Append the Provider to the deadline tick to Providers map
            DeadlineTickToProviders::<T>::append(deadline, provider_id);

            // Set the Provider as active in the randomness system, but with no seed commitment to answer
            ActiveProviders::<T>::insert(provider_id, None::<SeedCommitmentFor<T>>);

            // Emit the ProviderCycleInitialised event
            Self::deposit_event(Event::ProviderCycleInitialised {
                provider_id: *provider_id,
                first_seed_commitment_deadline_tick: deadline,
            });

            Ok(())
        }

        pub fn stop_provider_cycle(provider_id: &ProviderIdFor<T>) -> DispatchResult {
            // Make sure the Provider is currently active in the system
            ensure!(
                ActiveProviders::<T>::contains_key(provider_id),
                Error::<T>::ProviderIdNotValid
            );

            // Remove it from the ProvidersWithoutCommitment storage if it was there
            ProvidersWithoutCommitment::<T>::remove(provider_id);

            // Remove its next seed commitment from the seed commitment to deadline storage
            if let Some(Some(next_seed_commitment)) = ActiveProviders::<T>::get(provider_id) {
                SeedCommitmentToDeadline::<T>::remove(next_seed_commitment);
            }

            // Remove the Provider from the map of active Providers
            ActiveProviders::<T>::remove(provider_id);

            // Emit the ProviderCycleStopped event
            Self::deposit_event(Event::ProviderCycleStopped {
                provider_id: *provider_id,
            });

            Ok(())
        }

        fn set_up_next_seed(
            provider_id: &ProviderIdFor<T>,
            deadline: &BlockNumberFor<T>,
            current_tick: &BlockNumberFor<T>,
            next_seed_commitment: &SeedCommitmentFor<T>,
        ) -> Result<BlockNumberFor<T>, DispatchError> {
            // Calculate the tolerance window start for this deadline
            let seed_reveal_tolerance = T::MaxSeedTolerance::get();
            let tolerance_window_start = deadline.saturating_sub(seed_reveal_tolerance.into());

            // If the Provider is trying to send its next seed commitment before the tolerance window starts, they are early
            ensure!(
                *current_tick >= tolerance_window_start,
                Error::<T>::EarlySubmissionOfSeed
            );

            // If the deadline is in the past, the Provider is late in submitting their next seed commitment
            ensure!(current_tick < deadline, Error::<T>::LateSubmissionOfSeed);

            // Update the next commitment that this Provider has to answer
            ActiveProviders::<T>::insert(provider_id, Some(next_seed_commitment));
            // Add the new seed commitment to the pending commitments storage
            PendingCommitments::<T>::insert(*next_seed_commitment, *provider_id);
            // And append the Provider as a valid seed revealer for the deadline tick
            ReceivedCommitments::<T>::append(deadline, provider_id);

            // Calculate the Provider's seed period based on their current stake
            let min_seed_period = T::MinSeedPeriod::get();
            let stake = <ProvidersPalletFor<T> as ReadChallengeableProvidersInterface>::get_stake(
                *provider_id,
            )
            .ok_or(Error::<T>::ProviderIdNotValid)?;
            let seed_period = match T::StakeToSeedPeriod::get().checked_div(&stake) {
                Some(period) => {
                    let seed_period = StakeToBlockNumberFor::<T>::convert(period);
                    min_seed_period.max(seed_period)
                }
                None => min_seed_period,
            };

            // Calculate the deadline tick for the new seed commitment and store it
            let new_deadline = deadline.saturating_add(seed_period);
            SeedCommitmentToDeadline::<T>::insert(next_seed_commitment, new_deadline);

            // Append the Provider to the deadline tick to Providers map
            DeadlineTickToProviders::<T>::append(new_deadline, provider_id);

            Ok(new_deadline)
        }
    }

    /// Hooks of this pallet.
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        // TODO: Benchmark this and consume the weight correctly
        fn on_poll(_n: BlockNumberFor<T>, weight: &mut WeightMeter) {
            // First shift the queue, advancing one tick to the current one
            let queue_shift_weight = BoundedQueue::<T>::shift_queue();

            // Then, for the current tick, get the Providers that had to reveal their seed commitments
            let tick_to_target = pallet_proofs_dealer::ChallengesTicker::<T>::get();
            let due_providers_for_current_tick =
                BTreeSet::from_iter(DeadlineTickToProviders::<T>::take(tick_to_target));

            // And get the ones that actually submitted their seed commitments
            let providers_that_submitted =
                BTreeSet::from_iter(ReceivedCommitments::<T>::take(tick_to_target));

            // The difference between the sets are the Providers that did not submit their seed commitments
            let missing_providers: Vec<ProviderIdFor<T>> = due_providers_for_current_tick
                .difference(&providers_that_submitted)
                .cloned()
                .collect();

            // If there's any Provider that missed their randomness seed submission
            if !missing_providers.is_empty() {
                // Set the missing Providers to be marked as slashable in a future execution of the `on_idle` hook
                ProvidersToMarkAsSlashable::<T>::insert(tick_to_target, missing_providers.clone());
            }

            // Consume the weight used by this hook
            weight.consume(queue_shift_weight);
        }

        fn on_idle(_n: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
            // Initialise the variable that will hold the consumed weight of this hook
            let mut weight_meter = WeightMeter::with_limit(remaining_weight);

            // If there's not enough weight for the initial reads and writes that initialise
            // the `on_idle` hook processing, return
            if !weight_meter.can_consume(T::DbWeight::get().reads_writes(3, 1)) {
                return weight_meter.consumed();
            }

            // Get the next tick of this pallet
            let next_tick =
                pallet_proofs_dealer::ChallengesTicker::<T>::get().saturating_add(One::one());

            // Get the current tick to process
            let mut current_tick_to_process = TickToCheckForSlashableProviders::<T>::get();

            // Consume the weight used to initialise the `on_idle` hook processing
            weight_meter.consume(T::DbWeight::get().reads_writes(3, 1));

            // While there's enough weight to process at least one Provider AND the current tick to process is not greater than or equal to the next tick of this pallet,
            // process the Providers to mark as slashable
            while current_tick_to_process < next_tick
                && weight_meter.can_consume(T::DbWeight::get().reads_writes(1, 5))
            {
                // Check how many Providers have to be marked as slashable in this tick
                let providers_to_mark =
                    ProvidersToMarkAsSlashable::<T>::take(current_tick_to_process);

                // If there are any, process them, consuming the weight used to do so
                let should_advance_tick = if let Some(providers_to_mark) = providers_to_mark {
                    Self::process_providers_to_mark_as_slashable(
                        &mut weight_meter,
                        current_tick_to_process,
                        providers_to_mark,
                    )
                } else {
                    true
                };

                // Calculate the next tick to process
                if should_advance_tick {
                    current_tick_to_process = current_tick_to_process.saturating_add(One::one());
                }
            }

            // After this, the `current_tick_to_process` variable should hold the tick that has to be processed in the next `on_idle` hook execution
            // This is going to be either a tick that was left partially processed because of weight limitations, or the next tick of this pallet
            TickToCheckForSlashableProviders::<T>::put(current_tick_to_process);

            // Return the consumed weight of this hook
            weight_meter.consumed()
        }
    }

    /// Implementation of the internal functions of the `on_idle` hook.
    impl<T: Config> Pallet<T> {
        /// This function holds the logic that processes the Providers that have missed the deadline to reveal their seed commitments,
        /// marking them as slashable. If, because of weight limitations, it cannot fully process all Providers to mark as slashable in
        /// the current tick to process, it stores the remaining Providers to process them in the next execution.
        /// The function returns a boolean indicating if it has successfully processed all Providers to mark as slashable in the current tick.
        fn process_providers_to_mark_as_slashable(
            weight_meter: &mut WeightMeter,
            current_tick_to_process: BlockNumberFor<T>,
            providers_to_mark: Vec<ProviderIdFor<T>>,
        ) -> bool {
            // Iterate over the Providers to mark as slashable
            let mut current_provider_index = 0;
            let amount_of_providers_to_process = providers_to_mark.len();
            let seed_reveal_tolerance = T::MaxSeedTolerance::get();
            for provider_id in &providers_to_mark {
                // If there's not enough weight to process the next Provider (in the worst case scenario), break
                if !weight_meter.can_consume(T::DbWeight::get().reads_writes(1, 5)) {
                    break;
                }

                // Check if the Provider is active or inactive and modify storage accordingly
                ActiveProviders::<T>::mutate(
                    provider_id,
                    |maybe_active_provider_with_maybe_commitment| {
                        match maybe_active_provider_with_maybe_commitment {
                            // If the Provider is active in the system
                            Some(maybe_seed_commitment) => {
                                // Mark them as slashable
                                Self::mark_provider_as_slashable(provider_id);

                                // If they have a seed commitment that they had to answer
                                if let Some(seed_commitment) = maybe_seed_commitment {
                                    // Remove it from the seed commitment to deadline storage
                                    SeedCommitmentToDeadline::<T>::remove(seed_commitment);

                                    // And update the Provider to have no seed commitment to answer
                                    *maybe_seed_commitment = None;
                                }

                                // Add them to the DeadlineTickToProviders storage, with the seed tolerance as their new deadline
                                let new_deadline = current_tick_to_process
                                    .saturating_add(seed_reveal_tolerance.into());
                                DeadlineTickToProviders::<T>::append(new_deadline, provider_id);

                                // Add them to the ProvidersWithoutCommitment storage, with the seed tolerance as their new deadline
                                // This is done so they are not required to reveal their seed commitment in the next tick, only commit a new one
                                ProvidersWithoutCommitment::<T>::insert(provider_id, new_deadline);

                                // Emit the ProviderMarkedAsSlashable event
                                Self::deposit_event(Event::ProviderMarkedAsSlashable {
                                    provider_id: *provider_id,
                                    next_deadline: new_deadline,
                                });

                                // Consume the weight used to mark the Provider as slashable and reset it
                                weight_meter.consume(T::DbWeight::get().reads_writes(1, 5));
                            }
                            // If the Provider is not active in the system, delete them
                            None => {
                                // Remove it from the ProvidersWithoutCommitment storage if it was there
                                ProvidersWithoutCommitment::<T>::remove(provider_id);

                                // Consume the weight used to delete the Provider
                                weight_meter.consume(T::DbWeight::get().reads_writes(1, 2));
                            }
                        }
                    },
                );

                // Increment the current provider index
                current_provider_index += 1;
            }

            // If there are still Providers to mark as slashable, put them back in the storage and return false
            if current_provider_index < amount_of_providers_to_process {
                ProvidersToMarkAsSlashable::<T>::insert(
                    current_tick_to_process,
                    providers_to_mark[current_provider_index..].to_vec(),
                );

                false
            } else {
                // Otherwise, return true, since all Providers for this tick have been processed
                true
            }
        }

        /// This function marks a Provider as slashable, incrementing the number of accrued slashable events for them
        fn mark_provider_as_slashable(provider_id: &ProviderIdFor<T>) {
            // Missing a randomness seed submission has the same penalty as missing a proof submission
            pallet_proofs_dealer::SlashableProviders::<T>::mutate(provider_id, |accrued_slashes| {
                *accrued_slashes = Some(accrued_slashes.unwrap_or(0).saturating_add(1));
            });
        }
    }
}

impl<T: pallet::Config> CommitRevealRandomnessInterface for Pallet<T> {
    type ProviderId = ProviderIdFor<T>;

    fn initialise_randomness_cycle(
        who: &Self::ProviderId,
    ) -> frame_support::dispatch::DispatchResult {
        Self::initialise_provider_cycle(who)
    }

    fn stop_randomness_cycle(who: &Self::ProviderId) -> frame_support::dispatch::DispatchResult {
        Self::stop_provider_cycle(who)
    }
}

use crate::types::*;
use frame_system::pallet_prelude::BlockNumberFor;
pub struct CurrentBlockRandomness<T>(core::marker::PhantomData<T>);
impl<T: Config> Randomness<SeedFor<T>, BlockNumberFor<T>> for CurrentBlockRandomness<T> {
    /// Uses the current randomness stored in the head of the circular randomness queue alongside
    /// with a subject to generate a random seed that can be used for commitments from previous blocks.
    /// The subject is a byte array that is mixed with the current randomness to provide the final randomness.
    fn random(subject: &[u8]) -> (SeedFor<T>, BlockNumberFor<T>) {
        // Get the randomness for the current tick
        let (randomness, _) = pallet::Pallet::<T>::get_head_randomness();

        // Generate a seed from the subject
        let generated_seed = T::SeedGenerator::generate_seed(subject);

        // Mix the subject with the current randomness
        let final_randomness = <T as Config>::RandomSeedMixer::mix_randomness_seed(
            &randomness,
            &generated_seed,
            None::<T::Seed>,
        );

        // This randomness is valid for all ticks previous to the current one
        (
            final_randomness,
            pallet_proofs_dealer::ChallengesTicker::<T>::get(),
        )
    }

    /// Returns the current randomness seed, which is the head of the queue.
    fn random_seed() -> (SeedFor<T>, BlockNumberFor<T>) {
        // Get the randomness for the current tick
        let (randomness, _) = pallet::Pallet::<T>::get_head_randomness();

        // This randomness is valid for all ticks previous to the current one
        (
            randomness,
            pallet_proofs_dealer::ChallengesTicker::<T>::get(),
        )
    }
}
