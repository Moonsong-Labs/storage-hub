use frame_support::{assert_noop, assert_ok};
use pallet_file_system::types::ValuePropId;
use pallet_storage_providers::types::ValueProposition;
use shp_traits::ReadBucketsInterface;
use sp_core::{ByteArray, Hasher};
use sp_keyring::sr25519::Keyring;
use sp_runtime::{bounded_vec, BoundedVec};

use crate::{
    mock::{new_test_ext, BucketNfts, FileSystem, RuntimeOrigin, System, Test},
    types::{ItemMetadata, ProviderIdFor, ReadAccessRegex},
    Error, Event,
};

mod share_access_tests {

    use super::*;

    #[test]
    fn share_access_success() {
        new_test_ext().execute_with(|| {
            let issuer = Keyring::Alice.to_account_id();
            let issuer_origin = RuntimeOrigin::signed(issuer.clone());
            let recipient = Keyring::Bob.to_account_id();
            let msp = Keyring::Charlie.to_account_id();
            let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();

            let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

            assert_ok!(FileSystem::create_bucket(
                issuer_origin.clone(),
                msp_id,
                bucket_name.clone(),
                true,
                value_prop_id
            ));

            let bucket_id =
                <<Test as crate::Config>::Buckets as ReadBucketsInterface>::derive_bucket_id(
                    &issuer,
                    bucket_name,
                );

            // Dispatch a signed extrinsic.
            assert_ok!(BucketNfts::share_access(
                issuer_origin,
                recipient.clone(),
                bucket_id,
                999,
                Some(basic_read_access_regex())
            ));

            // Assert that the item exists in the collection.
            assert!(pallet_nfts::pallet::Account::<Test>::contains_key((
                recipient.clone(),
                0,
                999
            )));

            // Assert that the item metadata exists.
            assert!(pallet_nfts::pallet::ItemMetadataOf::<Test>::contains_key(
                0, 999
            ));

            // Assert that the correct event was deposited
            System::assert_last_event(Event::AccessShared { issuer, recipient }.into());
        });
    }

    #[test]
    fn share_access_not_collection_owner_fail() {
        new_test_ext().execute_with(|| {
            let issuer = Keyring::Alice.to_account_id();
            let issuer_origin = RuntimeOrigin::signed(issuer.clone());
            let recipient = Keyring::Bob.to_account_id();
            let msp = Keyring::Charlie.to_account_id();
            let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();

            let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

            assert_ok!(FileSystem::create_bucket(
                issuer_origin.clone(),
                msp_id,
                bucket_name.clone(),
                true,
                value_prop_id
            ));

            let bucket_id =
                <<Test as crate::Config>::Buckets as ReadBucketsInterface>::derive_bucket_id(
                    &issuer,
                    bucket_name,
                );

            // Should fail since the issuer is not the owner of the bucket
            assert_noop!(
                BucketNfts::share_access(
                    RuntimeOrigin::signed(Keyring::Bob.to_account_id()),
                    recipient.clone(),
                    bucket_id,
                    999,
                    Some(basic_read_access_regex())
                ),
                pallet_nfts::Error::<Test>::NoPermission
            );
        });
    }

    #[test]
    fn share_access_bucket_not_found_fail() {
        new_test_ext().execute_with(|| {
            let issuer = Keyring::Alice.to_account_id();
            let issuer_origin = RuntimeOrigin::signed(issuer.clone());
            let recipient = Keyring::Bob.to_account_id();
            let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
            let msp = Keyring::Charlie.to_account_id();

            let _ = add_msp_to_provider_storage(&msp);

            let bucket_id =
                <<Test as crate::Config>::Buckets as ReadBucketsInterface>::derive_bucket_id(
                    &issuer,
                    bucket_name,
                );

            // Should fail since the bucket does not exist
            assert_noop!(
                BucketNfts::share_access(
                    issuer_origin,
                    recipient.clone(),
                    bucket_id,
                    999,
                    Some(basic_read_access_regex())
                ),
                pallet_storage_providers::Error::<Test>::BucketNotFound
            );
        });
    }

    #[test]
    fn share_access_private_bucket_fail() {
        new_test_ext().execute_with(|| {
            let issuer = Keyring::Alice.to_account_id();
            let issuer_origin = RuntimeOrigin::signed(issuer.clone());
            let recipient = Keyring::Bob.to_account_id();
            let msp = Keyring::Charlie.to_account_id();
            let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();

            let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

            // Create a public bucket (no collection ID)
            assert_ok!(FileSystem::create_bucket(
                issuer_origin.clone(),
                msp_id,
                bucket_name.clone(),
                false,
                value_prop_id
            ));

            let bucket_id =
                <<Test as crate::Config>::Buckets as ReadBucketsInterface>::derive_bucket_id(
                    &issuer,
                    bucket_name,
                );

            // Should fail since public buckets do not have a corresponding collection
            assert_noop!(
                BucketNfts::share_access(
                    issuer_origin,
                    recipient.clone(),
                    bucket_id,
                    0,
                    Some(basic_read_access_regex())
                ),
                Error::<Test>::BucketIsNotPrivate
            );
        });
    }

    #[test]
    fn share_access_after_private_bucket_became_public_fail() {
        new_test_ext().execute_with(|| {
            let issuer = Keyring::Alice.to_account_id();
            let issuer_origin = RuntimeOrigin::signed(issuer.clone());
            let recipient = Keyring::Bob.to_account_id();
            let msp = Keyring::Charlie.to_account_id();
            let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();

            let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

            assert_ok!(FileSystem::create_bucket(
                issuer_origin.clone(),
                msp_id,
                bucket_name.clone(),
                true,
                value_prop_id
            ));

            let bucket_id =
                <<Test as crate::Config>::Buckets as ReadBucketsInterface>::derive_bucket_id(
                    &issuer,
                    bucket_name,
                );

            // Make the bucket public
            assert_ok!(FileSystem::update_bucket_privacy(
                issuer_origin.clone(),
                bucket_id,
                false
            ));

            // Should fail since public buckets do not have a corresponding collection
            assert_noop!(
                BucketNfts::share_access(
                    issuer_origin,
                    recipient.clone(),
                    bucket_id,
                    0,
                    Some(basic_read_access_regex())
                ),
                Error::<Test>::BucketIsNotPrivate
            );
        });
    }
}

mod update_read_access_tests {
    use super::*;

    #[test]
    fn update_read_access_success() {
        new_test_ext().execute_with(|| {
            let issuer = Keyring::Alice.to_account_id();
            let issuer_origin = RuntimeOrigin::signed(issuer.clone());
            let recipient = Keyring::Bob.to_account_id();
            let msp = Keyring::Charlie.to_account_id();
            let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();

            let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

            assert_ok!(FileSystem::create_bucket(
                issuer_origin.clone(),
                msp_id,
                bucket_name.clone(),
                true,
                value_prop_id
            ));

            let bucket_id =
                <<Test as crate::Config>::Buckets as ReadBucketsInterface>::derive_bucket_id(
                    &issuer,
                    bucket_name,
                );

            ItemMetadata::<Test>::new(None);

            // Share access to the bucket
            assert_ok!(BucketNfts::share_access(
                issuer_origin.clone(),
                recipient.clone(),
                bucket_id,
                999,
                Some(basic_read_access_regex())
            ));

            // Dispatch a signed extrinsic.
            assert_ok!(BucketNfts::update_read_access(
                issuer_origin,
                bucket_id,
                999,
                Some(basic_read_access_regex())
            ));

            // Assert that the item metadata exists.
            assert!(pallet_nfts::pallet::ItemMetadataOf::<Test>::contains_key(
                0, 999
            ));

            // Assert that the correct event was deposited
            System::assert_last_event(
                Event::ItemReadAccessUpdated {
                    admin: issuer,
                    bucket: bucket_id,
                    item_id: 999,
                }
                .into(),
            );
        });
    }

    #[test]
    fn update_read_access_not_collection_owner_fail() {
        new_test_ext().execute_with(|| {
            let issuer = Keyring::Alice.to_account_id();
            let issuer_origin = RuntimeOrigin::signed(issuer.clone());
            let recipient = Keyring::Bob.to_account_id();
            let msp = Keyring::Charlie.to_account_id();
            let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();

            let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

            assert_ok!(FileSystem::create_bucket(
                issuer_origin.clone(),
                msp_id,
                bucket_name.clone(),
                true,
                value_prop_id
            ));

            let bucket_id =
                <<Test as crate::Config>::Buckets as ReadBucketsInterface>::derive_bucket_id(
                    &issuer,
                    bucket_name,
                );

            // Share access to the bucket
            assert_ok!(BucketNfts::share_access(
                issuer_origin.clone(),
                recipient.clone(),
                bucket_id,
                999,
                Some(basic_read_access_regex())
            ));

            // Should fail since the issuer is not the owner of the bucket
            assert_noop!(
                BucketNfts::update_read_access(
                    RuntimeOrigin::signed(Keyring::Bob.to_account_id()),
                    bucket_id,
                    999,
                    Some(basic_read_access_regex())
                ),
                pallet_nfts::Error::<Test>::NoPermission
            );
        });
    }

    #[test]
    fn update_read_access_bucket_not_found_fail() {
        new_test_ext().execute_with(|| {
            let issuer = Keyring::Alice.to_account_id();
            let issuer_origin = RuntimeOrigin::signed(issuer.clone());
            let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
            let msp = Keyring::Charlie.to_account_id();

            let _ = add_msp_to_provider_storage(&msp);

            let bucket_id =
                <<Test as crate::Config>::Buckets as ReadBucketsInterface>::derive_bucket_id(
                    &issuer,
                    bucket_name,
                );

            // Should fail since the bucket does not exist
            assert_noop!(
                BucketNfts::update_read_access(
                    issuer_origin,
                    bucket_id,
                    999,
                    Some(basic_read_access_regex())
                ),
                pallet_storage_providers::Error::<Test>::BucketNotFound
            );
        });
    }

    #[test]
    fn update_read_access_item_not_found_fail() {
        new_test_ext().execute_with(|| {
            let issuer = Keyring::Alice.to_account_id();
            let issuer_origin = RuntimeOrigin::signed(issuer.clone());
            let msp = Keyring::Charlie.to_account_id();
            let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();

            let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

            assert_ok!(FileSystem::create_bucket(
                issuer_origin.clone(),
                msp_id,
                bucket_name.clone(),
                true,
                value_prop_id
            ));

            let bucket_id =
                <<Test as crate::Config>::Buckets as ReadBucketsInterface>::derive_bucket_id(
                    &issuer,
                    bucket_name,
                );

            // Should fail since the item does not exist
            assert_noop!(
                BucketNfts::update_read_access(
                    issuer_origin,
                    bucket_id,
                    999,
                    Some(basic_read_access_regex())
                ),
                pallet_nfts::Error::<Test>::UnknownItem
            );
        });
    }
}

fn basic_read_access_regex() -> ReadAccessRegex<Test> {
    BoundedVec::try_from(b"*".to_vec()).unwrap()
}

fn add_msp_to_provider_storage(
    msp: &sp_runtime::AccountId32,
) -> (ProviderIdFor<Test>, ValuePropId<Test>) {
    let msp_hash = <<Test as frame_system::Config>::Hashing as Hasher>::hash(msp.as_slice());

    let msp_info = pallet_storage_providers::types::MainStorageProvider {
        capacity: 100,
        capacity_used: 0,
        multiaddresses: BoundedVec::default(),
        last_capacity_change: frame_system::Pallet::<Test>::block_number(),
        owner_account: msp.clone(),
        payment_account: msp.clone(),
        sign_up_block: frame_system::Pallet::<Test>::block_number(),
    };

    pallet_storage_providers::MainStorageProviders::<Test>::insert(msp_hash, msp_info);
    pallet_storage_providers::AccountIdToMainStorageProviderId::<Test>::insert(
        msp.clone(),
        msp_hash,
    );

    let value_prop = ValueProposition::<Test>::new(1, bounded_vec![], 100);
    let value_prop_id = value_prop.derive_id();
    pallet_storage_providers::MainStorageProviderIdsToValuePropositions::<Test>::insert(
        msp_hash,
        value_prop_id,
        value_prop,
    );

    (msp_hash, value_prop_id)
}
