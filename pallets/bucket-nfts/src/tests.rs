use frame_support::{assert_noop, assert_ok};
use sp_core::{ByteArray, Hasher};
use sp_keyring::sr25519::Keyring;
use sp_runtime::BoundedVec;
use storage_hub_traits::ReadProvidersInterface;

use crate::{
    mock::{new_test_ext, BucketNfts, FileSystem, RuntimeOrigin, System, Test},
    Error, Event,
};

#[test]
fn share_access_success() {
    new_test_ext().execute_with(|| {
        let issuer = Keyring::Alice.to_account_id();
        let issuer_origin = RuntimeOrigin::signed(issuer.clone());
        let recipient = Keyring::Bob.to_account_id();
        let msp = Keyring::Charlie.to_account_id();
        let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();

        add_msp_to_provider_storage(&msp);

        assert_ok!(FileSystem::create_bucket(
            issuer_origin.clone(),
            msp,
            bucket_name.clone(),
            pallet_file_system::types::BucketPrivacy::Private
        ));

        let bucket_id =
            <<Test as crate::Config>::Providers as ReadProvidersInterface>::derive_bucket_id(
                &issuer,
                bucket_name,
            );

        // Dispatch a signed extrinsic.
        assert_ok!(BucketNfts::share_access(
            issuer_origin,
            recipient.clone(),
            bucket_id,
            999,
            BoundedVec::try_from(b"*".to_vec()).unwrap()
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
fn share_access_private_bucket_fail() {
    new_test_ext().execute_with(|| {
        let issuer = Keyring::Alice.to_account_id();
        let issuer_origin = RuntimeOrigin::signed(issuer.clone());
        let recipient = Keyring::Bob.to_account_id();
        let msp = Keyring::Charlie.to_account_id();
        let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();

        add_msp_to_provider_storage(&msp);

        // Create a public bucket (no collection ID)
        assert_ok!(FileSystem::create_bucket(
            issuer_origin.clone(),
            msp,
            bucket_name.clone(),
            pallet_file_system::types::BucketPrivacy::Public
        ));

        let bucket_id =
            <<Test as crate::Config>::Providers as ReadProvidersInterface>::derive_bucket_id(
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
                BoundedVec::try_from(b"*".to_vec()).unwrap()
            ),
            Error::<Test>::BucketIsNotPrivate
        );
    });
}

fn add_msp_to_provider_storage(msp: &sp_runtime::AccountId32) {
    // insert msp into storage of providers pallet
    let msp_hash = <<Test as frame_system::Config>::Hashing as Hasher>::hash(msp.as_slice());

    // Set up a structure with the information of the new MSP
    let msp_info = pallet_storage_providers::types::MainStorageProvider {
        buckets: BoundedVec::default(),
        capacity: 100,
        data_used: 0,
        multiaddresses: BoundedVec::default(),
        value_prop: pallet_storage_providers::types::ValueProposition {
            identifier: pallet_storage_providers::types::ValuePropId::<Test>::default(),
            data_limit: 100,
            protocols: BoundedVec::default(),
        },
        last_capacity_change: frame_system::Pallet::<Test>::block_number(),
    };

    pallet_storage_providers::MainStorageProviders::<Test>::insert(msp_hash, msp_info);
    pallet_storage_providers::AccountIdToMainStorageProviderId::<Test>::insert(
        msp.clone(),
        msp_hash,
    );
}
