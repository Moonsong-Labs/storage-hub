use crate::{
    mock::*,
    types::{
        FileLocation, MultiAddress, StorageData, StorageRequestBspsMetadata,
        StorageRequestMetadata, TargetBspsRequired,
    },
    Config, Error, Event, StorageRequestExpirations,
};
use frame_support::{assert_noop, assert_ok, traits::Hooks, weights::Weight};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, Get, Hash, Zero},
    AccountId32, BoundedVec, FixedU128,
};
use storage_hub_traits::SubscribeProvidersInterface;

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
        let multiaddresses: BoundedVec<
            MultiAddress<Test>,
            <Test as Config>::MaxDataServerMultiAddresses,
        > = BoundedVec::try_from(vec![multiaddr]).unwrap();

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
                bsps_volunteered: 0,
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
        let owner_account_id = AccountId32::new([1; 32]);
        let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let multiaddr = BoundedVec::try_from(vec![1]).unwrap();
        let multiaddresses: BoundedVec<
            MultiAddress<Test>,
            <Test as Config>::MaxDataServerMultiAddresses,
        > = BoundedVec::try_from(vec![multiaddr]).unwrap();
        let size = 4;

        // Dispatch a signed extrinsic.
        assert_ok!(FileSystem::issue_storage_request(
            owner_signed.clone(),
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
                bsps_volunteered: 0,
            })
        );

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
        let multiaddresses: BoundedVec<
            MultiAddress<Test>,
            <Test as Config>::MaxDataServerMultiAddresses,
        > = BoundedVec::try_from(vec![multiaddr]).unwrap();

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
        let multiaddresses: BoundedVec<
            MultiAddress<Test>,
            <Test as Config>::MaxDataServerMultiAddresses,
        > = BoundedVec::try_from(vec![multiaddr]).unwrap();

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
fn revoke_storage_request_not_owner_fail() {
    new_test_ext().execute_with(|| {
        let owner = RuntimeOrigin::signed(AccountId32::new([1; 32]));
        let not_owner = RuntimeOrigin::signed(AccountId32::new([2; 32]));
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

        assert_noop!(
            FileSystem::revoke_storage_request(not_owner.clone(), location.clone(), file_key),
            Error::<Test>::StorageRequestNotAuthorized
        );
    });
}

#[test]
fn bsp_volunteer_success() {
    new_test_ext().execute_with(|| {
        let owner = RuntimeOrigin::signed(AccountId32::new([1; 32]));
        let bsp_account_id = AccountId32::new([2; 32]);
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        // TODO: right now we are bypassing the volunteer assignment threshold
        let fingerprint = H256::zero();
        let multiaddr = BoundedVec::try_from(vec![1]).unwrap();
        let multiaddresses: BoundedVec<
            MultiAddress<Test>,
            <Test as Config>::MaxDataServerMultiAddresses,
        > = BoundedVec::try_from(vec![multiaddr]).unwrap();
        let storage_amount: StorageData<Test> = 100;

        // Dispatch storage request.
        assert_ok!(FileSystem::issue_storage_request(
            owner.clone(),
            location.clone(),
            fingerprint,
            4,
            multiaddresses.clone(),
        ));

        // Sign up account as a Backup Storage Provider
        assert_ok!(Providers::bsp_sign_up(
            bsp_signed.clone(),
            storage_amount,
            multiaddresses.clone(),
        ));

        // Dispatch BSP volunteer.
        assert_ok!(FileSystem::bsp_volunteer(
            bsp_signed.clone(),
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
fn bsp_volunteer_storage_request_not_found_fail() {
    new_test_ext().execute_with(|| {
        let bsp_account_id = AccountId32::new([2; 32]);
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let fingerprint = H256::zero();
        let multiaddr = BoundedVec::try_from(vec![1]).unwrap();
        let multiaddresses: BoundedVec<
            MultiAddress<Test>,
            <Test as Config>::MaxDataServerMultiAddresses,
        > = BoundedVec::try_from(vec![multiaddr]).unwrap();

        assert_ok!(Providers::bsp_sign_up(
            bsp_signed.clone(),
            100,
            multiaddresses.clone(),
        ));

        assert_noop!(
            FileSystem::bsp_volunteer(
                bsp_signed.clone(),
                location.clone(),
                fingerprint,
                multiaddresses.clone()
            ),
            Error::<Test>::StorageRequestNotFound
        );
    });
}

#[test]
fn bsp_already_volunteered_failed() {
    new_test_ext().execute_with(|| {
        let owner_account_id = AccountId32::new([1; 32]);
        let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
        let bsp_account_id = AccountId32::new([2; 32]);
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let size = 4;
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let multiaddr = BoundedVec::try_from(vec![1]).unwrap();
        let multiaddresses: BoundedVec<
            MultiAddress<Test>,
            <Test as Config>::MaxDataServerMultiAddresses,
        > = BoundedVec::try_from(vec![multiaddr]).unwrap();
        let storage_amount: StorageData<Test> = 100;

        // Dispatch storage request.
        assert_ok!(FileSystem::issue_storage_request(
            owner_signed.clone(),
            location.clone(),
            fingerprint,
            size,
            multiaddresses.clone(),
        ));

        // Sign up account as a Backup Storage Provider
        assert_ok!(Providers::bsp_sign_up(
            bsp_signed.clone(),
            storage_amount,
            multiaddresses.clone(),
        ));

        // Dispatch BSP volunteer.
        assert_ok!(FileSystem::bsp_volunteer(
            bsp_signed.clone(),
            location.clone(),
            fingerprint,
            multiaddresses.clone()
        ));

        assert_noop!(
            FileSystem::bsp_volunteer(
                bsp_signed.clone(),
                location.clone(),
                fingerprint,
                multiaddresses.clone()
            ),
            Error::<Test>::BspAlreadyVolunteered
        );
    });
}

#[test]
fn bsp_volunteer_threshold_too_high_fail() {
    new_test_ext().execute_with(|| {
        let owner_account_id = AccountId32::new([1; 32]);
        let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
        let bsp_account_id = AccountId32::new([2; 32]);
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let size = 4;
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let multiaddr = BoundedVec::try_from(vec![1]).unwrap();
        let multiaddresses: BoundedVec<
            MultiAddress<Test>,
            <Test as Config>::MaxDataServerMultiAddresses,
        > = BoundedVec::try_from(vec![multiaddr]).unwrap();
        let storage_amount: StorageData<Test> = 100;

        // Dispatch storage request.
        assert_ok!(FileSystem::issue_storage_request(
            owner_signed.clone(),
            location.clone(),
            fingerprint,
            size,
            multiaddresses.clone(),
        ));

        // Sign up account as a Backup Storage Provider
        assert_ok!(Providers::bsp_sign_up(
            bsp_signed.clone(),
            storage_amount,
            multiaddresses.clone(),
        ));

        crate::BspsAssignmentThreshold::<Test>::put(FixedU128::zero());

        // Dispatch BSP volunteer.
        assert_noop!(
            FileSystem::bsp_volunteer(
                bsp_signed.clone(),
                location.clone(),
                fingerprint,
                multiaddresses.clone()
            ),
            Error::<Test>::ThresholdTooHigh
        );
    });
}

#[test]
fn bsp_confirm_storing_success() {
    new_test_ext().execute_with(|| {
        let owner_account_id = AccountId32::new([1; 32]);
        let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
        let bsp_account_id = AccountId32::new([2; 32]);
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let size = 4;
        let fingerprint = H256::zero();
        let multiaddr = BoundedVec::try_from(vec![1]).unwrap();
        let multiaddresses: BoundedVec<
            MultiAddress<Test>,
            <Test as Config>::MaxDataServerMultiAddresses,
        > = BoundedVec::try_from(vec![multiaddr]).unwrap();
        let storage_amount: StorageData<Test> = 100;

        // Dispatch storage request.
        assert_ok!(FileSystem::issue_storage_request(
            owner_signed.clone(),
            location.clone(),
            fingerprint,
            size,
            multiaddresses.clone(),
        ));

        // Sign up account as a Backup Storage Provider
        assert_ok!(Providers::bsp_sign_up(
            bsp_signed.clone(),
            storage_amount,
            multiaddresses.clone(),
        ));

        // Dispatch BSP volunteer.
        assert_ok!(FileSystem::bsp_volunteer(
            bsp_signed.clone(),
            location.clone(),
            fingerprint,
            Default::default()
        ));

        // Dispatch BSP confirm storing.
        assert_ok!(FileSystem::bsp_confirm_storing(
            bsp_signed.clone(),
            location.clone(),
            H256::zero(), // TODO construct a real proof
            pallet_proofs_dealer::CompactProof {
                encoded_nodes: vec![],
            }
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
                bsps_confirmed: 1,
                bsps_volunteered: 1,
            })
        );

        // Assert that the RequestStorageBsps was updated
        assert_eq!(
            FileSystem::storage_request_bsps(location.clone(), bsp_account_id.clone())
                .expect("BSP should exist in storage"),
            StorageRequestBspsMetadata::<Test> {
                confirmed: true,
                _phantom: Default::default()
            }
        );

        // Assert that the correct event was deposited
        System::assert_last_event(
            Event::BspConfirmedStoring {
                who: bsp_account_id,
                location,
            }
            .into(),
        );
    });
}

#[test]
fn bsp_confirm_storing_storage_request_not_found_fail() {
    new_test_ext().execute_with(|| {
        let bsp_account_id = AccountId32::new([2; 32]);
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let multiaddr = BoundedVec::try_from(vec![1]).unwrap();
        let multiaddresses: BoundedVec<
            MultiAddress<Test>,
            <Test as Config>::MaxDataServerMultiAddresses,
        > = BoundedVec::try_from(vec![multiaddr]).unwrap();

        // Sign up account as a Backup Storage Provider
        assert_ok!(Providers::bsp_sign_up(
            bsp_signed.clone(),
            100,
            multiaddresses.clone(),
        ));

        assert_noop!(
            FileSystem::bsp_confirm_storing(
                bsp_signed.clone(),
                location.clone(),
                H256::zero(), // TODO construct a real proof
                pallet_proofs_dealer::CompactProof {
                    encoded_nodes: vec![],
                }
            ),
            Error::<Test>::StorageRequestNotFound
        );
    });
}

#[test]
fn bsp_confirm_storing_not_volunteered_fail() {
    new_test_ext().execute_with(|| {
        let owner_account_id = AccountId32::new([1; 32]);
        let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
        let bsp_account_id = AccountId32::new([2; 32]);
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let size = 4;
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let multiaddr = BoundedVec::try_from(vec![1]).unwrap();
        let multiaddresses: BoundedVec<
            MultiAddress<Test>,
            <Test as Config>::MaxDataServerMultiAddresses,
        > = BoundedVec::try_from(vec![multiaddr]).unwrap();
        let storage_amount: StorageData<Test> = 100;

        // Dispatch storage request.
        assert_ok!(FileSystem::issue_storage_request(
            owner_signed.clone(),
            location.clone(),
            fingerprint,
            size,
            multiaddresses.clone(),
        ));

        // Sign up account as a Backup Storage Provider
        assert_ok!(Providers::bsp_sign_up(
            bsp_signed.clone(),
            storage_amount,
            multiaddresses.clone(),
        ));

        assert_noop!(
            FileSystem::bsp_confirm_storing(
                bsp_signed.clone(),
                location.clone(),
                H256::zero(), // TODO construct a real proof
                pallet_proofs_dealer::CompactProof {
                    encoded_nodes: vec![],
                }
            ),
            Error::<Test>::BspNotVolunteered
        );
    });
}

#[test]
fn bsp_already_confirmed_fail() {
    new_test_ext().execute_with(|| {
        let owner_account_id = AccountId32::new([1; 32]);
        let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
        let bsp_account_id = AccountId32::new([2; 32]);
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let size = 4;
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let multiaddr = BoundedVec::try_from(vec![1]).unwrap();
        let multiaddresses: BoundedVec<
            MultiAddress<Test>,
            <Test as Config>::MaxDataServerMultiAddresses,
        > = BoundedVec::try_from(vec![multiaddr]).unwrap();
        let storage_amount: StorageData<Test> = 100;

        // Dispatch storage request.
        assert_ok!(FileSystem::issue_storage_request(
            owner_signed.clone(),
            location.clone(),
            fingerprint,
            size,
            multiaddresses.clone(),
        ));

        // Sign up account as a Backup Storage Provider
        assert_ok!(Providers::bsp_sign_up(
            bsp_signed.clone(),
            storage_amount,
            multiaddresses.clone(),
        ));

        // Dispatch BSP volunteer.
        assert_ok!(FileSystem::bsp_volunteer(
            bsp_signed.clone(),
            location.clone(),
            fingerprint,
            multiaddresses.clone()
        ));

        // Dispatch BSP confirm storing.
        assert_ok!(FileSystem::bsp_confirm_storing(
            bsp_signed.clone(),
            location.clone(),
            H256::zero(), // TODO construct a real proof
            pallet_proofs_dealer::CompactProof {
                encoded_nodes: vec![],
            }
        ));

        assert_noop!(
            FileSystem::bsp_confirm_storing(
                bsp_signed.clone(),
                location.clone(),
                H256::zero(), // TODO construct a real proof
                pallet_proofs_dealer::CompactProof {
                    encoded_nodes: vec![],
                }
            ),
            Error::<Test>::BspAlreadyConfirmed
        );
    });
}

#[test]
fn bsp_actions_not_a_bsp_fail() {
    new_test_ext().execute_with(|| {
        let owner_account_id = AccountId32::new([1; 32]);
        let bsp_account_id = AccountId32::new([2; 32]);
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let size = 4;
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let multiaddr = BoundedVec::try_from(vec![1]).unwrap();
        let multiaddresses: BoundedVec<
            MultiAddress<Test>,
            <Test as Config>::MaxDataServerMultiAddresses,
        > = BoundedVec::try_from(vec![multiaddr]).unwrap();

        // Dispatch storage request.
        assert_ok!(FileSystem::issue_storage_request(
            RuntimeOrigin::signed(owner_account_id.clone()),
            location.clone(),
            fingerprint,
            size,
            multiaddresses.clone(),
        ));

        assert_noop!(
            FileSystem::bsp_volunteer(
                bsp_signed.clone(),
                location.clone(),
                fingerprint,
                multiaddresses.clone()
            ),
            Error::<Test>::NotABsp
        );

        assert_noop!(
            FileSystem::bsp_volunteer(
                bsp_signed.clone(),
                location.clone(),
                fingerprint,
                multiaddresses.clone()
            ),
            Error::<Test>::NotABsp
        );

        assert_noop!(
            FileSystem::bsp_confirm_storing(
                bsp_signed.clone(),
                location.clone(),
                H256::zero(), // TODO construct a real proof
                pallet_proofs_dealer::CompactProof {
                    encoded_nodes: vec![],
                }
            ),
            Error::<Test>::NotABsp
        );
    });
}

#[test]
fn bsp_stop_storing_success() {
    new_test_ext().execute_with(|| {
        let owner_account_id = AccountId32::new([1; 32]);
        let owner = RuntimeOrigin::signed(owner_account_id.clone());
        let bsp_account_id = AccountId32::new([2; 32]);
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let file_key = H256::from_slice(&[1; 32]);
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let size = 4;
        // TODO: right now we are bypassing the volunteer assignment threshold
        let fingerprint = H256::zero();
        let multiaddr = BoundedVec::try_from(vec![1]).unwrap();
        let multiaddresses: BoundedVec<
            MultiAddress<Test>,
            <Test as Config>::MaxDataServerMultiAddresses,
        > = BoundedVec::try_from(vec![multiaddr]).unwrap();
        let storage_amount: StorageData<Test> = 100;

        // Dispatch storage request.
        assert_ok!(FileSystem::issue_storage_request(
            owner.clone(),
            location.clone(),
            fingerprint,
            size,
            multiaddresses.clone(),
        ));

        // Sign up account as a Backup Storage Provider
        assert_ok!(Providers::bsp_sign_up(
            bsp_signed.clone(),
            storage_amount,
            multiaddresses.clone(),
        ));

        // Dispatch BSP volunteer.
        assert_ok!(FileSystem::bsp_volunteer(
            bsp_signed.clone(),
            location.clone(),
            fingerprint,
            multiaddresses.clone()
        ));

        // Dispatch BSP confirm storing.
        assert_ok!(FileSystem::bsp_confirm_storing(
            bsp_signed.clone(),
            location.clone(),
            H256::zero(), // TODO construct a real proof
            pallet_proofs_dealer::CompactProof {
                encoded_nodes: vec![],
            }
        ));

        // Assert that the RequestStorageBsps now contains the BSP under the location
        assert_eq!(
            FileSystem::storage_request_bsps(location.clone(), bsp_account_id.clone())
                .expect("BSP should exist in storage"),
            StorageRequestBspsMetadata::<Test> {
                confirmed: true,
                _phantom: Default::default()
            }
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
                bsps_confirmed: 1,
                bsps_volunteered: 1,
            })
        );

        // Dispatch BSP stop storing.
        assert_ok!(FileSystem::bsp_stop_storing(
            bsp_signed.clone(),
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
                bsps_volunteered: 0,
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

#[test]
fn bsp_stop_storing_while_storage_request_open_success() {
    new_test_ext().execute_with(|| {
        let owner_account_id = AccountId32::new([1; 32]);
        let owner = RuntimeOrigin::signed(owner_account_id.clone());
        let bsp_account_id = AccountId32::new([2; 32]);
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let file_key = H256::from_slice(&[1; 32]);
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let size = 4;
        let multiaddr = BoundedVec::try_from(vec![1]).unwrap();
        let multiaddresses: BoundedVec<
            MultiAddress<Test>,
            <Test as Config>::MaxDataServerMultiAddresses,
        > = BoundedVec::try_from(vec![multiaddr]).unwrap();
        let storage_amount: StorageData<Test> = 100;

        // Dispatch storage request.
        assert_ok!(FileSystem::issue_storage_request(
            owner.clone(),
            location.clone(),
            H256::zero(),
            size,
            Default::default(),
        ));

        // Sign up account as a Backup Storage Provider
        assert_ok!(Providers::bsp_sign_up(
            bsp_signed.clone(),
            storage_amount,
            multiaddresses.clone(),
        ));

        // Dispatch BSP volunteer.
        assert_ok!(FileSystem::bsp_volunteer(
            bsp_signed.clone(),
            location.clone(),
            H256::zero(),
            Default::default()
        ));

        // Dispatch BSP confirm storing.
        assert_ok!(FileSystem::bsp_confirm_storing(
            bsp_signed.clone(),
            location.clone(),
            H256::zero(),
            pallet_proofs_dealer::CompactProof {
                encoded_nodes: vec![],
            }
        ));

        // Dispatch BSP stop storing.
        assert_ok!(FileSystem::bsp_stop_storing(
            bsp_signed.clone(),
            file_key,
            location.clone(),
            owner_account_id.clone(),
            H256::zero(),
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
                fingerprint: H256::zero(),
                size,
                user_multiaddresses: Default::default(),
                data_server_sps: BoundedVec::default(),
                bsps_required: TargetBspsRequired::<Test>::get(),
                bsps_confirmed: 0,
                bsps_volunteered: 0,
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

#[test]
fn bsp_stop_storing_not_volunteered_success() {
    new_test_ext().execute_with(|| {
        let owner_account_id = AccountId32::new([1; 32]);
        let owner = RuntimeOrigin::signed(owner_account_id.clone());
        let bsp_account_id = AccountId32::new([2; 32]);
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let file_key = H256::from_slice(&[1; 32]);
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let size = 4;
        let fingerprint = H256::zero();
        let multiaddr = BoundedVec::try_from(vec![1]).unwrap();
        let multiaddresses: BoundedVec<
            MultiAddress<Test>,
            <Test as Config>::MaxDataServerMultiAddresses,
        > = BoundedVec::try_from(vec![multiaddr]).unwrap();
        let storage_amount: StorageData<Test> = 100;

        // Dispatch storage request.
        assert_ok!(FileSystem::issue_storage_request(
            owner.clone(),
            location.clone(),
            fingerprint,
            size,
            Default::default(),
        ));

        // Sign up account as a Backup Storage Provider
        assert_ok!(Providers::bsp_sign_up(
            bsp_signed.clone(),
            storage_amount,
            multiaddresses.clone(),
        ));

        // Dispatch BSP stop storing.
        assert_ok!(FileSystem::bsp_stop_storing(
            bsp_signed.clone(),
            file_key,
            location.clone(),
            owner_account_id.clone(),
            fingerprint,
            size,
            false
        ));

        let current_bsps_required: <Test as Config>::StorageRequestBspsRequiredType =
            TargetBspsRequired::<Test>::get();

        // Assert that the storage request bsps_required was incremented
        assert_eq!(
            FileSystem::storage_requests(location.clone()),
            Some(StorageRequestMetadata {
                requested_at: 1,
                owner: owner_account_id.clone(),
                fingerprint,
                size,
                user_multiaddresses: Default::default(),
                data_server_sps: BoundedVec::default(),
                bsps_required: current_bsps_required.checked_add(1).unwrap(),
                bsps_confirmed: 0,
                bsps_volunteered: 0,
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

#[test]
fn bsp_stop_storing_no_storage_request_success() {
    new_test_ext().execute_with(|| {
        let bsp_account_id = AccountId32::new([2; 32]);
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let file_key = H256::from_slice(&[1; 32]);
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let owner_account_id = AccountId32::new([1; 32]);
        let size = 4;
        let fingerprint = H256::zero();

        // Dispatch BSP stop storing.
        assert_ok!(FileSystem::bsp_stop_storing(
            bsp_signed.clone(),
            file_key,
            location.clone(),
            owner_account_id.clone(),
            fingerprint,
            size,
            false
        ));

        // Assert that the storage request was created with one bsps_required
        assert_eq!(
            FileSystem::storage_requests(location.clone()),
            Some(StorageRequestMetadata {
                requested_at: 1,
                owner: owner_account_id.clone(),
                fingerprint,
                size,
                user_multiaddresses: Default::default(),
                data_server_sps: BoundedVec::default(),
                bsps_required: 1,
                bsps_confirmed: 0,
                bsps_volunteered: 0,
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

#[test]
fn compute_asymptotic_threshold_point_success() {
    new_test_ext().execute_with(|| {
        // Test the computation of the asymptotic threshold
        let threshold = FileSystem::compute_asymptotic_threshold_point(1)
            .expect("Threshold should be computable");

        // Assert that the computed threshold is as expected
        assert!(
            threshold > FixedU128::zero(),
            "Threshold should be positive"
        );
    });
}

#[test]
fn subscribe_bsp_sign_up_decreases_threshold_success() {
    new_test_ext().execute_with(|| {
        let initial_threshold = compute_set_get_initial_threshold();

        // Simulate the threshold decrease due to a new BSP sign up
        FileSystem::subscribe_bsp_sign_up(&AccountId32::new([2; 32]))
            .expect("BSP sign up should be successful");

        // Verify that the threshold decreased
        assert!(
            FileSystem::bsps_assignment_threshold() < initial_threshold,
            "Threshold should decrease after BSP sign up"
        );
    });
}

#[test]
fn subscribe_bsp_sign_off_increases_threshold_success() {
    new_test_ext().execute_with(|| {
        let initial_threshold = compute_set_get_initial_threshold();

        // Simulate the threshold increase due to a new BSP sign off
        FileSystem::subscribe_bsp_sign_off(&AccountId32::new([2; 32]))
            .expect("BSP sign off should be successful");

        // Verify that the threshold increased
        assert!(
            FileSystem::bsps_assignment_threshold() > initial_threshold,
            "Threshold should increase after BSP sign off"
        );
    });
}

#[test]
fn threshold_does_not_exceed_asymptote_success() {
    new_test_ext().execute_with(|| {
        crate::BspsAssignmentThreshold::<Test>::put(
            <Test as Config>::AssignmentThresholdAsymptote::get(),
        );

        // Simulate the threshold decrease due to a new BSP sign up
        FileSystem::subscribe_bsp_sign_up(&AccountId32::new([2; 32]))
            .expect("BSP sign up should be successful");

        // Verify that the threshold does is equal to the asymptote
        assert!(
            FileSystem::bsps_assignment_threshold()
                == <Test as Config>::AssignmentThresholdAsymptote::get(),
            "Threshold should not go below the asymptote"
        );
    });
}
