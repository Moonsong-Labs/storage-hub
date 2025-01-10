use super::{types::*, *};
use frame_benchmarking::v2::*;

#[benchmarks(where
    T: crate::Config<Fingerprint = <T as frame_system::Config>::Hash, Providers = pallet_storage_providers::Pallet<T>>
		+ pallet_storage_providers::Config<
			ProviderId = <T as frame_system::Config>::Hash,
			StorageDataUnit = u64
		>
		+ pallet_nfts::Config
		+ pallet_proofs_dealer::Config,
    <T as crate::Config>::Providers: shp_traits::MutateStorageProvidersInterface<StorageDataUnit = u64>
        + shp_traits::ReadProvidersInterface<ProviderId = <T as frame_system::Config>::Hash>,
    // Ensure the ValuePropId from our Providers trait matches that from pallet_storage_providers:
    <T as crate::Config>::Providers: shp_traits::ReadBucketsInterface<AccountId = <T as frame_system::Config>::AccountId, ProviderId = <T as frame_system::Config>::Hash, ReadAccessGroupId = <T as pallet_nfts::Config>::CollectionId> + shp_traits::MutateBucketsInterface<ValuePropId = <T as pallet_storage_providers::Config>::ValuePropId>,
	<T as crate::Config>::ProofDealer: shp_traits::ProofsDealerInterface<TickNumber = BlockNumberFor<T>>,
	<T as crate::Config>::Nfts: frame_support::traits::nonfungibles_v2::Inspect<<T as frame_system::Config>::AccountId, CollectionId = <T as pallet_nfts::Config>::CollectionId>,
)]
mod benchmarks {
    use super::*;
    use frame_support::{
        assert_ok,
        traits::{fungible::Mutate, Get, OnPoll},
        weights::WeightMeter,
        BoundedVec,
    };
    use frame_system::{pallet_prelude::BlockNumberFor, RawOrigin};
    use pallet_file_system_runtime_api::QueryFileEarliestVolunteerTickError;
    use pallet_storage_providers::types::ValueProposition;
    use shp_traits::{ReadBucketsInterface, ReadStorageProvidersInterface};
    use sp_core::Hasher;
    use sp_runtime::traits::{Hash, One};
    use sp_std::vec;

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
        let (msp_id, value_prop_id) = add_msp_to_provider_storage::<T>(&msp);

        #[extrinsic_call]
        _(
            signed_origin.clone(),
            msp_id,
            name,
            true,
            Some(value_prop_id),
        );

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
            add_msp_to_provider_storage::<T>(&initial_msp_account);

        // Register another MSP with a value proposition
        let new_msp_account: T::AccountId = account("MSP", 0, 1);
        mint_into_account::<T>(new_msp_account.clone(), 1_000_000_000_000_000)?;
        let (new_msp_id, _) = add_msp_to_provider_storage::<T>(&new_msp_account);

        // Create the bucket, assigning it to the initial MSP
        Pallet::<T>::create_bucket(
            signed_origin.clone().into(),
            initial_msp_id,
            name,
            true,
            Some(initial_value_prop_id),
        )?;

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(signed_origin, bucket_id, new_msp_id);

        /*********** Post-benchmark checks: ***********/
        // Ensure the PendingMoveBucketRequests storage has the created request
        let pending_move_bucket_request =
            PendingMoveBucketRequests::<T>::get(&new_msp_id, &bucket_id);
        assert!(pending_move_bucket_request.is_some());
        assert_eq!(pending_move_bucket_request.unwrap().requester, user.clone());

        // Ensure the PendingBucketsToMove storage has the bucket
        assert!(PendingBucketsToMove::<T>::contains_key(&bucket_id));

        // Ensure the expected event was emitted.
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::MoveBucketRequested {
                who: user,
                bucket_id,
                new_msp_id,
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
            add_msp_to_provider_storage::<T>(&initial_msp_account);

        // Register another MSP with a value proposition
        let new_msp_account: T::AccountId = account("MSP", 0, 1);
        mint_into_account::<T>(new_msp_account.clone(), 1_000_000_000_000_000)?;
        let (new_msp_id, _) = add_msp_to_provider_storage::<T>(&new_msp_account);

        // Create the bucket, assigning it to the initial MSP
        Pallet::<T>::create_bucket(
            signed_origin.clone().into(),
            initial_msp_id,
            name,
            true,
            Some(initial_value_prop_id),
        )?;

        // Request the move of the bucket to the new MSP
        Pallet::<T>::request_move_bucket(signed_origin.clone().into(), bucket_id, new_msp_id)?;

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
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::MoveBucketAccepted {
                bucket_id,
                msp_id: new_msp_id,
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
        let (msp_id, value_prop_id) = add_msp_to_provider_storage::<T>(&msp_account);

        // Create the bucket as private, creating the collection
        Pallet::<T>::create_bucket(
            signed_origin.clone().into(),
            msp_id,
            name,
            true,
            Some(value_prop_id),
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
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::BucketPrivacyUpdated {
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
        let (msp_id, value_prop_id) = add_msp_to_provider_storage::<T>(&msp_account);

        // Create the bucket as private, creating the collection
        Pallet::<T>::create_bucket(
            signed_origin.clone().into(),
            msp_id,
            name,
            true,
            Some(value_prop_id),
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
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::NewCollectionAndAssociation {
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
        let (msp_id, value_prop_id) = add_msp_to_provider_storage::<T>(&msp_account);

        // Create the bucket as private, creating the collection so it has to be deleted as well.
        Pallet::<T>::create_bucket(
            signed_origin.clone().into(),
            msp_id,
            name,
            true,
            Some(value_prop_id),
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
        let expected_event = <T as pallet::Config>::RuntimeEvent::from(Event::<T>::BucketDeleted {
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
        let size: StorageData<T> = 100;
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
        let (msp_id, value_prop_id) = add_msp_to_provider_storage::<T>(&msp);

        Pallet::<T>::create_bucket(
            signed_origin.clone().into(),
            msp_id,
            name,
            true,
            Some(value_prop_id),
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
            None,
        );

        Ok(())
    }

    #[benchmark]
    fn revoke_storage_request(
        n: Linear<
            1,
            {
                Into::<u64>::into(MaxReplicationTarget::<T>::get())
                    .try_into()
                    .unwrap()
            },
        >,
    ) -> Result<(), BenchmarkError> {
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
        let size: StorageData<T> = 100;
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
        let (msp_id, value_prop_id) = add_msp_to_provider_storage::<T>(&msp);

        Pallet::<T>::create_bucket(
            signed_origin.clone().into(),
            msp_id,
            name,
            true,
            Some(value_prop_id),
        )?;

        Pallet::<T>::issue_storage_request(
            signed_origin.clone().into(),
            bucket_id,
            location.clone(),
            fingerprint,
            size,
            msp_id,
            peer_ids,
            Some(n.into()),
        )?;

        let file_key = Pallet::<T>::compute_file_key(user, bucket_id, location, size, fingerprint);

        // The `revoke_storage_request` executes the `drain_prefix` function to remove all sub keys including the primary key
        // from `StorageRequestBsps`.
        for i in 0..n {
            let bsp_user: T::AccountId = account("bsp", i as u32, i as u32);
            mint_into_account::<T>(bsp_user.clone(), 1_000_000_000_000_000)?;
            let bsp_id = add_bsp_to_provider_storage::<T>(&bsp_user.clone());

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
        let (msp_id, value_prop_id) = add_msp_to_provider_storage::<T>(&msp);

        Pallet::<T>::create_bucket(
            signed_origin.clone().into(),
            msp_id,
            name,
            true,
            Some(value_prop_id),
        )?;

        #[extrinsic_call]
        _(RawOrigin::Signed(msp), bucket_id);

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
        let (msp_id, value_prop_id) = add_msp_to_provider_storage::<T>(&msp_account);

        // Register the BSP which will volunteer for the file
        let bsp_account: T::AccountId = account("BSP", 0, 0);
        let bsp_signed_origin = RawOrigin::Signed(bsp_account.clone());
        mint_into_account::<T>(bsp_account.clone(), 1_000_000_000_000_000)?;
        let bsp_id = add_bsp_to_provider_storage::<T>(&bsp_account);
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
            Some(value_prop_id),
        )?;

        // Issue the storage request from the user
        let location: FileLocation<T> = vec![1; MaxFilePathSize::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();
        let fingerprint =
            <<T as frame_system::Config>::Hashing as Hasher>::hash(b"benchmark_fingerprint");
        let size: StorageData<T> = 100;
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
            None,
        )?;

        // Compute the file key
        let file_key = Pallet::<T>::compute_file_key(
            user.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        );

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
                QueryFileEarliestVolunteerTickError::InternalError => {
                    return Err(BenchmarkError::Stop("Internal runtime API error."));
                }
            },
            Ok(earliest_volunteer_tick) => earliest_volunteer_tick,
        };

        // Advance the block number to the earliest tick where the BSP can volunteer
        run_to_block::<T>(tick_to_advance_to);

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(bsp_signed_origin, file_key);

        /*********** Post-benchmark checks: ***********/
        // Ensure the BSP has correctly volunteered for the file
        assert!(StorageRequestBsps::<T>::contains_key(&file_key, &bsp_id));

        // Ensure the expected event was emitted.
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::AcceptedBspVolunteer {
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

    // TODO: For the remaining extrinsics, we need a way to generate:
    // - Non-inclusion forest proofs for BSPs that have a decently high forest size (for the `bsp_confirm_storing` benchmark)
    // - Individual file key proofs
    // - Inclusion forest proofs for the aforementioned file keys (for all extrinsics related to file deletions)
    /* #[benchmark]
    fn bsp_confirm_storing() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Get a user account and mint some tokens into it
        let user: T::AccountId = account("Alice", 0, 0);
        let signed_origin = RawOrigin::Signed(user.clone());
        mint_into_account::<T>(user.clone(), 1_000_000_000_000_000)?;

        // Register a MSP with a value proposition
        let msp_account: T::AccountId = account("MSP", 0, 0);
        mint_into_account::<T>(msp_account.clone(), 1_000_000_000_000_000)?;
        let (msp_id, value_prop_id) = add_msp_to_provider_storage::<T>(&msp_account);

        // Register the BSP which will volunteer for the file and confirm storing it
        let bsp_account: T::AccountId = account("BSP", 0, 0);
        let bsp_signed_origin = RawOrigin::Signed(bsp_account.clone());
        mint_into_account::<T>(bsp_account.clone(), 1_000_000_000_000_000)?;
        let bsp_id = add_bsp_to_provider_storage::<T>(&bsp_account);

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
            Some(value_prop_id),
        )?;

        // Issue the storage request from the user
        let location: FileLocation<T> = vec![1; MaxFilePathSize::<T>::get().try_into().unwrap()]
            .try_into()
            .unwrap();
        let fingerprint =
            <<T as frame_system::Config>::Hashing as Hasher>::hash(b"benchmark_fingerprint");
        let size: StorageData<T> = 100;
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
        )?;

        // Compute the file key
        let file_key = Pallet::<T>::compute_file_key(
            user.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        );

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
                QueryFileEarliestVolunteerTickError::InternalError => {
                    return Err(BenchmarkError::Stop("Internal runtime API error."));
                }
            },
            Ok(earliest_volunteer_tick) => earliest_volunteer_tick,
        };

        // Advance the block number to the earliest tick where the BSP can volunteer
        run_to_block::<T>(tick_to_advance_to);

        // Volunteer for the file
        Pallet::<T>::bsp_volunteer(bsp_signed_origin.clone().into(), file_key)?;

        // Create the non-inclusion forest proof for the file

        // Create the file key proof

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(bsp_signed_origin, file_key);

        /*********** Post-benchmark checks: ***********/
        // Ensure the BSP has correctly confirmed storing the file
        assert!(StorageRequestBsps::<T>::contains_key(&file_key, &bsp_id));

        Ok(())
    } */

    fn run_to_block<T: crate::Config + pallet_proofs_dealer::Config>(n: BlockNumberFor<T>) {
        assert!(
            n > frame_system::Pallet::<T>::block_number(),
            "Cannot go back in time"
        );

        while frame_system::Pallet::<T>::block_number() < n {
            frame_system::Pallet::<T>::set_block_number(
                frame_system::Pallet::<T>::block_number() + One::one(),
            );
            pallet_proofs_dealer::Pallet::<T>::on_poll(
                frame_system::Pallet::<T>::block_number(),
                &mut WeightMeter::new(),
            );
            Pallet::<T>::on_poll(
                frame_system::Pallet::<T>::block_number(),
                &mut WeightMeter::new(),
            );
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

    fn add_msp_to_provider_storage<T>(msp: &T::AccountId) -> (ProviderIdFor<T>, ValuePropId<T>)
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
        let msp_hash = T::Hashing::hash_of(&msp);

        let capacity: StorageData<T> = 1024 * 1024 * 1024;
        let capacity_used: StorageData<T> = 0;

        let msp_info = pallet_storage_providers::types::MainStorageProvider {
            capacity,
            capacity_used,
            multiaddresses: BoundedVec::default(),
            last_capacity_change: frame_system::Pallet::<T>::block_number(),
            owner_account: msp.clone(),
            payment_account: msp.clone(),
            sign_up_block: frame_system::Pallet::<T>::block_number(),
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

        let value_prop_storage: StorageData<T> = 1000;
        // Use One::one() or a conversion that matches the expected balance type:
        let value_prop = ValueProposition::<T>::new(One::one(), commitment, value_prop_storage);
        let value_prop_id = value_prop.derive_id();

        pallet_storage_providers::MainStorageProviderIdsToValuePropositions::<T>::insert(
            msp_hash,
            value_prop_id,
            value_prop,
        );

        (msp_hash, value_prop_id)
    }

    fn add_bsp_to_provider_storage<T>(bsp_account: &T::AccountId) -> ProviderIdFor<T>
    where
        T: crate::Config
            + pallet_storage_providers::Config<
                ProviderId = <T as frame_system::Config>::Hash,
                StorageDataUnit = u64,
            >,
        T: crate::Config<Providers = pallet_storage_providers::Pallet<T>>,
    {
        // Derive the BSP ID from the hash of its account
        let bsp_id = T::Hashing::hash_of(&bsp_account);

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
}
