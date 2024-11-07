//! Benchmarking setup for pallet-payment-streams

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as PaymentStreams;

use frame_benchmarking::v2::*;

#[benchmarks]
mod benchmarks {
    use frame_support::{assert_ok, traits::fungible::Mutate, BoundedVec};
    use frame_system::RawOrigin;
    use shp_traits::runtime_benchmark_helper_interfaces::ForceRegisterProviders;

    use super::*;
    use crate::{pallet, Call, Config, Event, Pallet, *};

    type MultiAddress<T> =
        <<T as Config>::RegisterProvidersBenchmark as ForceRegisterProviders>::MultiAddress;
    type MaxMultiAddressAmount<T> = <<T as Config>::RegisterProvidersBenchmark as ForceRegisterProviders>::MaxNumberOfMultiAddresses;

    fn set_user_as_insolvent<T: Config>(user: <T as frame_system::Config>::AccountId) {
        UsersWithoutFunds::<T>::insert(user, frame_system::Pallet::<T>::block_number());
    }

    fn unset_user_as_insolvent<T: Config>(user: <T as frame_system::Config>::AccountId) {
        UsersWithoutFunds::<T>::remove(user);
    }

    fn update_last_chargeable_info<T: Config>(
        sp_id: ProviderIdFor<T>,
        new_info: ProviderLastChargeableInfo<T>,
    ) {
        LastChargeableInfo::<T>::insert(sp_id, new_info);
    }

    #[benchmark]
    fn create_fixed_rate_payment_stream() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Set up an account with some balance.
        let user_account: T::AccountId = account("Alice", 0, 0);
        let user_balance = match 1_000_000_000_000_000u128.try_into() {
            Ok(balance) => balance,
            Err(_) => return Err(BenchmarkError::Stop("Balance conversion failed.")),
        };
        assert_ok!(<T as crate::Config>::NativeBalance::mint_into(
            &user_account,
            user_balance,
        ));

        // Set up a BSP account with some balance.
        let bsp_account: T::AccountId = account("BSP", 0, 0);
        let bsp_id_seed = "benchmark_bsp";
        let initial_capacity = 1_000_000u32; // 1 TB
        let mut multiaddresses: BoundedVec<MultiAddress<T>, MaxMultiAddressAmount<T>> =
            BoundedVec::new();
        multiaddresses.force_push(
            "/ip4/127.0.0.1/udp/1234"
                .as_bytes()
                .to_vec()
                .try_into()
                .ok()
                .unwrap(),
        );
        let bsp_balance = match 1_000_000_000_000_000u128.try_into() {
            Ok(balance) => balance,
            Err(_) => return Err(BenchmarkError::Stop("Balance conversion failed.")),
        };
        assert_ok!(<T as crate::Config>::NativeBalance::mint_into(
            &bsp_account,
            bsp_balance,
        ));
        let bsp_register_result = <<T as crate::Config>::RegisterProvidersBenchmark as ForceRegisterProviders>::force_register_bsp(bsp_account, bsp_id_seed, initial_capacity.into(), multiaddresses);
        assert!(bsp_register_result.is_ok());
        let bsp_id = bsp_register_result.unwrap();

        // Rate of the to-be-created payment stream
        let rate = 100u32;

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(RawOrigin::Root, bsp_id, user_account.clone(), rate.into());

        // Verify the fixed-rate payment stream event was emitted.
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::FixedRatePaymentStreamCreated {
                user_account: user_account.clone(),
                provider_id: bsp_id,
                rate: rate.into(),
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the newly created payment stream exists in storage
        let new_payment_stream = FixedRatePaymentStreams::<T>::get(bsp_id, user_account);
        assert!(new_payment_stream.is_some());
        assert_eq!(new_payment_stream.unwrap().rate, rate.into());

        Ok(())
    }

    impl_benchmark_test_suite! {
        PaymentStreams,
            crate::mock::ExtBuilder::build(),
            crate::mock::Test,
    }
}
