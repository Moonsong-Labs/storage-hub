//! Benchmarking setup for pallet-payment-streams

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;

#[benchmarks(where
	// Runtime `T` implements `pallet_storage_providers::Config`, `pallet_proofs_dealer::Config` and this pallet's `Config`.
	T: pallet_storage_providers::Config + pallet_proofs_dealer::Config + crate::Config,
	// The Storage Providers pallet is the `Providers` pallet that this pallet requires.
	T: crate::Config<ProvidersPallet = pallet_storage_providers::Pallet<T>>,
	// The `ProviderId` inner type of the `ReadProvidersInterface` trait is `ProviderId` from `pallet-storage-providers`.
	<<T as crate::Config>::ProvidersPallet as shp_traits::ReadProvidersInterface>::ProviderId: From<<T as pallet_storage_providers::Config>::ProviderId>,
	// The `ProviderId` inner type of the `ReadChallengeableProvidersInterface` trait used in `pallet-proofs-dealer` is `ProviderId` from `pallet-storage-providers`.
	<<T as pallet_proofs_dealer::Config>::ProvidersPallet as shp_traits::ReadChallengeableProvidersInterface>::ProviderId: From<<T as pallet_storage_providers::Config>::ProviderId>,
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
    use pallet_storage_providers::{
        types::{MaxMultiAddressAmount, MultiAddress},
        TotalBspsCapacity, UsedBspsCapacity,
    };
    use shp_constants::GIGAUNIT;
    use sp_runtime::{
        traits::{Hash, One},
        BoundedBTreeSet,
    };

    use super::*;
    use crate::{pallet, Call, Config, Event, Pallet};

    type BalanceOf<T> = <<T as crate::Config>::NativeBalance as Inspect<
        <T as frame_system::Config>::AccountId,
    >>::Balance;

    fn set_user_as_insolvent<T: crate::Config>(user: <T as frame_system::Config>::AccountId) {
        UsersWithoutFunds::<T>::insert(user, frame_system::Pallet::<T>::block_number());
    }

    fn update_last_chargeable_info<T: crate::Config>(
        sp_id: ProviderIdFor<T>,
        new_info: ProviderLastChargeableInfo<T>,
    ) {
        LastChargeableInfo::<T>::insert(sp_id, new_info);
    }

    // The worst case scenario when calculating the treasury cut is when the used capacity is the total capacity,
    // since it has to calculate the taylor series for the 2^x function
    fn set_up_worst_case_scenario_for_treasury_cut<T: pallet_storage_providers::Config>() {
        let total_capacity = TotalBspsCapacity::<T>::get();
        UsedBspsCapacity::<T>::put(total_capacity);
    }

    fn register_provider<T>(
        index: u32,
    ) -> Result<
        (
            T::AccountId,
            <T as pallet_storage_providers::Config>::ProviderId,
        ),
        BenchmarkError,
    >
    where
        T: pallet_storage_providers::Config + pallet_proofs_dealer::Config + crate::Config,
        <<T as crate::Config>::ProvidersPallet as shp_traits::ReadProvidersInterface>::ProviderId:
            From<<T as pallet_storage_providers::Config>::ProviderId>,
    {
        let sp_account: T::AccountId = account("SP", index, 0);
        let sp_id_seed = format!("benchmark_sp_{}", index);
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

        // Make sure the sp_account is not already in use
        if pallet_storage_providers::AccountIdToBackupStorageProviderId::<T>::contains_key(
            &sp_account,
        ) {
            return Err(BenchmarkError::Stop("Provider account already in use."));
        }

        // Make sure the sp_id is not already in use
        if pallet_storage_providers::BackupStorageProviders::<T>::contains_key(&sp_id) {
            return Err(BenchmarkError::Stop("Provider ID already in use."));
        }

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

        Ok((sp_account, sp_id))
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
        let (_provider_account, provider_id) = register_provider::<T>(0)?;
        let provider_id: ProviderIdFor<T> = provider_id.into();

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
            <T as pallet::Config>::RuntimeEvent::from(Event::FixedRatePaymentStreamCreated {
                user_account: user_account.clone(),
                provider_id,
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
        let (_provider_account, provider_id) = register_provider::<T>(0)?;
        let provider_id: ProviderIdFor<T> = provider_id.into();

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
        let amount_provided = 100_000u32;
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

        // Worst case scenario: the used bsp capacity is the total capacity when charging the payment streams
        set_up_worst_case_scenario_for_treasury_cut::<T>();

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
            + (amount_provided_as_balance * CurrentPricePerGigaUnitPerTick::<T>::get()
                / GIGAUNIT.into());
        let charge_event = <T as pallet::Config>::RuntimeEvent::from(Event::PaymentStreamCharged {
            user_account: user_account.clone(),
            provider_id,
            amount: amount_charged,
            last_tick_charged: frame_system::Pallet::<T>::block_number(),
            charged_at_tick: frame_system::Pallet::<T>::block_number(),
        });
        frame_system::Pallet::<T>::assert_has_event(charge_event.into());
        // Verify the fixed-rate payment stream update event was emitted.
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::FixedRatePaymentStreamUpdated {
                user_account: user_account.clone(),
                provider_id,
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
        let (_provider_account, provider_id) = register_provider::<T>(0)?;
        let provider_id: ProviderIdFor<T> = provider_id.into();

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

        // Worst case scenario: the used bsp capacity is the total capacity when charging the payment streams
        set_up_worst_case_scenario_for_treasury_cut::<T>();

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(RawOrigin::Root, provider_id, user_account.clone());

        /*********** Post-benchmark checks: ***********/
        // Verify that the charge event was emitted.
        let charge_event = <T as pallet::Config>::RuntimeEvent::from(Event::PaymentStreamCharged {
            user_account: user_account.clone(),
            provider_id,
            amount: rate.into(),
            last_tick_charged: frame_system::Pallet::<T>::block_number(),
            charged_at_tick: frame_system::Pallet::<T>::block_number(),
        });
        frame_system::Pallet::<T>::assert_has_event(charge_event.into());
        // Verify the fixed-rate payment stream deletion event was emitted.
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::FixedRatePaymentStreamDeleted {
                user_account: user_account.clone(),
                provider_id,
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
        let (_provider_account, provider_id) = register_provider::<T>(0)?;
        let provider_id: ProviderIdFor<T> = provider_id.into();

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
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::DynamicRatePaymentStreamCreated {
                user_account: user_account.clone(),
                provider_id,
                amount_provided: amount_provided.into(),
            });
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
        let (_provider_account, provider_id) = register_provider::<T>(0)?;
        let provider_id: ProviderIdFor<T> = provider_id.into();

        // Amount of the to-be-created payment stream
        let initial_amount = 100_000u32;
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

        // Worst case scenario: the used bsp capacity is the total capacity when charging the payment streams
        set_up_worst_case_scenario_for_treasury_cut::<T>();

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
        let amount_charged: BalanceOf<T> = rate_as_balance
            + (initial_amount_as_balance * CurrentPricePerGigaUnitPerTick::<T>::get()
                / GIGAUNIT.into());
        let charge_event = <T as pallet::Config>::RuntimeEvent::from(Event::PaymentStreamCharged {
            user_account: user_account.clone(),
            provider_id,
            amount: amount_charged,
            last_tick_charged: frame_system::Pallet::<T>::block_number(),
            charged_at_tick: frame_system::Pallet::<T>::block_number(),
        });
        frame_system::Pallet::<T>::assert_has_event(charge_event.into());
        // Verify the dynamic-rate payment stream update event was emitted.
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::DynamicRatePaymentStreamUpdated {
                user_account: user_account.clone(),
                provider_id,
                new_amount_provided: new_amount_provided.into(),
            });
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
        let (_provider_account, provider_id) = register_provider::<T>(0)?;
        let provider_id: ProviderIdFor<T> = provider_id.into();

        // Amount of the to-be-created payment stream
        let amount_provided = 100_000u32;
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

        // Worst case scenario: the used bsp capacity is the total capacity when charging the payment streams
        set_up_worst_case_scenario_for_treasury_cut::<T>();

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(RawOrigin::Root, provider_id, user_account.clone());

        /*********** Post-benchmark checks: ***********/
        // Verify that the charge event was emitted.
        let amount_charged: BalanceOf<T> = rate_as_balance
            + (amount_provided_as_balance * CurrentPricePerGigaUnitPerTick::<T>::get()
                / GIGAUNIT.into());
        let charge_event = <T as pallet::Config>::RuntimeEvent::from(Event::PaymentStreamCharged {
            user_account: user_account.clone(),
            provider_id,
            amount: amount_charged,
            last_tick_charged: frame_system::Pallet::<T>::block_number(),
            charged_at_tick: frame_system::Pallet::<T>::block_number(),
        });
        frame_system::Pallet::<T>::assert_has_event(charge_event.into());

        // Verify the dynamic-rate payment stream deletion event was emitted.
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::DynamicRatePaymentStreamDeleted {
                user_account: user_account.clone(),
                provider_id,
            });
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
        let (provider_account, provider_id) = register_provider::<T>(0)?;
        let provider_id: ProviderIdFor<T> = provider_id.into();

        // Worst case scenario: the provider has to charge both types of payment streams in the extrinsic:
        // Create the dynamic-rate payment stream
        let amount_provided = 100_000u32;
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

        // Worst case scenario: the used bsp capacity is the total capacity when charging the payment streams
        set_up_worst_case_scenario_for_treasury_cut::<T>();

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(RawOrigin::Signed(provider_account), user_account.clone());

        /*********** Post-benchmark checks: ***********/
        // Verify that the charge event was emitted.
        let amount_charged: BalanceOf<T> = rate_as_balance
            + (amount_provided_as_balance * CurrentPricePerGigaUnitPerTick::<T>::get()
                / GIGAUNIT.into());
        let charge_event = <T as pallet::Config>::RuntimeEvent::from(Event::PaymentStreamCharged {
            user_account: user_account.clone(),
            provider_id,
            amount: amount_charged,
            last_tick_charged: frame_system::Pallet::<T>::block_number(),
            charged_at_tick: frame_system::Pallet::<T>::block_number(),
        });
        frame_system::Pallet::<T>::assert_has_event(charge_event.into());

        Ok(())
    }

    #[benchmark]
    fn charge_multiple_users_payment_streams(
        n: Linear<0, { <T as crate::Config>::MaxUsersToCharge::get() }>,
    ) -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Set up a Provider with an account with some balance.
        let (provider_account, provider_id) = register_provider::<T>(0)?;
        let provider_id: ProviderIdFor<T> = provider_id.into();

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
        let amount_provided = 100_000u32;
        let amount_provided_as_balance: BalanceOf<T> = amount_provided.into();
        for i in 0..n.into() {
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

        // Worst case scenario: the used bsp capacity is the total capacity when charging the payment streams
        set_up_worst_case_scenario_for_treasury_cut::<T>();

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(RawOrigin::Signed(provider_account), user_accounts.clone());

        /*********** Post-benchmark checks: ***********/
        // Verify that the charge event was emitted for each user
        let amount_charged: BalanceOf<T> = rate_as_balance
            + (amount_provided_as_balance * CurrentPricePerGigaUnitPerTick::<T>::get()
                / GIGAUNIT.into());
        for user_account in user_accounts.iter() {
            let charge_event =
                <T as pallet::Config>::RuntimeEvent::from(Event::PaymentStreamCharged {
                    user_account: user_account.clone(),
                    provider_id,
                    amount: amount_charged,
                    last_tick_charged: frame_system::Pallet::<T>::block_number(),
                    charged_at_tick: frame_system::Pallet::<T>::block_number(),
                });
            frame_system::Pallet::<T>::assert_has_event(charge_event.into());
        }

        Ok(())
    }

    #[benchmark]
    fn pay_outstanding_debt(n: Linear<1, { 1000 }>) -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Set up an account with some balance.
        let user_account: T::AccountId = account("Alice", 0, 0);
        let user_balance = match 1_000_000_000_000_000_000u128.try_into() {
            Ok(balance) => balance,
            Err(_) => return Err(BenchmarkError::Stop("Balance conversion failed.")),
        };
        assert_ok!(<T as crate::Config>::NativeBalance::mint_into(
            &user_account,
            user_balance,
        ));

        // Since we have to create a dynamic-rate and a fixed-rate payment stream per iteration
        // to analyze the worst case scenario, the easiest way is to set up
        // a Provider with an account with some balance per iteration number.
        let n: u32 = n.into();
        let mut provider_ids: Vec<ProviderIdFor<T>> = Vec::new();
        for i in 0..n {
            let (_provider_account, provider_id) = register_provider::<T>(i)?;
            let provider_id: ProviderIdFor<T> = provider_id.into();
            let amount_provided = 1000u32;
            let rate = 100u32;

            // Ensure that a dynamic-rate payment stream between the user and this provider does not exist
            let dynamic_rate_stream =
                DynamicRatePaymentStreams::<T>::get(provider_id, user_account.clone());
            match dynamic_rate_stream {
                Some(_) => {
                    return Err(BenchmarkError::Stop(
                        "Dynamic-rate payment stream already exists.",
                    ));
                }
                None => {}
            }

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

            // Ensure that a fixed-rate payment stream between the user and this provider does not exist
            let fixed_rate_stream =
                FixedRatePaymentStreams::<T>::get(provider_id, user_account.clone());
            match fixed_rate_stream {
                Some(_) => {
                    return Err(BenchmarkError::Stop(
                        "Fixed-rate payment stream already exists.",
                    ));
                }
                None => {}
            }

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

            provider_ids.push(provider_id);
        }

        // Update last chargeable info of each provider to one block ahead
        run_to_block::<T>(frame_system::Pallet::<T>::block_number() + One::one());
        let new_last_chargeable_info = ProviderLastChargeableInfo {
            last_chargeable_tick: frame_system::Pallet::<T>::block_number(),
            price_index: AccumulatedPriceIndex::<T>::get(),
        };
        for provider_id in provider_ids.clone() {
            update_last_chargeable_info::<T>(provider_id, new_last_chargeable_info.clone());
        }

        // Worst case scenario: the used bsp capacity is the total capacity when charging the payment streams
        set_up_worst_case_scenario_for_treasury_cut::<T>();

        // Make the user insolvent
        set_user_as_insolvent::<T>(user_account.clone());

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(RawOrigin::Signed(user_account.clone()), provider_ids);

        /*********** Post-benchmark checks: ***********/
        // Verify that the user paid all debts event was emitted for the user
        let user_paid_debts_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::UserPaidAllDebts {
                who: user_account.clone(),
            });
        frame_system::Pallet::<T>::assert_has_event(user_paid_debts_event.into());

        // Verify that the user has no remaining payment streams
        assert_eq!(RegisteredUsers::<T>::get(user_account), 0);

        Ok(())
    }

    #[benchmark]
    fn clear_insolvent_flag() -> Result<(), BenchmarkError> {
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

        // Make the user insolvent
        set_user_as_insolvent::<T>(user_account.clone());

        // Advance enough blocks to allow the user to clear the insolvent flag
        run_to_block::<T>(
            frame_system::Pallet::<T>::block_number()
                + <T as crate::Config>::UserWithoutFundsCooldown::get(),
        );

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(RawOrigin::Signed(user_account.clone()));

        /*********** Post-benchmark checks: ***********/
        // Verify that the user is no longer marked as insolvent
        assert!(!UsersWithoutFunds::<T>::contains_key(user_account.clone()));

        // Verify that the `UserSolvent` event was emitted
        let user_solvent_event = <T as pallet::Config>::RuntimeEvent::from(Event::UserSolvent {
            who: user_account.clone(),
        });
        frame_system::Pallet::<T>::assert_has_event(user_solvent_event.into());

        Ok(())
    }

    #[benchmark]
    fn price_index_update() {
        let mut meter: WeightMeter = WeightMeter::new();
        let current_price_index = AccumulatedPriceIndex::<T>::get();
        #[block]
        {
            Pallet::<T>::do_update_price_index(&mut meter);
        }
        assert_ne!(current_price_index, AccumulatedPriceIndex::<T>::get());
    }

    #[benchmark]
    fn tick_update() {
        let mut meter: WeightMeter = WeightMeter::new();
        let current_tick = OnPollTicker::<T>::get();
        #[block]
        {
            Pallet::<T>::do_advance_tick(&mut meter);
        }
        assert_ne!(current_tick, OnPollTicker::<T>::get());
    }

    /// This benchmarks the execution of the function `do_update_last_chargeable_info` and its variation
    /// of weight according to how many Providers have to be updated.
    #[benchmark]
    fn update_providers_last_chargeable_info(
        n: Linear<0, { <T as pallet_proofs_dealer::Config>::MaxSubmittersPerTick::get() }>,
    ) -> Result<(), BenchmarkError> {
        use pallet_proofs_dealer::types::ProviderIdFor as ProofsDealerProviderIdFor;
        /***********  Setup initial conditions: ***********/
        // Set up a full weight meter
        let mut meter: WeightMeter = WeightMeter::new();

        // For each provider, set up an account with some balance.
        let mut provider_ids: BoundedBTreeSet<
            ProofsDealerProviderIdFor<T>,
            <T as pallet_proofs_dealer::Config>::MaxSubmittersPerTick,
        > = BoundedBTreeSet::new();
        let mut provider_ids_payment_stream: Vec<ProviderIdFor<T>> = Vec::new();
        for i in 0..n.into() {
            let (_provider_account, provider_id) = register_provider::<T>(i)?;
            provider_ids
                .try_insert(provider_id.into())
                .map_err(|_| BenchmarkError::Stop("Max size of bounded set is accounted for."))?;
            provider_ids_payment_stream.push(provider_id.into());
        }

        // Set up the tickers to simulate a real scenario
        pallet_proofs_dealer::ChallengesTicker::<T>::set(10u32.into());
        LastSubmittersTickRegistered::<T>::set(9u32.into());
        OnPollTicker::<T>::set(20u32.into());

        // Simulate all providers having submitted a valid proof in the current tick
        pallet_proofs_dealer::ValidProofSubmittersLastTicks::<T>::insert(
            pallet_proofs_dealer::ChallengesTicker::<T>::get(),
            provider_ids.clone(),
        );

        // Simulate the `on_poll` hook of the `ProofsDealer` pallet being executed, which
        // increments the challenge ticker.
        pallet_proofs_dealer::ChallengesTicker::<T>::mutate(|ticker| *ticker += 1u32.into());

        // Simulate calling the `on_poll` hook of this pallet, which should increment the tick BUT call the
        // `do_update_last_chargeable_info` with the previous tick (which in this case is 20)
        /*********** Call the function to benchmark: ***********/
        #[block]
        {
            Pallet::<T>::do_update_last_chargeable_info(20u32.into(), &mut meter)
        }

        /*********** Post-benchmark checks: ***********/
        // Verify that the last chargeable info was updated for each provider
        for provider_id in provider_ids_payment_stream.iter() {
            let last_chargeable_info = ProviderLastChargeableInfo {
                last_chargeable_tick: 20u32.into(),
                price_index: AccumulatedPriceIndex::<T>::get(),
            };
            let last_chargeable_info_in_storage = LastChargeableInfo::<T>::get(provider_id);
            assert_eq!(last_chargeable_info, last_chargeable_info_in_storage);
        }

        Ok(())
    }

    impl_benchmark_test_suite! {
            Pallet,
            crate::mock::ExtBuilder::build(),
            crate::mock::Test,
    }
}
