use crate::{
    mock::*,
    types::{
        FileLocation, MultiAddress, StorageRequestBspsMetadata, StorageRequestMetadata,
        TargetBspsRequired,
    },
    Config, Event, StorageRequestExpirations,
};
use frame_support::{assert_ok, traits::Hooks, weights::Weight};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, Get, Hash},
    AccountId32, BoundedVec,
};

#[test]
fn request_storage_success() {
    new_test_ext().execute_with(|| {
        let owner_account_id = AccountId32::new([1; 32]);
        let user = RuntimeOrigin::signed(owner_account_id.clone());
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let size = 4;
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
            size,
            multiaddresses.clone(),
        ));

        // Assert that the storage was updated
        assert_eq!(
            FileSystem::storage_requests(location.clone()),
            Some(StorageRequestMetadata {
                requested_at: 1,
                owner: owner_account_id.clone(),
                fingerprint,
                size,
                user_multiaddresses: multiaddresses.clone(),
                data_server_sps: BoundedVec::default(),
                bsps_required: TargetBspsRequired::<Test>::get(),
                bsps_confirmed: 0,
            })
        );

        // Assert that the correct event was deposited
        System::assert_last_event(
            Event::NewStorageRequest {
                who: owner_account_id,
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
        let owner = RuntimeOrigin::signed(AccountId32::new([1; 32]));
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let multiaddr = BoundedVec::try_from(vec![1]).unwrap();
        let multiaddresses: BoundedVec<MultiAddress<Test>, <Test as Config>::MaxMultiAddresses> =
            BoundedVec::try_from(vec![multiaddr]).unwrap();

        // Dispatch a signed extrinsic.
        assert_ok!(FileSystem::issue_storage_request(
            owner.clone(),
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
        let owner = RuntimeOrigin::signed(AccountId32::new([1; 32]));
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
            owner.clone(),
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
        let owner = RuntimeOrigin::signed(AccountId32::new([1; 32]));
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
            owner.clone(),
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
        let owner = RuntimeOrigin::signed(AccountId32::new([1; 32]));
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let file_key = H256::zero();

        // Dispatch a signed extrinsic.
        assert_ok!(FileSystem::issue_storage_request(
            owner.clone(),
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
            owner.clone(),
            location.clone(),
            file_key
        ));

        // Assert that the correct event was deposited
        System::assert_last_event(Event::StorageRequestRevoked { location }.into());
    });
}

#[test]
fn bsp_volunteer_success() {
    new_test_ext().execute_with(|| {
        let owner = RuntimeOrigin::signed(AccountId32::new([1; 32]));
        let bsp_account_id = AccountId32::new([2; 32]);
        let bsp = RuntimeOrigin::signed(bsp_account_id.clone());
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        // TODO: right now we are bypassing the volunteer assignment threshold
        let fingerprint = H256::zero();
        let multiaddr = BoundedVec::try_from(vec![1]).unwrap();
        let multiaddresses: BoundedVec<MultiAddress<Test>, <Test as Config>::MaxMultiAddresses> =
            BoundedVec::try_from(vec![multiaddr]).unwrap();

        // Dispatch storage request.
        assert_ok!(FileSystem::issue_storage_request(
            owner.clone(),
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
            FileSystem::storage_request_bsps(location.clone(), bsp_account_id.clone())
                .expect("BSP should exist in storage"),
            StorageRequestBspsMetadata::<Test> {
                confirmed: false,
                _phantom: Default::default()
            }
        );

        // Assert that the correct event was deposited
        System::assert_last_event(
            Event::AcceptedBspVolunteer {
                who: bsp_account_id,
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
        let owner_account_id = AccountId32::new([1; 32]);
        let owner = RuntimeOrigin::signed(owner_account_id.clone());
        let bsp_account_id = AccountId32::new([2; 32]);
        let bsp = RuntimeOrigin::signed(bsp_account_id.clone());
        let file_key = H256::from_slice(&[1; 32]);
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let size = 4;
        // TODO: right now we are bypassing the volunteer assignment threshold
        let fingerprint = H256::zero();
        let multiaddr = BoundedVec::try_from(vec![1]).unwrap();
        let multiaddresses: BoundedVec<MultiAddress<Test>, <Test as Config>::MaxMultiAddresses> =
            BoundedVec::try_from(vec![multiaddr]).unwrap();

        // Dispatch storage request.
        assert_ok!(FileSystem::issue_storage_request(
            owner.clone(),
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
            FileSystem::storage_request_bsps(location.clone(), bsp_account_id.clone())
                .expect("BSP should exist in storage"),
            StorageRequestBspsMetadata::<Test> {
                confirmed: false,
                _phantom: Default::default()
            }
        );

        // Dispatch BSP stop storing.
        assert_ok!(FileSystem::bsp_stop_storing(
            bsp.clone(),
            file_key,
            location.clone(),
            owner_account_id.clone(),
            fingerprint,
            size,
            false
        ));

        // Assert that the RequestStorageBsps has the correct value
        assert!(
            FileSystem::storage_request_bsps(location.clone(), bsp_account_id.clone()).is_none()
        );

        // Assert that the storage was updated
        assert_eq!(
            FileSystem::storage_requests(location.clone()),
            Some(StorageRequestMetadata {
                requested_at: 1,
                owner: owner_account_id.clone(),
                fingerprint,
                size,
                user_multiaddresses: multiaddresses.clone(),
                data_server_sps: BoundedVec::default(),
                bsps_required: TargetBspsRequired::<Test>::get(),
                bsps_confirmed: 0,
            })
        );
        // Assert that the correct event was deposited
        System::assert_last_event(
            Event::BspStoppedStoring {
                bsp: bsp_account_id,
                file_key,
                owner: owner_account_id,
                location,
            }
            .into(),
        );
    });
}
