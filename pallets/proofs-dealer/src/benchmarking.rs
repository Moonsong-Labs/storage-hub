//! Benchmarking setup for pallet-proofs-dealer

use frame_benchmarking::v2::*;

#[benchmarks(
    where
        // Runtime `T` implements, `pallet_balances::Config` `pallet_storage_providers::Config` and this pallet's `Config`.
        T: pallet_balances::Config + pallet_storage_providers::Config + crate::Config,
        // The Storage Providers pallet is the `Providers` pallet that this pallet requires.
        T: crate::Config<ProvidersPallet = pallet_storage_providers::Pallet<T>>,
        // The `Balances` pallet is the `NativeBalance` pallet that this pallet requires.
        T: crate::Config<NativeBalance = pallet_balances::Pallet<T>>,
        // The `Balances` pallet is the `NativeBalance` pallet that `pallet_storage_providers::Config` requires.
        T: pallet_storage_providers::Config<NativeBalance = pallet_balances::Pallet<T>>,
        // The `Proof` inner type of the `ForestVerifier` trait is `CompactProof`.
        <T as crate::Config>::ForestVerifier: shp_traits::CommitmentVerifier<Proof = sp_trie::CompactProof>,
        // The `Proof` inner type of the `KeyVerifier` trait is `CompactProof`.
        <<T as crate::Config>::KeyVerifier as shp_traits::CommitmentVerifier>::Proof: From<sp_trie::CompactProof>,
        // The Storage Providers pallet's `HoldReason` type can be converted into the Native Balance's `Reason`.
        pallet_storage_providers::HoldReason: Into<<<T as pallet::Config>::NativeBalance as frame_support::traits::fungible::InspectHold<<T as frame_system::Config>::AccountId>>::Reason>,
        // The Storage Providers `MerklePatriciaRoot` type is the same as `frame_system::Hash`.
        T: pallet_storage_providers::Config<MerklePatriciaRoot = <T as frame_system::Config>::Hash>,
        // The Storage Providers `ProviderId` type is the same as `frame_system::Hash`.
        T: pallet_storage_providers::Config<ProviderId = <T as frame_system::Config>::Hash>,
)]
mod benchmarks {
    use codec::Decode;
    use frame_support::{
        assert_ok,
        traits::{
            fungible::{Mutate, MutateHold},
            Get,
        },
    };
    use frame_system::{pallet_prelude::BlockNumberFor, RawOrigin};
    use shp_traits::{ReadChallengeableProvidersInterface, TrieRemoveMutation};
    use sp_runtime::{traits::Hash, BoundedVec};
    use sp_std::vec::Vec;
    use sp_weights::WeightMeter;

    use super::*;
    use crate::{
        benchmark_proofs::{fetch_challenges, fetch_proof, get_provider_id, get_root, get_seed},
        pallet,
        types::{
            ChallengeTicksToleranceFor, KeyFor, MaxCustomChallengesPerBlockFor,
            MerkleTrieHashingFor, Proof, ProvidersPalletFor,
        },
        Call, ChallengesQueue, ChallengesTicker, Config, Event, LastCheckpointTick,
        LastTickProviderSubmittedAProofFor, Pallet, SlashableProviders, TickToChallengesSeed,
        TickToCheckForSlashableProviders, TickToCheckpointChallenges, TickToProvidersDeadlines,
    };

    #[benchmark]
    fn challenge() -> Result<(), BenchmarkError> {
        // Setup initial conditions.
        let caller: T::AccountId = whitelisted_caller();
        let file_key = MerkleTrieHashingFor::<T>::hash(b"file_key");
        let user_balance = match 1_000_000_000_000_000u128.try_into() {
            Ok(balance) => balance,
            Err(_) => return Err(BenchmarkError::Stop("Balance conversion failed.")),
        };
        assert_ok!(<T as crate::Config>::NativeBalance::mint_into(
            &caller,
            user_balance,
        ));

        // Call some extrinsic.
        #[extrinsic_call]
        Pallet::challenge(RawOrigin::Signed(caller.clone()), file_key);

        // Verify the challenge event was emitted.
        let expected_event = <T as pallet::Config>::RuntimeEvent::from(Event::<T>::NewChallenge {
            who: caller,
            key_challenged: file_key,
        });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the challenge is in the queue.
        let challenges_queue = ChallengesQueue::<T>::get();
        assert_eq!(challenges_queue.len(), 1);
        assert_eq!(challenges_queue[0], file_key);

        Ok(())
    }

    /// > Assumptions:
    /// > - In the runtime configuration, [`T::MaxCustomChallengesPerBlock`] = [`T::RandomChallengesPerBlock`].
    /// > - For the purpose of this benchmark, [`T::MaxCustomChallengesPerBlock`] = 2 * [`T::RandomChallengesPerBlock`].
    /// > - This allows to "simulate" random challenges with checkpoint challenges, crafting them carefully to
    /// >   fall exactly where we need them, to benchmark a specific scenario.
    ///
    /// * Case: Up to {[`T::MaxCustomChallengesPerBlock`] * 2} file key proofs in proof.
    ///
    /// There are [`T::MaxCustomChallengesPerBlock`] random challenges, which can be responded with 1 to
    /// [`T::MaxCustomChallengesPerBlock`] * 2 file key proofs, depending on the Forest of the BSP and
    /// where the challenges fall within it. Additionally, in the worst case scenario for this amount
    /// of file key proofs, there can be [`T::MaxCustomChallengesPerBlock`] more file keys proven in the
    /// forest proof, that correspond to an exact match of a challenge with TrieRemoveMutation.
    /// File keys that would be removed from the Forest, are not meant to also send a file key proof, and
    /// that is the case for an exact match of a custom challenge with TrieRemoveMutation.
    #[benchmark]
    fn submit_proof_no_checkpoint_challenges_key_proofs(
        n: Linear<1, { T::MaxCustomChallengesPerBlock::get() }>,
    ) -> Result<(), BenchmarkError> {
        let file_key_proofs_count: u32 = n.into();
        let (caller, proof) = setup_submit_proof::<T>(file_key_proofs_count)?;

        // Call some extrinsic.
        #[extrinsic_call]
        Pallet::submit_proof(RawOrigin::Signed(caller.clone()), proof, None);

        Ok(())
    }

    /// > Assumptions:
    /// > - In the runtime configuration, [`T::MaxCustomChallengesPerBlock`] = [`T::RandomChallengesPerBlock`].
    /// > - For the purpose of this benchmark, [`T::MaxCustomChallengesPerBlock`] = 2 * [`T::RandomChallengesPerBlock`].
    /// > - This allows to "simulate" random challenges with checkpoint challenges, crafting them carefully to
    /// >   fall exactly where we need them, to benchmark a specific scenario.
    ///
    /// * Case: {[`T::MaxCustomChallengesPerBlock`] * 2 + 1} to {[`T::MaxCustomChallengesPerBlock`] * 4} file key proofs in proof.
    ///
    /// If there are more than {[`T::MaxCustomChallengesPerBlock`] * 2} file key proofs, then it means that
    /// some of those file key proofs are a response to checkpoint challenges, so it is now impossible to
    /// have [`T::MaxCustomChallengesPerBlock`] file keys proven to be removed from the Forest. For example,
    /// if {[`T::MaxCustomChallengesPerBlock`] = 10} and there are 21 file key proofs, then at least one of those
    /// file keys proven is a consequence of a checkpoint challenge either not falling exactly in an existing
    /// leaf, or not having a TrieRemoveMutation. So the worst case scenario for 21 file keys proven is
    /// another 9 file keys proven with a TrieRemoveMutation. For 22 file keys proven, the worst case scenario
    /// is also 9 file keys proven with a TrieRemoveMutation. For 23, 8 file keys proven with a TrieRemoveMutation.
    /// For 24, also 8 file keys proven with a TrieRemoveMutation. It continues like this until with 40 file keys
    /// proven, the worst case scenario is 0 file keys proven with a TrieRemoveMutation. Basically, with 40 file
    /// keys proven, it means that there are 2 file keys proven for every random and checkpoint challenge, so no
    /// checkpoint challenge fell exactly in an existing leaf.
    #[benchmark]
    fn submit_proof_with_checkpoint_challenges_key_proofs(
        n: Linear<
            { T::MaxCustomChallengesPerBlock::get() + 1 },
            { T::MaxCustomChallengesPerBlock::get() * 2 },
        >,
    ) -> Result<(), BenchmarkError> {
        let file_key_proofs_count: u32 = n.into();
        let (caller, proof) = setup_submit_proof::<T>(file_key_proofs_count)?;

        // Call some extrinsic.
        #[extrinsic_call]
        Pallet::submit_proof(RawOrigin::Signed(caller.clone()), proof, None);

        Ok(())
    }

    /// > Assumptions:
    /// > - This is not a checkpoint challenge round. That function is benchmarked separately.
    #[benchmark]
    fn new_challenges_round(
        n: Linear<1, { T::MaxSlashableProvidersPerTick::get() }>,
    ) -> Result<(), BenchmarkError> {
        let slashable_providers_count: u32 = n.into();
        register_providers::<T>(slashable_providers_count)?;

        // Check that there are no slashable Providers before the execution.
        let slashable_providers_count_before = SlashableProviders::<T>::iter().count();
        assert_eq!(slashable_providers_count_before, 0);

        let mut meter: WeightMeter = WeightMeter::new();
        #[block]
        {
            Pallet::<T>::do_new_challenges_round(&mut meter);
        }

        // Check that the slashable Providers are updated to be `n` after the execution.
        let slashable_providers_count_after = SlashableProviders::<T>::iter().count();
        assert_eq!(
            slashable_providers_count_after,
            slashable_providers_count as usize
        );

        Ok(())
    }

    impl_benchmark_test_suite! {
            Pallet,
            crate::mock::new_test_ext(),
            crate::mock::Test,
    }

    fn setup_submit_proof<T>(n: u32) -> Result<(T::AccountId, Proof<T>), BenchmarkError>
    where
    // Runtime `T` implements, `pallet_balances::Config` `pallet_storage_providers::Config` and this pallet's `Config`.
        T: pallet_balances::Config + pallet_storage_providers::Config + crate::Config,
    // The Storage Providers pallet is the `Providers` pallet that this pallet requires.
        T: crate::Config<ProvidersPallet = pallet_storage_providers::Pallet<T>>,
    // The `Balances` pallet is the `NativeBalance` pallet that this pallet requires.
        T: crate::Config<NativeBalance = pallet_balances::Pallet<T>>,
    // The `Balances` pallet is the `NativeBalance` pallet that `pallet_storage_providers::Config` requires.
        T: pallet_storage_providers::Config<NativeBalance = pallet_balances::Pallet<T>>,
    // The `Proof` inner type of the `ForestVerifier` trait is `CompactProof`.
        <T as crate::Config>::ForestVerifier: shp_traits::CommitmentVerifier<Proof = sp_trie::CompactProof>,
    // The `Proof` inner type of the `KeyVerifier` trait is `CompactProof`.
        <<T as crate::Config>::KeyVerifier as shp_traits::CommitmentVerifier>::Proof: From<sp_trie::CompactProof>,
    // The Storage Providers pallet's `HoldReason` type can be converted into the Native Balance's `Reason`.
        pallet_storage_providers::HoldReason: Into<<<T as pallet::Config>::NativeBalance as frame_support::traits::fungible::InspectHold<<T as frame_system::Config>::AccountId>>::Reason>,
    // The Storage Providers `MerklePatriciaRoot` type is the same as `frame_system::Hash`.
        T: pallet_storage_providers::Config<MerklePatriciaRoot = <T as frame_system::Config>::Hash>,
    // The Storage Providers `ProviderId` type is the same as `frame_system::Hash`.
        T: pallet_storage_providers::Config<ProviderId = <T as frame_system::Config>::Hash>,
    {
        // Setup initial conditions.
        let caller: T::AccountId = whitelisted_caller();
        let provider_balance = match 1_000_000_000_000_000u128.try_into() {
            Ok(balance) => balance,
            Err(_) => return Err(BenchmarkError::Stop("Balance conversion failed.")),
        };
        assert_ok!(<T as crate::Config>::NativeBalance::mint_into(
            &caller,
            provider_balance,
        ));

        // Register caller as a Provider in Providers pallet.
        let encoded_provider_id = get_provider_id();
        let provider_id =
            <T as frame_system::Config>::Hash::decode(&mut encoded_provider_id.as_ref())
                .expect("Failed to decode provider ID from bytes.");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<T>::insert(
            &caller,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<T>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                capacity_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: caller.clone(),
                payment_account: caller.clone(),
                reputation_weight:
                    <T as pallet_storage_providers::Config>::StartingReputationWeight::get(),
                sign_up_block: Default::default(),
            },
        );

        // Hold some of the Provider's balance so it simulates it having a stake.
        assert_ok!(<T as crate::Config>::NativeBalance::hold(
            &pallet_storage_providers::HoldReason::StorageProviderDeposit.into(),
            &caller,
            provider_balance / 100u32.into(),
        ));

        // Set Provider's root to be the one that matches the proofs that will be submitted.
        let encoded_root = get_root();
        let root = <T as frame_system::Config>::Hash::decode(&mut encoded_root.as_ref())
            .expect("Root should be decodable as it is a hash");
        pallet_storage_providers::BackupStorageProviders::<T>::mutate(&provider_id, |provider| {
            provider.as_mut().expect("Provider should exist").root = root;
        });

        // Set Provider's last submitted proof block.
        let current_tick = ChallengesTicker::<T>::get();
        let last_tick_provider_submitted_proof = current_tick;
        LastTickProviderSubmittedAProofFor::<T>::insert(
            &provider_id,
            last_tick_provider_submitted_proof,
        );

        // Set Provider's deadline for submitting a proof.
        // It is the sum of this Provider's challenge period and the `ChallengesTicksTolerance`.
        let providers_stake =
            <ProvidersPalletFor<T> as ReadChallengeableProvidersInterface>::get_stake(provider_id)
                .unwrap();
        let challenge_period = crate::Pallet::<T>::stake_to_challenge_period(providers_stake);
        let challenge_ticks_tolerance: BlockNumberFor<T> = ChallengeTicksToleranceFor::<T>::get();
        let challenge_period_plus_tolerance = challenge_period + challenge_ticks_tolerance;
        let prev_deadline = current_tick + challenge_period_plus_tolerance;
        TickToProvidersDeadlines::<T>::insert(prev_deadline, provider_id, ());

        // Advance to the next challenge the Provider should listen to.
        let providers_stake =
            <ProvidersPalletFor<T> as ReadChallengeableProvidersInterface>::get_stake(provider_id)
                .unwrap();
        let challenge_period = crate::Pallet::<T>::stake_to_challenge_period(providers_stake);
        let current_block = frame_system::Pallet::<T>::block_number();
        let challenge_block = current_block + challenge_period;
        frame_system::Pallet::<T>::set_block_number(challenge_block);
        // Advance less than `ChallengeTicksTolerance` blocks.
        let challenge_ticks_tolerance: BlockNumberFor<T> = ChallengeTicksToleranceFor::<T>::get();
        let current_block = frame_system::Pallet::<T>::block_number();
        frame_system::Pallet::<T>::set_block_number(
            current_block + challenge_ticks_tolerance - 1u32.into(),
        );

        // Manually set the current tick.
        ChallengesTicker::<T>::set(frame_system::Pallet::<T>::block_number());

        // Set the seed for the challenge block.
        let encoded_seed = get_seed();
        let seed = <T as frame_system::Config>::Hash::decode(&mut encoded_seed.as_ref())
            .expect("Seed should be decodable as it is a hash");
        TickToChallengesSeed::<T>::insert(challenge_block, seed);

        // Calculate the custom challenges to respond to, so that we can generate a proof for each.
        let custom_challenges = generate_challenges::<T>(n);

        // Set the custom challenges in the last checkpoint challenge tick,
        // which in this case is going to be 1.
        let last_checkpoint_tick = 1u32.into();
        LastCheckpointTick::<T>::set(last_checkpoint_tick);
        TickToCheckpointChallenges::<T>::insert(last_checkpoint_tick, custom_challenges.clone());

        // Fetch proof for the challenged keys.
        let encoded_proof = fetch_proof(n);
        let proof =
            <Proof<T>>::decode(&mut encoded_proof.as_ref()).expect("Proof should be decodable");

        // Check that the proof has the expected number of file key proofs.
        assert_eq!(proof.key_proofs.len() as u32, n);

        Ok((caller, proof))
    }

    fn generate_challenges<T: Config>(
        n: u32,
    ) -> BoundedVec<(KeyFor<T>, Option<TrieRemoveMutation>), MaxCustomChallengesPerBlockFor<T>>
    {
        let encoded_challenges = fetch_challenges(n);
        let mut custom_challenges = Vec::new();
        for encoded_challenge in encoded_challenges {
            let typed_challenge =
                <T as crate::Config>::MerkleTrieHash::decode(&mut encoded_challenge.as_ref())
                    .expect("Challenge key should be decodable as it is a hash");

            let custom_challenge = (typed_challenge, Some(TrieRemoveMutation::default()));
            custom_challenges.push(custom_challenge);
        }
        BoundedVec::try_from(custom_challenges).expect("Length of custom challenges should be less than or equal to MaxCustomChallengesPerBlockFor")
    }

    fn register_providers<T>(n: u32) -> Result<(), BenchmarkError>
    where
    // Runtime `T` implements, `pallet_balances::Config` `pallet_storage_providers::Config` and this pallet's `Config`.
        T: pallet_balances::Config + pallet_storage_providers::Config + crate::Config,
    // The Storage Providers pallet is the `Providers` pallet that this pallet requires.
        T: crate::Config<ProvidersPallet = pallet_storage_providers::Pallet<T>>,
    // The `Balances` pallet is the `NativeBalance` pallet that this pallet requires.
        T: crate::Config<NativeBalance = pallet_balances::Pallet<T>>,
    // The `Balances` pallet is the `NativeBalance` pallet that `pallet_storage_providers::Config` requires.
        T: pallet_storage_providers::Config<NativeBalance = pallet_balances::Pallet<T>>,
    // The `Proof` inner type of the `ForestVerifier` trait is `CompactProof`.
        <T as crate::Config>::ForestVerifier: shp_traits::CommitmentVerifier<Proof = sp_trie::CompactProof>,
    // The `Proof` inner type of the `KeyVerifier` trait is `CompactProof`.
        <<T as crate::Config>::KeyVerifier as shp_traits::CommitmentVerifier>::Proof: From<sp_trie::CompactProof>,
    // The Storage Providers pallet's `HoldReason` type can be converted into the Native Balance's `Reason`.
        pallet_storage_providers::HoldReason: Into<<<T as pallet::Config>::NativeBalance as frame_support::traits::fungible::InspectHold<<T as frame_system::Config>::AccountId>>::Reason>,
    // The Storage Providers `MerklePatriciaRoot` type is the same as `frame_system::Hash`.
        T: pallet_storage_providers::Config<MerklePatriciaRoot = <T as frame_system::Config>::Hash>,
    // The Storage Providers `ProviderId` type is the same as `frame_system::Hash`.
        T: pallet_storage_providers::Config<ProviderId = <T as frame_system::Config>::Hash>,
    {
        let tick_to_check_for_slashable_providers = TickToCheckForSlashableProviders::<T>::get();
        for i in 0..n {
            // Setup initial conditions.
            let provider_account: T::AccountId = account("provider_account", i as u32, i);
            let provider_balance = match 1_000_000_000_000_000u128.try_into() {
                Ok(balance) => balance,
                Err(_) => return Err(BenchmarkError::Stop("Balance conversion failed.")),
            };
            assert_ok!(<T as crate::Config>::NativeBalance::mint_into(
                &provider_account,
                provider_balance,
            ));

            // Register caller as a Provider in Providers pallet.
            let provider_id = <T as frame_system::Config>::Hashing::hash(
                sp_runtime::format!("provider_id_{:?}", i).as_bytes(),
            );
            pallet_storage_providers::AccountIdToBackupStorageProviderId::<T>::insert(
                &provider_account,
                provider_id,
            );
            pallet_storage_providers::BackupStorageProviders::<T>::insert(
                &provider_id,
                pallet_storage_providers::types::BackupStorageProvider {
                    capacity: Default::default(),
                    capacity_used: Default::default(),
                    multiaddresses: Default::default(),
                    root: Default::default(),
                    last_capacity_change: Default::default(),
                    owner_account: provider_account.clone(),
                    payment_account: provider_account.clone(),
                    reputation_weight:
                        <T as pallet_storage_providers::Config>::StartingReputationWeight::get(),
                    sign_up_block: Default::default(),
                },
            );

            // Hold some of the Provider's balance so it simulates it having a stake.
            assert_ok!(<T as crate::Config>::NativeBalance::hold(
                &pallet_storage_providers::HoldReason::StorageProviderDeposit.into(),
                &provider_account,
                provider_balance / 100u32.into(),
            ));

            // Add Provider to the next deadline to check.
            TickToProvidersDeadlines::<T>::insert(
                tick_to_check_for_slashable_providers,
                provider_id,
                (),
            );
        }

        Ok(())
    }
}
