use super::{types::*, *};
use frame_benchmarking::v2::*;

#[benchmarks(where
    T: crate::Config<Fingerprint = <T as frame_system::Config>::Hash, Providers = pallet_storage_providers::Pallet<T>>
		+ pallet_storage_providers::Config<
			ProviderId = <T as frame_system::Config>::Hash,
			MerklePatriciaRoot = <T as frame_system::Config>::Hash,
			StorageDataUnit = u64
		>
		+ pallet_nfts::Config
		+ pallet_proofs_dealer::Config
		+ pallet_payment_streams::Config,
    <T as crate::Config>::Providers: shp_traits::MutateStorageProvidersInterface<StorageDataUnit = u64>
        + shp_traits::ReadProvidersInterface<ProviderId = <T as frame_system::Config>::Hash, MerkleHash = <T as frame_system::Config>::Hash> + shp_traits::ReadBucketsInterface<BucketId = <T as frame_system::Config>::Hash>,
    // Ensure the ValuePropId from our Providers trait matches that from pallet_storage_providers:
    <T as crate::Config>::Providers: shp_traits::ReadBucketsInterface<AccountId = <T as frame_system::Config>::AccountId, ProviderId = <T as frame_system::Config>::Hash, ReadAccessGroupId = <T as pallet_nfts::Config>::CollectionId>
									+ shp_traits::MutateBucketsInterface<AccountId = <T as frame_system::Config>::AccountId, ProviderId = <T as frame_system::Config>::Hash ,ValuePropId = <T as pallet_storage_providers::Config>::ValuePropId, BucketId = <T as frame_system::Config>::Hash>,
	<T as crate::Config>::ProofDealer: shp_traits::ProofsDealerInterface<TickNumber = BlockNumberFor<T>, MerkleHash = <T as frame_system::Config>::Hash, KeyProof = shp_file_key_verifier::types::FileKeyProof<{shp_constants::H_LENGTH}, {shp_constants::FILE_CHUNK_SIZE}, {shp_constants::FILE_SIZE_TO_CHALLENGES}>>,
	<T as crate::Config>::Nfts: frame_support::traits::nonfungibles_v2::Inspect<<T as frame_system::Config>::AccountId, CollectionId = <T as pallet_nfts::Config>::CollectionId>,
	<T as pallet_payment_streams::Config>::ProvidersPallet: shp_traits::ReadProvidersInterface<ProviderId = <T as frame_system::Config>::Hash>,
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
    use pallet_payment_streams::types::DynamicRatePaymentStream;
    use pallet_storage_providers::types::ValueProposition;
    use shp_traits::{
        MutateBucketsInterface, ProofsDealerInterface, ReadBucketsInterface,
        ReadProvidersInterface, ReadStorageProvidersInterface,
    };
    use sp_core::{Decode, Hasher};
    use sp_runtime::{
        traits::{Hash, One, Zero},
        Saturating,
    };
    use sp_std::{vec, vec::Vec};

    use crate::benchmark_proofs::*;

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
        _(
            signed_origin.clone(),
            Some(msp_id),
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
            add_msp_to_provider_storage::<T>(&initial_msp_account, None);

        // Register another MSP with a value proposition
        let new_msp_account: T::AccountId = account("MSP", 0, 1);
        mint_into_account::<T>(new_msp_account.clone(), 1_000_000_000_000_000)?;
        let (new_msp_id, _) = add_msp_to_provider_storage::<T>(&new_msp_account, None);

        // Create the bucket, assigning it to the initial MSP
        Pallet::<T>::create_bucket(
            signed_origin.clone().into(),
            Some(initial_msp_id),
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
            <T as pallet::Config>::RuntimeEvent::from(Event::MoveBucketRequested {
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
            add_msp_to_provider_storage::<T>(&initial_msp_account, None);

        // Register another MSP with a value proposition
        let new_msp_account: T::AccountId = account("MSP", 0, 1);
        mint_into_account::<T>(new_msp_account.clone(), 1_000_000_000_000_000)?;
        let (new_msp_id, _) = add_msp_to_provider_storage::<T>(&new_msp_account, None);

        // Create the bucket, assigning it to the initial MSP
        Pallet::<T>::create_bucket(
            signed_origin.clone().into(),
            Some(initial_msp_id),
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
        let expected_event = <T as pallet::Config>::RuntimeEvent::from(Event::MoveBucketAccepted {
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
        let (msp_id, value_prop_id) = add_msp_to_provider_storage::<T>(&msp_account, None);

        // Create the bucket as private, creating the collection
        Pallet::<T>::create_bucket(
            signed_origin.clone().into(),
            Some(msp_id),
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
            Some(msp_id),
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
            Some(msp_id),
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
        let (msp_id, value_prop_id) = add_msp_to_provider_storage::<T>(&msp, None);

        Pallet::<T>::create_bucket(
            signed_origin.clone().into(),
            Some(msp_id),
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
            Some(msp_id),
            peer_ids,
            None,
        );

        Ok(())
    }

    #[benchmark]
    fn msp_respond_storage_requests_multiple_buckets(
        n: Linear<1, { T::MaxBatchMspRespondStorageRequests::get() }>,
        m: Linear<1, { T::MaxBatchMspRespondStorageRequests::get() }>,
        l: Linear<1, { T::MaxBatchMspRespondStorageRequests::get() }>,
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

        let mut msp_total_response: StorageRequestMspResponse<T> = BoundedVec::new();
        // For each bucket to accept:
        for i in 1..amount_of_buckets_to_accept + 1 {
            // Create the bucket to store in the MSP
            let encoded_bucket_id = get_bucket_id(i);
            let bucket_id =
                <T as frame_system::Config>::Hash::decode(&mut encoded_bucket_id.as_ref())
                    .expect("Bucket ID should be decodable as it is a hash");
            <<T as crate::Config>::Providers as MutateBucketsInterface>::add_bucket(
                Some(msp_id),
                user_account.clone(),
                bucket_id,
                false,
                None,
                Some(value_prop_id),
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
            let mut file_keys_to_reject: BoundedVec<
                MerkleHash<T>,
                MaxBatchMspRespondStorageRequests<T>,
            > = BoundedVec::new();
            for j in 0..amount_of_file_keys_to_reject_per_bucket {
                let location: FileLocation<T> =
                    vec![j as u8; MaxFilePathSize::<T>::get().try_into().unwrap()]
                        .try_into()
                        .unwrap();
                let fingerprint = <<T as frame_system::Config>::Hashing as Hasher>::hash(
                    b"benchmark_fingerprint",
                );
                let size: StorageData<T> = 100;
                let storage_request_metadata = StorageRequestMetadata::<T> {
                    requested_at:
                        <<T as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick(),
                    owner: user_account.clone(),
                    bucket_id,
                    location: location.clone(),
                    fingerprint,
                    size,
                    msp: Some((msp_id, false)),
                    user_peer_ids: Default::default(),
                    bsps_required: T::DefaultReplicationTarget::get(),
                    bsps_confirmed: ReplicationTargetType::<T>::one(), // One BSP confirmed means the logic to enqueue a priority challenge is executed
                    bsps_volunteered: ReplicationTargetType::<T>::zero(),
                };
                let file_key = Pallet::<T>::compute_file_key(
                    user_account.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                <StorageRequests<T>>::insert(&file_key, storage_request_metadata);

                <BucketsWithStorageRequests<T>>::insert(&bucket_id, &file_key, ());

                file_keys_to_reject
                    .try_push(file_key)
                    .expect("File key amounts is limited by the same value as the bounded vector");
            }
            let reject_vec = file_keys_to_reject
                .iter()
                .map(|file_key| {
                    let reject_reason = RejectedStorageRequestReason::ReachedMaximumCapacity;
                    RejectedStorageRequest {
                        file_key: file_key.clone(),
                        reason: reject_reason,
                    }
                })
                .collect::<Vec<RejectedStorageRequest<T>>>();
            let reject: BoundedVec<
                RejectedStorageRequest<T>,
                MaxBatchMspRespondStorageRequests<T>,
            > = reject_vec
                .try_into()
                .expect("Reject amounts is limited by the same value as the bounded vector");

            // Build the accept response for this bucket:

            // Get the file keys to accept from the generated proofs.
            let mut file_keys_and_proofs: BoundedVec<
                FileKeyWithProof<T>,
                <T as Config>::MaxBatchMspRespondStorageRequests,
            > = BoundedVec::new();
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
                let location = file_key_proof.file_metadata.location.clone();
                let fingerprint_hash = file_key_proof.file_metadata.fingerprint.clone().as_hash();
                let fingerprint =
                    <T as frame_system::Config>::Hash::decode(&mut fingerprint_hash.as_ref())
                        .expect("Fingerprint should be decodable as it is a hash");
                let size = file_key_proof.file_metadata.file_size;
                let storage_request_metadata = StorageRequestMetadata::<T> {
                    requested_at:
                        <<T as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick(),
                    owner: user_account.clone(),
                    bucket_id,
                    location: location.clone().try_into().unwrap(),
                    fingerprint: fingerprint.into(),
                    size,
                    msp: Some((msp_id, false)),
                    user_peer_ids: Default::default(),
                    bsps_required: T::DefaultReplicationTarget::get(),
                    bsps_confirmed: T::DefaultReplicationTarget::get(), // All BSPs confirmed means the logic to delete the storage request is executed
                    bsps_volunteered: ReplicationTargetType::<T>::zero(),
                };
                <StorageRequests<T>>::insert(&file_keys_to_accept[j], storage_request_metadata);
                <BucketsWithStorageRequests<T>>::insert(&bucket_id, &file_keys_to_accept[j], ());

                // Create the FileKeyWithProof object
                let file_key_with_proof = FileKeyWithProof {
                    file_key: file_keys_to_accept[j],
                    proof: file_key_proof,
                };

                // Push it to the file keys and proofs bounded vector
                file_keys_and_proofs
                    .try_push(file_key_with_proof)
                    .expect("File key amounts is limited by the same value as the bounded vector");
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
                non_inclusion_forest_proof,
            };

            // Finally, build the response for this bucket and push it to the responses bounded vector
            let response = StorageRequestMspBucketResponse {
                bucket_id,
                accept: Some(accept),
                reject,
            };

            msp_total_response.try_push(response).expect(
                "Amount of buckets to accept is limited by the same value as the bounded vector",
            );
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
            Some(msp_id),
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
            Some(msp_id),
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
    fn bsp_confirm_storing(
        n: Linear<1, { T::MaxBatchMspRespondStorageRequests::get() }>,
    ) -> Result<(), BenchmarkError> {
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
            Some(msp_id),
            user_account.clone(),
            bucket_id,
            false,
            None,
            Some(value_prop_id),
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
            let location = file_key_proof.file_metadata.location.clone();
            let fingerprint_hash = file_key_proof.file_metadata.fingerprint.clone().as_hash();
            let fingerprint =
                <T as frame_system::Config>::Hash::decode(&mut fingerprint_hash.as_ref())
                    .expect("Fingerprint should be decodable as it is a hash");
            let size = file_key_proof.file_metadata.file_size;
            let storage_request_metadata = StorageRequestMetadata::<T> {
				requested_at:
					<<T as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick(),
				owner: user_account.clone(),
				bucket_id,
				location: location.clone().try_into().unwrap(),
				fingerprint: fingerprint.into(),
				size,
				msp: Some((msp_id, true)), // MSP accepted means the logic to delete the storage request is executed
				user_peer_ids: Default::default(),
				bsps_required: T::DefaultReplicationTarget::get(),
				bsps_confirmed: T::DefaultReplicationTarget::get().saturating_sub(ReplicationTargetType::<T>::one()), // All BSPs confirmed minus one means the logic to delete the storage request is executed
				bsps_volunteered: ReplicationTargetType::<T>::zero(),
			};
            <StorageRequests<T>>::insert(&file_key, storage_request_metadata);
            <BucketsWithStorageRequests<T>>::insert(&bucket_id, &file_key, ());

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
                confirmed_file_keys: file_keys_to_confirm.try_into().unwrap(),
                skipped_file_keys: BoundedVec::default(),
                new_root: new_bsp_root,
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        Ok(())
    }

    //#[benchmark]
    //fn bsp_request_stop_storing() -> Result<(), BenchmarkError> {}

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
        let msp_hash = if msp_id.is_some() {
            msp_id.unwrap()
        } else {
            T::Hashing::hash_of(&msp)
        };

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

        let bucket_data_limit: StorageData<T> = capacity;
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
        let bsp_id = if bsp_id.is_some() {
            bsp_id.unwrap()
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
}
