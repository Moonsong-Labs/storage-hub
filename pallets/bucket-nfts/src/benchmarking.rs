//! Benchmarking setup for pallet-bucket-nfts

#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::v2::*;

#[benchmarks(where
	T: crate::Config + pallet_nfts::Config + pallet_storage_providers::Config + pallet_balances::Config,
	T::Hash: From<H256>,
	T::ItemId: From<u32>,
	<T as pallet::Config>::RuntimeEvent: From<Event<T>>
)]
mod benchmarks {
    use super::*;
    use crate::{pallet, types::*, Call, Config, Event, Pallet};
    use frame_support::BoundedVec;
    use frame_system::{pallet_prelude::BlockNumberFor, RawOrigin};
    use sp_core::H256;
    use sp_runtime::traits::{StaticLookup, Zero};
    use sp_std::vec;

    type BucketIdFor<T> =
        <<T as crate::Config>::Buckets as shp_traits::ReadBucketsInterface>::BucketId;

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
        let recipient: T::AccountId = account("Recipient", 0, 0);
        let user: T::AccountId = account("User", 0, 0);
        let name: BoundedVec<
            u8,
            <<T as crate::Config>::Buckets as shp_traits::ReadBucketsInterface>::BucketNameLimit,
        > = vec![1u8; 32]
            .try_into()
            .map_err(|_| BenchmarkError::Stop("Failed to create bucket name"))?;

        // Convert ProviderId to BucketId
        let bucket_id: BucketIdFor<T> =
            <<T as crate::Config>::Buckets as shp_traits::ReadBucketsInterface>::derive_bucket_id(
                &user,
                name.clone(),
            );

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
        let admin: T::AccountId = whitelisted_caller();
        let user: T::AccountId = account("User", 0, 0);
        let name: BoundedVec<
            u8,
            <<T as crate::Config>::Buckets as shp_traits::ReadBucketsInterface>::BucketNameLimit,
        > = vec![1u8; 32]
            .try_into()
            .map_err(|_| BenchmarkError::Stop("Failed to create bucket name"))?;

        // Convert ProviderId to BucketId
        let bucket_id: BucketIdFor<T> =
            <<T as crate::Config>::Buckets as shp_traits::ReadBucketsInterface>::derive_bucket_id(
                &user,
                name.clone(),
            );

        let item_id = T::ItemId::from(1u32);

        // Create a read access regex pattern
        let read_access_regex: ReadAccessRegex<T> = vec![1u8; 32]
            .try_into()
            .map_err(|_| BenchmarkError::Stop("Failed to create read access regex"))?;

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(
            RawOrigin::Signed(admin.clone()),
            bucket_id,
            item_id,
            Some(read_access_regex.clone()),
        );

        /*********** Post-benchmark checks: ***********/
        // Verify that the event was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::ItemReadAccessUpdated {
                admin,
                bucket: bucket_id,
                item_id,
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        Ok(())
    }

    impl_benchmark_test_suite! {
            Pallet,
            crate::mock::ExtBuilder::build(),
            crate::mock::Test,
    }
}
