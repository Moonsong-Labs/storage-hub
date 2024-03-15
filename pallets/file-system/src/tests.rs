use crate::{mock::*, types::FileLocation, Config, Event, StorageRequestExpirations};
use frame_support::assert_ok;
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

        // Dispatch a signed extrinsic.
        assert_ok!(FileSystem::issue_storage_request(
            user.clone(),
            location.clone(),
            fingerprint,
            4,
            BoundedVec::try_from(vec![1]).unwrap(),
        ));

        // Assert that the correct event was deposited
        System::assert_last_event(
            Event::NewStorageRequest {
                who: 1,
                location: location.clone(),
                fingerprint,
                size: 4,
                user_multiaddr: BoundedVec::try_from(vec![1]).unwrap(),
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

        // Dispatch a signed extrinsic.
        assert_ok!(FileSystem::issue_storage_request(
            user.clone(),
            location.clone(),
            fingerprint,
            4,
            BoundedVec::try_from(vec![1]).unwrap(),
        ));

        let expected_expiration_block_number: BlockNumber =
            FileSystem::next_expiration_block_number().into();

        // Assert that the storage request expiration was appended to the list at `StorageRequestTtl`
        assert_eq!(
            FileSystem::storage_request_expirations(expected_expiration_block_number as u64)
                .expect("storage request expirations should exist"),
            vec![location]
        );

        roll_to(1 + expected_expiration_block_number as u64);

        // Assert that the storage request expiration was removed from the list at `StorageRequestTtl`
        assert_eq!(
            FileSystem::storage_request_expirations(expected_expiration_block_number as u64),
            None
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

        let mut expected_expiration_block_number: BlockNumber =
            FileSystem::next_expiration_block_number().into();

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
            BoundedVec::try_from(vec![1]).unwrap(),
        ));

        // Assert that the storage request expirations storage is at max capacity
        assert_eq!(
            FileSystem::storage_request_expirations(expected_expiration_block_number)
                .expect("storage request expirations should exist")
                .len(),
            max_storage_request_expiry as usize
        );

        expected_expiration_block_number = FileSystem::next_expiration_block_number().into();

        // Assert that the `CurrentExpirationBlock` storage is incremented by 1
        assert_eq!(
            FileSystem::current_expiration_block(),
            expected_expiration_block_number
        );

        // Go to block number after which the storage request expirations should be removed
        roll_to(expected_expiration_block_number);

        // Assert that the storage request expiration was removed from the list at `StorageRequestTtl`
        assert_eq!(
            FileSystem::storage_request_expirations(expected_expiration_block_number),
            None
        );

        // Go to block number after which the second set of storage request expirations should be removed
        roll_to(2 + expected_expiration_block_number);

        expected_expiration_block_number = FileSystem::next_expiration_block_number().into();

        // Assert that the storage request expiration was removed from the list at `StorageRequestTtl`
        assert_eq!(
            FileSystem::storage_request_expirations(expected_expiration_block_number),
            None
        );
    });
}

#[test]
fn request_storage_expiration_current_block_increment_when_on_idle_skips_success() {
    // new_test_ext().execute_with(|| todo!());
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

        // TODO add this after adding identity pallet
        // Register BSP in Identity Pallet.
        // assert_ok!(Identity::register_user(RuntimeOrigin::root(), 2));

        // Dispatch storage request.
        assert_ok!(FileSystem::issue_storage_request(
            user.clone(),
            location.clone(),
            fingerprint,
            4,
            BoundedVec::try_from(vec![1]).unwrap(),
        ));

        // Dispatch BSP volunteer.
        assert_ok!(FileSystem::bsp_volunteer(
            bsp.clone(),
            location.clone(),
            fingerprint,
            BoundedVec::try_from(vec![2]).unwrap()
        ));

        // Assert that the correct event was deposited
        System::assert_last_event(
            Event::AcceptedBspVolunteer {
                who: 2,
                location,
                fingerprint,
                bsp_multiaddress: BoundedVec::try_from(vec![2]).unwrap(),
            }
            .into(),
        );
    });
}
