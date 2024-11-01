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

    use super::*;
    use crate::{
        benchmark_proofs::{fetch_challenges, fetch_proof, get_provider_id, get_root, get_seed},
        pallet,
        types::{
            ChallengeTicksToleranceFor, KeyFor, MaxCustomChallengesPerBlockFor,
            MerkleTrieHashingFor, Proof, ProvidersPalletFor,
        },
        Call, ChallengesQueue, ChallengesTicker, Config, Event, LastCheckpointTick,
        LastTickProviderSubmittedAProofFor, Pallet, TickToChallengesSeed,
        TickToCheckpointChallenges, TickToProvidersDeadlines,
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

    #[benchmark]
    fn submit_proof() -> Result<(), BenchmarkError> {
        let n = 4;

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
        let custom_challenges = generate_challenges::<T>(n.clone());

        // Set the custom challenges in the last checkpoint challenge tick,
        // which in this case is going to be 1.
        let last_checkpoint_tick = 1u32.into();
        LastCheckpointTick::<T>::set(last_checkpoint_tick);
        TickToCheckpointChallenges::<T>::insert(last_checkpoint_tick, custom_challenges.clone());

        // Fetch proof for the challenged keys.
        let encoded_proof = fetch_proof(n);
        let proof =
            <Proof<T>>::decode(&mut encoded_proof.as_ref()).expect("Proof should be decodable");

        // Call some extrinsic.
        #[extrinsic_call]
        Pallet::submit_proof(RawOrigin::Signed(caller.clone()), proof, None);

        Ok(())
    }

    impl_benchmark_test_suite! {
            Pallet,
            crate::mock::new_test_ext(),
            crate::mock::Test,
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

            let custom_challenge = (typed_challenge, None);
            custom_challenges.push(custom_challenge);
        }
        BoundedVec::try_from(custom_challenges).expect("Length of custom challenges should be less than or equal to MaxCustomChallengesPerBlockFor")
    }
}
