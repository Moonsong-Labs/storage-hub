use crate::{
    mock::*,
    types::{FileLocation, MultiAddress, StorageRequestBspsMetadata},
    Config, Event, StorageRequestExpirations,
};
use frame_support::{assert_ok, traits::Hooks, weights::Weight};
use sp_runtime::{
    traits::{BlakeTwo256, Get, Hash},
    BoundedVec,
};

#[test]
fn request_storage_success() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited
        System::set_block_number(1);

        let user = RuntimeOrigin::signed(1);
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let multiaddr = BoundedVec::try_from(vec![1]).unwrap();
        let multiaddresses: BoundedVec<MultiAddress<Test>, <Test as Config>::MaxMultiAddresses> =
            BoundedVec::try_from(vec![multiaddr]).unwrap();

        // Dispatch a signed extrinsic.
        assert_ok!(FileSystem::issue_storage_request(
            user.clone(),
            location.clone(),
            fingerprint,
            4,
            multiaddresses.clone(),
        ));

        // Assert that the correct event was deposited
        System::assert_last_event(
            Event::NewStorageRequest {
                who: 1,
                location: location.clone(),
                fingerprint,
                size: 4,
                multiaddresses,
            }
            .into(),
        );
    });
}

#[test]
fn request_storage_expiration_clear_success() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited
        System::set_block_number(1);

        let user = RuntimeOrigin::signed(1);
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let multiaddr = BoundedVec::try_from(vec![1]).unwrap();
        let multiaddresses: BoundedVec<MultiAddress<Test>, <Test as Config>::MaxMultiAddresses> =
            BoundedVec::try_from(vec![multiaddr]).unwrap();

        // Dispatch a signed extrinsic.
        assert_ok!(FileSystem::issue_storage_request(
            user.clone(),
            location.clone(),
            fingerprint,
            4,
            multiaddresses,
        ));

        let expected_expiration_inserted_at_block_number: BlockNumber =
            FileSystem::next_expiration_insertion_block_number().into();

        // Assert that the storage request expiration was appended to the list at `StorageRequestTtl`
        assert_eq!(
            FileSystem::storage_request_expirations(expected_expiration_inserted_at_block_number),
            vec![location]
        );

        roll_to(expected_expiration_inserted_at_block_number + 1);

        // Assert that the storage request expiration was removed from the list at `StorageRequestTtl`
        assert_eq!(
            FileSystem::storage_request_expirations(expected_expiration_inserted_at_block_number),
            vec![]
        );
    });
}

#[test]
fn request_storage_expiration_current_block_increment_success() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited
        System::set_block_number(1);

        let user = RuntimeOrigin::signed(1);
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let multiaddr = BoundedVec::try_from(vec![1]).unwrap();
        let multiaddresses: BoundedVec<MultiAddress<Test>, <Test as Config>::MaxMultiAddresses> =
            BoundedVec::try_from(vec![multiaddr]).unwrap();

        let mut expected_expiration_block_number: BlockNumber =
            FileSystem::next_expiration_insertion_block_number().into();

        // Append storage request expiration to the list at `StorageRequestTtl`
        let max_storage_request_expiry: u32 = <Test as Config>::MaxExpiredStorageRequests::get();
        for _ in 0..(max_storage_request_expiry - 1) {
            assert_ok!(StorageRequestExpirations::<Test>::try_append(
                expected_expiration_block_number,
                location.clone()
            ));
        }

        // Dispatch a signed extrinsic.
        assert_ok!(FileSystem::issue_storage_request(
            user.clone(),
            location.clone(),
            fingerprint,
            4,
            multiaddresses,
        ));

        // Assert that the storage request expirations storage is at max capacity
        assert_eq!(
            FileSystem::storage_request_expirations(expected_expiration_block_number).len(),
            max_storage_request_expiry as usize
        );

        expected_expiration_block_number =
            FileSystem::next_expiration_insertion_block_number().into();

        // Assert that the `CurrentExpirationBlock` storage is incremented by 1
        assert_eq!(
            FileSystem::next_available_expiration_insertion_block(),
            expected_expiration_block_number
        );

        // Go to block number after which the storage request expirations should be removed
        roll_to(expected_expiration_block_number);

        // Assert that the storage request expiration was removed from the list at `StorageRequestTtl`
        assert_eq!(
            FileSystem::storage_request_expirations(expected_expiration_block_number),
            vec![]
        );
    });
}

#[test]
fn request_storage_clear_old_expirations_success() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited
        System::set_block_number(1);

        let user = RuntimeOrigin::signed(1);
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let multiaddr = BoundedVec::try_from(vec![1]).unwrap();
        let multiaddresses: BoundedVec<MultiAddress<Test>, <Test as Config>::MaxMultiAddresses> =
            BoundedVec::try_from(vec![multiaddr]).unwrap();

        let expected_expiration_block_number: BlockNumber =
            FileSystem::next_expiration_insertion_block_number().into();

        // Append storage request expiration to the list at `StorageRequestTtl`
        let max_storage_request_expiry: u32 = <Test as Config>::MaxExpiredStorageRequests::get();
        for _ in 0..(max_storage_request_expiry - 1) {
            assert_ok!(StorageRequestExpirations::<Test>::try_append(
                expected_expiration_block_number,
                location.clone()
            ));
        }

        // Dispatch a signed extrinsic.
        assert_ok!(FileSystem::issue_storage_request(
            user.clone(),
            location.clone(),
            fingerprint,
            4,
            multiaddresses,
        ));

        System::set_block_number(expected_expiration_block_number);

        // Assert that the `NextExpirationInsertionBlockNumber` storage is set to 0 initially
        assert_eq!(FileSystem::next_starting_block_to_clean_up(), 0);

        // Assert that the storage request expirations storage is at max capacity
        assert_eq!(
            FileSystem::storage_request_expirations(expected_expiration_block_number).len(),
            max_storage_request_expiry as usize
        );

        let used_weight = FileSystem::on_idle(System::block_number(), Weight::zero());

        // Assert that the weight used is zero
        assert_eq!(used_weight, Weight::zero());

        // Assert that the storage request expirations storage is at max capacity
        // TODO: Fix this test...
        assert_eq!(
            FileSystem::storage_request_expirations(expected_expiration_block_number).len(),
            max_storage_request_expiry as usize
        );

        // Assert that the `NextExpirationInsertionBlockNumber` storage did not update
        assert_eq!(FileSystem::next_starting_block_to_clean_up(), 0);

        // Go to block number after which the storage request expirations should be removed
        roll_to(expected_expiration_block_number + 1);

        // Assert that the storage request expiration was removed from the list at `StorageRequestTtl`
        assert_eq!(
            FileSystem::storage_request_expirations(expected_expiration_block_number),
            vec![]
        );

        // Assert that the `NextExpirationInsertionBlockNumber` storage is set to the next block number
        assert_eq!(
            FileSystem::next_starting_block_to_clean_up(),
            System::block_number() + 1
        );
    });
}

#[test]
fn revoke_request_storage_success() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited
        System::set_block_number(1);

        let user = RuntimeOrigin::signed(1);
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);

        // Dispatch a signed extrinsic.
        assert_ok!(FileSystem::issue_storage_request(
            user.clone(),
            location.clone(),
            fingerprint,
            4,
            Default::default()
        ));

        // Assert that the storage request expiration was appended to the list at `StorageRequestTtl`
        assert_eq!(
            FileSystem::storage_request_expirations(
                FileSystem::next_expiration_insertion_block_number()
            ),
            vec![location.clone()]
        );

        // Dispatch a signed extrinsic.
        assert_ok!(FileSystem::revoke_storage_request(
            user.clone(),
            location.clone()
        ));

        // Assert that the correct event was deposited
        System::assert_last_event(Event::StorageRequestRevoked { location }.into());
    });
}

#[test]
fn bsp_volunteer_success() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited
        System::set_block_number(1);

        let user = RuntimeOrigin::signed(1);
        let bsp = RuntimeOrigin::signed(2);
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let multiaddr = BoundedVec::try_from(vec![1]).unwrap();
        let multiaddresses: BoundedVec<MultiAddress<Test>, <Test as Config>::MaxMultiAddresses> =
            BoundedVec::try_from(vec![multiaddr]).unwrap();

        // Dispatch storage request.
        assert_ok!(FileSystem::issue_storage_request(
            user.clone(),
            location.clone(),
            fingerprint,
            4,
            multiaddresses.clone(),
        ));

        // Dispatch BSP volunteer.
        assert_ok!(FileSystem::bsp_volunteer(
            bsp.clone(),
            location.clone(),
            fingerprint,
            multiaddresses.clone()
        ));

        // Assert that the RequestStorageBsps has the correct value
        assert_eq!(
            FileSystem::storage_request_bsps(location.clone(), 2)
                .expect("BSP should exist in storage"),
            StorageRequestBspsMetadata::<Test> {
                confirmed: false,
                _phantom: Default::default()
            }
        );

        // Assert that the correct event was deposited
        System::assert_last_event(
            Event::AcceptedBspVolunteer {
                who: 2,
                location,
                fingerprint,
                multiaddresses,
            }
            .into(),
        );
    });
}

#[test]
fn bsp_stop_storing_success() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited
        System::set_block_number(1);

        let owner = 1;
        let user = RuntimeOrigin::signed(owner);
        let bsp = RuntimeOrigin::signed(2);
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let size = 4;
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let multiaddr = BoundedVec::try_from(vec![1]).unwrap();
        let multiaddresses: BoundedVec<MultiAddress<Test>, <Test as Config>::MaxMultiAddresses> =
            BoundedVec::try_from(vec![multiaddr]).unwrap();

        // Dispatch storage request.
        assert_ok!(FileSystem::issue_storage_request(
            user.clone(),
            location.clone(),
            fingerprint,
            size,
            multiaddresses.clone(),
        ));

        // Dispatch BSP volunteer.
        assert_ok!(FileSystem::bsp_volunteer(
            bsp.clone(),
            location.clone(),
            fingerprint,
            multiaddresses.clone()
        ));

        // Assert that the RequestStorageBsps has the correct value
        assert_eq!(
            FileSystem::storage_request_bsps(location.clone(), 2)
                .expect("BSP should exist in storage"),
            StorageRequestBspsMetadata::<Test> {
                confirmed: false,
                _phantom: Default::default()
            }
        );

        // Dispatch BSP stop storing.
        assert_ok!(FileSystem::bsp_stop_storing(
            bsp.clone(),
            location.clone(),
            owner,
            fingerprint,
            size,
            false
        ));

        // Assert that the RequestStorageBsps has the correct value
        assert!(FileSystem::storage_request_bsps(location.clone(), 2).is_none());

        // Assert that the correct event was deposited
        System::assert_last_event(
            Event::BspStoppedStoring {
                bsp: 2,
                owner,
                location,
            }
            .into(),
        );
    });
}
