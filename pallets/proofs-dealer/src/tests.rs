use std::collections::BTreeMap;
use std::vec;

use crate::pallet::Event;
use crate::types::{ChallengeHistoryLengthFor, KeyProof, RandomChallengesPerBlockFor};
use crate::{mock::*, types::Proof};
use crate::{
    BlockToChallengesSeed, BlockToCheckpointChallenges, LastBlockProviderSubmittedProofFor,
    LastCheckpointBlock,
};
use frame_support::{assert_noop, assert_ok, traits::fungible::Mutate};
use sp_core::{Get, Hasher};
use sp_runtime::BoundedVec;
use sp_runtime::{traits::BlakeTwo256, DispatchError};
use sp_trie::CompactProof;

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
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                data_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                payment_account: Default::default(),
            },
        );

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
        <ProofsDealer as shp_traits::ProofsDealerInterface>::challenge(&file_key).unwrap();

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
            assert_ok!(<ProofsDealer as shp_traits::ProofsDealerInterface>::challenge(&file_key));
        }

        // Dispatch challenge extrinsic.
        assert_noop!(
            <ProofsDealer as shp_traits::ProofsDealerInterface>::challenge(&file_key),
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
        <ProofsDealer as shp_traits::ProofsDealerInterface>::challenge_with_priority(
            &file_key, None,
        )
        .unwrap();

        // Check that the challenge is in the queue.
        let priority_challenges_queue = crate::PriorityChallengesQueue::<Test>::get();
        assert_eq!(priority_challenges_queue.len(), 1);
        assert_eq!(priority_challenges_queue[0], (file_key, None));
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
                <ProofsDealer as shp_traits::ProofsDealerInterface>::challenge_with_priority(
                    &file_key, None
                )
            );
        }

        // Dispatch challenge extrinsic.
        assert_noop!(
            <ProofsDealer as shp_traits::ProofsDealerInterface>::challenge_with_priority(
                &file_key, None
            ),
            crate::Error::<Test>::PriorityChallengesQueueOverflow
        );
    });
}

#[test]
fn submit_proof_success() {
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

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                data_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                payment_account: Default::default(),
            },
        );

        // Set Provider's last submitted proof block.
        LastBlockProviderSubmittedProofFor::<Test>::insert(&provider_id, System::block_number());

        // Set random seed for this block challenges.
        let seed = BlakeTwo256::hash(b"seed");
        println!("Block number: {:?}", System::block_number());
        BlockToChallengesSeed::<Test>::insert(System::block_number(), seed);

        // Calculate challenges from seed, so that we can mock a key proof for each.
        let challenges = crate::Pallet::<Test>::generate_challenges_from_seed(
            seed,
            &provider_id,
            RandomChallengesPerBlockFor::<Test>::get(),
        );

        // Creating a vec of empty key proofs for each challenge, to fail verification.
        let mut key_proofs = BTreeMap::new();
        for challenge in challenges {
            key_proofs.insert(
                challenge,
                KeyProof::<Test> {
                    proof: CompactProof {
                        encoded_nodes: vec![vec![0]],
                    },
                    challenge_count: Default::default(),
                },
            );
        }

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Advance less than `ChallengeHistoryLength` blocks.
        let challenge_history_length: u64 = ChallengeHistoryLengthFor::<Test>::get();
        run_n_blocks(challenge_history_length - 1);

        // Dispatch challenge extrinsic.
        assert_ok!(ProofsDealer::submit_proof(
            RuntimeOrigin::signed(1),
            proof,
            None
        ));
    });
}

#[test]
fn submit_proof_submitted_by_not_a_provider_success() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        System::set_block_number(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(2);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &2,
            user_balance
        ));

        // Register user as a Provider in Providers pallet.
        // The registered Provider ID will be different from the one that will be used in the proof.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                data_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                payment_account: Default::default(),
            },
        );

        // Set Provider's last submitted proof block.
        LastBlockProviderSubmittedProofFor::<Test>::insert(&provider_id, System::block_number());

        // Set random seed for this block challenges.
        let seed = BlakeTwo256::hash(b"seed");
        println!("Block number: {:?}", System::block_number());
        BlockToChallengesSeed::<Test>::insert(System::block_number(), seed);

        // Calculate challenges from seed, so that we can mock a key proof for each.
        let challenges = crate::Pallet::<Test>::generate_challenges_from_seed(
            seed,
            &provider_id,
            RandomChallengesPerBlockFor::<Test>::get(),
        );

        // Creating a vec of empty key proofs for each challenge, to fail verification.
        let mut key_proofs = BTreeMap::new();
        for challenge in challenges {
            key_proofs.insert(
                challenge,
                KeyProof::<Test> {
                    proof: CompactProof {
                        encoded_nodes: vec![vec![0]],
                    },
                    challenge_count: Default::default(),
                },
            );
        }

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Advance less than `ChallengeHistoryLength` blocks.
        let challenge_history_length: u64 = ChallengeHistoryLengthFor::<Test>::get();
        run_n_blocks(challenge_history_length - 1);

        // Dispatch challenge extrinsic.
        assert_ok!(ProofsDealer::submit_proof(
            RuntimeOrigin::signed(2),
            proof,
            Some(provider_id)
        ));
    });
}

#[test]
fn submit_proof_with_checkpoint_challenges_success() {
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

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                data_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                payment_account: Default::default(),
            },
        );

        // Set Provider's last submitted proof block.
        LastBlockProviderSubmittedProofFor::<Test>::insert(&provider_id, System::block_number());

        // Set random seed for this block challenges.
        let seed = BlakeTwo256::hash(b"seed");
        println!("Block number: {:?}", System::block_number());
        BlockToChallengesSeed::<Test>::insert(System::block_number(), seed);

        // Calculate challenges from seed, so that we can mock a key proof for each.
        let mut challenges = crate::Pallet::<Test>::generate_challenges_from_seed(
            seed,
            &provider_id,
            RandomChallengesPerBlockFor::<Test>::get(),
        );

        // Set last checkpoint challenge block.
        let checkpoint_challenge_block = System::block_number() + 1;
        LastCheckpointBlock::<Test>::set(checkpoint_challenge_block);

        // Make up custom challenges.
        let custom_challenges = BoundedVec::try_from(vec![
            (BlakeTwo256::hash(b"custom_challenge_1"), None),
            (BlakeTwo256::hash(b"custom_challenge_2"), None),
        ])
        .unwrap();

        // Set custom challenges in checkpoint block.
        BlockToCheckpointChallenges::<Test>::insert(
            checkpoint_challenge_block,
            custom_challenges.clone(),
        );

        // Add custom challenges to the challenges vector.
        challenges.extend(custom_challenges.iter().map(|(challenge, _)| *challenge));

        // Creating a vec of empty key proofs for each challenge, to fail verification.
        let mut key_proofs = BTreeMap::new();
        for challenge in challenges {
            key_proofs.insert(
                challenge,
                KeyProof::<Test> {
                    proof: CompactProof {
                        encoded_nodes: vec![vec![0]],
                    },
                    challenge_count: Default::default(),
                },
            );
        }

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Advance less than `ChallengeHistoryLength` blocks.
        let challenge_history_length: u64 = ChallengeHistoryLengthFor::<Test>::get();
        run_n_blocks(challenge_history_length - 1);

        // Dispatch challenge extrinsic.
        assert_ok!(ProofsDealer::submit_proof(
            RuntimeOrigin::signed(1),
            proof,
            None
        ));
    });
}

#[test]
fn submit_proof_caller_not_a_provider_fail() {
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

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs: Default::default(),
        };

        // Dispatch challenge extrinsic.
        assert_noop!(
            ProofsDealer::submit_proof(RuntimeOrigin::signed(1), proof, None),
            crate::Error::<Test>::NotProvider
        );
    });
}

#[test]
fn submit_proof_provider_passed_not_registered_fail() {
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

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs: Default::default(),
        };

        // Creating a Provider ID but not registering it.
        let provider_id = BlakeTwo256::hash(b"provider_id");

        // Dispatch challenge extrinsic.
        assert_noop!(
            ProofsDealer::submit_proof(RuntimeOrigin::signed(1), proof, Some(provider_id)),
            crate::Error::<Test>::NotProvider
        );
    });
}

#[test]
fn submit_proof_empty_key_proofs_fail() {
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

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs: Default::default(),
        };

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                data_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                payment_account: Default::default(),
            },
        );

        // Dispatch challenge extrinsic.
        assert_noop!(
            ProofsDealer::submit_proof(RuntimeOrigin::signed(1), proof, None),
            crate::Error::<Test>::EmptyKeyProofs
        );
    });
}

#[test]
fn submit_proof_no_record_of_last_proof_fail() {
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

        let mut key_proofs = BTreeMap::new();
        key_proofs.insert(
            BlakeTwo256::hash(b"key"),
            KeyProof::<Test> {
                proof: CompactProof {
                    encoded_nodes: vec![vec![0]],
                },
                challenge_count: Default::default(),
            },
        );

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                data_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                payment_account: Default::default(),
            },
        );

        // Dispatch challenge extrinsic.
        assert_noop!(
            ProofsDealer::submit_proof(RuntimeOrigin::signed(1), proof, None),
            crate::Error::<Test>::NoRecordOfLastSubmittedProof
        );
    });
}

#[test]
fn submit_proof_challenges_block_not_reached_fail() {
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

        let mut key_proofs = BTreeMap::new();
        key_proofs.insert(
            BlakeTwo256::hash(b"key"),
            KeyProof::<Test> {
                proof: CompactProof {
                    encoded_nodes: vec![vec![0]],
                },
                challenge_count: Default::default(),
            },
        );

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                data_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                payment_account: Default::default(),
            },
        );

        // Set Provider's last submitted proof block.
        LastBlockProviderSubmittedProofFor::<Test>::insert(&provider_id, 1);

        // Dispatch challenge extrinsic.
        assert_noop!(
            ProofsDealer::submit_proof(RuntimeOrigin::signed(1), proof, None),
            crate::Error::<Test>::ChallengesBlockNotReached
        );
    });
}

#[test]
#[should_panic(
    expected = "internal error: entered unreachable code: Challenges block is too old, beyond the history this pallet keeps track of. This should not be possible."
)]
fn submit_proof_challenges_block_too_old_fail() {
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

        let mut key_proofs = BTreeMap::new();
        key_proofs.insert(
            BlakeTwo256::hash(b"key"),
            KeyProof::<Test> {
                proof: CompactProof {
                    encoded_nodes: vec![vec![0]],
                },
                challenge_count: Default::default(),
            },
        );

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );

        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                data_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                payment_account: Default::default(),
            },
        );

        // Set Provider's last submitted proof block.
        LastBlockProviderSubmittedProofFor::<Test>::insert(&provider_id, 1);

        // Advance more than `ChallengeHistoryLength` blocks.
        let challenge_history_length: u64 = ChallengeHistoryLengthFor::<Test>::get();
        run_n_blocks(challenge_history_length + 1);

        // Dispatch challenge extrinsic.
        let _ = ProofsDealer::submit_proof(RuntimeOrigin::signed(1), proof, None);
    });
}

#[test]
#[should_panic(
    expected = "internal error: entered unreachable code: Seed for challenges block not found, when checked it should be within history."
)]
fn submit_proof_seed_not_found_fail() {
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

        let mut key_proofs = BTreeMap::new();
        key_proofs.insert(
            BlakeTwo256::hash(b"key"),
            KeyProof::<Test> {
                proof: CompactProof {
                    encoded_nodes: vec![vec![0]],
                },
                challenge_count: Default::default(),
            },
        );

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                data_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                payment_account: Default::default(),
            },
        );

        // Set Provider's last submitted proof block.
        LastBlockProviderSubmittedProofFor::<Test>::insert(&provider_id, 1);

        // Advance less than `ChallengeHistoryLength` blocks.
        let challenge_history_length: u64 = ChallengeHistoryLengthFor::<Test>::get();
        run_n_blocks(challenge_history_length - 1);

        // Dispatch challenge extrinsic.
        let _ = ProofsDealer::submit_proof(RuntimeOrigin::signed(1), proof, None);
    });
}

#[test]
#[should_panic(
    expected = "internal error: entered unreachable code: Checkpoint challenges not found, when dereferencing in last registered checkpoint challenge block."
)]
fn submit_proof_checkpoint_challenge_not_found_fail() {
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

        let mut key_proofs = BTreeMap::new();
        key_proofs.insert(
            BlakeTwo256::hash(b"key"),
            KeyProof::<Test> {
                proof: CompactProof {
                    encoded_nodes: vec![vec![0]],
                },
                challenge_count: Default::default(),
            },
        );

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                data_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                payment_account: Default::default(),
            },
        );

        // Set Provider's last submitted proof block.
        LastBlockProviderSubmittedProofFor::<Test>::insert(&provider_id, System::block_number());

        // Set random seed for this block challenges.
        let seed = BlakeTwo256::hash(b"seed");
        println!("Block number: {:?}", System::block_number());
        BlockToChallengesSeed::<Test>::insert(System::block_number(), seed);

        // Set last checkpoint challenge block.
        let checkpoint_challenge_block = System::block_number() + 1;
        LastCheckpointBlock::<Test>::set(checkpoint_challenge_block);

        // Advance less than `ChallengeHistoryLength` blocks.
        let challenge_history_length: u64 = ChallengeHistoryLengthFor::<Test>::get();
        run_n_blocks(challenge_history_length - 1);

        // Dispatch challenge extrinsic.
        let _ = ProofsDealer::submit_proof(RuntimeOrigin::signed(1), proof, None);
    });
}

#[test]
fn submit_proof_forest_proof_verification_fail() {
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

        // Mock key proofs.
        let mut key_proofs = BTreeMap::new();
        key_proofs.insert(
            BlakeTwo256::hash(b"key"),
            KeyProof::<Test> {
                proof: CompactProof {
                    encoded_nodes: vec![vec![0]],
                },
                challenge_count: Default::default(),
            },
        );

        // Create an empty forest proof to fail verification.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![],
            },
            key_proofs,
        };

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                data_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                payment_account: Default::default(),
            },
        );

        // Set Provider's last submitted proof block.
        LastBlockProviderSubmittedProofFor::<Test>::insert(&provider_id, System::block_number());

        // Set random seed for this block challenges.
        let seed = BlakeTwo256::hash(b"seed");
        println!("Block number: {:?}", System::block_number());
        BlockToChallengesSeed::<Test>::insert(System::block_number(), seed);

        // Advance less than `ChallengeHistoryLength` blocks.
        let challenge_history_length: u64 = ChallengeHistoryLengthFor::<Test>::get();
        run_n_blocks(challenge_history_length - 1);

        // Dispatch challenge extrinsic.
        assert_noop!(
            ProofsDealer::submit_proof(RuntimeOrigin::signed(1), proof, None),
            crate::Error::<Test>::ForestProofVerificationFailed
        );
    });
}

#[test]
fn submit_proof_no_key_proofs_for_keys_verified_in_forest_fail() {
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

        // Creating empty key proof to fail verification.
        let mut key_proofs = BTreeMap::new();
        key_proofs.insert(
            BlakeTwo256::hash(b"key"),
            KeyProof::<Test> {
                proof: CompactProof {
                    encoded_nodes: vec![],
                },
                challenge_count: Default::default(),
            },
        );

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                data_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                payment_account: Default::default(),
            },
        );

        // Set Provider's last submitted proof block.
        LastBlockProviderSubmittedProofFor::<Test>::insert(&provider_id, System::block_number());

        // Set random seed for this block challenges.
        let seed = BlakeTwo256::hash(b"seed");
        println!("Block number: {:?}", System::block_number());
        BlockToChallengesSeed::<Test>::insert(System::block_number(), seed);

        // Advance less than `ChallengeHistoryLength` blocks.
        let challenge_history_length: u64 = ChallengeHistoryLengthFor::<Test>::get();
        run_n_blocks(challenge_history_length - 1);

        // Dispatch challenge extrinsic.
        // The forest proof will pass because it's not empty, so the MockVerifier will accept it,
        // and it will return the generated challenges as keys proven. The key proofs are an empty
        // vector, so it will fail saying that there are no key proofs for the keys proven.
        assert_noop!(
            ProofsDealer::submit_proof(RuntimeOrigin::signed(1), proof, None),
            crate::Error::<Test>::KeyProofNotFound
        );
    });
}

#[test]
fn submit_proof_out_checkpoint_challenges_fail() {
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

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                data_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                payment_account: Default::default(),
            },
        );

        // Set Provider's last submitted proof block.
        LastBlockProviderSubmittedProofFor::<Test>::insert(&provider_id, System::block_number());

        // Set random seed for this block challenges.
        let seed = BlakeTwo256::hash(b"seed");
        println!("Block number: {:?}", System::block_number());
        BlockToChallengesSeed::<Test>::insert(System::block_number(), seed);

        // Calculate challenges from seed, so that we can mock a key proof for each.
        let challenges = crate::Pallet::<Test>::generate_challenges_from_seed(
            seed,
            &provider_id,
            RandomChallengesPerBlockFor::<Test>::get(),
        );

        // Set last checkpoint challenge block.
        let checkpoint_challenge_block = System::block_number() + 1;
        LastCheckpointBlock::<Test>::set(checkpoint_challenge_block);

        // Make up custom challenges.
        let custom_challenges = BoundedVec::try_from(vec![
            (BlakeTwo256::hash(b"custom_challenge_1"), None),
            (BlakeTwo256::hash(b"custom_challenge_2"), None),
        ])
        .unwrap();

        // Set custom challenges in checkpoint block.
        BlockToCheckpointChallenges::<Test>::insert(
            checkpoint_challenge_block,
            custom_challenges.clone(),
        );

        // Creating a vec of empty key proofs for each challenge, to fail verification.
        let mut key_proofs = BTreeMap::new();
        for challenge in challenges {
            key_proofs.insert(
                challenge,
                KeyProof::<Test> {
                    proof: CompactProof {
                        encoded_nodes: vec![vec![0]],
                    },
                    challenge_count: Default::default(),
                },
            );
        }

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Advance less than `ChallengeHistoryLength` blocks.
        let challenge_history_length: u64 = ChallengeHistoryLengthFor::<Test>::get();
        run_n_blocks(challenge_history_length - 1);

        // Dispatch challenge extrinsic.
        // The forest proof will pass because it's not empty, so the MockVerifier will accept it,
        // and it will return the generated challenges as keys proven. The key proofs only contain
        // proofs for the regular challenges, not the checkpoint challenges, so it will fail saying
        // that there are no key proofs for the keys proven.
        assert_noop!(
            ProofsDealer::submit_proof(RuntimeOrigin::signed(1), proof, None),
            crate::Error::<Test>::KeyProofNotFound
        );
    });
}

#[test]
fn submit_proof_key_proof_verification_fail() {
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

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                data_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                payment_account: Default::default(),
            },
        );

        // Set Provider's last submitted proof block.
        LastBlockProviderSubmittedProofFor::<Test>::insert(&provider_id, System::block_number());

        // Set random seed for this block challenges.
        let seed = BlakeTwo256::hash(b"seed");
        println!("Block number: {:?}", System::block_number());
        BlockToChallengesSeed::<Test>::insert(System::block_number(), seed);

        // Calculate challenges from seed, so that we can mock a key proof for each.
        let challenges = crate::Pallet::<Test>::generate_challenges_from_seed(
            seed,
            &provider_id,
            RandomChallengesPerBlockFor::<Test>::get(),
        );

        // Creating a vec of empty key proofs for each challenge, to fail verification.
        let mut key_proofs = BTreeMap::new();
        for challenge in challenges {
            key_proofs.insert(
                challenge,
                KeyProof::<Test> {
                    proof: CompactProof {
                        encoded_nodes: vec![],
                    },
                    challenge_count: Default::default(),
                },
            );
        }

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Advance less than `ChallengeHistoryLength` blocks.
        let challenge_history_length: u64 = ChallengeHistoryLengthFor::<Test>::get();
        run_n_blocks(challenge_history_length - 1);

        // Dispatch challenge extrinsic.
        // The forest proof will pass because it's not empty, so the MockVerifier will accept it,
        // and it will return the generated challenges as keys proven. There will be key proofs
        // for each key proven, but they are empty, so it will fail saying that the verification
        // failed.
        assert_noop!(
            ProofsDealer::submit_proof(RuntimeOrigin::signed(1), proof, None),
            crate::Error::<Test>::KeyProofVerificationFailed
        );
    });
}
