use super::{types::*, *};
use frame_benchmarking::v2::*;
use frame_system::pallet_prelude::BlockNumberFor;
use sp_runtime::traits::One;

#[benchmarks(where
    T: crate::Config<Fingerprint = <T as frame_system::Config>::Hash, Providers = pallet_storage_providers::Pallet<T>>
  		+ pallet_storage_providers::Config<
  			ProviderId = <T as frame_system::Config>::Hash,
  			StorageDataUnit = u64
  		>,
    <T as crate::Config>::Providers: shp_traits::MutateStorageProvidersInterface<StorageDataUnit = u64>
        + shp_traits::ReadProvidersInterface<ProviderId = <T as frame_system::Config>::Hash>,
    // Ensure the ValuePropId from our Providers trait matches that from pallet_storage_providers:
    <T as crate::Config>::Providers: shp_traits::ReadBucketsInterface<AccountId = <T as frame_system::Config>::AccountId, ProviderId = <T as frame_system::Config>::Hash> + shp_traits::MutateBucketsInterface<ValuePropId = <T as pallet_storage_providers::Config>::ValuePropId>,
	<T as crate::Config>::ProofDealer: shp_traits::ProofsDealerInterface<TickNumber = BlockNumberFor<T>>,
)]
mod benchmarks {
    use super::*;
    use frame_support::{
        assert_ok,
        traits::{fungible::Mutate, Get},
        BoundedVec,
    };
    use frame_system::RawOrigin;
    use pallet_storage_providers::types::ValueProposition;
    use shp_traits::ReadBucketsInterface;
    use sp_core::Hasher;
    use sp_runtime::traits::Hash;
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
            Some(msp_id),
            name,
            true,
            Some(value_prop_id),
        );

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
            Some(msp_id),
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
            Some(msp_id),
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
            Some(msp_id),
            name,
            true,
            Some(value_prop_id),
        )?;

        #[extrinsic_call]
        _(RawOrigin::Signed(msp), bucket_id);

        Ok(())
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
