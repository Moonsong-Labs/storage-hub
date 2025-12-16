use super::{types::*, *};
use frame_benchmarking::v2::*;

#[benchmarks(where
	// T is the runtime which has to be comprised of the following pallets:
    T: crate::Config<Fingerprint = <T as frame_system::Config>::Hash, Providers = pallet_storage_providers::Pallet<T>>
		+ pallet_storage_providers::Config<
			ProviderId = <T as frame_system::Config>::Hash,
			MerklePatriciaRoot = <T as frame_system::Config>::Hash,
			StorageDataUnit = u64
		>
		+ pallet_nfts::Config
		+ pallet_proofs_dealer::Config
		+ pallet_payment_streams::Config,
	// The Providers element of this pallet's config has to implement the `MutateStorageProvidersInterface`, `ReadProvidersInterface`, `ReadBucketsInterface` and `MutateBucketsInterface` traits with the following types:
    <T as crate::Config>::Providers: shp_traits::MutateStorageProvidersInterface<StorageDataUnit = u64>
        + shp_traits::ReadProvidersInterface<ProviderId = <T as frame_system::Config>::Hash, MerkleHash = <T as frame_system::Config>::Hash>
		+ shp_traits::ReadBucketsInterface<BucketId = <T as frame_system::Config>::Hash, AccountId = <T as frame_system::Config>::AccountId, ProviderId = <T as frame_system::Config>::Hash, ReadAccessGroupId = <T as pallet_nfts::Config>::CollectionId, MerkleHash = <T as frame_system::Config>::Hash>
		+ shp_traits::MutateBucketsInterface<AccountId = <T as frame_system::Config>::AccountId, ProviderId = <T as frame_system::Config>::Hash ,ValuePropId = <T as pallet_storage_providers::Config>::ValuePropId, BucketId = <T as frame_system::Config>::Hash>,
    // The ProofDealer element of this pallet's config has to implement the `ProofsDealerInterface` trait with the following types:
	<T as crate::Config>::ProofDealer: shp_traits::ProofsDealerInterface<TickNumber = BlockNumberFor<T>, MerkleHash = <T as frame_system::Config>::Hash, KeyProof = shp_file_key_verifier::types::FileKeyProof<{shp_constants::H_LENGTH}, {shp_constants::FILE_CHUNK_SIZE}, {shp_constants::FILE_SIZE_TO_CHALLENGES}>>,
	// The Nfts element of this pallet's config has to implement the `Inspect` trait with the following types:
	<T as crate::Config>::Nfts: frame_support::traits::nonfungibles_v2::Inspect<<T as frame_system::Config>::AccountId, CollectionId = <T as pallet_nfts::Config>::CollectionId>,
	// The ProvidersPallet element of the Payment Streams pallet's config has to implement the `ReadProvidersInterface` trait with the following types:
	<T as pallet_payment_streams::Config>::ProvidersPallet: shp_traits::ReadProvidersInterface<ProviderId = <T as frame_system::Config>::Hash>,
	// The PaymentStreams element of this pallet's config has to implement the `PaymentStreamsInterface` trait with Balance matching the payment streams pallet's NativeBalance:
	<T as crate::Config>::PaymentStreams: shp_traits::PaymentStreamsInterface<Balance = <T as pallet_payment_streams::Config>::NativeBalance>,
	// The Storage Providers pallet's `HoldReason` type must be able to be converted into the Currency's `Reason`.
	pallet_payment_streams::HoldReason: Into<<<T as pallet::Config>::Currency as frame_support::traits::fungible::InspectHold<<T as frame_system::Config>::AccountId>>::Reason>,
    // Payment streams balance type conversion
	<<T as pallet_payment_streams::Config>::NativeBalance as frame_support::traits::fungible::Inspect<<T as frame_system::Config>::AccountId>>::Balance: From<u64>,
	// Payment streams Units type conversion
	<T as pallet_payment_streams::Config>::Units: From<u64>,
	// Ensure balance types match between payment streams and storage providers
	<<T as pallet_payment_streams::Config>::NativeBalance as frame_support::traits::fungible::Inspect<<T as frame_system::Config>::AccountId>>::Balance: From<<<T as pallet_storage_providers::Config>::NativeBalance as frame_support::traits::fungible::Inspect<<T as frame_system::Config>::AccountId>>::Balance>,
)]
mod benchmarks {
    use super::*;
    use codec::Encode;
    use frame_support::{
        assert_ok, ensure,
        traits::{
            fungible::{Mutate, MutateHold},
            Get, OnFinalize, OnPoll,
        },
        weights::WeightMeter,
        BoundedVec,
    };
    use frame_system::{pallet_prelude::BlockNumberFor, RawOrigin};
    use pallet_file_system_runtime_api::QueryFileEarliestVolunteerTickError;
    use pallet_payment_streams::types::DynamicRatePaymentStream;
    use pallet_storage_providers::types::ValueProposition;
    use shp_traits::{
        MessageAdapter, MutateBucketsInterface, PaymentStreamsInterface, ProofsDealerInterface,
        ReadBucketsInterface, ReadProvidersInterface, ReadStorageProvidersInterface,
    };
    use sp_core::{Decode, Hasher};
    use sp_runtime::traits::Verify;
    use sp_runtime::{
        traits::{Bounded, Hash, One, Zero},
        Saturating,
    };
    use sp_std::{vec, vec::Vec};

    use crate::benchmark_proofs::*;

    // Type alias for payment streams balance type (NativeBalance)
    type PaymentStreamsBalanceOf<T> = <<T as pallet_payment_streams::Config>::NativeBalance as frame_support::traits::fungible::Inspect<
        <T as frame_system::Config>::AccountId,
    >>::Balance;

    #[benchmark]
    fn create_bucket() -> Result<(), BenchmarkError> {
        let user: T::AccountId = account("Alice", 0, 0);
        let signed_origin = RawOrigin::Signed(user.clone());
        mint_into_account::<T>(user.clone(), 1_000_000_000_000_000)?;

        let name: BucketNameFor<T> = vec![1; BucketNameLimitFor::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();

        // Register MSP with value proposition
        let msp: T::AccountId = account("MSP", 0, 0);
        mint_into_account::<T>(msp.clone(), 1_000_000_000_000_000)?;
        let (msp_id, value_prop_id) = add_msp_to_provider_storage::<T>(&msp, None);

        #[extrinsic_call]
        _(signed_origin.clone(), msp_id, name, true, value_prop_id);

        Ok(())
    }

    #[benchmark]
    fn request_move_bucket() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Get a user account and mint some tokens into it
        let user: T::AccountId = account("Alice", 0, 0);
        let signed_origin = RawOrigin::Signed(user.clone());
        mint_into_account::<T>(user.clone(), 1_000_000_000_000_000)?;

        // Set up parameters for the bucket to use
        let name: BucketNameFor<T> = vec![1; BucketNameLimitFor::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();
        let bucket_id = <<T as crate::Config>::Providers as ReadBucketsInterface>::derive_bucket_id(
            &user,
            name.clone(),
        );

        // Register a MSP with a value proposition
        let initial_msp_account: T::AccountId = account("MSP", 0, 0);
        mint_into_account::<T>(initial_msp_account.clone(), 1_000_000_000_000_000)?;
        let (initial_msp_id, initial_value_prop_id) =
            add_msp_to_provider_storage::<T>(&initial_msp_account, None);

        // Register another MSP with a value proposition
        let new_msp_account: T::AccountId = account("MSP", 0, 1);
        mint_into_account::<T>(new_msp_account.clone(), 1_000_000_000_000_000)?;
        let (new_msp_id, new_value_prop_id) =
            add_msp_to_provider_storage::<T>(&new_msp_account, None);

        // Create the bucket, assigning it to the initial MSP
        Pallet::<T>::create_bucket(
            signed_origin.clone().into(),
            initial_msp_id,
            name,
            true,
            initial_value_prop_id,
        )?;

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(signed_origin, bucket_id, new_msp_id, new_value_prop_id);

        /*********** Post-benchmark checks: ***********/
        // Ensure the PendingMoveBucketRequests storage has the created request
        let pending_move_bucket_request = PendingMoveBucketRequests::<T>::get(&bucket_id);
        assert!(pending_move_bucket_request.is_some());
        let pending_move_bucket_request = pending_move_bucket_request.unwrap();
        assert_eq!(pending_move_bucket_request.requester, user.clone());
        assert_eq!(pending_move_bucket_request.new_msp_id, new_msp_id);
        assert_eq!(
            pending_move_bucket_request.new_value_prop_id,
            new_value_prop_id
        );

        // Ensure the expected event was emitted.
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::MoveBucketRequested {
                who: user,
                bucket_id,
                new_msp_id,
                new_value_prop_id,
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        Ok(())
    }

    #[benchmark]
    fn msp_respond_move_bucket_request() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Get a user account and mint some tokens into it
        let user: T::AccountId = account("Alice", 0, 0);
        let signed_origin = RawOrigin::Signed(user.clone());
        mint_into_account::<T>(user.clone(), 1_000_000_000_000_000)?;

        // Set up parameters for the bucket to use
        let name: BucketNameFor<T> = vec![1; BucketNameLimitFor::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();
        let bucket_id = <<T as crate::Config>::Providers as ReadBucketsInterface>::derive_bucket_id(
            &user,
            name.clone(),
        );

        // Register a MSP with a value proposition
        let initial_msp_account: T::AccountId = account("MSP", 0, 0);
        mint_into_account::<T>(initial_msp_account.clone(), 1_000_000_000_000_000)?;
        let (initial_msp_id, initial_value_prop_id) =
            add_msp_to_provider_storage::<T>(&initial_msp_account, None);

        // Register another MSP with a value proposition
        let new_msp_account: T::AccountId = account("MSP", 0, 1);
        mint_into_account::<T>(new_msp_account.clone(), 1_000_000_000_000_000)?;
        let (new_msp_id, new_value_prop_id) =
            add_msp_to_provider_storage::<T>(&new_msp_account, None);

        // Create the bucket, assigning it to the initial MSP
        Pallet::<T>::create_bucket(
            signed_origin.clone().into(),
            initial_msp_id,
            name,
            true,
            initial_value_prop_id,
        )?;

        // Request the move of the bucket to the new MSP
        Pallet::<T>::request_move_bucket(
            signed_origin.clone().into(),
            bucket_id,
            new_msp_id,
            new_value_prop_id,
        )?;

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(
            RawOrigin::Signed(new_msp_account.clone()),
            bucket_id,
            BucketMoveRequestResponse::Accepted,
        );

        /*********** Post-benchmark checks: ***********/
        // Ensure the bucket is now stored by the new MSP
        assert!(
            <<T as crate::Config>::Providers as ReadBucketsInterface>::is_bucket_stored_by_msp(
                &new_msp_id,
                &bucket_id
            )
        );

        // Ensure the expected event was emitted.
        let expected_event = <T as pallet::Config>::RuntimeEvent::from(Event::MoveBucketAccepted {
            bucket_id,
            old_msp_id: Some(initial_msp_id),
            new_msp_id,
            value_prop_id: new_value_prop_id,
        });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        Ok(())
    }

    #[benchmark]
    fn update_bucket_privacy() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Get a user account and mint some tokens into it
        let user: T::AccountId = account("Alice", 0, 0);
        let signed_origin = RawOrigin::Signed(user.clone());
        mint_into_account::<T>(user.clone(), 1_000_000_000_000_000)?;

        // Set up parameters for the bucket to use
        let name: BucketNameFor<T> = vec![1; BucketNameLimitFor::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();
        let bucket_id = <<T as crate::Config>::Providers as ReadBucketsInterface>::derive_bucket_id(
            &user,
            name.clone(),
        );

        // Register a MSP with a value proposition
        let msp_account: T::AccountId = account("MSP", 0, 0);
        mint_into_account::<T>(msp_account.clone(), 1_000_000_000_000_000)?;
        let (msp_id, value_prop_id) = add_msp_to_provider_storage::<T>(&msp_account, None);

        // Create the bucket as private, creating the collection
        Pallet::<T>::create_bucket(
            signed_origin.clone().into(),
            msp_id,
            name,
            true,
            value_prop_id,
        )?;

        // The worst-case scenario is when the bucket has an associated collection but it doesn't exist in storage,
        // since it has to perform an extra check compared to the bucket being public from the start
        // So, we delete the collection from storage
        let collection_id = T::Providers::get_read_access_group_id_of_bucket(&bucket_id)?.unwrap();
        pallet_nfts::Collection::<T>::remove(collection_id);

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(signed_origin, bucket_id, true);

        /*********** Post-benchmark checks: ***********/
        // Ensure the bucket is still private
        assert!(T::Providers::is_bucket_private(&bucket_id).unwrap());

        // Ensure it has a collection now, after we deleted the previous one
        let new_collection_id =
            T::Providers::get_read_access_group_id_of_bucket(&bucket_id)?.unwrap();
        assert!(pallet_nfts::Collection::<T>::contains_key(
            new_collection_id
        ));
        assert_ne!(collection_id, new_collection_id);

        // Ensure the expected event was emitted.
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::BucketPrivacyUpdated {
                who: user,
                bucket_id,
                private: true,
                collection_id: Some(new_collection_id),
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        Ok(())
    }

    #[benchmark]
    fn create_and_associate_collection_with_bucket() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Get a user account and mint some tokens into it
        let user: T::AccountId = account("Alice", 0, 0);
        let signed_origin = RawOrigin::Signed(user.clone());
        mint_into_account::<T>(user.clone(), 1_000_000_000_000_000)?;

        // Set up parameters for the bucket to use
        let name: BucketNameFor<T> = vec![1; BucketNameLimitFor::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();
        let bucket_id = <<T as crate::Config>::Providers as ReadBucketsInterface>::derive_bucket_id(
            &user,
            name.clone(),
        );

        // Register a MSP with a value proposition
        let msp_account: T::AccountId = account("MSP", 0, 0);
        mint_into_account::<T>(msp_account.clone(), 1_000_000_000_000_000)?;
        let (msp_id, value_prop_id) = add_msp_to_provider_storage::<T>(&msp_account, None);

        // Create the bucket as private, creating the collection
        Pallet::<T>::create_bucket(
            signed_origin.clone().into(),
            msp_id,
            name,
            true,
            value_prop_id,
        )?;

        // The worst-case scenario is when the bucket has an associated collection but it doesn't exist in storage,
        // since it has to perform an extra check compared to the bucket being public from the start
        // So, we delete the collection from storage
        let collection_id = T::Providers::get_read_access_group_id_of_bucket(&bucket_id)?.unwrap();
        pallet_nfts::Collection::<T>::remove(collection_id);

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(signed_origin, bucket_id);

        /*********** Post-benchmark checks: ***********/
        // Ensure the bucket is still private
        assert!(T::Providers::is_bucket_private(&bucket_id).unwrap());

        // Ensure it has a collection now, after we deleted the previous one
        let new_collection_id =
            T::Providers::get_read_access_group_id_of_bucket(&bucket_id)?.unwrap();
        assert!(pallet_nfts::Collection::<T>::contains_key(
            new_collection_id
        ));
        assert_ne!(collection_id, new_collection_id);

        // Ensure the expected event was emitted.
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::NewCollectionAndAssociation {
                who: user,
                bucket_id,
                collection_id: new_collection_id,
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        Ok(())
    }

    #[benchmark]
    fn delete_bucket() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Get a user account and mint some tokens into it
        let user: T::AccountId = account("Alice", 0, 0);
        let signed_origin = RawOrigin::Signed(user.clone());
        mint_into_account::<T>(user.clone(), 1_000_000_000_000_000)?;

        // Set up parameters for the bucket to use
        let name: BucketNameFor<T> = vec![1; BucketNameLimitFor::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();
        let bucket_id = <<T as crate::Config>::Providers as ReadBucketsInterface>::derive_bucket_id(
            &user,
            name.clone(),
        );

        // Register a MSP with a value proposition
        let msp_account: T::AccountId = account("MSP", 0, 0);
        mint_into_account::<T>(msp_account.clone(), 1_000_000_000_000_000)?;
        let (msp_id, value_prop_id) = add_msp_to_provider_storage::<T>(&msp_account, None);

        // Create the bucket as private, creating the collection so it has to be deleted as well.
        Pallet::<T>::create_bucket(
            signed_origin.clone().into(),
            msp_id,
            name,
            true,
            value_prop_id,
        )?;

        // Get the collection ID of the bucket
        let collection_id = T::Providers::get_read_access_group_id_of_bucket(&bucket_id)?.unwrap();

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(signed_origin, bucket_id);

        /*********** Post-benchmark checks: ***********/
        // The bucket should have been deleted.
        assert!(!T::Providers::bucket_exists(&bucket_id));

        // And the collection should have been deleted as well
        assert!(!pallet_nfts::Collection::<T>::contains_key(collection_id));

        // Ensure the expected event was emitted.
        let expected_event = <T as pallet::Config>::RuntimeEvent::from(Event::BucketDeleted {
            who: user,
            bucket_id,
            maybe_collection_id: Some(collection_id),
        });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        Ok(())
    }

    #[benchmark]
    fn issue_storage_request() -> Result<(), BenchmarkError> {
        let user: T::AccountId = account("Alice", 0, 0);
        let signed_origin = RawOrigin::Signed(user.clone());
        mint_into_account::<T>(user.clone(), 1_000_000_000_000_000)?;

        let name: BucketNameFor<T> = vec![1; BucketNameLimitFor::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();
        let bucket_id = <<T as crate::Config>::Providers as ReadBucketsInterface>::derive_bucket_id(
            &user,
            name.clone(),
        );
        let location = vec![1; MaxFilePathSize::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();
        let fingerprint =
            <<T as frame_system::Config>::Hashing as Hasher>::hash(b"benchmark_fingerprint");
        let size: StorageDataUnit<T> = 100;
        let peer_id: PeerId<T> = vec![1; MaxPeerIdSize::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();
        let peer_ids: PeerIds<T> =
            vec![peer_id; MaxNumberOfPeerIds::<T>::get().try_into().unwrap()]
                .try_into()
                .unwrap();

        // Register MSP with value proposition
        let msp: T::AccountId = account("MSP", 0, 0);
        mint_into_account::<T>(msp.clone(), 1_000_000_000_000_000)?;
        let (msp_id, value_prop_id) = add_msp_to_provider_storage::<T>(&msp, None);

        Pallet::<T>::create_bucket(
            signed_origin.clone().into(),
            msp_id,
            name,
            true,
            value_prop_id,
        )?;

        #[extrinsic_call]
        _(
            signed_origin,
            bucket_id,
            location,
            fingerprint,
            size,
            msp_id,
            peer_ids,
            ReplicationTarget::Standard,
        );

        Ok(())
    }

    #[benchmark]
    fn revoke_storage_request(
        n: Linear<
            1,
            {
                Into::<u64>::into(T::MaxReplicationTarget::get())
                    .try_into()
                    .unwrap()
            },
        >,
    ) -> Result<(), BenchmarkError> {
        let replication_target: u32 = n.into();
        let user: T::AccountId = account("Alice", 0, 0);
        let signed_origin = RawOrigin::Signed(user.clone());
        mint_into_account::<T>(user.clone(), 1_000_000_000_000_000)?;

        let name: BucketNameFor<T> = vec![1; BucketNameLimitFor::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();
        let bucket_id = <<T as crate::Config>::Providers as ReadBucketsInterface>::derive_bucket_id(
            &user,
            name.clone(),
        );
        let location: FileLocation<T> = vec![1; MaxFilePathSize::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();
        let fingerprint =
            <<T as frame_system::Config>::Hashing as Hasher>::hash(b"benchmark_fingerprint");
        let size: StorageDataUnit<T> = 100;
        let peer_id: PeerId<T> = vec![1; MaxPeerIdSize::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();
        let peer_ids: PeerIds<T> =
            vec![peer_id; MaxNumberOfPeerIds::<T>::get().try_into().unwrap()]
                .try_into()
                .unwrap();

        // Register MSP with value proposition
        let msp: T::AccountId = account("MSP", 0, 0);
        mint_into_account::<T>(msp.clone(), 1_000_000_000_000_000)?;
        let (msp_id, value_prop_id) = add_msp_to_provider_storage::<T>(&msp, None);

        Pallet::<T>::create_bucket(
            signed_origin.clone().into(),
            msp_id,
            name,
            true,
            value_prop_id,
        )?;

        Pallet::<T>::issue_storage_request(
            signed_origin.clone().into(),
            bucket_id,
            location.clone(),
            fingerprint,
            size,
            msp_id,
            peer_ids,
            ReplicationTarget::Custom(replication_target.into()),
        )?;

        let file_key =
            Pallet::<T>::compute_file_key(user, bucket_id, location, size, fingerprint).unwrap();

        // The `revoke_storage_request` executes the `drain_prefix` function to remove all sub keys including the primary key
        // from `StorageRequestBsps`.
        for i in 0..replication_target {
            let bsp_user: T::AccountId = account("bsp", i as u32, i as u32);
            mint_into_account::<T>(bsp_user.clone(), 1_000_000_000_000_000)?;
            let bsp_id = add_bsp_to_provider_storage::<T>(&bsp_user.clone(), None);

            StorageRequestBsps::<T>::insert(
                file_key,
                bsp_id,
                StorageRequestBspsMetadata::<T> {
                    confirmed: true,
                    _phantom: Default::default(),
                },
            );
        }

        // Mutate the storage request to have bsps_volunteered equal to MaxReplicationTarget
        StorageRequests::<T>::mutate(file_key, |storage_request| {
            storage_request.as_mut().unwrap().bsps_volunteered = n.into();
            // Setting this greater than 0 triggers a priority challenge
            storage_request.as_mut().unwrap().bsps_confirmed = n.into();
        });

        #[extrinsic_call]
        _(signed_origin, file_key);

        Ok(())
    }

    #[benchmark]
    fn msp_stop_storing_bucket() -> Result<(), BenchmarkError> {
        let user: T::AccountId = account("Alice", 0, 0);
        let signed_origin = RawOrigin::Signed(user.clone());
        mint_into_account::<T>(user.clone(), 1_000_000_000_000_000)?;

        let name: BucketNameFor<T> = vec![1; BucketNameLimitFor::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();
        let bucket_id = <<T as crate::Config>::Providers as ReadBucketsInterface>::derive_bucket_id(
            &user,
            name.clone(),
        );

        // Register MSP with value proposition
        let msp: T::AccountId = account("MSP", 0, 0);
        mint_into_account::<T>(msp.clone(), 1_000_000_000_000_000)?;
        let (msp_id, value_prop_id) = add_msp_to_provider_storage::<T>(&msp, None);

        Pallet::<T>::create_bucket(
            signed_origin.clone().into(),
            msp_id,
            name,
            true,
            value_prop_id,
        )?;

        #[extrinsic_call]
        _(RawOrigin::Signed(msp), bucket_id);

        Ok(())
    }

    #[benchmark]
    fn msp_respond_storage_requests_multiple_buckets(
        n: Linear<1, 10>,
        m: Linear<1, 10>,
        l: Linear<1, 10>,
    ) -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Get from the linear variables the amount of buckets to accept, the amount of file keys to accept per bucket and the amount to reject.
        let amount_of_buckets_to_accept: u32 = n.into();
        let amount_of_file_keys_to_accept_per_bucket: u32 = m.into();
        let amount_of_file_keys_to_reject_per_bucket: u32 = l.into();

        // Get the user account for the generated proofs and load it up with some balance.
        let user_as_bytes: [u8; 32] = get_user_account().clone().try_into().unwrap();
        let user_account: T::AccountId = T::AccountId::decode(&mut &user_as_bytes[..]).unwrap();
        mint_into_account::<T>(user_account.clone(), 1_000_000_000_000_000_000_000)?;

        // Register an account as a MSP with the specific MSP ID from the generated proofs
        let msp_account: T::AccountId = whitelisted_caller();
        mint_into_account::<T>(msp_account.clone(), 1_000_000_000_000_000_000_000)?;
        let encoded_msp_id = get_msp_id();
        let msp_id = <T as frame_system::Config>::Hash::decode(&mut encoded_msp_id.as_ref())
            .expect("Failed to decode provider ID from bytes.");
        let (_, value_prop_id) = add_msp_to_provider_storage::<T>(&msp_account, Some(msp_id));

        let mut msp_total_response: StorageRequestMspResponse<T> = Vec::new();
        // For each bucket to accept:
        for i in 1..amount_of_buckets_to_accept + 1 {
            // Create the bucket to store in the MSP
            let encoded_bucket_id = get_bucket_id(i);
            let bucket_id =
                <T as frame_system::Config>::Hash::decode(&mut encoded_bucket_id.as_ref())
                    .expect("Bucket ID should be decodable as it is a hash");
            <<T as crate::Config>::Providers as MutateBucketsInterface>::add_bucket(
                msp_id,
                user_account.clone(),
                bucket_id,
                false,
                None,
                value_prop_id,
            )?;

            // Update the bucket's size and root to match the generated proofs
            let bucket_size = 2 * 1024 * 1024;
            let encoded_bucket_root = get_bucket_root(i);
            let bucket_root =
                <T as frame_system::Config>::Hash::decode(&mut encoded_bucket_root.as_ref())
                    .expect("Bucket root should be decodable as it is a hash");
            pallet_storage_providers::Buckets::<T>::mutate(&bucket_id, |bucket| {
                let bucket = bucket.as_mut().expect("Bucket should exist.");
                bucket.size = bucket_size;
                bucket.root = bucket_root;
            });

            // Build the reject response for this bucket:

            // Create all the storage requests for the files to reject
            let mut file_keys_to_reject: Vec<MerkleHash<T>> = Vec::new();
            for j in 0..amount_of_file_keys_to_reject_per_bucket {
                let location: FileLocation<T> =
                    vec![j as u8; MaxFilePathSize::<T>::get().try_into().unwrap()]
                        .try_into()
                        .unwrap();
                let fingerprint = <<T as frame_system::Config>::Hashing as Hasher>::hash(
                    b"benchmark_fingerprint",
                );
                let size: StorageDataUnit<T> = 100;
                let storage_request_metadata = StorageRequestMetadata::<T> {
                    requested_at:
                        <<T as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick(),
					expires_at: BlockNumberFor::<T>::max_value(),
                    owner: user_account.clone(),
                    bucket_id,
                    location: location.clone(),
                    fingerprint,
                    size,
                    msp: Some((msp_id, false)),
                    user_peer_ids: Default::default(),
                    bsps_required: T::StandardReplicationTarget::get(),
                    bsps_confirmed: ReplicationTargetType::<T>::one(), // One BSP confirmed means the logic to enqueue a priority challenge is executed
                    bsps_volunteered: ReplicationTargetType::<T>::zero(),
					deposit_paid: Default::default(),
                };
                let file_key = Pallet::<T>::compute_file_key(
                    user_account.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                )
                .unwrap();

                <StorageRequests<T>>::insert(&file_key, storage_request_metadata);

                <BucketsWithStorageRequests<T>>::insert(&bucket_id, &file_key, ());

                file_keys_to_reject.push(file_key);
            }
            let reject = file_keys_to_reject
                .iter()
                .map(|file_key| {
                    let reject_reason = RejectedStorageRequestReason::ReachedMaximumCapacity;
                    RejectedStorageRequest {
                        file_key: *file_key,
                        reason: reject_reason,
                    }
                })
                .collect::<Vec<RejectedStorageRequest<T>>>();

            // Build the accept response for this bucket:

            // Get the file keys to accept from the generated proofs.
            let mut file_keys_and_proofs: Vec<FileKeyWithProof<T>> = Vec::new();
            let encoded_file_keys_to_accept =
                fetch_file_keys_to_accept(amount_of_file_keys_to_accept_per_bucket, i as u32);
            let file_keys_to_accept = encoded_file_keys_to_accept
                .iter()
                .map(|encoded_file_key| {
                    let file_key =
                        <T as frame_system::Config>::Hash::decode(&mut encoded_file_key.as_ref())
                            .expect("File key should be decodable as it is a hash");
                    file_key
                })
                .collect::<Vec<<T as frame_system::Config>::Hash>>();

            // For each file key to accept...
            for j in 0..file_keys_to_accept.len() {
                // Get its file key proof from the generated proofs.
                let encoded_file_key_proof = fetch_file_key_proof(
                    amount_of_file_keys_to_accept_per_bucket,
                    i as u32,
                    j as u32,
                );
                let file_key_proof = <KeyProof<T>>::decode(&mut encoded_file_key_proof.as_ref())
                    .expect("File key proof should be decodable");

                // Create the storage request for it:
                let location = file_key_proof.file_metadata.location().clone();
                let fingerprint_hash = file_key_proof.file_metadata.fingerprint().clone().as_hash();
                let fingerprint =
                    <T as frame_system::Config>::Hash::decode(&mut fingerprint_hash.as_ref())
                        .expect("Fingerprint should be decodable as it is a hash");
                let size = file_key_proof.file_metadata.file_size();
                let storage_request_metadata = StorageRequestMetadata::<T> {
                    requested_at:
                        <<T as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick(),
					expires_at: BlockNumberFor::<T>::max_value(),
                    owner: user_account.clone(),
                    bucket_id,
                    location: location.clone().try_into().unwrap(),
                    fingerprint: fingerprint.into(),
                    size,
                    msp: Some((msp_id, false)),
                    user_peer_ids: Default::default(),
                    bsps_required: T::StandardReplicationTarget::get(),
                    bsps_confirmed: T::StandardReplicationTarget::get(), // All BSPs confirmed means the logic to delete the storage request is executed
                    bsps_volunteered: T::MaxReplicationTarget::get().into(), // Maximize the BSPs volunteered since the logic has to drain them from storage
					deposit_paid: Default::default(),
                };
                <StorageRequests<T>>::insert(&file_keys_to_accept[j], storage_request_metadata);
                <BucketsWithStorageRequests<T>>::insert(&bucket_id, &file_keys_to_accept[j], ());
                // Add the volunteered BSPs to the StorageRequestBsps storage for this file key
                for i in 0u64..T::MaxReplicationTarget::get().into() {
                    let bsp_user: T::AccountId = account("bsp_volunteered", i as u32, i as u32);
                    mint_into_account::<T>(bsp_user.clone(), 1_000_000_000_000_000)?;
                    let bsp_id = add_bsp_to_provider_storage::<T>(&bsp_user.clone(), None);
                    StorageRequestBsps::<T>::insert(
                        file_keys_to_accept[j],
                        bsp_id,
                        StorageRequestBspsMetadata::<T> {
                            confirmed: false,
                            _phantom: Default::default(),
                        },
                    );
                }

                // Create the FileKeyWithProof object
                let file_key_with_proof = FileKeyWithProof {
                    file_key: file_keys_to_accept[j],
                    proof: file_key_proof,
                };

                // Push it to the file keys and proofs bounded vector
                file_keys_and_proofs.push(file_key_with_proof);
            }

            // Get the non-inclusion forest proof for this amount of file keys
            let encoded_non_inclusion_forest_proof =
                fetch_non_inclusion_proofs(amount_of_file_keys_to_accept_per_bucket, i);
            let non_inclusion_forest_proof =
                <<<T as Config>::ProofDealer as ProofsDealerInterface>::ForestProof>::decode(
                    &mut encoded_non_inclusion_forest_proof.as_ref(),
                )
                .expect("Non-inclusion forest proof should be decodable");

            let accept = StorageRequestMspAcceptedFileKeys {
                file_keys_and_proofs,
                forest_proof: non_inclusion_forest_proof,
            };

            // Finally, build the response for this bucket and push it to the responses bounded vector
            let response = StorageRequestMspBucketResponse {
                bucket_id,
                accept: Some(accept),
                reject,
            };

            msp_total_response.push(response);
        }

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(RawOrigin::Signed(msp_account.clone()), msp_total_response);

        /*********** Post-benchmark checks: ***********/
        Ok(())
    }

    #[benchmark]
    fn bsp_volunteer() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Get a user account and mint some tokens into it
        let user: T::AccountId = account("Alice", 0, 0);
        let signed_origin = RawOrigin::Signed(user.clone());
        mint_into_account::<T>(user.clone(), 1_000_000_000_000_000)?;

        // Register a MSP with a value proposition
        let msp_account: T::AccountId = account("MSP", 0, 0);
        mint_into_account::<T>(msp_account.clone(), 1_000_000_000_000_000)?;
        let (msp_id, value_prop_id) = add_msp_to_provider_storage::<T>(&msp_account, None);

        // Register the BSP which will volunteer for the file
        let bsp_account: T::AccountId = account("BSP", 0, 0);
        let bsp_signed_origin = RawOrigin::Signed(bsp_account.clone());
        mint_into_account::<T>(bsp_account.clone(), 1_000_000_000_000_000)?;
        let bsp_id = add_bsp_to_provider_storage::<T>(&bsp_account, None);
        let bsp_multiaddresses = <<T as crate::Config>::Providers as ReadStorageProvidersInterface>::get_bsp_multiaddresses(&bsp_id)?;

        // Create the bucket, assigning it to the MSP
        let name: BucketNameFor<T> = vec![1; BucketNameLimitFor::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();
        let bucket_id = <<T as crate::Config>::Providers as ReadBucketsInterface>::derive_bucket_id(
            &user,
            name.clone(),
        );
        Pallet::<T>::create_bucket(
            signed_origin.clone().into(),
            msp_id,
            name,
            true,
            value_prop_id,
        )?;

        // Issue the storage request from the user
        let location: FileLocation<T> = vec![1; MaxFilePathSize::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();
        let fingerprint =
            <<T as frame_system::Config>::Hashing as Hasher>::hash(b"benchmark_fingerprint");
        let size: StorageDataUnit<T> = 100;
        let peer_id: PeerId<T> = vec![1; MaxPeerIdSize::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();
        let peer_ids: PeerIds<T> =
            vec![peer_id; MaxNumberOfPeerIds::<T>::get().try_into().unwrap()]
                .try_into()
                .unwrap();
        Pallet::<T>::issue_storage_request(
            signed_origin.clone().into(),
            bucket_id,
            location.clone(),
            fingerprint,
            size,
            msp_id,
            peer_ids,
            ReplicationTarget::Standard,
        )?;

        // Compute the file key
        let file_key = Pallet::<T>::compute_file_key(
            user.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        )
        .unwrap();

        // Query the earliest that this BSP can volunteer for this file
        let query_result = Pallet::<T>::query_earliest_file_volunteer_tick(bsp_id, file_key);

        // Check if an error was returned and, if so, which one
        let tick_to_advance_to = match query_result {
            Err(error) => match error {
                QueryFileEarliestVolunteerTickError::FailedToEncodeFingerprint => {
                    return Err(BenchmarkError::Stop("Failed to encode fingerprint."));
                }
                QueryFileEarliestVolunteerTickError::FailedToEncodeBsp => {
                    return Err(BenchmarkError::Stop("Failed to encode BSP."));
                }
                QueryFileEarliestVolunteerTickError::ThresholdArithmeticError => {
                    return Err(BenchmarkError::Stop("Threshold arithmetic error."));
                }
                QueryFileEarliestVolunteerTickError::StorageRequestNotFound => {
                    return Err(BenchmarkError::Stop("Storage request not found."));
                }
                QueryFileEarliestVolunteerTickError::FailedToComputeEligibilityCriteria => {
                    return Err(BenchmarkError::Stop(
                        "Failed to compute eligibility criteria for BSP.",
                    ));
                }
                QueryFileEarliestVolunteerTickError::InternalError => {
                    return Err(BenchmarkError::Stop("Internal runtime API error."));
                }
            },
            Ok(earliest_volunteer_tick) => earliest_volunteer_tick,
        };

        // Advance the block number to the earliest tick where the BSP can volunteer, only if
        // it's bigger than the current block number
        if tick_to_advance_to > frame_system::Pallet::<T>::block_number() {
            run_to_block::<T>(tick_to_advance_to);
        }

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(bsp_signed_origin, file_key);

        /*********** Post-benchmark checks: ***********/
        // Ensure the BSP has correctly volunteered for the file
        assert!(StorageRequestBsps::<T>::contains_key(&file_key, &bsp_id));

        // Ensure the expected event was emitted.
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::AcceptedBspVolunteer {
                bsp_id,
                multiaddresses: bsp_multiaddresses,
                bucket_id,
                location,
                fingerprint,
                owner: user,
                size,
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        Ok(())
    }

    #[benchmark]
    fn bsp_confirm_storing(n: Linear<1, 10>) -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Get from the linear variable the amount of files to confirm storing
        let amount_of_files_to_confirm_storing: u32 = n.into();

        // Get the user account for the generated proofs and load it up with some balance.
        let user_as_bytes: [u8; 32] = get_user_account().clone().try_into().unwrap();
        let user_account: T::AccountId = T::AccountId::decode(&mut &user_as_bytes[..]).unwrap();
        mint_into_account::<T>(user_account.clone(), 1_000_000_000_000_000_000_000)?;

        // Register an account as a MSP with the specific MSP ID from the generated proofs
        let msp_account: T::AccountId = account("MSP", 0, 0);
        mint_into_account::<T>(msp_account.clone(), 1_000_000_000_000_000_000_000)?;
        let encoded_msp_id = get_msp_id();
        let msp_id = <T as frame_system::Config>::Hash::decode(&mut encoded_msp_id.as_ref())
            .expect("Failed to decode provider ID from bytes.");
        let (_, value_prop_id) = add_msp_to_provider_storage::<T>(&msp_account, Some(msp_id));

        // Register the BSP which will volunteer for the file and confirm storing it
        let bsp_account: T::AccountId = account("BSP", 0, 0);
        mint_into_account::<T>(bsp_account.clone(), 1_000_000_000_000_000)?;
        let encoded_bsp_id = get_bsp_id();
        let bsp_id = <T as frame_system::Config>::Hash::decode(&mut encoded_bsp_id.as_ref())
            .expect("Failed to decode provider ID from bytes.");
        let bsp_signed_origin = RawOrigin::Signed(bsp_account.clone());
        add_bsp_to_provider_storage::<T>(&bsp_account, Some(bsp_id));

        // Change the root of the BSP to match the non-inclusion forest proof.
        let encoded_bsp_root = get_bucket_root(1);
        let bsp_root = <T as frame_system::Config>::Hash::decode(&mut encoded_bsp_root.as_ref())
            .expect("BSP root should be decodable as it is a hash");
        pallet_storage_providers::BackupStorageProviders::<T>::mutate(&bsp_id, |bsp| {
            let bsp = bsp.as_mut().expect("BSP should exist.");
            bsp.root = bsp_root
        });

        // Create the bucket to store in the MSP
        let encoded_bucket_id = get_bucket_id(1);
        let bucket_id = <T as frame_system::Config>::Hash::decode(&mut encoded_bucket_id.as_ref())
            .expect("Bucket ID should be decodable as it is a hash");
        <<T as crate::Config>::Providers as MutateBucketsInterface>::add_bucket(
            msp_id,
            user_account.clone(),
            bucket_id,
            false,
            None,
            value_prop_id,
        )?;

        // Update the bucket's size and root to match the generated proofs
        let bucket_size = 2 * 1024 * 1024;
        let encoded_bucket_root = get_bucket_root(1);
        let bucket_root =
            <T as frame_system::Config>::Hash::decode(&mut encoded_bucket_root.as_ref())
                .expect("Bucket root should be decodable as it is a hash");
        pallet_storage_providers::Buckets::<T>::mutate(&bucket_id, |bucket| {
            let bucket = bucket.as_mut().expect("Bucket should exist.");
            bucket.size = bucket_size;
            bucket.root = bucket_root;
        });

        // Create the dynamic-rate payment stream between the user and the BSP to account for the worst-case scenario
        // of updating it in the confirm
        pallet_payment_streams::DynamicRatePaymentStreams::<T>::insert(
            &bsp_id,
            &user_account,
            DynamicRatePaymentStream {
                amount_provided: 100u32.into(),
                price_index_when_last_charged: 0u32.into(),
                user_deposit: 100u32.into(),
                out_of_funds_tick: None,
            },
        );

        // Set the used capacity to the total capacity to simulate the worst-case scenario of treasury cut calculation when charging the payment stream.
        let total_capacity = pallet_storage_providers::TotalBspsCapacity::<T>::get();
        pallet_storage_providers::UsedBspsCapacity::<T>::put(total_capacity);

        // Update the last chargeable info of the BSP to make it actually charge the user
        pallet_payment_streams::LastChargeableInfo::<T>::insert(
            &bsp_id,
            pallet_payment_streams::types::ProviderLastChargeableInfo {
                last_chargeable_tick: frame_system::Pallet::<T>::block_number(),
                price_index: 100u32.into(),
            },
        );

        // Get the file keys to confirm from the generated proofs.
        let mut file_keys_and_proofs: BoundedVec<
            FileKeyWithProof<T>,
            <T as Config>::MaxBatchConfirmStorageRequests,
        > = BoundedVec::new();
        let encoded_file_keys_to_confirm =
            fetch_file_keys_for_bsp_confirm(amount_of_files_to_confirm_storing);
        let file_keys_to_confirm = encoded_file_keys_to_confirm
            .iter()
            .map(|encoded_file_key| {
                let file_key =
                    <T as frame_system::Config>::Hash::decode(&mut encoded_file_key.as_ref())
                        .expect("File key should be decodable as it is a hash");
                file_key
            })
            .collect::<Vec<<T as frame_system::Config>::Hash>>();

        // For each file key to confirm...
        for i in 0..file_keys_to_confirm.len() {
            // Get the file key to confirm
            let file_key = file_keys_to_confirm[i];

            // Get its file key proof from the generated proofs.
            let encoded_file_key_proof = fetch_file_key_proof_for_bsp_confirm(i as u32);
            let file_key_proof = <KeyProof<T>>::decode(&mut encoded_file_key_proof.as_ref())
                .expect("File key proof should be decodable");

            // Create the storage request for it:
            let location = file_key_proof.file_metadata.location().clone();
            let fingerprint_hash = file_key_proof.file_metadata.fingerprint().clone().as_hash();
            let fingerprint =
                <T as frame_system::Config>::Hash::decode(&mut fingerprint_hash.as_ref())
                    .expect("Fingerprint should be decodable as it is a hash");
            let size = file_key_proof.file_metadata.file_size();
            let storage_request_metadata = StorageRequestMetadata::<T> {
				requested_at:
					<<T as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick(),
				expires_at: BlockNumberFor::<T>::max_value(),
				owner: user_account.clone(),
				bucket_id,
				location: location.clone().try_into().unwrap(),
				fingerprint: fingerprint.into(),
				size,
				msp: Some((msp_id, true)), // MSP accepted means the logic to delete the storage request is executed
				user_peer_ids: Default::default(),
				bsps_required: T::StandardReplicationTarget::get(),
				bsps_confirmed: T::StandardReplicationTarget::get().saturating_sub(ReplicationTargetType::<T>::one()), // All BSPs confirmed minus one means the logic to delete the storage request is executed
				bsps_volunteered: T::MaxReplicationTarget::get().into(), // Maximize the BSPs volunteered since the logic has to drain them from storage
				deposit_paid: Default::default(),
			};
            <StorageRequests<T>>::insert(&file_key, storage_request_metadata);
            <BucketsWithStorageRequests<T>>::insert(&bucket_id, &file_key, ());
            // Add the volunteered BSPs to the StorageRequestBsps storage for this file key
            for i in 0u64..T::MaxReplicationTarget::get().into() {
                let bsp_user: T::AccountId = account("bsp_volunteered", i as u32, i as u32);
                mint_into_account::<T>(bsp_user.clone(), 1_000_000_000_000_000)?;
                let bsp_id = add_bsp_to_provider_storage::<T>(&bsp_user.clone(), None);
                StorageRequestBsps::<T>::insert(
                    file_key,
                    bsp_id,
                    StorageRequestBspsMetadata::<T> {
                        confirmed: false,
                        _phantom: Default::default(),
                    },
                );
            }

            // Create the FileKeyWithProof object
            let file_key_with_proof = FileKeyWithProof {
                file_key,
                proof: file_key_proof,
            };

            // Push it to the file keys and proofs bounded vector
            file_keys_and_proofs
                .try_push(file_key_with_proof)
                .expect("File key amounts is limited by the same value as the bounded vector");

            // Query the earliest that this BSP can volunteer for this file
            let query_result = Pallet::<T>::query_earliest_file_volunteer_tick(bsp_id, file_key);

            // Check if an error was returned and, if so, which one
            let tick_to_advance_to = match query_result {
                Err(error) => match error {
                    QueryFileEarliestVolunteerTickError::FailedToEncodeFingerprint => {
                        return Err(BenchmarkError::Stop("Failed to encode fingerprint."));
                    }
                    QueryFileEarliestVolunteerTickError::FailedToEncodeBsp => {
                        return Err(BenchmarkError::Stop("Failed to encode BSP."));
                    }
                    QueryFileEarliestVolunteerTickError::ThresholdArithmeticError => {
                        return Err(BenchmarkError::Stop("Threshold arithmetic error."));
                    }
                    QueryFileEarliestVolunteerTickError::StorageRequestNotFound => {
                        return Err(BenchmarkError::Stop("Storage request not found."));
                    }
                    QueryFileEarliestVolunteerTickError::FailedToComputeEligibilityCriteria => {
                        return Err(BenchmarkError::Stop(
                            "Failed to compute eligibility criteria for BSP.",
                        ));
                    }
                    QueryFileEarliestVolunteerTickError::InternalError => {
                        return Err(BenchmarkError::Stop("Internal runtime API error."));
                    }
                },
                Ok(earliest_volunteer_tick) => earliest_volunteer_tick,
            };

            // Advance the block number to the earliest tick where the BSP can volunteer, if it's not in the past
            if tick_to_advance_to > frame_system::Pallet::<T>::block_number() {
                run_to_block::<T>(tick_to_advance_to);
            }

            // Volunteer for the file
            Pallet::<T>::bsp_volunteer(bsp_signed_origin.clone().into(), file_key)?;
        }

        // Get the non-inclusion forest proof for this amount of file keys
        let encoded_non_inclusion_forest_proof =
            fetch_non_inclusion_proofs(amount_of_files_to_confirm_storing, 1);
        let non_inclusion_forest_proof =
            <<<T as Config>::ProofDealer as ProofsDealerInterface>::ForestProof>::decode(
                &mut encoded_non_inclusion_forest_proof.as_ref(),
            )
            .expect("Non-inclusion forest proof should be decodable");

        // Collect the file metadata before the extrinsic call (since file_keys_and_proofs will be moved)
        let confirmed_file_keys_with_metadata: Vec<_> = file_keys_and_proofs
            .iter()
            .map(|file_key_with_proof| {
                (
                    file_key_with_proof.file_key,
                    file_key_with_proof.proof.file_metadata.clone(),
                )
            })
            .collect();

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(
            bsp_signed_origin,
            non_inclusion_forest_proof,
            file_keys_and_proofs,
        );

        /*********** Post-benchmark checks: ***********/
        // Ensure the expected events were emitted.
        for file_key in file_keys_to_confirm.clone() {
            let expected_event =
                <T as pallet::Config>::RuntimeEvent::from(Event::StorageRequestFulfilled {
                    file_key,
                });
            frame_system::Pallet::<T>::assert_has_event(expected_event.into());
        }

        let new_bsp_root = pallet_storage_providers::Pallet::<T>::get_root(bsp_id).unwrap();
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::BspConfirmedStoring {
                who: bsp_account,
                bsp_id,
                confirmed_file_keys: confirmed_file_keys_with_metadata.try_into().unwrap(),
                skipped_file_keys: BoundedVec::default(),
                new_root: new_bsp_root,
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        Ok(())
    }

    #[benchmark]
    fn bsp_request_stop_storing() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Get the user account for the generated proofs and load it up with some balance.
        let user_as_bytes: [u8; 32] = get_user_account().clone().try_into().unwrap();
        let user_account: T::AccountId = T::AccountId::decode(&mut &user_as_bytes[..]).unwrap();
        mint_into_account::<T>(user_account.clone(), 1_000_000_000_000_000_000_000)?;

        // Register an account as a MSP with the specific MSP ID from the generated proofs
        let msp_account: T::AccountId = account("MSP", 0, 0);
        mint_into_account::<T>(msp_account.clone(), 1_000_000_000_000_000_000_000)?;
        let encoded_msp_id = get_msp_id();
        let msp_id = <T as frame_system::Config>::Hash::decode(&mut encoded_msp_id.as_ref())
            .expect("Failed to decode provider ID from bytes.");
        let (_, value_prop_id) = add_msp_to_provider_storage::<T>(&msp_account, Some(msp_id));

        // Register the BSP which will request to stop storing the file
        let bsp_account: T::AccountId = account("BSP", 0, 0);
        mint_into_account::<T>(bsp_account.clone(), 1_000_000_000_000_000)?;
        let encoded_bsp_id = get_bsp_id();
        let bsp_id = <T as frame_system::Config>::Hash::decode(&mut encoded_bsp_id.as_ref())
            .expect("Failed to decode provider ID from bytes.");
        let bsp_signed_origin = RawOrigin::Signed(bsp_account.clone());
        add_bsp_to_provider_storage::<T>(&bsp_account, Some(bsp_id));

        // Change the root of the BSP to match the inclusion forest proof.
        let encoded_bsp_root = get_bsp_root();
        let bsp_root = <T as frame_system::Config>::Hash::decode(&mut encoded_bsp_root.as_ref())
            .expect("BSP root should be decodable as it is a hash");
        pallet_storage_providers::BackupStorageProviders::<T>::mutate(&bsp_id, |bsp| {
            let bsp = bsp.as_mut().expect("BSP should exist.");
            bsp.root = bsp_root
        });

        // Get the file's metadata
        let file_metadata = fetch_file_key_metadata_for_inclusion_proof();
        let file_fingerprint = <T as frame_system::Config>::Hash::decode(
            &mut file_metadata.fingerprint().as_hash().as_ref(),
        )
        .expect("Fingerprint should be decodable as it is a hash");
        let file_location: FileLocation<T> = file_metadata.location().clone().try_into().unwrap();
        let file_size = file_metadata.file_size();
        let file_bucket_id =
            <T as frame_system::Config>::Hash::decode(&mut file_metadata.bucket_id().as_ref())
                .expect("Bucket ID should be decodable as it is a hash");

        // Create the bucket to store in the MSP
        <<T as crate::Config>::Providers as MutateBucketsInterface>::add_bucket(
            msp_id,
            user_account.clone(),
            file_bucket_id,
            false,
            None,
            value_prop_id,
        )?;

        // Get the file key for the BSP to request stop storing
        let encoded_file_key = fetch_file_key_for_inclusion_proof();
        let file_key = <T as frame_system::Config>::Hash::decode(&mut encoded_file_key.as_ref())
            .expect("File key should be decodable as it is a hash");

        // Get the inclusion proof for the file key
        let encoded_inclusion_proof = fetch_inclusion_proof();
        let inclusion_proof =
            <<<T as Config>::ProofDealer as ProofsDealerInterface>::ForestProof>::decode(
                &mut encoded_inclusion_proof.as_ref(),
            )
            .expect("Inclusion forest proof should be decodable");

        // Worst-case scenario is for the storage request to not exist previously (so it has to be created) and for the BSP
        // to be able to serve the file (since there's an extra write to storage):
        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(
            bsp_signed_origin,
            file_key,
            file_bucket_id,
            file_location.clone(),
            user_account.clone(),
            file_fingerprint,
            file_size,
            true,
            inclusion_proof,
        );

        /*********** Post-benchmark checks: ***********/
        // Ensure the expected event was emitted.
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::BspRequestedToStopStoring {
                bsp_id,
                file_key,
                owner: user_account,
                location: file_location,
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Ensure the storage request was opened for the file key
        assert!(StorageRequests::<T>::contains_key(&file_key));

        // Ensure the BSP was added to the BSPs of the storage request
        assert!(StorageRequestBsps::<T>::contains_key(&file_key, &bsp_id));

        // Ensure the pending stop storage request was added to storage
        assert!(PendingStopStoringRequests::<T>::contains_key(
            &bsp_id, &file_key
        ));

        Ok(())
    }

    #[benchmark]
    fn bsp_confirm_stop_storing() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Get the user account for the generated proofs and load it up with some balance.
        let user_as_bytes: [u8; 32] = get_user_account().clone().try_into().unwrap();
        let user_account: T::AccountId = T::AccountId::decode(&mut &user_as_bytes[..]).unwrap();
        mint_into_account::<T>(user_account.clone(), 1_000_000_000_000_000_000_000)?;

        // Register an account as a MSP with the specific MSP ID from the generated proofs
        let msp_account: T::AccountId = account("MSP", 0, 0);
        mint_into_account::<T>(msp_account.clone(), 1_000_000_000_000_000_000_000)?;
        let encoded_msp_id = get_msp_id();
        let msp_id = <T as frame_system::Config>::Hash::decode(&mut encoded_msp_id.as_ref())
            .expect("Failed to decode provider ID from bytes.");
        let (_, value_prop_id) = add_msp_to_provider_storage::<T>(&msp_account, Some(msp_id));

        // Register the BSP which will confirm to stop storing the file
        let bsp_account: T::AccountId = account("BSP", 0, 0);
        mint_into_account::<T>(bsp_account.clone(), 1_000_000_000_000_000)?;
        let encoded_bsp_id = get_bsp_id();
        let bsp_id = <T as frame_system::Config>::Hash::decode(&mut encoded_bsp_id.as_ref())
            .expect("Failed to decode provider ID from bytes.");
        let bsp_signed_origin = RawOrigin::Signed(bsp_account.clone());
        add_bsp_to_provider_storage::<T>(&bsp_account, Some(bsp_id));

        // Get the file's metadata
        let file_metadata = fetch_file_key_metadata_for_inclusion_proof();
        let file_fingerprint = <T as frame_system::Config>::Hash::decode(
            &mut file_metadata.fingerprint().as_hash().as_ref(),
        )
        .expect("Fingerprint should be decodable as it is a hash");
        let file_location: FileLocation<T> = file_metadata.location().clone().try_into().unwrap();
        let file_size = file_metadata.file_size();
        let file_bucket_id =
            <T as frame_system::Config>::Hash::decode(&mut file_metadata.bucket_id().as_ref())
                .expect("Bucket ID should be decodable as it is a hash");

        // Increase the used capacity of the BSP to match the file size, so its challenge and randomness cycles gets reset when confirming
        // to stop storing the file (worst-case scenario). Also, change the root of the BSP to match the inclusion forest proof.
        let encoded_bsp_root = get_bsp_root();
        let bsp_root = <T as frame_system::Config>::Hash::decode(&mut encoded_bsp_root.as_ref())
            .expect("BSP root should be decodable as it is a hash");
        pallet_storage_providers::BackupStorageProviders::<T>::mutate(&bsp_id, |bsp| {
            let bsp = bsp.as_mut().expect("BSP should exist.");
            bsp.root = bsp_root;
            bsp.capacity_used += file_size;
        });
        // Set the used capacity to the total capacity to simulate the worst-case scenario of treasury cut calculation when charging the payment stream.
        pallet_storage_providers::UsedBspsCapacity::<T>::set(file_size);
        pallet_storage_providers::TotalBspsCapacity::<T>::set(file_size);

        // Create the dynamic-rate payment stream between the user and the BSP to account for the worst-case scenario
        // of deleting it in the confirm stop storing
        pallet_payment_streams::DynamicRatePaymentStreams::<T>::insert(
            &bsp_id,
            &user_account,
            DynamicRatePaymentStream {
                amount_provided: (file_size as u32).into(), // This is so the payment stream gets deleted, worst-case scenario
                price_index_when_last_charged: 0u32.into(),
                user_deposit: 100u32.into(),
                out_of_funds_tick: None,
            },
        );

        // Hold some of the user's balance so it simulates it having a deposit for the payment stream.
        assert_ok!(<T as crate::Config>::Currency::hold(
            &pallet_payment_streams::HoldReason::PaymentStreamDeposit.into(),
            &user_account,
            100u32.into(),
        ));

        // Update the last chargeable info of the BSP to make it actually charge the user
        pallet_payment_streams::LastChargeableInfo::<T>::insert(
            &bsp_id,
            pallet_payment_streams::types::ProviderLastChargeableInfo {
                last_chargeable_tick: frame_system::Pallet::<T>::block_number(),
                price_index: 100u32.into(),
            },
        );

        // Create the bucket to store in the MSP
        <<T as crate::Config>::Providers as MutateBucketsInterface>::add_bucket(
            msp_id,
            user_account.clone(),
            file_bucket_id,
            false,
            None,
            value_prop_id,
        )?;

        // Get the file key for the BSP to request stop storing
        let encoded_file_key = fetch_file_key_for_inclusion_proof();
        let file_key = <T as frame_system::Config>::Hash::decode(&mut encoded_file_key.as_ref())
            .expect("File key should be decodable as it is a hash");

        // Get the inclusion proof for the file key
        let encoded_inclusion_proof = fetch_inclusion_proof();
        let inclusion_proof =
            <<<T as Config>::ProofDealer as ProofsDealerInterface>::ForestProof>::decode(
                &mut encoded_inclusion_proof.as_ref(),
            )
            .expect("Inclusion forest proof should be decodable");

        // The BSP requests to stop storing the file
        Pallet::<T>::bsp_request_stop_storing(
            bsp_signed_origin.clone().into(),
            file_key,
            file_bucket_id,
            file_location.clone(),
            user_account.clone(),
            file_fingerprint,
            file_size,
            true,
            inclusion_proof.clone(),
        )?;

        // Advance enough blocks so the BSP is allowed to confirm to stop storing the file
        run_to_block::<T>(
            frame_system::Pallet::<T>::block_number() + T::MinWaitForStopStoring::get(),
        );

        // Get some variables for comparison after the call
        let previous_bsp_capacity_used =
            <<T as crate::Config>::Providers as ReadStorageProvidersInterface>::get_used_capacity(
                &bsp_id,
            );
        let previous_bsp_root =
            <<T as crate::Config>::Providers as ReadProvidersInterface>::get_root(bsp_id).unwrap();

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(bsp_signed_origin, file_key, inclusion_proof);

        /*********** Post-benchmark checks: ***********/
        // Get the new values after calling the extrinsic:
        let new_bsp_capacity_used =
            <<T as crate::Config>::Providers as ReadStorageProvidersInterface>::get_used_capacity(
                &bsp_id,
            );
        let new_bsp_root =
            <<T as crate::Config>::Providers as ReadProvidersInterface>::get_root(bsp_id).unwrap();

        // Ensure the expected event was emitted.
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::BspConfirmStoppedStoring {
                bsp_id,
                file_key,
                new_root: new_bsp_root,
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Ensure the pending stop storage request was removed from storage
        assert!(!PendingStopStoringRequests::<T>::contains_key(
            &bsp_id, &file_key
        ));

        // Ensure the new capacity used of the BSP is the previous one minus the file size
        assert_eq!(
            new_bsp_capacity_used,
            previous_bsp_capacity_used - file_size,
            "BSP capacity used should be the previous one minus the file size."
        );

        // Ensure the root of the BSP was updated
        assert_ne!(
            new_bsp_root, previous_bsp_root,
            "BSP root should have been updated."
        );

        // Ensure the payment stream between the user and the BSP has been deleted
        assert!(
            !pallet_payment_streams::DynamicRatePaymentStreams::<T>::contains_key(
                &bsp_id,
                &user_account
            )
        );

        Ok(())
    }

    #[benchmark]
    fn stop_storing_for_insolvent_user_bsp() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Get the user account for the generated proofs and load it up with some balance.
        let user_as_bytes: [u8; 32] = get_user_account().clone().try_into().unwrap();
        let user_account: T::AccountId = T::AccountId::decode(&mut &user_as_bytes[..]).unwrap();
        mint_into_account::<T>(user_account.clone(), 1_000_000_000_000_000_000_000)?;

        // Register an account as a MSP with the specific MSP ID from the generated proofs
        let msp_account: T::AccountId = account("MSP", 0, 0);
        mint_into_account::<T>(msp_account.clone(), 1_000_000_000_000_000_000_000)?;
        let encoded_msp_id = get_msp_id();
        let msp_id = <T as frame_system::Config>::Hash::decode(&mut encoded_msp_id.as_ref())
            .expect("Failed to decode provider ID from bytes.");
        let (_, value_prop_id) = add_msp_to_provider_storage::<T>(&msp_account, Some(msp_id));

        // Register the BSP which will stop storing the file for the insolvent user
        let bsp_account: T::AccountId = account("BSP", 0, 0);
        mint_into_account::<T>(bsp_account.clone(), 1_000_000_000_000_000)?;
        let encoded_bsp_id = get_bsp_id();
        let bsp_id = <T as frame_system::Config>::Hash::decode(&mut encoded_bsp_id.as_ref())
            .expect("Failed to decode provider ID from bytes.");
        let bsp_signed_origin = RawOrigin::Signed(bsp_account.clone());
        add_bsp_to_provider_storage::<T>(&bsp_account, Some(bsp_id));

        // Get the file's metadata
        let file_metadata = fetch_file_key_metadata_for_inclusion_proof();
        let file_fingerprint = <T as frame_system::Config>::Hash::decode(
            &mut file_metadata.fingerprint().as_hash().as_ref(),
        )
        .expect("Fingerprint should be decodable as it is a hash");
        let file_location: FileLocation<T> = file_metadata.location().clone().try_into().unwrap();
        let file_size = file_metadata.file_size();
        let file_bucket_id =
            <T as frame_system::Config>::Hash::decode(&mut file_metadata.bucket_id().as_ref())
                .expect("Bucket ID should be decodable as it is a hash");

        // Increase the used capacity of the BSP to match the file size, so its challenge and randomness cycles gets reset when confirming
        // to stop storing the file (worst-case scenario). Also, change the root of the BSP to match the inclusion forest proof.
        let encoded_bsp_root = get_bsp_root();
        let bsp_root = <T as frame_system::Config>::Hash::decode(&mut encoded_bsp_root.as_ref())
            .expect("BSP root should be decodable as it is a hash");
        pallet_storage_providers::BackupStorageProviders::<T>::mutate(&bsp_id, |bsp| {
            let bsp = bsp.as_mut().expect("BSP should exist.");
            bsp.root = bsp_root;
            bsp.capacity_used += file_size;
        });
        // Set the used capacity to the total capacity to simulate the worst-case scenario of treasury cut calculation when charging the payment stream.
        pallet_storage_providers::UsedBspsCapacity::<T>::set(file_size);
        pallet_storage_providers::TotalBspsCapacity::<T>::set(file_size);

        // Create the dynamic-rate payment stream between the user and the BSP to account for the worst-case scenario
        // of charging it and deleting it in the stop storing for insolvent user
        pallet_payment_streams::DynamicRatePaymentStreams::<T>::insert(
            &bsp_id,
            &user_account,
            DynamicRatePaymentStream {
                amount_provided: (file_size as u32).into(),
                price_index_when_last_charged: 0u32.into(),
                user_deposit: 100u32.into(),
                out_of_funds_tick: None,
            },
        );

        // Hold some of the user's balance so it simulates it having a deposit for the payment stream.
        assert_ok!(<T as crate::Config>::Currency::hold(
            &pallet_payment_streams::HoldReason::PaymentStreamDeposit.into(),
            &user_account,
            100u32.into(),
        ));

        // Update the last chargeable info of the BSP to make it actually charge the user
        pallet_payment_streams::LastChargeableInfo::<T>::insert(
            &bsp_id,
            pallet_payment_streams::types::ProviderLastChargeableInfo {
                last_chargeable_tick: frame_system::Pallet::<T>::block_number(),
                price_index: 100u32.into(),
            },
        );

        // Create the bucket to store in the MSP
        <<T as crate::Config>::Providers as MutateBucketsInterface>::add_bucket(
            msp_id,
            user_account.clone(),
            file_bucket_id,
            false,
            None,
            value_prop_id,
        )?;

        // Get the file key for the BSP to request stop storing
        let encoded_file_key = fetch_file_key_for_inclusion_proof();
        let file_key = <T as frame_system::Config>::Hash::decode(&mut encoded_file_key.as_ref())
            .expect("File key should be decodable as it is a hash");

        // Get the inclusion proof for the file key
        let encoded_inclusion_proof = fetch_inclusion_proof();
        let inclusion_proof =
            <<<T as Config>::ProofDealer as ProofsDealerInterface>::ForestProof>::decode(
                &mut encoded_inclusion_proof.as_ref(),
            )
            .expect("Inclusion forest proof should be decodable");

        // Flag the owner of the file as insolvent
        pallet_payment_streams::UsersWithoutFunds::<T>::insert(
            &user_account,
            frame_system::Pallet::<T>::block_number(),
        );

        // Get some variables for comparison after the call
        let previous_bsp_capacity_used =
            <<T as crate::Config>::Providers as ReadStorageProvidersInterface>::get_used_capacity(
                &bsp_id,
            );
        let previous_bsp_root =
            <<T as crate::Config>::Providers as ReadProvidersInterface>::get_root(bsp_id).unwrap();

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        stop_storing_for_insolvent_user(
            bsp_signed_origin.clone(),
            file_key,
            file_bucket_id,
            file_location.clone(),
            user_account.clone(),
            file_fingerprint,
            file_size,
            inclusion_proof.clone(),
        );

        /*********** Post-benchmark checks: ***********/
        // Get the new values after calling the extrinsic:
        let new_bsp_capacity_used =
            <<T as crate::Config>::Providers as ReadStorageProvidersInterface>::get_used_capacity(
                &bsp_id,
            );
        let new_bsp_root =
            <<T as crate::Config>::Providers as ReadProvidersInterface>::get_root(bsp_id).unwrap();

        // Ensure the expected event was emitted.
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::SpStopStoringInsolventUser {
                sp_id: bsp_id,
                file_key,
                new_root: new_bsp_root,
                owner: user_account.clone(),
                location: file_location,
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Ensure the new capacity used of the BSP is the previous one minus the file size
        assert_eq!(
            new_bsp_capacity_used,
            previous_bsp_capacity_used - file_size,
            "BSP capacity used should be the previous one minus the file size."
        );

        // Ensure the root of the BSP was updated
        assert_ne!(
            new_bsp_root, previous_bsp_root,
            "BSP root should have been updated."
        );

        // Ensure the payment stream between the user and the BSP has been deleted
        assert!(
            !pallet_payment_streams::DynamicRatePaymentStreams::<T>::contains_key(
                &bsp_id,
                &user_account
            )
        );

        Ok(())
    }

    #[benchmark]
    fn stop_storing_for_insolvent_user_msp() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Get the user account for the generated proofs and load it up with some balance.
        let user_as_bytes: [u8; 32] = get_user_account().clone().try_into().unwrap();
        let user_account: T::AccountId = T::AccountId::decode(&mut &user_as_bytes[..]).unwrap();
        mint_into_account::<T>(user_account.clone(), 1_000_000_000_000_000_000_000)?;

        // Register an account as a MSP with the specific MSP ID from the generated proofs
        let msp_account: T::AccountId = account("MSP", 0, 0);
        let msp_signed_origin = RawOrigin::Signed(msp_account.clone());
        mint_into_account::<T>(msp_account.clone(), 1_000_000_000_000_000_000_000)?;
        let encoded_msp_id = get_msp_id();
        let msp_id = <T as frame_system::Config>::Hash::decode(&mut encoded_msp_id.as_ref())
            .expect("Failed to decode provider ID from bytes.");
        let (_, value_prop_id) = add_msp_to_provider_storage::<T>(&msp_account, Some(msp_id));

        // Get the file's metadata
        let file_metadata = fetch_file_key_metadata_for_inclusion_proof();
        let file_fingerprint = <T as frame_system::Config>::Hash::decode(
            &mut file_metadata.fingerprint().as_hash().as_ref(),
        )
        .expect("Fingerprint should be decodable as it is a hash");
        let file_location: FileLocation<T> = file_metadata.location().clone().try_into().unwrap();
        let file_size = file_metadata.file_size();
        let file_bucket_id =
            <T as frame_system::Config>::Hash::decode(&mut file_metadata.bucket_id().as_ref())
                .expect("Bucket ID should be decodable as it is a hash");

        // Create the bucket to store in the MSP
        <<T as crate::Config>::Providers as MutateBucketsInterface>::add_bucket(
            msp_id,
            user_account.clone(),
            file_bucket_id,
            false,
            None,
            value_prop_id,
        )?;

        // Increase the used capacity of the MSP to match the file size
        pallet_storage_providers::MainStorageProviders::<T>::mutate(&msp_id, |msp| {
            let msp = msp.as_mut().expect("MSP should exist.");
            msp.capacity_used += file_size;
        });

        // Update the fixed-rate payment stream between the user and the MSP to account for file being stored
        pallet_payment_streams::FixedRatePaymentStreams::<T>::mutate(
            &msp_id,
            &user_account,
            |payment_stream| {
                let payment_stream = payment_stream
                    .as_mut()
                    .expect("Payment stream should exist.");
                payment_stream.rate += 100_000u32.into();
            },
        );

        // Hold some of the user's balance so it simulates it having a deposit for the payment stream.
        assert_ok!(<T as crate::Config>::Currency::hold(
            &pallet_payment_streams::HoldReason::PaymentStreamDeposit.into(),
            &user_account,
            100u32.into(),
        ));

        // Set the bucket's root and size to match what's in the inclusion proof
        let encoded_bucket_root = get_bsp_root();
        let bucket_root =
            <T as frame_system::Config>::Hash::decode(&mut encoded_bucket_root.as_ref())
                .expect("Bucket root should be decodable as it is a hash");
        pallet_storage_providers::Buckets::<T>::mutate(&file_bucket_id, |bucket| {
            let bucket = bucket.as_mut().expect("Bucket should exist.");
            bucket.root = bucket_root;
            bucket.size += file_size;
        });

        // Get the file key for the MSP to stop storing
        let encoded_file_key = fetch_file_key_for_inclusion_proof();
        let file_key = <T as frame_system::Config>::Hash::decode(&mut encoded_file_key.as_ref())
            .expect("File key should be decodable as it is a hash");

        // Get the inclusion proof for the file key
        let encoded_inclusion_proof = fetch_inclusion_proof();
        let inclusion_proof =
            <<<T as Config>::ProofDealer as ProofsDealerInterface>::ForestProof>::decode(
                &mut encoded_inclusion_proof.as_ref(),
            )
            .expect("Inclusion forest proof should be decodable");

        // Flag the owner of the file as insolvent
        pallet_payment_streams::UsersWithoutFunds::<T>::insert(
            &user_account,
            frame_system::Pallet::<T>::block_number(),
        );

        // Get some variables for comparison after the call
        let previous_msp_capacity_used =
            <<T as crate::Config>::Providers as ReadStorageProvidersInterface>::get_used_capacity(
                &msp_id,
            );
        let previous_bucket_size = pallet_storage_providers::Buckets::<T>::get(&file_bucket_id)
            .unwrap()
            .size;
        let previous_bucket_root = pallet_storage_providers::Buckets::<T>::get(&file_bucket_id)
            .unwrap()
            .root;

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        stop_storing_for_insolvent_user(
            msp_signed_origin.clone(),
            file_key,
            file_bucket_id,
            file_location.clone(),
            user_account.clone(),
            file_fingerprint,
            file_size,
            inclusion_proof.clone(),
        );

        /*********** Post-benchmark checks: ***********/
        // Get the new values after calling the extrinsic:
        let new_msp_capacity_used =
            <<T as crate::Config>::Providers as ReadStorageProvidersInterface>::get_used_capacity(
                &msp_id,
            );
        let new_bucket_size = pallet_storage_providers::Buckets::<T>::get(&file_bucket_id)
            .unwrap()
            .size;
        let new_bucket_root = pallet_storage_providers::Buckets::<T>::get(&file_bucket_id)
            .unwrap()
            .root;

        // Ensure the expected event was emitted.
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::SpStopStoringInsolventUser {
                sp_id: msp_id,
                file_key,
                new_root: new_bucket_root,
                owner: user_account.clone(),
                location: file_location,
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Ensure the new capacity used of the MSP is the previous one minus the file size
        assert_eq!(
            new_msp_capacity_used,
            previous_msp_capacity_used - file_size,
            "BSP capacity used should be the previous one minus the file size."
        );

        // Ensure the root of the Bucket was updated
        assert_ne!(
            new_bucket_root, previous_bucket_root,
            "BSP root should have been updated."
        );

        // Ensure the size of the Bucket was updated to the previous size minus the file size
        assert_eq!(
            new_bucket_size,
            previous_bucket_size - file_size,
            "Bucket size should have been updated."
        );

        // Ensure the payment stream between the user and the MSP has been deleted
        assert!(
            !pallet_payment_streams::FixedRatePaymentStreams::<T>::contains_key(
                &msp_id,
                &user_account
            )
        );

        Ok(())
    }

    #[benchmark]
    fn msp_stop_storing_bucket_for_insolvent_user() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Get a user account and mint some tokens into it.
        let user: T::AccountId = account("Alice", 0, 0);
        let signed_origin = RawOrigin::Signed(user.clone());
        mint_into_account::<T>(user.clone(), 1_000_000_000_000_000)?;

        // Set up parameters for the bucket to use.
        let name: BucketNameFor<T> = vec![1; BucketNameLimitFor::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();
        let bucket_id = <<T as crate::Config>::Providers as ReadBucketsInterface>::derive_bucket_id(
            &user,
            name.clone(),
        );

        // Register a MSP with a value proposition.
        let msp: T::AccountId = account("MSP", 0, 0);
        let signed_msp_origin = RawOrigin::Signed(msp.clone());
        mint_into_account::<T>(msp.clone(), 1_000_000_000_000_000)?;
        let (msp_id, value_prop_id) = add_msp_to_provider_storage::<T>(&msp, None);

        // Create the bucket as private, creating the collection so it has to be deleted as well.
        Pallet::<T>::create_bucket(
            signed_origin.clone().into(),
            msp_id,
            name,
            true,
            value_prop_id,
        )?;

        // Get the collection ID of the bucket.
        let collection_id = T::Providers::get_read_access_group_id_of_bucket(&bucket_id)?.unwrap();

        // Increase the used capacity of the bucket and the MSP, to simulate it currently being used.
        let bucket_size = 100;
        pallet_storage_providers::Buckets::<T>::mutate(&bucket_id, |bucket| {
            let bucket = bucket.as_mut().expect("Bucket should exist.");
            bucket.size += bucket_size;
        });
        let previous_msp_capacity_used =
            pallet_storage_providers::MainStorageProviders::<T>::mutate(&msp_id, |msp| {
                let msp = msp.as_mut().expect("MSP should exist.");
                msp.capacity_used += bucket_size;
                msp.capacity_used
            });

        // Flag the owner of the file as insolvent.
        pallet_payment_streams::UsersWithoutFunds::<T>::insert(
            &user,
            frame_system::Pallet::<T>::block_number(),
        );

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(signed_msp_origin, bucket_id);

        /*********** Post-benchmark checks: ***********/
        // Ensure the expected event was emitted.
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::MspStopStoringBucketInsolventUser {
                msp_id,
                owner: user.clone(),
                bucket_id,
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // The bucket should have been deleted.
        assert!(!T::Providers::bucket_exists(&bucket_id));

        // And the collection should have been deleted as well.
        assert!(!pallet_nfts::Collection::<T>::contains_key(collection_id));

        // Ensure the payment stream between the user and the MSP has been deleted.
        assert!(
            !pallet_payment_streams::FixedRatePaymentStreams::<T>::contains_key(&msp_id, &user)
        );

        // Ensure the used capacity of the MSP has been updated.
        assert_eq!(
            pallet_storage_providers::MainStorageProviders::<T>::get(&msp_id)
                .unwrap()
                .capacity_used,
            previous_msp_capacity_used - bucket_size,
        );

        Ok(())
    }

    #[benchmark]
    fn on_poll_hook() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Set the total used capacity of the network to be the same as the total capacity of the network,
        // since this makes the price updater use the second order Taylor series approximation, which
        // is the most computationally expensive.
        let total_capacity: StorageDataUnit<T> = 1024 * 1024 * 1024;
        pallet_storage_providers::UsedBspsCapacity::<T>::put(total_capacity);
        pallet_storage_providers::TotalBspsCapacity::<T>::put(total_capacity);

        // Get the current price per giga unit per tick before updating
        let current_price_per_giga_unit_per_tick =
            pallet_payment_streams::CurrentPricePerGigaUnitPerTick::<T>::get();

        /*********** Call the function to benchmark: ***********/
        #[block]
        {
            Pallet::<T>::do_on_poll(&mut WeightMeter::new());
        }

        /*********** Post-benchmark checks: ***********/
        // Ensure the price per giga unit per tick was updated
        assert_ne!(
            pallet_payment_streams::CurrentPricePerGigaUnitPerTick::<T>::get(),
            current_price_per_giga_unit_per_tick,
            "Price per giga unit per tick should have been updated."
        );

        Ok(())
    }

    #[benchmark]
    fn process_expired_storage_request_msp_accepted_or_no_msp(
        n: Linear<
            0,
            {
                <<T as pallet::Config>::ReplicationTargetType as Into<u64>>::into(
                    T::MaxReplicationTarget::get().into(),
                ) as u32
            },
        >,
    ) -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Get the amount of BSPs to add to the storage request
        let amount_of_bsps = n.into();

        // Get a user account and mint some tokens into it
        let user: T::AccountId = account("Alice", 0, 0);
        let signed_origin = RawOrigin::Signed(user.clone());
        mint_into_account::<T>(user.clone(), 1_000_000_000_000_000)?;

        // Register a MSP with a value proposition
        let msp_account: T::AccountId = account("MSP", 0, 0);
        mint_into_account::<T>(msp_account.clone(), 1_000_000_000_000_000)?;
        let (msp_id, value_prop_id) = add_msp_to_provider_storage::<T>(&msp_account, None);

        // Create the bucket, assigning it to the MSP
        let name: BucketNameFor<T> = vec![1; BucketNameLimitFor::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();
        let bucket_id = <<T as crate::Config>::Providers as ReadBucketsInterface>::derive_bucket_id(
            &user,
            name.clone(),
        );
        Pallet::<T>::create_bucket(
            signed_origin.clone().into(),
            msp_id,
            name,
            true,
            value_prop_id,
        )?;

        // Issue the storage request from the user
        let location: FileLocation<T> = vec![1; MaxFilePathSize::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();
        let fingerprint =
            <<T as frame_system::Config>::Hashing as Hasher>::hash(b"benchmark_fingerprint");
        let size: StorageDataUnit<T> = 100;
        let peer_id: PeerId<T> = vec![1; MaxPeerIdSize::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();
        let peer_ids: PeerIds<T> =
            vec![peer_id; MaxNumberOfPeerIds::<T>::get().try_into().unwrap()]
                .try_into()
                .unwrap();
        Pallet::<T>::issue_storage_request(
            signed_origin.clone().into(),
            bucket_id,
            location.clone(),
            fingerprint,
            size,
            msp_id,
            peer_ids,
            ReplicationTarget::Standard,
        )?;

        // Compute the file key
        let file_key = Pallet::<T>::compute_file_key(
            user.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        )
        .unwrap();

        // Simulate the MSP accepting the storage request
        StorageRequests::<T>::mutate(file_key, |storage_request| {
            storage_request.as_mut().unwrap().msp = Some((msp_id, true));
        });

        // Add n BSPs to the StorageRequestBsps mapping since that's the one that is drained in the benchmarked function
        for i in 0..amount_of_bsps {
            let bsp_account: T::AccountId = account("BSP", i, 0);
            let bsp_id = T::Hashing::hash_of(&bsp_account);
            <StorageRequestBsps<T>>::insert(
                &file_key,
                &bsp_id,
                StorageRequestBspsMetadata::<T> {
                    confirmed: false,
                    _phantom: Default::default(),
                },
            )
        }

        /*********** Call the function to benchmark: ***********/
        #[block]
        {
            Pallet::<T>::process_expired_storage_request(file_key, &mut WeightMeter::new());
        }

        /*********** Post-benchmark checks: ***********/
        // Ensure the expected event was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::StorageRequestExpired { file_key });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Ensure the Storage Request no longer exists in storage
        assert!(!StorageRequests::<T>::contains_key(&file_key));

        // Ensure the StorageRequestBsps mapping is empty for this file key
        let mut storage_request_bsps_for_file_key = StorageRequestBsps::<T>::iter_prefix(&file_key);
        assert!(storage_request_bsps_for_file_key.next().is_none());

        Ok(())
    }

    #[benchmark]
    fn process_expired_storage_request_msp_rejected(
        n: Linear<
            0,
            {
                <<T as pallet::Config>::ReplicationTargetType as Into<u64>>::into(
                    T::MaxReplicationTarget::get().into(),
                ) as u32
            },
        >,
    ) -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Get the amount of BSPs to add to the storage request
        let amount_of_bsps = n.into();

        // Get a user account and mint some tokens into it
        let user: T::AccountId = account("Alice", 0, 0);
        let signed_origin = RawOrigin::Signed(user.clone());
        mint_into_account::<T>(user.clone(), 1_000_000_000_000_000)?;

        // Register a MSP with a value proposition
        let msp_account: T::AccountId = account("MSP", 0, 0);
        mint_into_account::<T>(msp_account.clone(), 1_000_000_000_000_000)?;
        let (msp_id, value_prop_id) = add_msp_to_provider_storage::<T>(&msp_account, None);

        // Create the bucket, assigning it to the MSP
        let name: BucketNameFor<T> = vec![1; BucketNameLimitFor::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();
        let bucket_id = <<T as crate::Config>::Providers as ReadBucketsInterface>::derive_bucket_id(
            &user,
            name.clone(),
        );
        Pallet::<T>::create_bucket(
            signed_origin.clone().into(),
            msp_id,
            name,
            true,
            value_prop_id,
        )?;

        // Issue the storage request from the user
        let location: FileLocation<T> = vec![1; MaxFilePathSize::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();
        let fingerprint =
            <<T as frame_system::Config>::Hashing as Hasher>::hash(b"benchmark_fingerprint");
        let size: StorageDataUnit<T> = 100;
        let peer_id: PeerId<T> = vec![1; MaxPeerIdSize::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();
        let peer_ids: PeerIds<T> =
            vec![peer_id; MaxNumberOfPeerIds::<T>::get().try_into().unwrap()]
                .try_into()
                .unwrap();
        Pallet::<T>::issue_storage_request(
            signed_origin.clone().into(),
            bucket_id,
            location.clone(),
            fingerprint,
            size,
            msp_id,
            peer_ids,
            ReplicationTarget::Standard,
        )?;

        // Compute the file key
        let file_key = Pallet::<T>::compute_file_key(
            user.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        )
        .unwrap();

        // Simulate the MSP rejecting the storage request
        StorageRequests::<T>::mutate(file_key, |storage_request| {
            storage_request.as_mut().unwrap().msp = Some((msp_id, false));
        });

        // Add n BSPs to the StorageRequestBsps mapping since that's the one that is drained in the benchmarked function
        for i in 0..amount_of_bsps {
            let bsp_account: T::AccountId = account("BSP", i, 0);
            let bsp_id = T::Hashing::hash_of(&bsp_account);
            <StorageRequestBsps<T>>::insert(
                &file_key,
                &bsp_id,
                StorageRequestBspsMetadata::<T> {
                    confirmed: false,
                    _phantom: Default::default(),
                },
            )
        }

        // Simulate at least one BSP having confirmed the storage request so it has to queue up a priority challenge
        // when cleaning it up after expiration.
        StorageRequests::<T>::mutate(file_key, |storage_request| {
            storage_request.as_mut().unwrap().bsps_confirmed = ReplicationTargetType::<T>::one();
        });

        /*********** Call the function to benchmark: ***********/
        #[block]
        {
            Pallet::<T>::process_expired_storage_request(file_key, &mut WeightMeter::new());
        }

        /*********** Post-benchmark checks: ***********/
        // Ensure the expected event was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::StorageRequestRejected {
                file_key,
                msp_id,
                bucket_id,
                reason: RejectedStorageRequestReason::RequestExpired,
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Ensure the Storage Request no longer exists in storage
        assert!(!StorageRequests::<T>::contains_key(&file_key));

        // Ensure the StorageRequestBsps mapping is empty for this file key
        let mut storage_request_bsps_for_file_key = StorageRequestBsps::<T>::iter_prefix(&file_key);
        assert!(storage_request_bsps_for_file_key.next().is_none());

        Ok(())
    }

    #[benchmark]
    fn process_expired_move_bucket_request() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Get a user account and mint some tokens into it
        let user: T::AccountId = account("Alice", 0, 0);
        let signed_origin = RawOrigin::Signed(user.clone());
        mint_into_account::<T>(user.clone(), 1_000_000_000_000_000)?;

        // Register a MSP with a value proposition
        let msp_account: T::AccountId = account("MSP", 0, 0);
        mint_into_account::<T>(msp_account.clone(), 1_000_000_000_000_000)?;
        let (msp_id, value_prop_id) = add_msp_to_provider_storage::<T>(&msp_account, None);

        // Create the bucket, assigning it to the MSP
        let name: BucketNameFor<T> = vec![1; BucketNameLimitFor::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();
        let bucket_id = <<T as crate::Config>::Providers as ReadBucketsInterface>::derive_bucket_id(
            &user,
            name.clone(),
        );
        Pallet::<T>::create_bucket(
            signed_origin.clone().into(),
            msp_id,
            name,
            true,
            value_prop_id,
        )?;

        // Add the bucket to the PendingMoveBucketRequests storage
        PendingMoveBucketRequests::<T>::insert(
            &bucket_id,
            MoveBucketRequestMetadata {
                requester: user.clone(),
                new_msp_id: msp_id,
                new_value_prop_id: value_prop_id,
            },
        );

        /*********** Call the function to benchmark: ***********/
        #[block]
        {
            Pallet::<T>::process_expired_move_bucket_request(bucket_id, &mut WeightMeter::new());
        }

        /*********** Post-benchmark checks: ***********/
        // Ensure the expected event was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::MoveBucketRequestExpired {
                bucket_id,
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Ensure the bucket was removed from the PendingMoveBucketRequests storage
        assert!(!PendingMoveBucketRequests::<T>::contains_key(&bucket_id));

        Ok(())
    }

    fn run_to_block<T: crate::Config + pallet_proofs_dealer::Config>(n: BlockNumberFor<T>) {
        let mut current_block = frame_system::Pallet::<T>::block_number();

        if n == current_block {
            return;
        }

        assert!(n > current_block, "Cannot go back in time");

        while current_block < n {
            pallet_proofs_dealer::Pallet::<T>::on_finalize(current_block);

            frame_system::Pallet::<T>::set_block_number(current_block + One::one());
            current_block = frame_system::Pallet::<T>::block_number();

            pallet_proofs_dealer::Pallet::<T>::on_poll(current_block, &mut WeightMeter::new());
            Pallet::<T>::on_poll(current_block, &mut WeightMeter::new());
        }
    }

    fn mint_into_account<T: crate::Config>(
        account: T::AccountId,
        amount: u128,
    ) -> Result<(), BenchmarkError> {
        let user_balance = amount
            .try_into()
            .map_err(|_| BenchmarkError::Stop("Balance conversion failed."))?;
        assert_ok!(<T as crate::Config>::Currency::mint_into(
            &account,
            user_balance,
        ));
        Ok(())
    }

    fn add_msp_to_provider_storage<T>(
        msp: &T::AccountId,
        msp_id: Option<ProviderIdFor<T>>,
    ) -> (ProviderIdFor<T>, ValuePropId<T>)
    where
        T: crate::Config<Fingerprint = <T as frame_system::Config>::Hash>,
        T: pallet_storage_providers::Config<
            ProviderId = <T as frame_system::Config>::Hash,
            StorageDataUnit = u64,
        >,
        <T as crate::Config>::Providers: shp_traits::MutateStorageProvidersInterface<StorageDataUnit = u64>
            + shp_traits::ReadProvidersInterface<ProviderId = <T as frame_system::Config>::Hash>,
        // Ensure the ValuePropId from our Providers trait matches that from pallet_storage_providers
        <T as crate::Config>::Providers: shp_traits::MutateBucketsInterface<
            ValuePropId = <T as pallet_storage_providers::Config>::ValuePropId,
        >,
    {
        let msp_hash = if let Some(msp_id) = msp_id {
            msp_id
        } else {
            T::Hashing::hash_of(&msp)
        };

        let capacity: StorageDataUnit<T> = 1024 * 1024 * 1024;
        let capacity_used: StorageDataUnit<T> = 0;

        let msp_info = pallet_storage_providers::types::MainStorageProvider {
            capacity,
            capacity_used,
            multiaddresses: BoundedVec::default(),
            last_capacity_change: frame_system::Pallet::<T>::block_number(),
            owner_account: msp.clone(),
            payment_account: msp.clone(),
            sign_up_block: frame_system::Pallet::<T>::block_number(),
            amount_of_value_props: 1u32,
            amount_of_buckets: T::BucketCount::zero(),
        };

        pallet_storage_providers::MainStorageProviders::<T>::insert(msp_hash, msp_info);
        pallet_storage_providers::AccountIdToMainStorageProviderId::<T>::insert(
            msp.clone(),
            msp_hash,
        );

        let commitment = vec![
            1;
            <T as pallet_storage_providers::Config>::MaxCommitmentSize::get()
                .try_into()
                .unwrap()
        ]
        .try_into()
        .unwrap();

        let bucket_data_limit: StorageDataUnit<T> = capacity;
        // Use One::one() or a conversion that matches the expected balance type:
        let value_prop = ValueProposition::<T>::new(One::one(), commitment, bucket_data_limit);
        let value_prop_id = value_prop.derive_id();

        pallet_storage_providers::MainStorageProviderIdsToValuePropositions::<T>::insert(
            msp_hash,
            value_prop_id,
            value_prop,
        );

        (msp_hash, value_prop_id)
    }

    fn add_bsp_to_provider_storage<T>(
        bsp_account: &T::AccountId,
        bsp_id: Option<ProviderIdFor<T>>,
    ) -> ProviderIdFor<T>
    where
        T: crate::Config
            + pallet_storage_providers::Config<
                ProviderId = <T as frame_system::Config>::Hash,
                StorageDataUnit = u64,
            >,
        T: crate::Config<Providers = pallet_storage_providers::Pallet<T>>,
    {
        // Derive the BSP ID from the hash of its account
        let bsp_id = if let Some(bsp_id) = bsp_id {
            bsp_id
        } else {
            T::Hashing::hash_of(&bsp_account)
        };

        // Create the BSP info
        let bsp_info = pallet_storage_providers::types::BackupStorageProvider {
            capacity: 1024 * 1024 * 1024,
            capacity_used: 0,
            multiaddresses: BoundedVec::default(),
            root: <T as pallet_storage_providers::Config>::DefaultMerkleRoot::get(),
            last_capacity_change: frame_system::Pallet::<T>::block_number(),
            owner_account: bsp_account.clone(),
            payment_account: bsp_account.clone(),
            reputation_weight:
                <T as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            sign_up_block: frame_system::Pallet::<T>::block_number(),
        };

        // Insert the BSP info into storage
        pallet_storage_providers::BackupStorageProviders::<T>::insert(bsp_id, bsp_info);
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<T>::insert(
            bsp_account.clone(),
            bsp_id,
        );

        // Update the Global Reputation Weight
        pallet_storage_providers::GlobalBspsReputationWeight::<T>::mutate(|reputation_weight| {
            *reputation_weight =
                <T as pallet_storage_providers::Config>::StartingReputationWeight::get();
        });

        bsp_id
    }

    #[benchmark]
    fn delete_files_bucket(n: Linear<1, 10>) -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Get the number of file keys to delete from the linear parameter
        let number_of_file_keys: u32 = n.into();

        // Get the user account for the generated proofs and load it up with some balance.
        let user_as_bytes: [u8; 32] = get_user_account().clone().try_into().unwrap();
        let user_account: T::AccountId = T::AccountId::decode(&mut &user_as_bytes[..]).unwrap();
        mint_into_account::<T>(user_account.clone(), 1_000_000_000_000_000_000_000)?;

        // Register an account as a MSP with the specific MSP ID from the generated proofs
        let msp_account: T::AccountId = account("MSP", 0, 0);
        mint_into_account::<T>(msp_account.clone(), 1_000_000_000_000_000_000_000)?;
        let encoded_msp_id = get_msp_id();
        let msp_id = <T as frame_system::Config>::Hash::decode(&mut encoded_msp_id.as_ref())
            .expect("Failed to decode provider ID from bytes.");
        let (_, value_prop_id) = add_msp_to_provider_storage::<T>(&msp_account, Some(msp_id));

        // Create the bucket to store in the MSP (bucket 1)
        let encoded_bucket_id = get_bucket_id(1);
        let bucket_id = <T as frame_system::Config>::Hash::decode(&mut encoded_bucket_id.as_ref())
            .expect("Bucket ID should be decodable as it is a hash");
        <<T as crate::Config>::Providers as MutateBucketsInterface>::add_bucket(
            msp_id,
            user_account.clone(),
            bucket_id,
            false,
            None,
            value_prop_id,
        )?;

        // Register a BSP to be used in storage requests
        let bsp_account_for_sr: T::AccountId = account("BSP_SR", 0, 0);
        mint_into_account::<T>(bsp_account_for_sr.clone(), 1_000_000_000_000_000)?;
        let encoded_bsp_id_for_sr = get_bsp_id();
        let bsp_id_for_sr =
            <T as frame_system::Config>::Hash::decode(&mut encoded_bsp_id_for_sr.as_ref())
                .expect("BSP ID should be decodable");
        add_bsp_to_provider_storage::<T>(&bsp_account_for_sr, Some(bsp_id_for_sr));

        // Update the bucket's root to match the generated proofs
        let encoded_bucket_root = get_bucket_root(1);
        let bucket_root =
            <T as frame_system::Config>::Hash::decode(&mut encoded_bucket_root.as_ref())
                .expect("Bucket root should be decodable as it is a hash");
        let mut total_bucket_size: StorageDataUnit<T> = 0;

        // Get file keys and metadata for deletion
        // We use fetch_file_metadata_for_deletion to get metadata directly, then reconstruct file keys
        // Note: These correspond to stored files in bucket 1 (same as bucket inclusion proofs)
        let mut file_deletions: Vec<FileDeletionRequest<T>> = Vec::new();
        let mut file_keys_for_deletion: Vec<MerkleHash<T>> = Vec::new();

        for j in 0..number_of_file_keys {
            // Get file metadata for deletion (location, size, fingerprint)
            let (location_bytes, size, fingerprint_bytes) =
                fetch_file_metadata_for_deletion(number_of_file_keys, 1, j);

            let location: FileLocation<T> = location_bytes
                .try_into()
                .expect("Location bytes should convert to FileLocation");
            let fingerprint =
                <T as frame_system::Config>::Hash::decode(&mut fingerprint_bytes.as_ref())
                    .expect("Fingerprint should be decodable as it is a hash");
            total_bucket_size += size;

            // Reconstruct file key from metadata
            let file_key = Pallet::<T>::compute_file_key(
                user_account.clone(),
                bucket_id,
                location.clone(),
                size,
                fingerprint.into(),
            )?;
            file_keys_for_deletion.push(file_key);

            // Get deletion signature
            let encoded_signatures = fetch_file_deletion_signatures(number_of_file_keys, 1);
            let signature_bytes = encoded_signatures[j as usize].clone();
            let signature =
                <<T as pallet::Config>::OffchainSignature>::decode(&mut signature_bytes.as_ref())
                    .expect("Deletion signature should be decodable");

            // Create FileOperationIntention
            let signed_intention = FileOperationIntention::<T> {
                file_key,
                operation: FileOperation::Delete,
            };

            // pre-verify signature
            let signed_intention_encoded = signed_intention.encode();
            let to_verify = <T as crate::Config>::IntentionMsgAdapter::bytes_to_verify(
                &signed_intention_encoded,
            );
            let is_valid_pre_check = signature.verify(&to_verify[..], &user_account);
            ensure!(
                is_valid_pre_check,
                BenchmarkError::Stop("Signature should be valid")
            );

            // Create FileDeletionRequest
            file_deletions.push(FileDeletionRequest::<T> {
                file_owner: user_account.clone(),
                signed_intention,
                signature,
                bucket_id,
                location: location.clone(),
                size,
                fingerprint: fingerprint.into(),
            });

            // Set up storage request for this file with active storage request (worst-case scenario).
            // This ensures the benchmark captures the cost of cleanup_storage_request and add_incomplete_storage_request.
            let expiration_tick = frame_system::Pallet::<T>::block_number() + 100u32.into();
            let deposit: BalanceOf<T> = T::BaseStorageRequestCreationDeposit::get();

            let storage_request_metadata = StorageRequestMetadata::<T> {
                requested_at: frame_system::Pallet::<T>::block_number(),
                expires_at: expiration_tick,
                owner: user_account.clone(),
                bucket_id,
                location: location.clone(),
                fingerprint: fingerprint.into(),
                size,
                msp: Some((msp_id, true)), // MSP accepted triggers incomplete storage request creation
                user_peer_ids: Default::default(),
                bsps_required: ReplicationTargetType::<T>::one(),
                bsps_confirmed: ReplicationTargetType::<T>::one(),
                bsps_volunteered: ReplicationTargetType::<T>::one(),
                deposit_paid: deposit,
            };

            <T as pallet::Config>::Currency::hold(
                &HoldReason::StorageRequestCreationHold.into(),
                &user_account,
                deposit,
            )?;

            <StorageRequests<T>>::insert(&file_key, storage_request_metadata);
            <BucketsWithStorageRequests<T>>::insert(&bucket_id, &file_key, ());

            <StorageRequestExpirations<T>>::mutate(expiration_tick, |items| {
                let _ = items.try_push(file_key);
            });

            // Add the registered BSP so incomplete storage request persists after bucket deletion
            StorageRequestBsps::<T>::insert(
                &file_key,
                &bsp_id_for_sr,
                StorageRequestBspsMetadata::<T> {
                    confirmed: true,
                    _phantom: Default::default(),
                },
            );
        }

        // Update bucket size to match total
        pallet_storage_providers::Buckets::<T>::mutate(&bucket_id, |bucket| {
            let bucket = bucket.as_mut().expect("Bucket should exist.");
            bucket.size = total_bucket_size;
            bucket.root = bucket_root;
        });

        // Increase MSP used capacity to match bucket size
        pallet_storage_providers::MainStorageProviders::<T>::mutate(&msp_id, |msp| {
            let msp = msp.as_mut().expect("MSP should exist.");
            msp.capacity_used += total_bucket_size;
        });

        // Create fixed-rate payment stream between user and MSP
        let value_prop =
            pallet_storage_providers::MainStorageProviderIdsToValuePropositions::<T>::get(
                &msp_id,
                &value_prop_id,
            )
            .unwrap();
        let price_per_giga_unit = value_prop.price_per_giga_unit_of_data_per_block;
        // ZeroSizeBucketFixedRate returns NativeBalance balance type from storage-providers
        let zero_sized_bucket_rate_providers =
            <T as pallet_storage_providers::Config>::ZeroSizeBucketFixedRate::get();
        // Convert to payment streams balance type
        let zero_sized_bucket_rate: PaymentStreamsBalanceOf<T> =
            zero_sized_bucket_rate_providers.into();
        let bucket_size_in_giga_units: u64 = total_bucket_size / (shp_constants::GIGAUNIT as u64);
        // Convert price_per_giga_unit to payment streams balance type and calculate rate
        let price_per_giga_unit_ps: PaymentStreamsBalanceOf<T> = price_per_giga_unit.into();
        let payment_stream_rate = PaymentStreamsBalanceOf::<T>::from(bucket_size_in_giga_units)
            .saturating_mul(price_per_giga_unit_ps)
            .saturating_add(zero_sized_bucket_rate);

        pallet_payment_streams::FixedRatePaymentStreams::<T>::insert(
            &msp_id,
            &user_account,
            pallet_payment_streams::types::FixedRatePaymentStream {
                rate: payment_stream_rate,
                last_charged_tick: frame_system::Pallet::<T>::block_number(),
                user_deposit: PaymentStreamsBalanceOf::<T>::from(0u64),
                out_of_funds_tick: None,
            },
        );

        // Get bucket inclusion forest proof
        let encoded_inclusion_proof = fetch_bucket_inclusion_proofs(number_of_file_keys, 1);
        let forest_proof = <ForestProof<T>>::decode(&mut encoded_inclusion_proof.as_ref())
            .expect("Bucket inclusion proof should be decodable");

        // Get old bucket root for event assertion
        let old_bucket_root_buckets =
            <<T as crate::Config>::Providers as ReadBucketsInterface>::get_root_bucket(&bucket_id)
                .unwrap();
        // Types are constrained to be equal, so no conversion needed
        let old_bucket_root: MerkleHash<T> = old_bucket_root_buckets;

        // Get some variables for comparison after the call
        let previous_msp_capacity_used =
            <<T as crate::Config>::Providers as ReadStorageProvidersInterface>::get_used_capacity(
                &msp_id,
            );
        let previous_bucket_size = pallet_storage_providers::Buckets::<T>::get(&bucket_id)
            .unwrap()
            .size;

        // Sanity check: ensure each deletion request's signed intention file key matches
        // the file key we reconstructed from metadata and are passing to delete_files.
        for (request, expected_file_key) in file_deletions.iter().zip(file_keys_for_deletion.iter())
        {
            assert_eq!(
                request.signed_intention.file_key,
                *expected_file_key,
                "Signed intention file key does not match reconstructed file key for bucket deletion benchmark"
            );
        }

        /*********** Call the extrinsic to benchmark: ***********/
        let file_deletions_bounded: BoundedVec<
            FileDeletionRequest<T>,
            T::MaxFileDeletionsPerExtrinsic,
        > = file_deletions
            .try_into()
            .expect("Should fit in bounded vec");
        let file_keys_bounded: BoundedVec<MerkleHash<T>, T::MaxFileDeletionsPerExtrinsic> =
            file_keys_for_deletion
                .try_into()
                .expect("Should fit in bounded vec");

        #[extrinsic_call]
        delete_files(
            RawOrigin::Signed(user_account.clone()),
            file_deletions_bounded,
            None,
            forest_proof,
        );

        /*********** Post-benchmark checks: ***********/
        // Get new values after calling the extrinsic
        let new_msp_capacity_used =
            <<T as crate::Config>::Providers as ReadStorageProvidersInterface>::get_used_capacity(
                &msp_id,
            );
        let new_bucket_size = pallet_storage_providers::Buckets::<T>::get(&bucket_id)
            .unwrap()
            .size;
        let new_bucket_root_storage = pallet_storage_providers::Buckets::<T>::get(&bucket_id)
            .unwrap()
            .root;
        // Convert to MerkleHash<T> for event and assertions
        let new_bucket_root: MerkleHash<T> = new_bucket_root_storage.into();

        // Ensure the expected event was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::BucketFileDeletionsCompleted {
                user: user_account.clone(),
                file_keys: file_keys_bounded.clone(),
                bucket_id,
                msp_id: Some(msp_id),
                old_root: old_bucket_root,
                new_root: new_bucket_root,
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Ensure MSP capacity decreased by total file size
        assert_eq!(
            new_msp_capacity_used,
            previous_msp_capacity_used - total_bucket_size,
            "MSP capacity used should have decreased by total file size"
        );

        // Ensure bucket size decreased by total file size
        assert_eq!(
            new_bucket_size,
            previous_bucket_size - total_bucket_size,
            "Bucket size should have decreased by total file size"
        );

        // Ensure bucket root was updated
        assert_ne!(
            new_bucket_root, old_bucket_root,
            "Bucket root should have been updated"
        );

        // Ensure payment stream was updated (should be zero-sized bucket rate now)
        let payment_stream_value = <<T as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_inner_fixed_rate_payment_stream_value(&msp_id, &user_account);
        let zero_sized_bucket_rate_providers =
            <T as pallet_storage_providers::Config>::ZeroSizeBucketFixedRate::get();
        let expected_zero_rate: PaymentStreamsBalanceOf<T> =
            zero_sized_bucket_rate_providers.into();
        assert_eq!(
            payment_stream_value,
            Some(expected_zero_rate),
            "Payment stream should be updated to zero-sized bucket rate"
        );

        // Ensure incomplete storage requests were created (since MSP had accepted the storage requests)
        for file_key in file_keys_bounded.iter() {
            assert!(
                crate::IncompleteStorageRequests::<T>::contains_key(file_key),
                "IncompleteStorageRequest should be created for file with active storage request"
            );
        }

        // Ensure storage requests were cleaned up
        for file_key in file_keys_bounded.iter() {
            assert!(
                !StorageRequests::<T>::contains_key(file_key),
                "StorageRequest should be removed after deletion"
            );
        }

        Ok(())
    }

    #[benchmark]
    fn delete_files_bsp(n: Linear<1, 10>) -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Get the number of file keys to delete from the linear parameter
        let number_of_file_keys: u32 = n.into();

        // Get the user account for the generated proofs and load it up with some balance.
        let user_as_bytes: [u8; 32] = get_user_account().clone().try_into().unwrap();
        let user_account: T::AccountId = T::AccountId::decode(&mut &user_as_bytes[..]).unwrap();
        mint_into_account::<T>(user_account.clone(), 1_000_000_000_000_000_000_000)?;

        // Register an account as a MSP (needed for bucket creation)
        let msp_account: T::AccountId = account("MSP", 0, 0);
        mint_into_account::<T>(msp_account.clone(), 1_000_000_000_000_000_000_000)?;
        let encoded_msp_id = get_msp_id();
        let msp_id = <T as frame_system::Config>::Hash::decode(&mut encoded_msp_id.as_ref())
            .expect("Failed to decode provider ID from bytes.");
        let (_, value_prop_id) = add_msp_to_provider_storage::<T>(&msp_account, Some(msp_id));

        // Register the BSP which will have files deleted
        let bsp_account: T::AccountId = account("BSP", 0, 0);
        mint_into_account::<T>(bsp_account.clone(), 1_000_000_000_000_000)?;
        let encoded_bsp_id = get_bsp_id();
        let bsp_id = <T as frame_system::Config>::Hash::decode(&mut encoded_bsp_id.as_ref())
            .expect("Failed to decode provider ID from bytes.");
        add_bsp_to_provider_storage::<T>(&bsp_account, Some(bsp_id));

        // Create a bucket for the user (needed for file ownership validation)
        let encoded_bucket_id = get_bucket_id(1);
        let bucket_id = <T as frame_system::Config>::Hash::decode(&mut encoded_bucket_id.as_ref())
            .expect("Bucket ID should be decodable as it is a hash");
        <<T as crate::Config>::Providers as MutateBucketsInterface>::add_bucket(
            msp_id,
            user_account.clone(),
            bucket_id,
            false,
            None,
            value_prop_id,
        )?;

        // Set BSP root to match the generated proofs
        let encoded_bsp_root = get_bsp_root();
        let bsp_root = <T as frame_system::Config>::Hash::decode(&mut encoded_bsp_root.as_ref())
            .expect("BSP root should be decodable as it is a hash");
        pallet_storage_providers::BackupStorageProviders::<T>::mutate(&bsp_id, |bsp| {
            let bsp = bsp.as_mut().expect("BSP should exist.");
            bsp.root = bsp_root;
        });

        // Get file keys and metadata for deletion
        // We use fetch_file_metadata_for_deletion to get metadata directly, then reconstruct file keys
        // Note: These correspond to stored files in bucket 1 that are also in the BSP's forest
        let mut file_deletions: Vec<FileDeletionRequest<T>> = Vec::new();
        let mut file_keys_for_deletion: Vec<MerkleHash<T>> = Vec::new();
        let mut total_file_size: StorageDataUnit<T> = 0;

        for j in 0..number_of_file_keys {
            // Get file metadata for deletion (location, size, fingerprint)
            let (location_bytes, size, fingerprint_bytes) =
                fetch_file_metadata_for_deletion(number_of_file_keys, 1, j);

            let location: FileLocation<T> = location_bytes
                .try_into()
                .expect("Location bytes should convert to FileLocation");
            let fingerprint =
                <T as frame_system::Config>::Hash::decode(&mut fingerprint_bytes.as_ref())
                    .expect("Fingerprint should be decodable as it is a hash");
            total_file_size += size;

            // Reconstruct file key from metadata
            let file_key = Pallet::<T>::compute_file_key(
                user_account.clone(),
                bucket_id,
                location.clone(),
                size,
                fingerprint.into(),
            )?;
            file_keys_for_deletion.push(file_key);

            // Get deletion signature
            let encoded_signatures = fetch_file_deletion_signatures(number_of_file_keys, 1);
            let signature_bytes = encoded_signatures[j as usize].clone();
            let signature =
                <<T as pallet::Config>::OffchainSignature>::decode(&mut signature_bytes.as_ref())
                    .expect("Deletion signature should be decodable");

            // Create FileOperationIntention
            let signed_intention = FileOperationIntention::<T> {
                file_key,
                operation: FileOperation::Delete,
            };

            // pre-verify signature
            let signed_intention_encoded = signed_intention.encode();
            let to_verify = <T as crate::Config>::IntentionMsgAdapter::bytes_to_verify(
                &signed_intention_encoded,
            );
            let is_valid_pre_check = signature.verify(&to_verify[..], &user_account);
            ensure!(
                is_valid_pre_check,
                BenchmarkError::Stop("Signature should be valid")
            );

            // Create FileDeletionRequest
            file_deletions.push(FileDeletionRequest::<T> {
                file_owner: user_account.clone(),
                signed_intention,
                signature,
                bucket_id,
                location: location.clone(),
                size,
                fingerprint: fingerprint.into(),
            });

            // Set up storage request for this file with active storage request (worst-case scenario).
            // This ensures the benchmark captures the cost of cleanup_storage_request and add_incomplete_storage_request.
            let expiration_tick = frame_system::Pallet::<T>::block_number() + 100u32.into();
            let deposit: BalanceOf<T> = T::BaseStorageRequestCreationDeposit::get();

            let storage_request_metadata = StorageRequestMetadata::<T> {
                requested_at: frame_system::Pallet::<T>::block_number(),
                expires_at: expiration_tick,
                owner: user_account.clone(),
                bucket_id,
                location: location.clone(),
                fingerprint: fingerprint.into(),
                size,
                msp: Some((msp_id, true)), // MSP accepted triggers incomplete storage request creation
                user_peer_ids: Default::default(),
                bsps_required: ReplicationTargetType::<T>::one(),
                bsps_confirmed: ReplicationTargetType::<T>::one(),
                bsps_volunteered: ReplicationTargetType::<T>::one(),
                deposit_paid: deposit,
            };

            <T as pallet::Config>::Currency::hold(
                &HoldReason::StorageRequestCreationHold.into(),
                &user_account,
                deposit,
            )?;

            <StorageRequests<T>>::insert(&file_key, storage_request_metadata);
            <BucketsWithStorageRequests<T>>::insert(&bucket_id, &file_key, ());

            <StorageRequestExpirations<T>>::mutate(expiration_tick, |items| {
                let _ = items.try_push(file_key);
            });

            // Add the BSP being deleted so incomplete storage request persists after BSP removal
            StorageRequestBsps::<T>::insert(
                &file_key,
                &bsp_id,
                StorageRequestBspsMetadata::<T> {
                    confirmed: true,
                    _phantom: Default::default(),
                },
            );
        }

        // Increase BSP used capacity to match file size
        pallet_storage_providers::BackupStorageProviders::<T>::mutate(&bsp_id, |bsp| {
            let bsp = bsp.as_mut().expect("BSP should exist.");
            bsp.capacity_used += total_file_size;
        });

        // Create dynamic-rate payment stream between user and BSP
        let amount_provided = total_file_size;
        pallet_payment_streams::DynamicRatePaymentStreams::<T>::insert(
            &bsp_id,
            &user_account,
            DynamicRatePaymentStream {
                amount_provided: <T as pallet_payment_streams::Config>::Units::from(
                    amount_provided,
                ),
                price_index_when_last_charged: PaymentStreamsBalanceOf::<T>::from(0u64),
                user_deposit: PaymentStreamsBalanceOf::<T>::from(100u64),
                out_of_funds_tick: None,
            },
        );

        // Set the used capacity to the total capacity to simulate worst-case scenario
        pallet_storage_providers::UsedBspsCapacity::<T>::set(total_file_size);
        pallet_storage_providers::TotalBspsCapacity::<T>::set(total_file_size);

        // Get BSP inclusion forest proof
        let encoded_inclusion_proof = fetch_bsp_inclusion_proofs(number_of_file_keys);
        let forest_proof = <ForestProof<T>>::decode(&mut encoded_inclusion_proof.as_ref())
            .expect("BSP inclusion proof should be decodable");

        // Get old BSP root for event assertion
        let old_bsp_root_providers =
            <<T as crate::Config>::Providers as ReadProvidersInterface>::get_root(bsp_id).unwrap();
        // Convert to MerkleHash<T> for event
        let old_bsp_root: MerkleHash<T> = old_bsp_root_providers.into();

        // Get some variables for comparison after the call
        let previous_bsp_capacity_used =
            <<T as crate::Config>::Providers as ReadStorageProvidersInterface>::get_used_capacity(
                &bsp_id,
            );

        // Sanity check: ensure each deletion request's signed intention file key matches
        // the file key we reconstructed from metadata and are passing to delete_files.
        for (request, expected_file_key) in file_deletions.iter().zip(file_keys_for_deletion.iter())
        {
            assert_eq!(
                request.signed_intention.file_key,
                *expected_file_key,
                "Signed intention file key does not match reconstructed file key for BSP deletion benchmark"
            );
        }

        /*********** Call the extrinsic to benchmark: ***********/
        let file_deletions_bounded: BoundedVec<
            FileDeletionRequest<T>,
            T::MaxFileDeletionsPerExtrinsic,
        > = file_deletions
            .try_into()
            .expect("Should fit in bounded vec");
        let file_keys_bounded: BoundedVec<MerkleHash<T>, T::MaxFileDeletionsPerExtrinsic> =
            file_keys_for_deletion
                .try_into()
                .expect("Should fit in bounded vec");
        let users_bounded: BoundedVec<T::AccountId, T::MaxFileDeletionsPerExtrinsic> =
            vec![user_account.clone(); number_of_file_keys as usize]
                .try_into()
                .expect("Should fit in bounded vec");

        #[extrinsic_call]
        delete_files(
            RawOrigin::Signed(user_account.clone()),
            file_deletions_bounded,
            Some(bsp_id),
            forest_proof,
        );

        /*********** Post-benchmark checks: ***********/
        // Get new values after calling the extrinsic
        let new_bsp_capacity_used =
            <<T as crate::Config>::Providers as ReadStorageProvidersInterface>::get_used_capacity(
                &bsp_id,
            );
        let new_bsp_root_providers =
            <<T as crate::Config>::Providers as ReadProvidersInterface>::get_root(bsp_id).unwrap();
        // Convert to MerkleHash<T> for event and assertions
        let new_bsp_root: MerkleHash<T> = new_bsp_root_providers.into();

        // Ensure the expected event was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::BspFileDeletionsCompleted {
                users: users_bounded,
                file_keys: file_keys_bounded.clone(),
                bsp_id,
                old_root: old_bsp_root,
                new_root: new_bsp_root,
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Ensure BSP capacity decreased by total file size
        assert_eq!(
            new_bsp_capacity_used,
            previous_bsp_capacity_used - total_file_size,
            "BSP capacity used should have decreased by total file size"
        );

        // Ensure BSP root was updated
        assert_ne!(
            new_bsp_root, old_bsp_root,
            "BSP root should have been updated"
        );

        // Ensure dynamic-rate payment stream was removed
        assert!(
            !pallet_payment_streams::DynamicRatePaymentStreams::<T>::contains_key(
                &bsp_id,
                &user_account
            ),
            "Dynamic-rate payment stream should be removed after deletion"
        );

        // Ensure incomplete storage requests were created (since MSP had accepted the storage requests)
        for file_key in file_keys_bounded.iter() {
            assert!(
                crate::IncompleteStorageRequests::<T>::contains_key(file_key),
                "IncompleteStorageRequest should be created for file with active storage request"
            );
        }

        // Ensure storage requests were cleaned up
        for file_key in file_keys_bounded.iter() {
            assert!(
                !StorageRequests::<T>::contains_key(file_key),
                "StorageRequest should be removed after deletion"
            );
        }

        Ok(())
    }

    #[benchmark]
    fn delete_files_for_incomplete_storage_request_bucket(
        n: Linear<1, 10>,
    ) -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Get the number of file keys to delete from the linear parameter
        let number_of_file_keys: u32 = n.into();

        // Get the user account for the generated proofs and load it up with some balance.
        let user_as_bytes: [u8; 32] = get_user_account().clone().try_into().unwrap();
        let user_account: T::AccountId = T::AccountId::decode(&mut &user_as_bytes[..]).unwrap();
        mint_into_account::<T>(user_account.clone(), 1_000_000_000_000_000_000_000)?;

        // Register an account as a MSP with the specific MSP ID from the generated proofs
        let msp_account: T::AccountId = account("MSP", 0, 0);
        mint_into_account::<T>(msp_account.clone(), 1_000_000_000_000_000_000_000)?;
        let encoded_msp_id = get_msp_id();
        let msp_id = <T as frame_system::Config>::Hash::decode(&mut encoded_msp_id.as_ref())
            .expect("Failed to decode provider ID from bytes.");
        let (_, value_prop_id) = add_msp_to_provider_storage::<T>(&msp_account, Some(msp_id));

        // Create the bucket to store in the MSP (bucket 1)
        let encoded_bucket_id = get_bucket_id(1);
        let bucket_id = <T as frame_system::Config>::Hash::decode(&mut encoded_bucket_id.as_ref())
            .expect("Bucket ID should be decodable as it is a hash");
        <<T as crate::Config>::Providers as MutateBucketsInterface>::add_bucket(
            msp_id,
            user_account.clone(),
            bucket_id,
            false,
            None,
            value_prop_id,
        )?;

        // Update the bucket's root to match the generated proofs
        let encoded_bucket_root = get_bucket_root(1);
        let bucket_root =
            <T as frame_system::Config>::Hash::decode(&mut encoded_bucket_root.as_ref())
                .expect("Bucket root should be decodable as it is a hash");
        let mut total_bucket_size: StorageDataUnit<T> = 0;

        // Get file keys and metadata for deletion
        // We use fetch_file_metadata_for_deletion to get metadata directly, then reconstruct file keys
        // Note: These correspond to stored files in bucket 1 (same as bucket inclusion proofs)
        let mut file_keys_for_deletion: Vec<MerkleHash<T>> = Vec::new();

        for j in 0..number_of_file_keys {
            // Get file metadata for deletion (location, size, fingerprint)
            let (location_bytes, size, fingerprint_bytes) =
                fetch_file_metadata_for_deletion(number_of_file_keys, 1, j);

            let location: FileLocation<T> = location_bytes
                .try_into()
                .expect("Location bytes should convert to FileLocation");
            let fingerprint =
                <T as frame_system::Config>::Hash::decode(&mut fingerprint_bytes.as_ref())
                    .expect("Fingerprint should be decodable as it is a hash");
            total_bucket_size += size;

            // Reconstruct file key from metadata
            let file_key = Pallet::<T>::compute_file_key(
                user_account.clone(),
                bucket_id,
                location.clone(),
                size,
                fingerprint.into(),
            )?;
            file_keys_for_deletion.push(file_key);

            // Create IncompleteStorageRequestMetadata for bucket deletion
            let incomplete_metadata = IncompleteStorageRequestMetadata::<T> {
                owner: user_account.clone(),
                bucket_id,
                location: location.clone(),
                file_size: size,
                fingerprint: fingerprint.into(),
                pending_bsp_removals: BoundedVec::default(),
                pending_bucket_removal: true,
            };

            // Insert into IncompleteStorageRequests storage map
            crate::IncompleteStorageRequests::<T>::insert(&file_key, incomplete_metadata);
        }

        // Update bucket size to match total
        pallet_storage_providers::Buckets::<T>::mutate(&bucket_id, |bucket| {
            let bucket = bucket.as_mut().expect("Bucket should exist.");
            bucket.size = total_bucket_size;
            bucket.root = bucket_root;
        });

        // Increase MSP used capacity to match bucket size
        pallet_storage_providers::MainStorageProviders::<T>::mutate(&msp_id, |msp| {
            let msp = msp.as_mut().expect("MSP should exist.");
            msp.capacity_used += total_bucket_size;
        });

        // Create fixed-rate payment stream between user and MSP
        let value_prop =
            pallet_storage_providers::MainStorageProviderIdsToValuePropositions::<T>::get(
                &msp_id,
                &value_prop_id,
            )
            .unwrap();
        let price_per_giga_unit = value_prop.price_per_giga_unit_of_data_per_block;
        // ZeroSizeBucketFixedRate returns NativeBalance balance type from storage-providers
        let zero_sized_bucket_rate_providers =
            <T as pallet_storage_providers::Config>::ZeroSizeBucketFixedRate::get();
        // Convert to payment streams balance type
        let zero_sized_bucket_rate: PaymentStreamsBalanceOf<T> =
            zero_sized_bucket_rate_providers.into();
        let bucket_size_in_giga_units: u64 = total_bucket_size / (shp_constants::GIGAUNIT as u64);
        // Convert price_per_giga_unit to payment streams balance type and calculate rate
        let price_per_giga_unit_ps: PaymentStreamsBalanceOf<T> = price_per_giga_unit.into();
        let payment_stream_rate = PaymentStreamsBalanceOf::<T>::from(bucket_size_in_giga_units)
            .saturating_mul(price_per_giga_unit_ps)
            .saturating_add(zero_sized_bucket_rate);

        pallet_payment_streams::FixedRatePaymentStreams::<T>::insert(
            &msp_id,
            &user_account,
            pallet_payment_streams::types::FixedRatePaymentStream {
                rate: payment_stream_rate,
                last_charged_tick: frame_system::Pallet::<T>::block_number(),
                user_deposit: PaymentStreamsBalanceOf::<T>::from(0u64),
                out_of_funds_tick: None,
            },
        );

        // Get bucket inclusion forest proof
        let encoded_inclusion_proof = fetch_bucket_inclusion_proofs(number_of_file_keys, 1);
        let forest_proof = <ForestProof<T>>::decode(&mut encoded_inclusion_proof.as_ref())
            .expect("Bucket inclusion proof should be decodable");

        // Get old bucket root for event assertion
        let old_bucket_root_buckets =
            <<T as crate::Config>::Providers as ReadBucketsInterface>::get_root_bucket(&bucket_id)
                .unwrap();
        // Types are constrained to be equal, so no conversion needed
        let old_bucket_root: MerkleHash<T> = old_bucket_root_buckets;

        // Get some variables for comparison after the call
        let previous_msp_capacity_used =
            <<T as crate::Config>::Providers as ReadStorageProvidersInterface>::get_used_capacity(
                &msp_id,
            );
        let previous_bucket_size = pallet_storage_providers::Buckets::<T>::get(&bucket_id)
            .unwrap()
            .size;

        /*********** Call the extrinsic to benchmark: ***********/
        // Clone file_keys_for_deletion before converting to BoundedVec so we can use it later
        let file_keys_for_deletion_clone = file_keys_for_deletion.clone();
        let file_keys_bounded: BoundedVec<MerkleHash<T>, T::MaxFileDeletionsPerExtrinsic> =
            file_keys_for_deletion
                .try_into()
                .expect("Should fit in bounded vec");
        // Clone file_keys_bounded before passing to extrinsic call so we can use it in event assertion
        let file_keys_bounded_clone = file_keys_bounded.clone();

        #[extrinsic_call]
        delete_files_for_incomplete_storage_request(
            RawOrigin::Signed(user_account.clone()),
            file_keys_bounded,
            None,
            forest_proof,
        );

        /*********** Post-benchmark checks: ***********/
        // Get new values after calling the extrinsic
        let new_msp_capacity_used =
            <<T as crate::Config>::Providers as ReadStorageProvidersInterface>::get_used_capacity(
                &msp_id,
            );
        let new_bucket_size = pallet_storage_providers::Buckets::<T>::get(&bucket_id)
            .unwrap()
            .size;
        let new_bucket_root_storage = pallet_storage_providers::Buckets::<T>::get(&bucket_id)
            .unwrap()
            .root;
        // Convert to MerkleHash<T> for event and assertions
        let new_bucket_root: MerkleHash<T> = new_bucket_root_storage.into();

        // Ensure the expected event was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::BucketFileDeletionsCompleted {
                user: user_account.clone(),
                file_keys: file_keys_bounded_clone,
                bucket_id,
                msp_id: Some(msp_id),
                old_root: old_bucket_root,
                new_root: new_bucket_root,
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Ensure MSP capacity decreased by total file size
        assert_eq!(
            new_msp_capacity_used,
            previous_msp_capacity_used - total_bucket_size,
            "MSP capacity used should have decreased by total file size"
        );

        // Ensure bucket size decreased by total file size
        assert_eq!(
            new_bucket_size,
            previous_bucket_size - total_bucket_size,
            "Bucket size should have decreased by total file size"
        );

        // Ensure bucket root was updated
        assert_ne!(
            new_bucket_root, old_bucket_root,
            "Bucket root should have been updated"
        );

        // Ensure payment stream was updated (should be zero-sized bucket rate now)
        let payment_stream_value = <<T as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_inner_fixed_rate_payment_stream_value(&msp_id, &user_account);
        let zero_sized_bucket_rate_providers =
            <T as pallet_storage_providers::Config>::ZeroSizeBucketFixedRate::get();
        let expected_zero_rate: PaymentStreamsBalanceOf<T> =
            zero_sized_bucket_rate_providers.into();
        assert_eq!(
            payment_stream_value,
            Some(expected_zero_rate),
            "Payment stream should be updated to zero-sized bucket rate"
        );

        // Ensure IncompleteStorageRequests entries were removed (since pending_bucket_removal becomes false and pending_bsp_removals is empty)
        for file_key in file_keys_for_deletion_clone.iter() {
            assert!(
                !crate::IncompleteStorageRequests::<T>::contains_key(file_key),
                "IncompleteStorageRequest should be removed after deletion"
            );
        }

        Ok(())
    }

    #[benchmark]
    fn delete_files_for_incomplete_storage_request_bsp(
        n: Linear<1, 10>,
    ) -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Get the number of file keys to delete from the linear parameter
        let number_of_file_keys: u32 = n.into();

        // Get the user account for the generated proofs and load it up with some balance.
        let user_as_bytes: [u8; 32] = get_user_account().clone().try_into().unwrap();
        let user_account: T::AccountId = T::AccountId::decode(&mut &user_as_bytes[..]).unwrap();
        mint_into_account::<T>(user_account.clone(), 1_000_000_000_000_000_000_000)?;

        // Register an account as a MSP (needed for bucket creation)
        let msp_account: T::AccountId = account("MSP", 0, 0);
        mint_into_account::<T>(msp_account.clone(), 1_000_000_000_000_000_000_000)?;
        let encoded_msp_id = get_msp_id();
        let msp_id = <T as frame_system::Config>::Hash::decode(&mut encoded_msp_id.as_ref())
            .expect("Failed to decode provider ID from bytes.");
        let (_, value_prop_id) = add_msp_to_provider_storage::<T>(&msp_account, Some(msp_id));

        // Register the BSP which will have files deleted
        let bsp_account: T::AccountId = account("BSP", 0, 0);
        mint_into_account::<T>(bsp_account.clone(), 1_000_000_000_000_000)?;
        let encoded_bsp_id = get_bsp_id();
        let bsp_id = <T as frame_system::Config>::Hash::decode(&mut encoded_bsp_id.as_ref())
            .expect("Failed to decode provider ID from bytes.");
        add_bsp_to_provider_storage::<T>(&bsp_account, Some(bsp_id));

        // Create a bucket for the user (needed for file ownership validation)
        let encoded_bucket_id = get_bucket_id(1);
        let bucket_id = <T as frame_system::Config>::Hash::decode(&mut encoded_bucket_id.as_ref())
            .expect("Bucket ID should be decodable as it is a hash");
        <<T as crate::Config>::Providers as MutateBucketsInterface>::add_bucket(
            msp_id,
            user_account.clone(),
            bucket_id,
            false,
            None,
            value_prop_id,
        )?;

        // Set BSP root to match the generated proofs
        let encoded_bsp_root = get_bsp_root();
        let bsp_root = <T as frame_system::Config>::Hash::decode(&mut encoded_bsp_root.as_ref())
            .expect("BSP root should be decodable as it is a hash");
        pallet_storage_providers::BackupStorageProviders::<T>::mutate(&bsp_id, |bsp| {
            let bsp = bsp.as_mut().expect("BSP should exist.");
            bsp.root = bsp_root;
        });

        // Get file keys and metadata for deletion
        // We use fetch_file_metadata_for_deletion to get metadata directly, then reconstruct file keys
        // Note: These correspond to stored files in bucket 1 that are also in the BSP's forest
        let mut file_keys_for_deletion: Vec<MerkleHash<T>> = Vec::new();
        let mut total_file_size: StorageDataUnit<T> = 0;

        for j in 0..number_of_file_keys {
            // Get file metadata for deletion (location, size, fingerprint)
            let (location_bytes, size, fingerprint_bytes) =
                fetch_file_metadata_for_deletion(number_of_file_keys, 1, j);

            let location: FileLocation<T> = location_bytes
                .try_into()
                .expect("Location bytes should convert to FileLocation");
            let fingerprint =
                <T as frame_system::Config>::Hash::decode(&mut fingerprint_bytes.as_ref())
                    .expect("Fingerprint should be decodable as it is a hash");
            total_file_size += size;

            // Reconstruct file key from metadata
            let file_key = Pallet::<T>::compute_file_key(
                user_account.clone(),
                bucket_id,
                location.clone(),
                size,
                fingerprint.into(),
            )?;
            file_keys_for_deletion.push(file_key);

            // Create IncompleteStorageRequestMetadata for BSP deletion
            let pending_bsp_removals: BoundedVec<ProviderIdFor<T>, MaxReplicationTarget<T>> =
                vec![bsp_id].try_into().expect("Should fit in bounded vec");
            let incomplete_metadata = IncompleteStorageRequestMetadata::<T> {
                owner: user_account.clone(),
                bucket_id,
                location: location.clone(),
                file_size: size,
                fingerprint: fingerprint.into(),
                pending_bsp_removals,
                pending_bucket_removal: false,
            };

            // Insert into IncompleteStorageRequests storage map
            crate::IncompleteStorageRequests::<T>::insert(&file_key, incomplete_metadata);
        }

        // Increase BSP used capacity to match file size
        pallet_storage_providers::BackupStorageProviders::<T>::mutate(&bsp_id, |bsp| {
            let bsp = bsp.as_mut().expect("BSP should exist.");
            bsp.capacity_used += total_file_size;
        });

        // Create dynamic-rate payment stream between user and BSP
        let amount_provided = total_file_size;
        pallet_payment_streams::DynamicRatePaymentStreams::<T>::insert(
            &bsp_id,
            &user_account,
            DynamicRatePaymentStream {
                amount_provided: <T as pallet_payment_streams::Config>::Units::from(
                    amount_provided,
                ),
                price_index_when_last_charged: PaymentStreamsBalanceOf::<T>::from(0u64),
                user_deposit: PaymentStreamsBalanceOf::<T>::from(100u64),
                out_of_funds_tick: None,
            },
        );

        // Set the used capacity to the total capacity to simulate worst-case scenario
        pallet_storage_providers::UsedBspsCapacity::<T>::set(total_file_size);
        pallet_storage_providers::TotalBspsCapacity::<T>::set(total_file_size);

        // Get BSP inclusion forest proof
        let encoded_inclusion_proof = fetch_bsp_inclusion_proofs(number_of_file_keys);
        let forest_proof = <ForestProof<T>>::decode(&mut encoded_inclusion_proof.as_ref())
            .expect("BSP inclusion proof should be decodable");

        // Get old BSP root for event assertion
        let old_bsp_root_providers =
            <<T as crate::Config>::Providers as ReadProvidersInterface>::get_root(bsp_id).unwrap();
        // Convert to MerkleHash<T> for event
        let old_bsp_root: MerkleHash<T> = old_bsp_root_providers.into();

        // Get some variables for comparison after the call
        let previous_bsp_capacity_used =
            <<T as crate::Config>::Providers as ReadStorageProvidersInterface>::get_used_capacity(
                &bsp_id,
            );

        /*********** Call the extrinsic to benchmark: ***********/
        // Clone file_keys_for_deletion before converting to BoundedVec so we can use it later
        let file_keys_for_deletion_clone = file_keys_for_deletion.clone();
        let file_keys_bounded: BoundedVec<MerkleHash<T>, T::MaxFileDeletionsPerExtrinsic> =
            file_keys_for_deletion
                .try_into()
                .expect("Should fit in bounded vec");
        // Clone file_keys_bounded before passing to extrinsic call so we can use it in event assertion
        let file_keys_bounded_clone = file_keys_bounded.clone();
        let users_bounded: BoundedVec<T::AccountId, T::MaxFileDeletionsPerExtrinsic> =
            vec![user_account.clone(); number_of_file_keys as usize]
                .try_into()
                .expect("Should fit in bounded vec");

        #[extrinsic_call]
        delete_files_for_incomplete_storage_request(
            RawOrigin::Signed(user_account.clone()),
            file_keys_bounded,
            Some(bsp_id),
            forest_proof,
        );

        /*********** Post-benchmark checks: ***********/
        // Get new values after calling the extrinsic
        let new_bsp_capacity_used =
            <<T as crate::Config>::Providers as ReadStorageProvidersInterface>::get_used_capacity(
                &bsp_id,
            );
        let new_bsp_root_providers =
            <<T as crate::Config>::Providers as ReadProvidersInterface>::get_root(bsp_id).unwrap();
        // Convert to MerkleHash<T> for event and assertions
        let new_bsp_root: MerkleHash<T> = new_bsp_root_providers.into();

        // Ensure the expected event was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::BspFileDeletionsCompleted {
                users: users_bounded,
                file_keys: file_keys_bounded_clone,
                bsp_id,
                old_root: old_bsp_root,
                new_root: new_bsp_root,
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Ensure BSP capacity decreased by total file size
        assert_eq!(
            new_bsp_capacity_used,
            previous_bsp_capacity_used - total_file_size,
            "BSP capacity used should have decreased by total file size"
        );

        // Ensure BSP root was updated
        assert_ne!(
            new_bsp_root, old_bsp_root,
            "BSP root should have been updated"
        );

        // Ensure dynamic-rate payment stream was removed
        assert!(
            !pallet_payment_streams::DynamicRatePaymentStreams::<T>::contains_key(
                &bsp_id,
                &user_account
            ),
            "Dynamic-rate payment stream should be removed after deletion"
        );

        // Ensure IncompleteStorageRequests entries were removed
        for file_key in file_keys_for_deletion_clone.iter() {
            assert!(
                !crate::IncompleteStorageRequests::<T>::contains_key(file_key),
                "IncompleteStorageRequest should be removed after deletion"
            );
        }

        Ok(())
    }
}
