use crate::{mock::*, Event};
use frame_support::{
    assert_ok,
    pallet_prelude::Weight,
    traits::{OnFinalize, OnIdle, OnInitialize},
};
use sp_core::blake2_256;
use sp_runtime::testing::H256;

#[test]
fn set_babe_randomness_is_mandatory() {
    use frame_support::dispatch::{DispatchClass, GetDispatchInfo};

    let info = crate::Call::<Test>::set_babe_randomness {}.get_dispatch_info();
    assert_eq!(info.class, DispatchClass::Mandatory);
}

#[test]
fn set_babe_randomness_works() {
    ExtBuilder::build().execute_with(|| {
        // Test starts before inherent inclusion
        // Get the last relay epoch for which randomness was processed (should be 0).
        let last_processed_relay_epoch = Randomness::relay_epoch();
        assert_eq!(last_processed_relay_epoch, 0);

        // Get the randomness for that relay epoch (should be None since it was not set).
        let randomness = Randomness::latest_babe_randomness();
        assert_eq!(randomness, None);

        // Include the inherent in the block to set the randomness.
        // For mock, the relay epoch is equal to the block number, the randomness is the Blake2 256 bit hash of the relay epoch
        // and its valid block is current block number - 1
        assert_ok!(Randomness::set_babe_randomness(RuntimeOrigin::none()));

        // Get the last relay epoch for which randomness was processed (should be 1).
        let last_processed_relay_epoch = Randomness::relay_epoch();
        assert_eq!(last_processed_relay_epoch, 1);

        // Get the randomness for that relay epoch (should be the Blake2 256 bit hash of the epoch index).
        let randomness = Randomness::latest_babe_randomness();
        assert_eq!(
            randomness,
            Some((
                H256::from_slice(&blake2_256(&last_processed_relay_epoch.to_le_bytes())),
                System::block_number() - 1
            ))
        );

        // Check that the event was emitted
        System::assert_last_event(
            Event::<Test>::NewRandomnessAvailable {
                randomness_seed: H256::from_slice(&blake2_256(
                    &last_processed_relay_epoch.to_le_bytes(),
                )),
                from_epoch: 1,
                valid_until_block: System::block_number() - 1,
            }
            .into(),
        );

        // Check that the InherentIncluded storage was indeed set
        assert!(Randomness::inherent_included().is_some());

        // Advance a block (it should not panic since the inherent was included in the current one)
        AllPalletsWithSystem::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        AllPalletsWithSystem::on_initialize(System::block_number());
        AllPalletsWithSystem::on_idle(System::block_number(), Weight::MAX);
    });
}

#[test]
fn set_babe_randomness_works_after_a_few_epochs() {
    ExtBuilder::build().execute_with(|| {
        // Test starts before inherent inclusion
        // Get the last relay epoch for which randomness was processed (should be 0).
        let last_processed_relay_epoch = Randomness::relay_epoch();
        assert_eq!(last_processed_relay_epoch, 0);

        // Get the randomness for that relay epoch (should be None since it was not set).
        let randomness = Randomness::latest_babe_randomness();
        assert_eq!(randomness, None);

        for i in 1..10 {
            // Include the inherent in the block to set the randomness.
            // For mock, the relay epoch is equal to the block number, the randomness is the Blake2 256 bit hash of the relay epoch
            // and its valid block is current block number - 1
            assert_ok!(Randomness::set_babe_randomness(RuntimeOrigin::none()));

            // Get the last relay epoch for which randomness was processed (should be equal to i since we start the index at 1).
            let last_processed_relay_epoch = Randomness::relay_epoch();
            assert_eq!(last_processed_relay_epoch, i);

            // Get the randomness for that relay epoch (should be the Blake2 256 bit hash of the epoch index).
            let randomness = Randomness::latest_babe_randomness();
            assert_eq!(
                randomness,
                Some((
                    H256::from_slice(&blake2_256(&last_processed_relay_epoch.to_le_bytes())),
                    System::block_number() - 1
                ))
            );

            // Check that the event was emitted
            System::assert_last_event(
                Event::<Test>::NewRandomnessAvailable {
                    randomness_seed: H256::from_slice(&blake2_256(
                        &last_processed_relay_epoch.to_le_bytes(),
                    )),
                    from_epoch: i,
                    valid_until_block: System::block_number() - 1,
                }
                .into(),
            );

            // Check that the InherentIncluded storage was indeed set
            assert!(Randomness::inherent_included().is_some());

            // Advance a block (it should not panic since the inherent was included in the current one)
            AllPalletsWithSystem::on_finalize(System::block_number());
            System::set_block_number(System::block_number() + 1);
            AllPalletsWithSystem::on_initialize(System::block_number());
            AllPalletsWithSystem::on_idle(System::block_number(), Weight::MAX);
        }
    });
}
