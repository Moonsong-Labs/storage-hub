use crate::mock::*;
use crate::pallet::Event;
use frame_support::{assert_noop, assert_ok, traits::fungible::Mutate};
use sp_core::{Get, Hasher};
use sp_runtime::{traits::BlakeTwo256, DispatchError};

fn run_n_blocks(n: u64) {
    while System::block_number() < n {
        System::set_block_number(System::block_number() + 1);
        // Trigger any on_initialize or on_finalize logic here.
        // TODO: Add `on_initialize` trigger.
    }
}

#[test]
fn challenge_submit_succeed() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        System::set_block_number(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(1);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));

        // Mock a FileKey.
        let file_key = BlakeTwo256::hash(b"file_key");

        // Dispatch challenge extrinsic.
        assert_ok!(ProofsDealer::challenge(RuntimeOrigin::signed(1), file_key));

        // Check that the event is emitted.
        System::assert_last_event(
            Event::NewChallenge {
                who: 1,
                key_challenged: file_key,
            }
            .into(),
        );

        // Check user's balance after challenge.
        let challenge_fee: u128 = <Test as crate::Config>::ChallengesFee::get();
        assert_eq!(
            <Test as crate::Config>::NativeBalance::usable_balance(&1),
            user_balance - challenge_fee
        );

        // Check that the challenge is in the queue.
        let challenges_queue = crate::ChallengesQueue::<Test>::get();
        assert_eq!(challenges_queue.len(), 1);
        assert_eq!(challenges_queue[0], file_key);
    });
}

#[test]
fn challenge_submit_twice_succeed() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        System::set_block_number(1);

        // Create two users and add funds to the accounts.
        let user_1 = RuntimeOrigin::signed(1);
        let user_2 = RuntimeOrigin::signed(2);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &2,
            user_balance
        ));

        // Mock two FileKeys.
        let file_key_1 = BlakeTwo256::hash(b"file_key_1");
        let file_key_2 = BlakeTwo256::hash(b"file_key_2");

        // Dispatch challenge extrinsic twice.
        assert_ok!(ProofsDealer::challenge(
            RuntimeOrigin::signed(1),
            file_key_1
        ));

        // Check that the event is emitted.
        System::assert_last_event(
            Event::NewChallenge {
                who: 1,
                key_challenged: file_key_1,
            }
            .into(),
        );

        assert_ok!(ProofsDealer::challenge(
            RuntimeOrigin::signed(2),
            file_key_2
        ));

        // Check that the event is emitted.
        System::assert_last_event(
            Event::NewChallenge {
                who: 2,
                key_challenged: file_key_2,
            }
            .into(),
        );

        // Check users' balance after challenge.
        let challenge_fee: u128 = <Test as crate::Config>::ChallengesFee::get();
        assert_eq!(
            <Test as crate::Config>::NativeBalance::usable_balance(&1),
            user_balance - challenge_fee
        );
        assert_eq!(
            <Test as crate::Config>::NativeBalance::usable_balance(&2),
            user_balance - challenge_fee
        );

        // Check that the challenge is in the queue.
        let challenges_queue = crate::ChallengesQueue::<Test>::get();
        assert_eq!(challenges_queue.len(), 2);
        assert_eq!(challenges_queue[0], file_key_1);
        assert_eq!(challenges_queue[1], file_key_2);
    });
}

#[test]
fn challenge_submit_existing_challenge_succeed() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        System::set_block_number(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(1);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));

        // Mock a FileKey.
        let file_key = BlakeTwo256::hash(b"file_key");

        // Dispatch challenge extrinsic twice.
        assert_ok!(ProofsDealer::challenge(RuntimeOrigin::signed(1), file_key));
        assert_ok!(ProofsDealer::challenge(RuntimeOrigin::signed(1), file_key));

        // Check that the event is emitted.
        System::assert_last_event(
            Event::NewChallenge {
                who: 1,
                key_challenged: file_key,
            }
            .into(),
        );

        // Check user's balance after challenge.
        let challenge_fee: u128 = <Test as crate::Config>::ChallengesFee::get();
        assert_eq!(
            <Test as crate::Config>::NativeBalance::usable_balance(&1),
            user_balance - challenge_fee * 2
        );

        // Check that the challenge is in the queue.
        let challenges_queue = crate::ChallengesQueue::<Test>::get();
        assert_eq!(challenges_queue.len(), 1);
        assert_eq!(challenges_queue[0], file_key);
    });
}

#[test]
fn challenge_submit_in_two_rounds_succeed() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        System::set_block_number(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(1);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));

        // Mock a FileKey.
        let file_key = BlakeTwo256::hash(b"file_key");

        // Dispatch challenge extrinsic twice.
        assert_ok!(ProofsDealer::challenge(RuntimeOrigin::signed(1), file_key));

        // Check that the event is emitted.
        System::assert_last_event(
            Event::NewChallenge {
                who: 1,
                key_challenged: file_key,
            }
            .into(),
        );

        // Check user's balance after challenge.
        let challenge_fee: u128 = <Test as crate::Config>::ChallengesFee::get();
        assert_eq!(
            <Test as crate::Config>::NativeBalance::usable_balance(&1),
            user_balance - challenge_fee
        );

        // Check that the challenge is in the queue.
        let challenges_queue = crate::ChallengesQueue::<Test>::get();
        assert_eq!(challenges_queue.len(), 1);
        assert_eq!(challenges_queue[0], file_key);

        // Advance `CheckpointChallengePeriod` blocks.
        let challenge_period: u32 = <Test as crate::Config>::CheckpointChallengePeriod::get();
        run_n_blocks(challenge_period as u64 + 1);

        // Dispatch challenge extrinsic twice.
        let file_key = BlakeTwo256::hash(b"file_key_2");
        assert_ok!(ProofsDealer::challenge(RuntimeOrigin::signed(1), file_key));

        // Check that the event is emitted.
        System::assert_last_event(
            Event::NewChallenge {
                who: 1,
                key_challenged: file_key,
            }
            .into(),
        );

        // Check user's balance after challenge.
        assert_eq!(
            <Test as crate::Config>::NativeBalance::usable_balance(&1),
            user_balance - challenge_fee * 2
        );

        // TODO: Uncomment when `on_initialize` trigger is added.
        // // Check that the challenge is in the queue.
        // let challenges_queue = crate::ChallengesQueue::<Test>::get();
        // assert_eq!(challenges_queue.len(), 1);
        // assert_eq!(challenges_queue[0], file_key);
    });
}

#[test]
fn challenge_submit_by_registered_provider_with_no_funds_succeed() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        System::set_block_number(1);

        // Create user with no funds.
        let user = RuntimeOrigin::signed(1);

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToMainStorageProviderId::<Test>::insert(&1, provider_id);

        // Mock a FileKey.
        let file_key = BlakeTwo256::hash(b"file_key");

        // Dispatch challenge extrinsic.
        assert_ok!(ProofsDealer::challenge(RuntimeOrigin::signed(1), file_key));

        // Check that the event is emitted.
        System::assert_last_event(
            Event::NewChallenge {
                who: 1,
                key_challenged: file_key,
            }
            .into(),
        );

        // Check that the challenge is in the queue.
        let challenges_queue = crate::ChallengesQueue::<Test>::get();
        assert_eq!(challenges_queue.len(), 1);
        assert_eq!(challenges_queue[0], file_key);
    });
}

#[test]
fn challenge_wrong_origin_fail() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        System::set_block_number(1);

        // Mock a FileKey.
        let file_key = BlakeTwo256::hash(b"file_key");

        // Dispatch challenge extrinsic with wrong origin.
        assert_noop!(
            ProofsDealer::challenge(RuntimeOrigin::none(), file_key),
            DispatchError::BadOrigin
        );
    });
}

#[test]
fn challenge_submit_by_regular_user_with_no_funds_fail() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        System::set_block_number(1);

        // Create user with no funds.
        let user = RuntimeOrigin::signed(1);

        // Mock a FileKey.
        let file_key = BlakeTwo256::hash(b"file_key");

        // Dispatch challenge extrinsic.
        assert_noop!(
            ProofsDealer::challenge(RuntimeOrigin::signed(1), file_key),
            crate::Error::<Test>::FeeChargeFailed
        );
    });
}

#[test]
fn challenge_overflow_challenges_queue_fail() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        System::set_block_number(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(1);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));

        // Mock a FileKey.
        let file_key = BlakeTwo256::hash(b"file_key");

        // Fill the challenges queue.
        let queue_size: u32 = <Test as crate::Config>::ChallengesQueueLength::get();
        for i in 0..queue_size {
            let file_key = BlakeTwo256::hash(&i.to_le_bytes());
            assert_ok!(ProofsDealer::challenge(RuntimeOrigin::signed(1), file_key));
        }

        // Dispatch challenge extrinsic.
        assert_noop!(
            ProofsDealer::challenge(RuntimeOrigin::signed(1), file_key),
            crate::Error::<Test>::ChallengesQueueOverflow
        );
    });
}

#[test]
fn proofs_dealer_trait_verify_proof_succeed() {
    new_test_ext().execute_with(|| {
        // TODO
        assert!(true)
    });
}

#[test]
fn proofs_dealer_trait_verify_proof_fail() {
    new_test_ext().execute_with(|| {
        // TODO
        assert!(true)
    });
}

#[test]
fn proofs_dealer_trait_challenge_succeed() {
    new_test_ext().execute_with(|| {
        // Mock a FileKey.
        let file_key = BlakeTwo256::hash(b"file_key");

        // Challenge using trait.
        <ProofsDealer as storage_hub_traits::ProofsDealer>::challenge(&file_key).unwrap();

        // Check that the challenge is in the queue.
        let challenges_queue = crate::ChallengesQueue::<Test>::get();
        assert_eq!(challenges_queue.len(), 1);
        assert_eq!(challenges_queue[0], file_key);
    });
}

#[test]
fn proofs_dealer_trait_challenge_overflow_challenges_queue_fail() {
    new_test_ext().execute_with(|| {
        // Mock a FileKey.
        let file_key = BlakeTwo256::hash(b"file_key");

        // Fill the challenges queue.
        let queue_size: u32 = <Test as crate::Config>::ChallengesQueueLength::get();
        for i in 0..queue_size {
            let file_key = BlakeTwo256::hash(&i.to_le_bytes());
            assert_ok!(<ProofsDealer as storage_hub_traits::ProofsDealer>::challenge(&file_key));
        }

        // Dispatch challenge extrinsic.
        assert_noop!(
            <ProofsDealer as storage_hub_traits::ProofsDealer>::challenge(&file_key),
            crate::Error::<Test>::ChallengesQueueOverflow
        );
    });
}

#[test]
fn proofs_dealer_trait_challenge_with_priority_succeed() {
    new_test_ext().execute_with(|| {
        // Mock a FileKey.
        let file_key = BlakeTwo256::hash(b"file_key");

        // Challenge using trait.
        <ProofsDealer as storage_hub_traits::ProofsDealer>::challenge_with_priority(&file_key)
            .unwrap();

        // Check that the challenge is in the queue.
        let priority_challenges_queue = crate::PriorityChallengesQueue::<Test>::get();
        assert_eq!(priority_challenges_queue.len(), 1);
        assert_eq!(priority_challenges_queue[0], file_key);
    });
}

#[test]
fn proofs_dealer_trait_challenge_with_priority_overflow_challenges_queue_fail() {
    new_test_ext().execute_with(|| {
        // Mock a FileKey.
        let file_key = BlakeTwo256::hash(b"file_key");

        // Fill the challenges queue.
        let queue_size: u32 = <Test as crate::Config>::ChallengesQueueLength::get();
        for i in 0..queue_size {
            let file_key = BlakeTwo256::hash(&i.to_le_bytes());
            assert_ok!(
                <ProofsDealer as storage_hub_traits::ProofsDealer>::challenge_with_priority(
                    &file_key
                )
            );
        }

        // Dispatch challenge extrinsic.
        assert_noop!(
            <ProofsDealer as storage_hub_traits::ProofsDealer>::challenge_with_priority(&file_key),
            crate::Error::<Test>::PriorityChallengesQueueOverflow
        );
    });
}
