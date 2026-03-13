//! Benchmarking setup for pallet-bucket-nfts

#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::v2::*;
#[benchmarks(where
    T: crate::Config
        + pallet_nfts::Config
        + pallet_storage_providers::Config<
            ProviderId = <T as frame_system::Config>::Hash,
            MerklePatriciaRoot = <T as frame_system::Config>::Hash,
            StorageDataUnit = u64,
            ReadAccessGroupId = <T as pallet_nfts::Config>::CollectionId,
        >
        + pallet_balances::Config
        + pallet_file_system::Config<
            Fingerprint = <T as frame_system::Config>::Hash,
            Providers = pallet_storage_providers::Pallet<T>,
        >,
    <T as crate::Config>::Buckets: shp_traits::ReadBucketsInterface<
        BucketId = <T as pallet_storage_providers::Config>::ProviderId,
        AccountId = <T as frame_system::Config>::AccountId,
        ProviderId = <T as pallet_storage_providers::Config>::ProviderId,
        ReadAccessGroupId = <T as pallet_nfts::Config>::CollectionId,
        BucketNameLimit = <T as pallet_storage_providers::Config>::BucketNameLimit,
    >,
    <T as pallet_nfts::Config>::ItemId: From<u32>,
)]
mod benchmarks {
    use super::*;
    use crate::{pallet, types::*, Call, Config, Event, Pallet};
    use alloc::vec;
    use frame_support::{assert_ok, traits::fungible::Mutate, BoundedVec};
    use frame_system::{pallet_prelude::BlockNumberFor, RawOrigin};
    use pallet_storage_providers::types::{
        BucketId, MainStorageProvider, ProviderIdFor, StorageDataUnit, ValuePropIdFor,
        ValueProposition,
    };
    use shp_traits::ReadBucketsInterface;
    use sp_core::Get;
    use sp_runtime::traits::{Hash, One, StaticLookup, Zero};

    fn run_to_block<T: crate::Config>(n: BlockNumberFor<T>) {
        assert!(
            n > frame_system::Pallet::<T>::block_number(),
            "Cannot go back in time"
        );

        frame_system::Pallet::<T>::set_block_number(frame_system::Pallet::<T>::block_number() + n);
    }

    #[benchmark]
    fn share_access() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Make sure the block number is not 0 so events can be deposited.
        if frame_system::Pallet::<T>::block_number() == Zero::zero() {
            run_to_block::<T>(1u32.into());
        }

        // Get accounts
        let issuer: T::AccountId = whitelisted_caller();
        mint_into_account::<T>(issuer.clone(), 1_000_000_000_000_000)?;
        let recipient: T::AccountId = account("Recipient", 0, 0);

        // Register a MSP with a value proposition
        let msp_account: T::AccountId = account("MSP", 0, 0);
        mint_into_account::<T>(msp_account.clone(), 1_000_000_000_000_000)?;
        let (msp_id, value_prop_id) = add_msp_to_provider_storage::<T>(&msp_account);

        // Create bucket with private access
        let (bucket_id, _read_access_group_id) =
            create_bucket_and_collection::<T>(issuer.clone(), msp_id, value_prop_id)?;

        let item_id = T::ItemId::from(1u32);

        // Create a read access regex pattern
        let read_access_regex: ReadAccessRegex<T> = vec![1u8; 32]
            .try_into()
            .map_err(|_| BenchmarkError::Stop("Failed to create read access regex"))?;

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(
            RawOrigin::Signed(issuer.clone()),
            T::Lookup::unlookup(recipient.clone()),
            bucket_id,
            item_id,
            Some(read_access_regex.clone()),
        );

        /*********** Post-benchmark checks: ***********/
        // Verify that the event was emitted
        let expected_event = <T as pallet::Config>::RuntimeEvent::from(Event::AccessShared {
            issuer,
            recipient: recipient.clone(),
        });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        Ok(())
    }

    #[benchmark]
    fn update_read_access() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Make sure the block number is not 0 so events can be deposited.
        if frame_system::Pallet::<T>::block_number() == Zero::zero() {
            run_to_block::<T>(1u32.into());
        }

        // Get account
        let issuer: T::AccountId = whitelisted_caller();
        mint_into_account::<T>(issuer.clone(), 1_000_000_000_000_000)?;
        let recipient: T::AccountId = account("Recipient", 0, 0);
        // Register a MSP with a value proposition
        let msp_account: T::AccountId = account("MSP", 0, 0);
        mint_into_account::<T>(msp_account.clone(), 1_000_000_000_000_000)?;
        let (msp_id, value_prop_id) = add_msp_to_provider_storage::<T>(&msp_account);

        // Create bucket with private access
        let (bucket_id, _read_access_group_id) =
            create_bucket_and_collection::<T>(issuer.clone(), msp_id, value_prop_id)?;

        let item_id = T::ItemId::from(1u32);

        // Create a read access regex pattern
        let read_access_regex: ReadAccessRegex<T> = vec![1u8; 32]
            .try_into()
            .map_err(|_| BenchmarkError::Stop("Failed to create read access regex"))?;

        assert_ok!(Pallet::<T>::share_access(
            RawOrigin::Signed(issuer.clone()).into(),
            T::Lookup::unlookup(recipient),
            bucket_id,
            item_id,
            Some(read_access_regex.clone()),
        ));

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(
            RawOrigin::Signed(issuer.clone()),
            bucket_id,
            item_id,
            Some(read_access_regex),
        );

        /*********** Post-benchmark checks: ***********/
        // Verify that the event was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::ItemReadAccessUpdated {
                admin: issuer,
                bucket: bucket_id,
                item_id,
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        Ok(())
    }

    fn mint_into_account<T: crate::Config + pallet_balances::Config>(
        account: T::AccountId,
        amount: u128,
    ) -> Result<(), BenchmarkError> {
        let user_balance = amount
            .try_into()
            .map_err(|_| BenchmarkError::Stop("Balance conversion failed."))?;
        assert_ok!(
            <pallet_balances::Pallet::<T> as Mutate<T::AccountId>>::mint_into(
                &account,
                user_balance,
            )
        );
        Ok(())
    }

    fn create_bucket_and_collection<T>(
        user: T::AccountId,
        msp_id: ProviderIdFor<T>,
        value_prop_id: ValuePropIdFor<T>,
    ) -> Result<(BucketId<T>, T::ReadAccessGroupId), BenchmarkError>
    where
        T: crate::Config
            + pallet_storage_providers::Config<
                ReadAccessGroupId = <T as pallet_nfts::Config>::CollectionId,
            > + pallet_file_system::Config<Providers = pallet_storage_providers::Pallet<T>>,
        <T as crate::Config>::Buckets: shp_traits::ReadBucketsInterface<
            BucketId = <T as pallet_storage_providers::Config>::ProviderId,
            BucketNameLimit = <T as pallet_storage_providers::Config>::BucketNameLimit,
        >,
    {
        let name: BoundedVec<
            u8,
            <<T as crate::Config>::Buckets as shp_traits::ReadBucketsInterface>::BucketNameLimit,
        > = vec![1u8; 32]
            .try_into()
            .map_err(|_| BenchmarkError::Stop("Failed to create bucket name"))?;

        let bucket_id = <<T as crate::Config>::Buckets as ReadBucketsInterface>::derive_bucket_id(
            &user,
            name.clone(),
        );

        pallet_file_system::Pallet::<T>::create_bucket(
            RawOrigin::Signed(user).into(),
            msp_id,
            name,
            true,
            value_prop_id,
        )?;

        let read_access_group_id = <<T as crate::Config>::Buckets as ReadBucketsInterface>::get_read_access_group_id_of_bucket(&bucket_id)?
            .unwrap();

        Ok((bucket_id, read_access_group_id))
    }

    fn add_msp_to_provider_storage<T>(msp: &T::AccountId) -> (ProviderIdFor<T>, ValuePropIdFor<T>)
    where
        T: pallet_nfts::Config
            + pallet_storage_providers::Config<
                ProviderId = <T as frame_system::Config>::Hash,
                StorageDataUnit = u64,
                ReadAccessGroupId = <T as pallet_nfts::Config>::CollectionId,
            >,
    {
        let msp_hash = T::Hashing::hash_of(&msp);
        let msp_id = ProviderIdFor::<T>::from(msp_hash);

        let capacity = StorageDataUnit::<T>::from(1024u32 * 1024u32 * 1024u32);
        let capacity_used = StorageDataUnit::<T>::from(0u32);

        let msp_info = MainStorageProvider {
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

        pallet_storage_providers::MainStorageProviders::<T>::insert(msp_id, msp_info);
        pallet_storage_providers::AccountIdToMainStorageProviderId::<T>::insert(
            msp.clone(),
            msp_id,
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
        let value_prop = ValueProposition::<T>::new(One::one(), commitment, bucket_data_limit);
        let value_prop_id = value_prop.derive_id();

        pallet_storage_providers::MainStorageProviderIdsToValuePropositions::<T>::insert(
            msp_id,
            value_prop_id,
            value_prop,
        );

        (msp_id, value_prop_id)
    }

    impl_benchmark_test_suite! {
        Pallet,
        crate::mock::new_test_ext(),
        crate::mock::Test,
    }
}
