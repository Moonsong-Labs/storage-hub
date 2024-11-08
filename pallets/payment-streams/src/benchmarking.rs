//! Benchmarking setup for pallet-payment-streams

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;

#[benchmarks(where
	// Runtime `T` implements `pallet_storage_providers::Config` and this pallet's `Config`.
	T: pallet_storage_providers::Config + crate::Config,
	// The Storage Providers pallet is the `Providers` pallet that this pallet requires.
	T: crate::Config<ProvidersPallet = pallet_storage_providers::Pallet<T>>,
	// The `ProviderId` inner type of the `ReadProvidersInterface` trait is `ProviderId` from `pallet-storage-providers`.
	<<T as crate::Config>::ProvidersPallet as shp_traits::ReadProvidersInterface>::ProviderId: From<<T as pallet_storage_providers::Config>::ProviderId>,
)]
mod benchmarks {
    use frame_support::{
        assert_ok,
        traits::{
            fungible::{Inspect, Mutate},
            Get, OnPoll,
        },
        weights::WeightMeter,
        BoundedVec,
    };
    use frame_system::RawOrigin;
    use pallet_storage_providers::types::{MaxMultiAddressAmount, MultiAddress};
    use sp_runtime::traits::{Hash, One};

    use super::*;
    use crate::{pallet, Call, Config, Event, Pallet};

    type BalanceOf<T> = <<T as crate::Config>::NativeBalance as Inspect<
        <T as frame_system::Config>::AccountId,
    >>::Balance;

    fn set_user_as_insolvent<T: crate::Config>(user: <T as frame_system::Config>::AccountId) {
        UsersWithoutFunds::<T>::insert(user, frame_system::Pallet::<T>::block_number());
    }

    fn unset_user_as_insolvent<T: crate::Config>(user: <T as frame_system::Config>::AccountId) {
        UsersWithoutFunds::<T>::remove(user);
    }

    fn update_last_chargeable_info<T: crate::Config>(
        sp_id: ProviderIdFor<T>,
        new_info: ProviderLastChargeableInfo<T>,
    ) {
        LastChargeableInfo::<T>::insert(sp_id, new_info);
    }

    fn register_provider<T>() -> Result<(T::AccountId, ProviderIdFor<T>), BenchmarkError>
    where
        T: pallet_storage_providers::Config + crate::Config,
        <<T as crate::Config>::ProvidersPallet as shp_traits::ReadProvidersInterface>::ProviderId:
            From<<T as pallet_storage_providers::Config>::ProviderId>,
    {
        let sp_account: T::AccountId = account("SP", 0, 0);
        let sp_id_seed = "benchmark_sp";
        let sp_id = <<T as pallet_storage_providers::Config>::ProviderIdHashing as Hash>::hash(
            sp_id_seed.as_bytes(),
        );
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
        let sp_balance = match 1_000_000_000_000_000u128.try_into() {
            Ok(balance) => balance,
            Err(_) => return Err(BenchmarkError::Stop("Balance conversion failed.")),
        };
        assert_ok!(<T as crate::Config>::NativeBalance::mint_into(
            &sp_account,
            sp_balance,
        ));

        pallet_storage_providers::AccountIdToBackupStorageProviderId::<T>::insert(
            &sp_account,
            sp_id,
        );
        pallet_storage_providers::BackupStorageProviders::<T>::insert(
            &sp_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: initial_capacity.into(),
                capacity_used: Default::default(),
                multiaddresses,
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: sp_account.clone(),
                payment_account: sp_account.clone(),
                reputation_weight:
                    <T as pallet_storage_providers::Config>::StartingReputationWeight::get(),
                sign_up_block: Default::default(),
            },
        );

        Ok((sp_account, sp_id.into()))
    }

    fn run_to_block<T: crate::Config>(n: BlockNumberFor<T>) {
        assert!(
            n > frame_system::Pallet::<T>::block_number(),
            "Cannot go back in time"
        );

        while frame_system::Pallet::<T>::block_number() < n {
            frame_system::Pallet::<T>::set_block_number(
                frame_system::Pallet::<T>::block_number() + One::one(),
            );
            Pallet::<T>::on_poll(
                frame_system::Pallet::<T>::block_number(),
                &mut WeightMeter::new(),
            );
        }
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

        // Set up a Provider with an account with some balance.
        let (_provider_account, provider_id) = register_provider::<T>()?;

        // Rate of the to-be-created payment stream
        let rate = 100u32;

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(
            RawOrigin::Root,
            provider_id,
            user_account.clone(),
            rate.into(),
        );

        /*********** Post-benchmark checks: ***********/
        // Verify the fixed-rate payment stream creation event was emitted.
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::FixedRatePaymentStreamCreated {
                user_account: user_account.clone(),
                provider_id: provider_id.clone(),
                rate: rate.into(),
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the newly created payment stream exists in storage
        let new_payment_stream = FixedRatePaymentStreams::<T>::get(provider_id, user_account);
        assert!(new_payment_stream.is_some());
        assert_eq!(new_payment_stream.unwrap().rate, rate.into());

        Ok(())
    }

    #[benchmark]
    fn update_fixed_rate_payment_stream() -> Result<(), BenchmarkError> {
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

        // Set up a Provider with an account with some balance.
        let (_provider_account, provider_id) = register_provider::<T>()?;

        // Rate of the to-be-created payment stream
        let initial_rate = 100u32;
        let initial_rate_as_balance: BalanceOf<T> = initial_rate.into();

        // Create the fixed-rate payment stream
        Pallet::<T>::create_fixed_rate_payment_stream(
            RawOrigin::Root.into(),
            provider_id,
            user_account.clone(),
            initial_rate.into(),
        )
        .map_err(|_| BenchmarkError::Stop("Fixed rate payment stream not created successfully."))?;

        // Worst-case scenario is when the runtime has to actually charge both this fixed-rate and a dynamic-rate
        // payment stream when updating this one (because somehow the provider has both types), so we create a dynamic-rate
        // payment stream for the user and update the last chargeable info of the provider
        let amount_provided = 1000u32;
        let amount_provided_as_balance: BalanceOf<T> = amount_provided.into();
        Pallet::<T>::create_dynamic_rate_payment_stream(
            RawOrigin::Root.into(),
            provider_id,
            user_account.clone(),
            amount_provided.into(),
        )
        .map_err(|_| {
            BenchmarkError::Stop("Dynamic rate payment stream not created successfully.")
        })?;
        run_to_block::<T>(frame_system::Pallet::<T>::block_number() + One::one());
        let new_last_chargeable_info = ProviderLastChargeableInfo {
            last_chargeable_tick: frame_system::Pallet::<T>::block_number(),
            price_index: AccumulatedPriceIndex::<T>::get(),
        };
        update_last_chargeable_info::<T>(provider_id, new_last_chargeable_info);

        // New rate of the to-be-updated payment stream
        let new_rate = 200u32;

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(
            RawOrigin::Root,
            provider_id,
            user_account.clone(),
            new_rate.into(),
        );

        /*********** Post-benchmark checks: ***********/
        // Verify that the charge event was emitted.
        let amount_charged: BalanceOf<T> = initial_rate_as_balance
            + (amount_provided_as_balance * CurrentPricePerUnitPerTick::<T>::get());
        let charge_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::PaymentStreamCharged {
                user_account: user_account.clone(),
                provider_id: provider_id.clone(),
                amount: amount_charged,
                last_tick_charged: frame_system::Pallet::<T>::block_number(),
                charged_at_tick: frame_system::Pallet::<T>::block_number(),
            });
        frame_system::Pallet::<T>::assert_has_event(charge_event.into());
        // Verify the fixed-rate payment stream update event was emitted.
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::FixedRatePaymentStreamUpdated {
                user_account: user_account.clone(),
                provider_id: provider_id.clone(),
                new_rate: new_rate.into(),
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the payment stream exists in storage and was updated
        let new_payment_stream = FixedRatePaymentStreams::<T>::get(provider_id, user_account);
        assert!(new_payment_stream.is_some());
        assert_eq!(new_payment_stream.unwrap().rate, new_rate.into());

        Ok(())
    }

    #[benchmark]
    fn delete_fixed_rate_payment_stream() -> Result<(), BenchmarkError> {
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

        // Set up a Provider with an account with some balance.
        let (_provider_account, provider_id) = register_provider::<T>()?;

        // Rate of the to-be-created payment stream
        let rate = 100u32;

        // Create the fixed-rate payment stream
        Pallet::<T>::create_fixed_rate_payment_stream(
            RawOrigin::Root.into(),
            provider_id,
            user_account.clone(),
            rate.into(),
        )
        .map_err(|_| BenchmarkError::Stop("Fixed rate payment stream not created successfully."))?;

        // Worst-case scenario is when the runtime has to actually charge the payment stream when deleting it,
        // so we update the last chargeable info of the provider
        run_to_block::<T>(frame_system::Pallet::<T>::block_number() + One::one());
        let new_last_chargeable_info = ProviderLastChargeableInfo {
            last_chargeable_tick: frame_system::Pallet::<T>::block_number(),
            price_index: AccumulatedPriceIndex::<T>::get(),
        };
        update_last_chargeable_info::<T>(provider_id, new_last_chargeable_info);

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(RawOrigin::Root, provider_id, user_account.clone());

        /*********** Post-benchmark checks: ***********/
        // Verify that the charge event was emitted.
        let charge_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::PaymentStreamCharged {
                user_account: user_account.clone(),
                provider_id: provider_id.clone(),
                amount: rate.into(),
                last_tick_charged: frame_system::Pallet::<T>::block_number(),
                charged_at_tick: frame_system::Pallet::<T>::block_number(),
            });
        frame_system::Pallet::<T>::assert_has_event(charge_event.into());
        // Verify the fixed-rate payment stream deletion event was emitted.
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::FixedRatePaymentStreamDeleted {
                user_account: user_account.clone(),
                provider_id: provider_id.clone(),
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the newly created payment stream was correctly deleted
        let new_payment_stream = FixedRatePaymentStreams::<T>::get(provider_id, user_account);
        assert!(new_payment_stream.is_none());

        Ok(())
    }

    #[benchmark]
    fn create_dynamic_rate_payment_stream() -> Result<(), BenchmarkError> {
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

        // Set up a Provider with an account with some balance.
        let (_provider_account, provider_id) = register_provider::<T>()?;

        // Amount of the to-be-created payment stream
        let amount_provided = 1000u32;

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(
            RawOrigin::Root,
            provider_id,
            user_account.clone(),
            amount_provided.into(),
        );

        /*********** Post-benchmark checks: ***********/
        // Verify the dynamic-rate payment stream creation event was emitted.
        let expected_event = <T as pallet::Config>::RuntimeEvent::from(
            Event::<T>::DynamicRatePaymentStreamCreated {
                user_account: user_account.clone(),
                provider_id: provider_id.clone(),
                amount_provided: amount_provided.into(),
            },
        );
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the newly created payment stream exists in storage
        let new_payment_stream = DynamicRatePaymentStreams::<T>::get(provider_id, user_account);
        assert!(new_payment_stream.is_some());
        assert_eq!(
            new_payment_stream.unwrap().amount_provided,
            amount_provided.into()
        );

        Ok(())
    }

    #[benchmark]
    fn update_dynamic_rate_payment_stream() -> Result<(), BenchmarkError> {
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

        // Set up a Provider with an account with some balance.
        let (_provider_account, provider_id) = register_provider::<T>()?;

        // Amount of the to-be-created payment stream
        let initial_amount = 1000u32;
        let initial_amount_as_balance: BalanceOf<T> = initial_amount.into();

        // Create the dynamic-rate payment stream
        Pallet::<T>::create_dynamic_rate_payment_stream(
            RawOrigin::Root.into(),
            provider_id,
            user_account.clone(),
            initial_amount.into(),
        )
        .map_err(|_| {
            BenchmarkError::Stop("Dynamic rate payment stream not created successfully.")
        })?;

        // Worst-case scenario is when the runtime has to actually charge both this dynamic-rate and a fixed-rate
        // payment stream when updating this one (because somehow the provider has both types), so we create a fixed-rate
        // payment stream for the user and update the last chargeable info of the provider
        let rate = 100u32;
        let rate_as_balance: BalanceOf<T> = rate.into();
        Pallet::<T>::create_fixed_rate_payment_stream(
            RawOrigin::Root.into(),
            provider_id,
            user_account.clone(),
            rate.into(),
        )
        .map_err(|_| BenchmarkError::Stop("Fixed rate payment stream not created successfully."))?;
        run_to_block::<T>(frame_system::Pallet::<T>::block_number() + One::one());
        let new_last_chargeable_info = ProviderLastChargeableInfo {
            last_chargeable_tick: frame_system::Pallet::<T>::block_number(),
            price_index: AccumulatedPriceIndex::<T>::get(),
        };
        update_last_chargeable_info::<T>(provider_id, new_last_chargeable_info);

        // New provided amount of the to-be-updated payment stream
        let new_amount_provided = 200u32;

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(
            RawOrigin::Root,
            provider_id,
            user_account.clone(),
            new_amount_provided.into(),
        );

        /*********** Post-benchmark checks: ***********/
        // Verify that the charge event was emitted.
        let amount_charged: BalanceOf<T> =
            rate_as_balance + (initial_amount_as_balance * CurrentPricePerUnitPerTick::<T>::get());
        let charge_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::PaymentStreamCharged {
                user_account: user_account.clone(),
                provider_id: provider_id.clone(),
                amount: amount_charged,
                last_tick_charged: frame_system::Pallet::<T>::block_number(),
                charged_at_tick: frame_system::Pallet::<T>::block_number(),
            });
        frame_system::Pallet::<T>::assert_has_event(charge_event.into());
        // Verify the dynamic-rate payment stream update event was emitted.
        let expected_event = <T as pallet::Config>::RuntimeEvent::from(
            Event::<T>::DynamicRatePaymentStreamUpdated {
                user_account: user_account.clone(),
                provider_id: provider_id.clone(),
                new_amount_provided: new_amount_provided.into(),
            },
        );
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the payment stream exists in storage and was updated
        let new_payment_stream = DynamicRatePaymentStreams::<T>::get(provider_id, user_account);
        assert!(new_payment_stream.is_some());
        assert_eq!(
            new_payment_stream.unwrap().amount_provided,
            new_amount_provided.into()
        );

        Ok(())
    }

    #[benchmark]
    fn delete_dynamic_rate_payment_stream() -> Result<(), BenchmarkError> {
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

        // Set up a Provider with an account with some balance.
        let (_provider_account, provider_id) = register_provider::<T>()?;

        // Amount of the to-be-created payment stream
        let amount_provided = 1000u32;
        let amount_provided_as_balance: BalanceOf<T> = amount_provided.into();

        // Create the dynamic-rate payment stream
        Pallet::<T>::create_dynamic_rate_payment_stream(
            RawOrigin::Root.into(),
            provider_id,
            user_account.clone(),
            amount_provided.into(),
        )
        .map_err(|_| {
            BenchmarkError::Stop("Dynamic rate payment stream not created successfully.")
        })?;

        // Worst-case scenario is when the runtime has to actually charge both this dynamic-rate and a fixed-rate
        // payment stream when updating this one (because somehow the provider has both types), so we create a fixed-rate
        // payment stream for the user and update the last chargeable info of the provider
        let rate = 100u32;
        let rate_as_balance: BalanceOf<T> = rate.into();
        Pallet::<T>::create_fixed_rate_payment_stream(
            RawOrigin::Root.into(),
            provider_id,
            user_account.clone(),
            rate.into(),
        )
        .map_err(|_| BenchmarkError::Stop("Fixed rate payment stream not created successfully."))?;
        run_to_block::<T>(frame_system::Pallet::<T>::block_number() + One::one());
        let new_last_chargeable_info = ProviderLastChargeableInfo {
            last_chargeable_tick: frame_system::Pallet::<T>::block_number(),
            price_index: AccumulatedPriceIndex::<T>::get(),
        };
        update_last_chargeable_info::<T>(provider_id, new_last_chargeable_info);

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(RawOrigin::Root, provider_id, user_account.clone());

        /*********** Post-benchmark checks: ***********/
        // Verify that the charge event was emitted.
        let amount_charged: BalanceOf<T> =
            rate_as_balance + (amount_provided_as_balance * CurrentPricePerUnitPerTick::<T>::get());
        let charge_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::PaymentStreamCharged {
                user_account: user_account.clone(),
                provider_id: provider_id.clone(),
                amount: amount_charged,
                last_tick_charged: frame_system::Pallet::<T>::block_number(),
                charged_at_tick: frame_system::Pallet::<T>::block_number(),
            });
        frame_system::Pallet::<T>::assert_has_event(charge_event.into());

        // Verify the dynamic-rate payment stream deletion event was emitted.
        let expected_event = <T as pallet::Config>::RuntimeEvent::from(
            Event::<T>::DynamicRatePaymentStreamDeleted {
                user_account: user_account.clone(),
                provider_id: provider_id.clone(),
            },
        );
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the newly created payment stream was correctly deleted
        let new_payment_stream = DynamicRatePaymentStreams::<T>::get(provider_id, user_account);
        assert!(new_payment_stream.is_none());

        Ok(())
    }

    #[benchmark]
    fn charge_payment_streams() -> Result<(), BenchmarkError> {
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

        // Set up a Provider with an account with some balance.
        let (provider_account, provider_id) = register_provider::<T>()?;

        // Worst case scenario: the provider has to charge both types of payment streams in the extrinsic:
        // Create the dynamic-rate payment stream
        let amount_provided = 1000u32;
        let amount_provided_as_balance: BalanceOf<T> = amount_provided.into();
        Pallet::<T>::create_dynamic_rate_payment_stream(
            RawOrigin::Root.into(),
            provider_id,
            user_account.clone(),
            amount_provided.into(),
        )
        .map_err(|_| {
            BenchmarkError::Stop("Dynamic rate payment stream not created successfully.")
        })?;

        // Create the fixed-rate payment stream
        let rate = 100u32;
        let rate_as_balance: BalanceOf<T> = rate.into();
        Pallet::<T>::create_fixed_rate_payment_stream(
            RawOrigin::Root.into(),
            provider_id,
            user_account.clone(),
            rate.into(),
        )
        .map_err(|_| BenchmarkError::Stop("Fixed rate payment stream not created successfully."))?;

        // Update last chargeable info of the provider
        run_to_block::<T>(frame_system::Pallet::<T>::block_number() + One::one());
        let new_last_chargeable_info = ProviderLastChargeableInfo {
            last_chargeable_tick: frame_system::Pallet::<T>::block_number(),
            price_index: AccumulatedPriceIndex::<T>::get(),
        };
        update_last_chargeable_info::<T>(provider_id, new_last_chargeable_info);

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(RawOrigin::Signed(provider_account), user_account.clone());

        /*********** Post-benchmark checks: ***********/
        // Verify that the charge event was emitted.
        let amount_charged: BalanceOf<T> =
            rate_as_balance + (amount_provided_as_balance * CurrentPricePerUnitPerTick::<T>::get());
        let charge_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::PaymentStreamCharged {
                user_account: user_account.clone(),
                provider_id: provider_id.clone(),
                amount: amount_charged,
                last_tick_charged: frame_system::Pallet::<T>::block_number(),
                charged_at_tick: frame_system::Pallet::<T>::block_number(),
            });
        frame_system::Pallet::<T>::assert_has_event(charge_event.into());

        Ok(())
    }

    #[benchmark]
    fn charge_multiple_users_payment_streams() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Get the maximum amount of users that can be batch charged
        let max_users = <T as crate::Config>::MaxUsersToCharge::get();

        // Set up a Provider with an account with some balance.
        let (provider_account, provider_id) = register_provider::<T>()?;

        // Set up `max_users` accounts with some balance and create a fixed-rate and a dynamic-rate
        // payment stream with each one
        let mut user_accounts: BoundedVec<T::AccountId, <T as crate::Config>::MaxUsersToCharge> =
            BoundedVec::new();
        let user_balance = match 1_000_000_000_000_000u128.try_into() {
            Ok(balance) => balance,
            Err(_) => return Err(BenchmarkError::Stop("Balance conversion failed.")),
        };
        let rate = 100u32;
        let rate_as_balance: BalanceOf<T> = rate.into();
        let amount_provided = 1000u32;
        let amount_provided_as_balance: BalanceOf<T> = amount_provided.into();
        for i in 0..max_users {
            let user_account: T::AccountId = account("Alice", 0, i);
            assert_ok!(<T as crate::Config>::NativeBalance::mint_into(
                &user_account,
                user_balance,
            ));
            user_accounts
                .try_push(user_account.clone())
                .expect("Max size of bounded vec is accounted for.");

            // Worst case scenario: the provider has to charge both types of payment streams in the extrinsic:
            // Create the dynamic-rate payment stream
            Pallet::<T>::create_dynamic_rate_payment_stream(
                RawOrigin::Root.into(),
                provider_id,
                user_account.clone(),
                amount_provided.into(),
            )
            .map_err(|_| {
                BenchmarkError::Stop("Dynamic rate payment stream not created successfully.")
            })?;

            // Create the fixed-rate payment stream
            Pallet::<T>::create_fixed_rate_payment_stream(
                RawOrigin::Root.into(),
                provider_id,
                user_account.clone(),
                rate.into(),
            )
            .map_err(|_| {
                BenchmarkError::Stop("Fixed rate payment stream not created successfully.")
            })?;
        }

        // Update last chargeable info of the provider
        run_to_block::<T>(frame_system::Pallet::<T>::block_number() + One::one());
        let new_last_chargeable_info = ProviderLastChargeableInfo {
            last_chargeable_tick: frame_system::Pallet::<T>::block_number(),
            price_index: AccumulatedPriceIndex::<T>::get(),
        };
        update_last_chargeable_info::<T>(provider_id, new_last_chargeable_info);

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(RawOrigin::Signed(provider_account), user_accounts.clone());

        /*********** Post-benchmark checks: ***********/
        // Verify that the charge event was emitted for each user
        let amount_charged: BalanceOf<T> =
            rate_as_balance + (amount_provided_as_balance * CurrentPricePerUnitPerTick::<T>::get());
        for user_account in user_accounts.iter() {
            let charge_event =
                <T as pallet::Config>::RuntimeEvent::from(Event::<T>::PaymentStreamCharged {
                    user_account: user_account.clone(),
                    provider_id: provider_id.clone(),
                    amount: amount_charged,
                    last_tick_charged: frame_system::Pallet::<T>::block_number(),
                    charged_at_tick: frame_system::Pallet::<T>::block_number(),
                });
            frame_system::Pallet::<T>::assert_has_event(charge_event.into());
        }

        Ok(())
    }

    #[benchmark]
    fn pay_outstanding_debt() -> Result<(), BenchmarkError> {}

    impl_benchmark_test_suite! {
            Pallet,
            crate::mock::ExtBuilder::build(),
            crate::mock::Test,
    }
}
