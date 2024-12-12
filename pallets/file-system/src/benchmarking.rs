use super::{types::*, *};
use frame_benchmarking::v2::*;

#[benchmarks(where
	T: crate::Config<Fingerprint = <T as frame_system::Config>::Hash>,
    T: pallet_storage_providers::Config<ProviderId = <T as frame_system::Config>::Hash>,
    <T as crate::Config>::Providers: shp_traits::MutateStorageProvidersInterface<StorageDataUnit = u64>
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
    use shp_traits::{ReadBucketsInterface, ReadProvidersInterface};
    use sp_core::Hasher;
    use sp_runtime::traits::Hash;
    use sp_std::vec;

    #[benchmark]
    fn create_bucket() -> Result<(), BenchmarkError> {
        // Setup user account
        let user: T::AccountId = account("Alice", 0, 0);
        let signed_origin = RawOrigin::Signed(user.clone());
        mint_into_account::<T>(user.clone(), 1_000_000_000_000_000)?;

        // Setup parameters
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
        let fingerprint = <T as frame_system::Config>::Hashing::hash_of(b"tes");
        let size: StorageData<T> = 100;
        let peer_id = BoundedVec::try_from(vec![1]).unwrap();
        let peer_ids: PeerIds<T> = BoundedVec::try_from(vec![peer_id]).unwrap();

        // Register MSP with value proposition
        let msp: T::AccountId = account("MSP", 0, 0);
        mint_into_account::<T>(msp, 1_000_000_000_000_000)?;
        let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

        // Benchmark create_bucket extrinsic
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
        // Setup user account
        let user: T::AccountId = account("Alice", 0, 0);
        let signed_origin = RawOrigin::Signed(user.clone());
        mint_into_account::<T>(user.clone(), 1_000_000_000_000_000)?;

        // Setup parameters
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
        let fingerprint = <<T as frame_system::Config>::Hashing as Hasher>::hash(b"tes");
        let size: StorageData<T> = 100;
        let peer_id = BoundedVec::try_from(vec![1]).unwrap();
        let peer_ids: PeerIds<T> = BoundedVec::try_from(vec![peer_id]).unwrap();

        // Create required bucket for issuing storage request.
        Pallet::<T>::create_bucket(signed_origin.clone().into(), None, name, true, None)?;

        // Benchmark issue_storage_request extrinsic
        #[extrinsic_call]
        _(
            signed_origin,
            bucket_id,
            location,
            fingerprint,
            size,
            None,
            peer_ids,
        );

        Ok(())
    }

    fn mint_into_account<T: crate::Config>(
        account: T::AccountId,
        amount: u128,
    ) -> Result<(), BenchmarkError> {
        let user_balance = match amount.try_into() {
            Ok(balance) => balance,
            Err(_) => return Err(BenchmarkError::Stop("Balance conversion failed.")),
        };
        assert_ok!(<T as crate::Config>::Currency::mint_into(
            &account,
            user_balance,
        ));

        Ok(())
    }

    fn add_msp_to_provider_storage<T>(msp: &T::AccountId) -> (ProviderIdFor<T>, ValuePropId<T>)
    where
        T: crate::Config<Fingerprint = <T as frame_system::Config>::Hash>,
        T: pallet_storage_providers::Config<ProviderId = <T as frame_system::Config>::Hash>,
        <T as crate::Config>::Providers: shp_traits::MutateStorageProvidersInterface<StorageDataUnit = u64>
            + ReadProvidersInterface<ProviderId = <T as frame_system::Config>::Hash>,
        T: pallet_storage_providers::Config<StorageDataUnit = u64>,
    {
        let msp_hash = <<T as frame_system::Config>::Hashing as Hasher>::hash(msp.as_slice());

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
        let value_prop = ValueProposition::<T>::new(1, commitment, value_prop_storage);
        let value_prop_id = value_prop.derive_id();

        pallet_storage_providers::MainStorageProviderIdsToValuePropositions::<T>::insert(
            msp_hash,
            value_prop_id,
            value_prop,
        );

        (msp_hash, value_prop_id)
    }
}
