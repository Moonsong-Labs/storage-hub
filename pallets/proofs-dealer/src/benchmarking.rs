//! Benchmarking setup for pallet-proofs-dealer
use frame_benchmarking::v2::*;

#[benchmarks]
mod benchmarks {
    use frame_support::{assert_ok, traits::fungible::Mutate};
    use frame_system::RawOrigin;
    use sp_runtime::traits::Hash;

    use super::*;
    use crate::{
        pallet, types::MerkleTrieHashingFor, Call, ChallengesQueue, Config, Event, Pallet,
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

    impl_benchmark_test_suite! {
            Pallet,
            crate::mock::new_test_ext(),
            crate::mock::Test,
    }
}
