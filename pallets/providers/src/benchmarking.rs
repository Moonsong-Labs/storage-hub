//! Benchmarking setup for pallet-storage-providers

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;

#[benchmarks(where T: crate::Config + pallet_randomness::Config)]
mod benchmarks {
    use frame_support::{
        assert_ok,
        traits::{
            fungible::{Inspect, InspectHold, Mutate},
            Get,
        },
        BoundedVec,
    };
    use frame_system::{pallet_prelude::BlockNumberFor, RawOrigin};
    use sp_runtime::{
        format,
        traits::{Hash, One},
    };

    use super::*;
    use crate::{pallet, types::*, Call, Config, Event, Pallet};

    type BalanceOf<T> = <<T as crate::Config>::NativeBalance as Inspect<
        <T as frame_system::Config>::AccountId,
    >>::Balance;

    fn run_to_block<T: crate::Config>(n: BlockNumberFor<T>) {
        assert!(
            n > frame_system::Pallet::<T>::block_number(),
            "Cannot go back in time"
        );

        while frame_system::Pallet::<T>::block_number() < n {
            frame_system::Pallet::<T>::set_block_number(
                frame_system::Pallet::<T>::block_number() + One::one(),
            );
        }
    }

    fn register_provider<T: crate::Config>(
        index: u32,
    ) -> Result<(T::AccountId, T::ProviderId), BenchmarkError> {
        let sp_account: T::AccountId = account("SP", index, 0);
        let sp_id_seed = format!("benchmark_sp_{}", index);
        let sp_id = <<T as crate::Config>::ProviderIdHashing as Hash>::hash(sp_id_seed.as_bytes());
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
        if AccountIdToBackupStorageProviderId::<T>::contains_key(&sp_account) {
            return Err(BenchmarkError::Stop("Provider account already in use."));
        }

        // Make sure the sp_id is not already in use
        if BackupStorageProviders::<T>::contains_key(&sp_id) {
            return Err(BenchmarkError::Stop("Provider ID already in use."));
        }

        AccountIdToBackupStorageProviderId::<T>::insert(&sp_account, sp_id);
        BackupStorageProviders::<T>::insert(
            &sp_id,
            BackupStorageProvider {
                capacity: initial_capacity.into(),
                capacity_used: Default::default(),
                multiaddresses,
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: sp_account.clone(),
                payment_account: sp_account.clone(),
                reputation_weight: <T as crate::Config>::StartingReputationWeight::get(),
                sign_up_block: Default::default(),
            },
        );

        Ok((sp_account, sp_id))
    }

    #[benchmark]
    fn request_msp_sign_up() -> Result<(), BenchmarkError> {
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

        // Setup the parameters of the MSP to register
        let capacity = 100000u32;
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
        let value_prop_price_per_unit_of_data_per_block = 1u32;
        let commitment: BoundedVec<u8, <T as crate::Config>::MaxCommitmentSize> =
            vec![1, 2, 3].try_into().unwrap();
        let value_prop_max_data_limit = 100u32;
        let payment_account = user_account.clone();

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(
            RawOrigin::Signed(user_account.clone()),
            capacity.into(),
            multiaddresses.clone(),
            value_prop_price_per_unit_of_data_per_block.into(),
            commitment,
            value_prop_max_data_limit.into(),
            payment_account,
        );

        /*********** Post-benchmark checks: ***********/
        // Verify that the event of the MSP requesting to sign up was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::MspRequestSignUpSuccess {
                who: user_account.clone(),
                capacity: capacity.into(),
                multiaddresses: multiaddresses.clone(),
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the funds were held from the MSP's account for the provider's deposit
        let held_funds = <T as crate::Config>::NativeBalance::balance_on_hold(
            &HoldReason::StorageProviderDeposit.into(),
            &user_account,
        );
        assert!(held_funds > 0u32.into());

        // Verify that the request to sign up exists in storage
        let msp_sign_up_request = SignUpRequests::<T>::get(user_account);
        assert!(msp_sign_up_request.is_some());
        let sign_up_info = msp_sign_up_request.unwrap().sp_sign_up_request;
        match sign_up_info {
            SignUpRequestSpParams::MainStorageProvider(sign_up_request_params) => {
                let msp_info = sign_up_request_params.msp_info;
                assert_eq!(msp_info.capacity, capacity.into());
                assert_eq!(msp_info.multiaddresses, multiaddresses);
            }
            SignUpRequestSpParams::BackupStorageProvider(_) => {
                return Err(BenchmarkError::Stop(
                    "Expected MainStorageProvider sign up request.",
                ));
            }
        }

        Ok(())
    }

    #[benchmark]
    fn request_bsp_sign_up() -> Result<(), BenchmarkError> {
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

        // Setup the parameters of the BSP to register
        let capacity = 100000u32;
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
        let payment_account = user_account.clone();

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(
            RawOrigin::Signed(user_account.clone()),
            capacity.into(),
            multiaddresses.clone(),
            payment_account,
        );

        /*********** Post-benchmark checks: ***********/
        // Verify that the event of the BSP requesting to sign up was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::BspRequestSignUpSuccess {
                who: user_account.clone(),
                capacity: capacity.into(),
                multiaddresses: multiaddresses.clone(),
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the funds were held from the BSP's account for the provider's deposit
        let held_funds = <T as crate::Config>::NativeBalance::balance_on_hold(
            &HoldReason::StorageProviderDeposit.into(),
            &user_account,
        );
        assert!(held_funds > 0u32.into());

        // Verify that the request to sign up exists in storage
        let bsp_sign_up_request = SignUpRequests::<T>::get(user_account);
        assert!(bsp_sign_up_request.is_some());
        let sign_up_info = bsp_sign_up_request.unwrap().sp_sign_up_request;
        match sign_up_info {
            SignUpRequestSpParams::MainStorageProvider(_) => {
                return Err(BenchmarkError::Stop(
                    "Expected BackupStorageProvider sign up request.",
                ));
            }
            SignUpRequestSpParams::BackupStorageProvider(bsp_info) => {
                assert_eq!(bsp_info.capacity, capacity.into());
                assert_eq!(bsp_info.multiaddresses, multiaddresses);
            }
        }

        Ok(())
    }

    #[benchmark]
    fn confirm_sign_up_bsp() -> Result<(), BenchmarkError> {
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

        // Setup the parameters of the BSP to register
        let capacity = 100000u32;
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
        let payment_account = user_account.clone();

        // Request the sign up of the BSP
        Pallet::<T>::request_bsp_sign_up(
            RawOrigin::Signed(user_account.clone()).into(),
            capacity.into(),
            multiaddresses.clone(),
            payment_account,
        )
        .map_err(|_| BenchmarkError::Stop("Failed to request BSP sign up."))?;

        // Verify that the event of the BSP requesting to sign up was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::BspRequestSignUpSuccess {
                who: user_account.clone(),
                capacity: capacity.into(),
                multiaddresses: multiaddresses.clone(),
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Advance enough blocks to set up a valid random seed
        let random_seed = <T as frame_system::Config>::Hashing::hash(b"random_seed");
        run_to_block::<T>(10u32.into());
        pallet_randomness::LatestOneEpochAgoRandomness::<T>::set(Some((
            random_seed,
            frame_system::Pallet::<T>::block_number(),
        )));

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        confirm_sign_up(RawOrigin::Signed(user_account.clone()), None);

        /*********** Post-benchmark checks: ***********/
        // Verify that the event of the sign up confirmation was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::BspSignUpSuccess {
                who: user_account.clone(),
                bsp_id: AccountIdToBackupStorageProviderId::<T>::get(&user_account).unwrap(),
                capacity: capacity.into(),
                multiaddresses: multiaddresses.clone(),
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the BSP is now in the providers' storage
        let bsp_id = AccountIdToBackupStorageProviderId::<T>::get(&user_account).unwrap();
        let bsp = BackupStorageProviders::<T>::get(&bsp_id);
        assert!(bsp.is_some());

        Ok(())
    }

    #[benchmark(extra)]
    fn confirm_sign_up_msp() -> Result<(), BenchmarkError> {
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

        // Setup the parameters of the MSP to register
        let capacity = 100000u32;
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
        let value_prop_price_per_unit_of_data_per_block = 1u32;
        let commitment: BoundedVec<u8, <T as crate::Config>::MaxCommitmentSize> =
            vec![1, 2, 3].try_into().unwrap();
        let value_prop_max_data_limit = 100u32;
        let payment_account = user_account.clone();

        // Request the sign up of the MSP
        Pallet::<T>::request_msp_sign_up(
            RawOrigin::Signed(user_account.clone()).into(),
            capacity.into(),
            multiaddresses.clone(),
            value_prop_price_per_unit_of_data_per_block.into(),
            commitment.clone(),
            value_prop_max_data_limit.into(),
            payment_account,
        )
        .map_err(|_| BenchmarkError::Stop("Failed to request MSP sign up."))?;

        // Verify that the event of the MSP requesting to sign up was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::MspRequestSignUpSuccess {
                who: user_account.clone(),
                capacity: 100000u32.into(),
                multiaddresses: multiaddresses.clone(),
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Advance enough blocks to set up a valid random seed
        let random_seed = <T as frame_system::Config>::Hashing::hash(b"random_seed");
        run_to_block::<T>(10u32.into());
        pallet_randomness::LatestOneEpochAgoRandomness::<T>::set(Some((
            random_seed,
            frame_system::Pallet::<T>::block_number(),
        )));

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        confirm_sign_up(RawOrigin::Signed(user_account.clone()), None);

        /*********** Post-benchmark checks: ***********/
        // Verify that the event of the sign up confirmation was emitted
        let value_prop = ValueProposition::<T>::new(
            value_prop_price_per_unit_of_data_per_block.into(),
            commitment,
            value_prop_max_data_limit.into(),
        );
        let value_prop_with_id = ValuePropositionWithId::<T> {
            id: value_prop.derive_id(),
            value_prop: value_prop.clone(),
        };
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::MspSignUpSuccess {
                who: user_account.clone(),
                msp_id: AccountIdToMainStorageProviderId::<T>::get(&user_account).unwrap(),
                capacity: 100000u32.into(),
                multiaddresses: multiaddresses.clone(),
                value_prop: value_prop_with_id,
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the MSP is now in the providers' storage
        let msp_id = AccountIdToMainStorageProviderId::<T>::get(&user_account).unwrap();
        let msp = MainStorageProviders::<T>::get(&msp_id);
        assert!(msp.is_some());

        Ok(())
    }

    #[benchmark]
    fn cancel_sign_up() -> Result<(), BenchmarkError> {
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

        // Setup the parameters of the BSP to register
        let capacity = 100000u32;
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
        let payment_account = user_account.clone();

        // Request the sign up of the BSP
        Pallet::<T>::request_bsp_sign_up(
            RawOrigin::Signed(user_account.clone()).into(),
            capacity.into(),
            multiaddresses.clone(),
            payment_account,
        )
        .map_err(|_| BenchmarkError::Stop("Failed to request BSP sign up."))?;

        // Verify that the event of the BSP requesting to sign up was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::BspRequestSignUpSuccess {
                who: user_account.clone(),
                capacity: capacity.into(),
                multiaddresses: multiaddresses.clone(),
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(RawOrigin::Signed(user_account.clone()));

        /*********** Post-benchmark checks: ***********/
        // Verify that the event of the sign up cancellation was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::SignUpRequestCanceled {
                who: user_account.clone(),
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the request to sign up was removed from storage
        let bsp_sign_up_request = SignUpRequests::<T>::get(user_account.clone());
        assert!(bsp_sign_up_request.is_none());

        // And that the deposit was returned
        let held_funds = <T as crate::Config>::NativeBalance::balance_on_hold(
            &HoldReason::StorageProviderDeposit.into(),
            &user_account,
        );
        assert_eq!(held_funds, 0u32.into());

        Ok(())
    }

    #[benchmark]
    fn msp_sign_off() -> Result<(), BenchmarkError> {
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

        // Setup the parameters of the MSP to register
        let capacity = 100000u32;
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
        let value_prop_price_per_unit_of_data_per_block = 1u32;
        let commitment: BoundedVec<u8, <T as crate::Config>::MaxCommitmentSize> =
            vec![1, 2, 3].try_into().unwrap();
        let value_prop_max_data_limit = 100u32;
        let payment_account = user_account.clone();

        // Request the sign up of the MSP
        Pallet::<T>::request_msp_sign_up(
            RawOrigin::Signed(user_account.clone()).into(),
            capacity.into(),
            multiaddresses.clone(),
            value_prop_price_per_unit_of_data_per_block.into(),
            commitment.clone(),
            value_prop_max_data_limit.into(),
            payment_account,
        )
        .map_err(|_| BenchmarkError::Stop("Failed to request MSP sign up."))?;

        // Verify that the event of the MSP requesting to sign up was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::MspRequestSignUpSuccess {
                who: user_account.clone(),
                capacity: 100000u32.into(),
                multiaddresses: multiaddresses.clone(),
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Advance enough blocks to set up a valid random seed
        let random_seed = <T as frame_system::Config>::Hashing::hash(b"random_seed");
        run_to_block::<T>(10u32.into());
        pallet_randomness::LatestOneEpochAgoRandomness::<T>::set(Some((
            random_seed,
            frame_system::Pallet::<T>::block_number(),
        )));

        // Confirm the sign up of the MSP
        Pallet::<T>::confirm_sign_up(RawOrigin::Signed(user_account.clone()).into(), None);

        // Verify that the MSP is now in the providers' storage
        let msp_id = AccountIdToMainStorageProviderId::<T>::get(&user_account).unwrap();
        let msp = MainStorageProviders::<T>::get(&msp_id);
        assert!(msp.is_some());

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(RawOrigin::Signed(user_account.clone()));

        /*********** Post-benchmark checks: ***********/
        // Verify that the event of the MSP sign off was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::MspSignOffSuccess {
                who: user_account.clone(),
                msp_id,
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the MSP was removed from the providers' storage
        let msp = MainStorageProviders::<T>::get(&msp_id);
        assert!(msp.is_none());

        Ok(())
    }

    impl_benchmark_test_suite! {
            Pallet,
            crate::mock::ExtBuilder::build(),
            crate::mock::Test,
    }
}
