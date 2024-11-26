//! Benchmarking setup for pallet-storage-providers

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;

pub trait BenchmarkHelpers<T: crate::Config> {
    type ProviderId: From<<T as crate::Config>::ProviderId>;
    fn set_accrued_failed_proofs(provider_id: Self::ProviderId, value: u32);
    fn get_accrued_failed_proofs(provider_id: Self::ProviderId) -> u32;
}

impl<T: crate::Config> BenchmarkHelpers<T> for () {
    type ProviderId = <T as crate::Config>::ProviderId;
    fn set_accrued_failed_proofs(_provider_id: Self::ProviderId, _value: u32) {}
    fn get_accrued_failed_proofs(_provider_id: Self::ProviderId) -> u32 {
        0
    }
}

#[benchmarks(where
	T: crate::Config + pallet_randomness::Config
)]
mod benchmarks {
    use frame_support::{
        assert_ok,
        traits::{
            fungible::{Inspect, InspectHold, Mutate},
            tokens::Fortitude,
            Get,
        },
        BoundedVec,
    };
    use frame_system::{pallet_prelude::BlockNumberFor, RawOrigin};
    use sp_runtime::traits::{Bounded, Hash, Zero};
    use sp_std::vec;

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

        frame_system::Pallet::<T>::set_block_number(frame_system::Pallet::<T>::block_number() + n);
    }

    #[benchmark]
    fn request_msp_sign_up() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Make sure the block number is not 0 so events can be deposited.
        if frame_system::Pallet::<T>::block_number() == Zero::zero() {
            run_to_block::<T>(1u32.into());
        }

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
        let value_prop_price_per_unit_of_data_per_block = BalanceOf::<T>::max_value();
        let commitment: BoundedVec<u8, <T as crate::Config>::MaxCommitmentSize> = vec![
                1;
                <T as crate::Config>::MaxCommitmentSize::get()
                    .try_into()
                    .unwrap()
            ]
        .try_into()
        .unwrap();
        let value_prop_max_data_limit = StorageDataUnit::<T>::max_value();
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
        // Make sure the block number is not 0 so events can be deposited.
        if frame_system::Pallet::<T>::block_number() == Zero::zero() {
            run_to_block::<T>(1u32.into());
        }

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
        // Make sure the block number is not 0 so events can be deposited.
        if frame_system::Pallet::<T>::block_number() == Zero::zero() {
            run_to_block::<T>(1u32.into());
        }

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
                root: T::DefaultMerkleRoot::get(),
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the BSP is now in the providers' storage
        let bsp_id = AccountIdToBackupStorageProviderId::<T>::get(&user_account).unwrap();
        let bsp = BackupStorageProviders::<T>::get(&bsp_id);
        assert!(bsp.is_some());

        Ok(())
    }

    #[benchmark]
    fn confirm_sign_up_msp() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Make sure the block number is not 0 so events can be deposited.
        if frame_system::Pallet::<T>::block_number() == Zero::zero() {
            run_to_block::<T>(1u32.into());
        }

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
        // Make sure the block number is not 0 so events can be deposited.
        if frame_system::Pallet::<T>::block_number() == Zero::zero() {
            run_to_block::<T>(1u32.into());
        }

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
        // Make sure the block number is not 0 so events can be deposited.
        if frame_system::Pallet::<T>::block_number() == Zero::zero() {
            run_to_block::<T>(1u32.into());
        }

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
        Pallet::<T>::confirm_sign_up(RawOrigin::Signed(user_account.clone()).into(), None)
            .map_err(|_| BenchmarkError::Stop("Failed to confirm MSP sign up."))?;

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

    #[benchmark]
    fn bsp_sign_off() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Make sure the block number is not 0 so events can be deposited.
        if frame_system::Pallet::<T>::block_number() == Zero::zero() {
            run_to_block::<T>(1u32.into());
        }

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

        // Confirm the sign up of the BSP
        Pallet::<T>::confirm_sign_up(RawOrigin::Signed(user_account.clone()).into(), None)
            .map_err(|_| BenchmarkError::Stop("Failed to confirm BSP sign up."))?;

        // Verify that the BSP is now in the providers' storage
        let bsp_id = AccountIdToBackupStorageProviderId::<T>::get(&user_account).unwrap();
        let bsp = BackupStorageProviders::<T>::get(&bsp_id);
        assert!(bsp.is_some());

        // Advance enough blocks to allow the BSP to sign off
        run_to_block::<T>(
            frame_system::Pallet::<T>::block_number()
                + <T as crate::Config>::BspSignUpLockPeriod::get(),
        );

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(RawOrigin::Signed(user_account.clone()));

        /*********** Post-benchmark checks: ***********/
        // Verify that the event of the BSP sign off was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::BspSignOffSuccess {
                who: user_account.clone(),
                bsp_id,
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the BSP was removed from the providers' storage
        let bsp = MainStorageProviders::<T>::get(&bsp_id);
        assert!(bsp.is_none());

        Ok(())
    }

    #[benchmark]
    fn change_capacity_bsp_less_deposit() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Make sure the block number is not 0 so events can be deposited.
        if frame_system::Pallet::<T>::block_number() == Zero::zero() {
            run_to_block::<T>(1u32.into());
        }

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
        let initial_capacity = 100000u32;
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
            initial_capacity.into(),
            multiaddresses.clone(),
            payment_account,
        )
        .map_err(|_| BenchmarkError::Stop("Failed to request BSP sign up."))?;

        // Verify that the event of the BSP requesting to sign up was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::BspRequestSignUpSuccess {
                who: user_account.clone(),
                capacity: initial_capacity.into(),
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

        // Confirm the sign up of the BSP
        Pallet::<T>::confirm_sign_up(RawOrigin::Signed(user_account.clone()).into(), None)
            .map_err(|_| BenchmarkError::Stop("Failed to confirm BSP sign up."))?;

        // Verify that the BSP is now in the providers' storage
        let bsp_id = AccountIdToBackupStorageProviderId::<T>::get(&user_account).unwrap();
        let bsp = BackupStorageProviders::<T>::get(&bsp_id);
        assert!(bsp.is_some());

        // Get the current deposit of the BSP
        let initial_deposit = <T as crate::Config>::NativeBalance::balance_on_hold(
            &HoldReason::StorageProviderDeposit.into(),
            &user_account,
        );

        // Advance enough blocks to allow the BSP to change its capacity
        run_to_block::<T>(
            frame_system::Pallet::<T>::block_number()
                + <T as crate::Config>::MinBlocksBetweenCapacityChanges::get(),
        );

        // Make the new capacity less than the previous one so part of the deposit has to be released
        let new_capacity = 50000u32;

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        change_capacity(RawOrigin::Signed(user_account.clone()), new_capacity.into());

        /*********** Post-benchmark checks: ***********/
        // Verify that the event of the capacity change was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::CapacityChanged {
                who: user_account.clone(),
                provider_id: StorageProviderId::BackupStorageProvider(bsp_id),
                old_capacity: initial_capacity.into(),
                new_capacity: new_capacity.into(),
                next_block_when_change_allowed: frame_system::Pallet::<T>::block_number()
                    + <T as crate::Config>::MinBlocksBetweenCapacityChanges::get(),
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the capacity was changed
        let bsp = BackupStorageProviders::<T>::get(&bsp_id).unwrap();
        assert_eq!(bsp.capacity, new_capacity.into());

        // And that part of the deposit was released
        let current_deposit = <T as crate::Config>::NativeBalance::balance_on_hold(
            &HoldReason::StorageProviderDeposit.into(),
            &user_account,
        );
        assert!(current_deposit < initial_deposit);

        Ok(())
    }

    #[benchmark]
    fn change_capacity_bsp_more_deposit() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Make sure the block number is not 0 so events can be deposited.
        if frame_system::Pallet::<T>::block_number() == Zero::zero() {
            run_to_block::<T>(1u32.into());
        }

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
        let initial_capacity = 100000u32;
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
            initial_capacity.into(),
            multiaddresses.clone(),
            payment_account,
        )
        .map_err(|_| BenchmarkError::Stop("Failed to request BSP sign up."))?;

        // Verify that the event of the BSP requesting to sign up was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::BspRequestSignUpSuccess {
                who: user_account.clone(),
                capacity: initial_capacity.into(),
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

        // Confirm the sign up of the BSP
        Pallet::<T>::confirm_sign_up(RawOrigin::Signed(user_account.clone()).into(), None)
            .map_err(|_| BenchmarkError::Stop("Failed to confirm BSP sign up."))?;

        // Verify that the BSP is now in the providers' storage
        let bsp_id = AccountIdToBackupStorageProviderId::<T>::get(&user_account).unwrap();
        let bsp = BackupStorageProviders::<T>::get(&bsp_id);
        assert!(bsp.is_some());

        // Get the current deposit of the BSP
        let initial_deposit = <T as crate::Config>::NativeBalance::balance_on_hold(
            &HoldReason::StorageProviderDeposit.into(),
            &user_account,
        );

        // Advance enough blocks to allow the BSP to change its capacity
        run_to_block::<T>(
            frame_system::Pallet::<T>::block_number()
                + <T as crate::Config>::MinBlocksBetweenCapacityChanges::get(),
        );

        // Make the new capacity more than the previous one so funds have to be held
        let new_capacity = 150000u32;

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        change_capacity(RawOrigin::Signed(user_account.clone()), new_capacity.into());

        /*********** Post-benchmark checks: ***********/
        // Verify that the event of the capacity change was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::CapacityChanged {
                who: user_account.clone(),
                provider_id: StorageProviderId::BackupStorageProvider(bsp_id),
                old_capacity: initial_capacity.into(),
                new_capacity: new_capacity.into(),
                next_block_when_change_allowed: frame_system::Pallet::<T>::block_number()
                    + <T as crate::Config>::MinBlocksBetweenCapacityChanges::get(),
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the capacity was changed
        let bsp = BackupStorageProviders::<T>::get(&bsp_id).unwrap();
        assert_eq!(bsp.capacity, new_capacity.into());

        // And that more deposit was held
        let current_deposit = <T as crate::Config>::NativeBalance::balance_on_hold(
            &HoldReason::StorageProviderDeposit.into(),
            &user_account,
        );
        assert!(current_deposit > initial_deposit);

        Ok(())
    }

    #[benchmark]
    fn change_capacity_msp_less_deposit() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Make sure the block number is not 0 so events can be deposited.
        if frame_system::Pallet::<T>::block_number() == Zero::zero() {
            run_to_block::<T>(1u32.into());
        }

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
        let initial_capacity = 100000u32;
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
            initial_capacity.into(),
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
                capacity: initial_capacity.into(),
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
        Pallet::<T>::confirm_sign_up(RawOrigin::Signed(user_account.clone()).into(), None)
            .map_err(|_| BenchmarkError::Stop("Failed to confirm MSP sign up."))?;

        // Verify that the MSP is now in the providers' storage
        let msp_id = AccountIdToMainStorageProviderId::<T>::get(&user_account).unwrap();
        let msp = MainStorageProviders::<T>::get(&msp_id);
        assert!(msp.is_some());

        // Get the current deposit of the MSP
        let initial_deposit = <T as crate::Config>::NativeBalance::balance_on_hold(
            &HoldReason::StorageProviderDeposit.into(),
            &user_account,
        );

        // Advance enough blocks to allow the MSP to change its capacity
        run_to_block::<T>(
            frame_system::Pallet::<T>::block_number()
                + <T as crate::Config>::MinBlocksBetweenCapacityChanges::get(),
        );

        // Make the new capacity less than the previous one so part of the deposit has to be released
        let new_capacity = 50000u32;

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        change_capacity(RawOrigin::Signed(user_account.clone()), new_capacity.into());

        /*********** Post-benchmark checks: ***********/
        // Verify that the event of the capacity change was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::CapacityChanged {
                who: user_account.clone(),
                provider_id: StorageProviderId::MainStorageProvider(msp_id),
                old_capacity: initial_capacity.into(),
                new_capacity: new_capacity.into(),
                next_block_when_change_allowed: frame_system::Pallet::<T>::block_number()
                    + <T as crate::Config>::MinBlocksBetweenCapacityChanges::get(),
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the capacity was changed
        let msp = MainStorageProviders::<T>::get(&msp_id).unwrap();
        assert_eq!(msp.capacity, new_capacity.into());

        // And that part of the deposit was released
        let current_deposit = <T as crate::Config>::NativeBalance::balance_on_hold(
            &HoldReason::StorageProviderDeposit.into(),
            &user_account,
        );
        assert!(current_deposit < initial_deposit);

        Ok(())
    }

    #[benchmark]
    fn change_capacity_msp_more_deposit() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Make sure the block number is not 0 so events can be deposited.
        if frame_system::Pallet::<T>::block_number() == Zero::zero() {
            run_to_block::<T>(1u32.into());
        }

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
        let initial_capacity = 100000u32;
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
            initial_capacity.into(),
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
                capacity: initial_capacity.into(),
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
        Pallet::<T>::confirm_sign_up(RawOrigin::Signed(user_account.clone()).into(), None)
            .map_err(|_| BenchmarkError::Stop("Failed to confirm MSP sign up."))?;

        // Verify that the MSP is now in the providers' storage
        let msp_id = AccountIdToMainStorageProviderId::<T>::get(&user_account).unwrap();
        let msp = MainStorageProviders::<T>::get(&msp_id);
        assert!(msp.is_some());

        // Get the current deposit of the MSP
        let initial_deposit = <T as crate::Config>::NativeBalance::balance_on_hold(
            &HoldReason::StorageProviderDeposit.into(),
            &user_account,
        );

        // Advance enough blocks to allow the MSP to change its capacity
        run_to_block::<T>(
            frame_system::Pallet::<T>::block_number()
                + <T as crate::Config>::MinBlocksBetweenCapacityChanges::get(),
        );

        // Make the new capacity more than the previous one so funds have to be held
        let new_capacity = 150000u32;

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        change_capacity(RawOrigin::Signed(user_account.clone()), new_capacity.into());

        /*********** Post-benchmark checks: ***********/
        // Verify that the event of the capacity change was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::CapacityChanged {
                who: user_account.clone(),
                provider_id: StorageProviderId::MainStorageProvider(msp_id),
                old_capacity: initial_capacity.into(),
                new_capacity: new_capacity.into(),
                next_block_when_change_allowed: frame_system::Pallet::<T>::block_number()
                    + <T as crate::Config>::MinBlocksBetweenCapacityChanges::get(),
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the capacity was changed
        let msp = MainStorageProviders::<T>::get(&msp_id).unwrap();
        assert_eq!(msp.capacity, new_capacity.into());

        // And that more deposit was held
        let current_deposit = <T as crate::Config>::NativeBalance::balance_on_hold(
            &HoldReason::StorageProviderDeposit.into(),
            &user_account,
        );
        assert!(current_deposit > initial_deposit);

        Ok(())
    }

    #[benchmark]
    fn add_value_prop() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Make sure the block number is not 0 so events can be deposited.
        if frame_system::Pallet::<T>::block_number() == Zero::zero() {
            run_to_block::<T>(1u32.into());
        }

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
        let initial_capacity = 100000u32;
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
            initial_capacity.into(),
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
                capacity: initial_capacity.into(),
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
        Pallet::<T>::confirm_sign_up(RawOrigin::Signed(user_account.clone()).into(), None)
            .map_err(|_| BenchmarkError::Stop("Failed to confirm MSP sign up."))?;

        // Verify that the MSP is now in the providers' storage
        let msp_id = AccountIdToMainStorageProviderId::<T>::get(&user_account).unwrap();
        let msp = MainStorageProviders::<T>::get(&msp_id);
        assert!(msp.is_some());

        // Setup the parameters of the value proposition to add. Since the extrinsic has to derive the ID
        // by concatenating and then hashing the encoded parameters, to get the worst case scenario we make
        // this parameters as big as possible.
        let value_prop_price_per_unit_of_data_per_block = BalanceOf::<T>::max_value();
        let commitment: BoundedVec<u8, <T as crate::Config>::MaxCommitmentSize> = vec![
                1;
                <T as crate::Config>::MaxCommitmentSize::get()
                    .try_into()
                    .unwrap()
            ]
        .try_into()
        .unwrap();
        let value_prop_max_data_limit = StorageDataUnit::<T>::max_value();

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(
            RawOrigin::Signed(user_account.clone()),
            value_prop_price_per_unit_of_data_per_block.into(),
            commitment.clone(),
            value_prop_max_data_limit.into(),
        );

        /*********** Post-benchmark checks: ***********/
        // Verify that the event of the value proposition addition was emitted
        let value_prop = ValueProposition::<T>::new(
            value_prop_price_per_unit_of_data_per_block.into(),
            commitment.clone(),
            value_prop_max_data_limit.into(),
        );
        let value_prop_id = value_prop.derive_id();
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::ValuePropAdded {
                msp_id,
                value_prop_id,
                value_prop: value_prop.clone(),
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the value proposition was added
        let value_prop_in_storage =
            MainStorageProviderIdsToValuePropositions::<T>::get(&msp_id, &value_prop_id);
        assert!(value_prop_in_storage.is_some());
        assert_eq!(value_prop_in_storage.unwrap(), value_prop);

        Ok(())
    }

    #[benchmark]
    fn make_value_prop_unavailable() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Make sure the block number is not 0 so events can be deposited.
        if frame_system::Pallet::<T>::block_number() == Zero::zero() {
            run_to_block::<T>(1u32.into());
        }

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
        let initial_capacity = 100000u32;
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
            initial_capacity.into(),
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
                capacity: initial_capacity.into(),
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
        Pallet::<T>::confirm_sign_up(RawOrigin::Signed(user_account.clone()).into(), None)
            .map_err(|_| BenchmarkError::Stop("Failed to confirm MSP sign up."))?;

        // Verify that the MSP is now in the providers' storage
        let msp_id = AccountIdToMainStorageProviderId::<T>::get(&user_account).unwrap();
        let msp = MainStorageProviders::<T>::get(&msp_id);
        assert!(msp.is_some());

        // Setup the parameters of the value proposition to add.
        let value_prop_price_per_unit_of_data_per_block: BalanceOf<T> = 1u32.into();
        let commitment: BoundedVec<u8, <T as crate::Config>::MaxCommitmentSize> =
            vec![3, 2, 1].try_into().unwrap();
        let value_prop_max_data_limit: T::StorageDataUnit = 100u32.into();

        // Add the value proposition to the MSP
        Pallet::<T>::add_value_prop(
            RawOrigin::Signed(user_account.clone()).into(),
            value_prop_price_per_unit_of_data_per_block.into(),
            commitment.clone(),
            value_prop_max_data_limit.into(),
        )
        .map_err(|_| BenchmarkError::Stop("Failed to add value proposition."))?;

        // Verify that the event of the value proposition addition was emitted
        let value_prop = ValueProposition::<T>::new(
            value_prop_price_per_unit_of_data_per_block.into(),
            commitment.clone(),
            value_prop_max_data_limit.into(),
        );
        let value_prop_id = value_prop.derive_id();
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::ValuePropAdded {
                msp_id,
                value_prop_id,
                value_prop,
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(RawOrigin::Signed(user_account.clone()), value_prop_id);

        /*********** Post-benchmark checks: ***********/
        // Verify that the event of the value proposition being made unavailable was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::ValuePropUnavailable {
                msp_id,
                value_prop_id,
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the value proposition was indeed made unavailable
        let value_prop_in_storage =
            MainStorageProviderIdsToValuePropositions::<T>::get(&msp_id, &value_prop_id);
        assert!(value_prop_in_storage.is_some());
        assert_eq!(value_prop_in_storage.unwrap().available, false);

        Ok(())
    }

    #[benchmark]
    fn add_multiaddress() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Make sure the block number is not 0 so events can be deposited.
        if frame_system::Pallet::<T>::block_number() == Zero::zero() {
            run_to_block::<T>(1u32.into());
        }

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
        // (we register a BSP since the extrinsic first checks if the account is a MSP, so
        // the worst case scenario is for the provider to be a BSP)
        let initial_capacity = 100000u32;
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
            initial_capacity.into(),
            multiaddresses.clone(),
            payment_account,
        )
        .map_err(|_| BenchmarkError::Stop("Failed to request BSP sign up."))?;

        // Verify that the event of the BSP requesting to sign up was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::BspRequestSignUpSuccess {
                who: user_account.clone(),
                capacity: initial_capacity.into(),
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

        // Confirm the sign up of the BSP
        Pallet::<T>::confirm_sign_up(RawOrigin::Signed(user_account.clone()).into(), None)
            .map_err(|_| BenchmarkError::Stop("Failed to confirm BSP sign up."))?;

        // Verify that the BSP is now in the providers' storage
        let bsp_id = AccountIdToBackupStorageProviderId::<T>::get(&user_account).unwrap();
        let bsp = BackupStorageProviders::<T>::get(&bsp_id);
        assert!(bsp.is_some());

        // Setup the multiaddress to add. The worst case scenario is to make it as big as possible since
        // it has to be copied to storage.
        let new_multiaddress: MultiAddress<T> = vec![
            1;
            <T as crate::Config>::MaxMultiAddressSize::get(
            )
            .try_into()
            .unwrap()
        ]
        .try_into()
        .unwrap();

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(
            RawOrigin::Signed(user_account.clone()),
            new_multiaddress.clone(),
        );

        /*********** Post-benchmark checks: ***********/
        // Verify that the event of the added multiaddress was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::MultiAddressAdded {
                provider_id: bsp_id,
                new_multiaddress: new_multiaddress.clone(),
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the multiaddress was added to the BSP
        let bsp = BackupStorageProviders::<T>::get(&bsp_id).unwrap();
        assert!(bsp.multiaddresses.contains(&new_multiaddress));

        Ok(())
    }

    #[benchmark]
    fn remove_multiaddress() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Make sure the block number is not 0 so events can be deposited.
        if frame_system::Pallet::<T>::block_number() == Zero::zero() {
            run_to_block::<T>(1u32.into());
        }

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
        // (we register a BSP since the extrinsic first checks if the account is a MSP, so
        // the worst case scenario is for the provider to be a BSP)
        let initial_capacity = 100000u32;
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
            initial_capacity.into(),
            multiaddresses.clone(),
            payment_account,
        )
        .map_err(|_| BenchmarkError::Stop("Failed to request BSP sign up."))?;

        // Verify that the event of the BSP requesting to sign up was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::BspRequestSignUpSuccess {
                who: user_account.clone(),
                capacity: initial_capacity.into(),
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

        // Confirm the sign up of the BSP
        Pallet::<T>::confirm_sign_up(RawOrigin::Signed(user_account.clone()).into(), None)
            .map_err(|_| BenchmarkError::Stop("Failed to confirm BSP sign up."))?;

        // Verify that the BSP is now in the providers' storage
        let bsp_id = AccountIdToBackupStorageProviderId::<T>::get(&user_account).unwrap();
        let bsp = BackupStorageProviders::<T>::get(&bsp_id);
        assert!(bsp.is_some());

        // Since the extrinsic iterates over all the provider's multiaddresses to find the one to delete, we fill
        // the provider with the maximum amount of multiaddresses and try to delete the last one.
        for i in 0..<T as crate::Config>::MaxMultiAddressAmount::get() - 1 {
            let new_multiaddress: MultiAddress<T> = vec![
				i as u8;
				<T as crate::Config>::MaxMultiAddressSize::get()
				.try_into()
				.unwrap()
			]
            .try_into()
            .unwrap();
            Pallet::<T>::add_multiaddress(
                RawOrigin::Signed(user_account.clone()).into(),
                new_multiaddress.clone(),
            )
            .map_err(|_| BenchmarkError::Stop("Failed to add multiaddress."))?;
            // Verify that the multiaddress was added to the BSP
            let bsp = BackupStorageProviders::<T>::get(&bsp_id).unwrap();
            assert!(bsp.multiaddresses.contains(&new_multiaddress));
        }

        // Setup the multiaddress to remove.
        let multiaddress_to_remove: MultiAddress<T> = vec![
			(<T as crate::Config>::MaxMultiAddressAmount::get() - 2) as u8;
			<T as crate::Config>::MaxMultiAddressSize::get()
			.try_into()
			.unwrap()
		]
        .try_into()
        .unwrap();

        // Make sure the multiaddress to remove is present in the provider
        let bsp = BackupStorageProviders::<T>::get(&bsp_id).unwrap();
        assert!(bsp.multiaddresses.contains(&multiaddress_to_remove));

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(
            RawOrigin::Signed(user_account.clone()),
            multiaddress_to_remove.clone(),
        );

        /*********** Post-benchmark checks: ***********/
        // Verify that the event of removing a multiaddress was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::MultiAddressRemoved {
                provider_id: bsp_id,
                removed_multiaddress: multiaddress_to_remove.clone(),
            });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the multiaddress is no longer present in the BSP
        let bsp = BackupStorageProviders::<T>::get(&bsp_id).unwrap();
        assert!(!bsp.multiaddresses.contains(&multiaddress_to_remove));

        Ok(())
    }

    #[benchmark]
    fn force_msp_sign_up() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Make sure the block number is not 0 so events can be deposited.
        if frame_system::Pallet::<T>::block_number() == Zero::zero() {
            run_to_block::<T>(1u32.into());
        }

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
        let msp_id_seed = "benchmark_force_msp";
        let msp_id =
            <<T as crate::Config>::ProviderIdHashing as Hash>::hash(msp_id_seed.as_bytes());
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
        let value_prop_price_per_unit_of_data_per_block = BalanceOf::<T>::max_value();
        let commitment: BoundedVec<u8, <T as crate::Config>::MaxCommitmentSize> = vec![
                1;
                <T as crate::Config>::MaxCommitmentSize::get()
                    .try_into()
                    .unwrap()
            ]
        .try_into()
        .unwrap();
        let value_prop_max_data_limit = StorageDataUnit::<T>::max_value();
        let payment_account = user_account.clone();

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(
            RawOrigin::Root,
            user_account.clone(),
            msp_id,
            capacity.into(),
            multiaddresses.clone(),
            value_prop_price_per_unit_of_data_per_block.into(),
            commitment.clone(),
            value_prop_max_data_limit.into(),
            payment_account,
        );

        /*********** Post-benchmark checks: ***********/
        // Verify that the event of the MSP requesting to sign up was emitted
        let msp_request_sign_up_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::MspRequestSignUpSuccess {
                who: user_account.clone(),
                capacity: capacity.into(),
                multiaddresses: multiaddresses.clone(),
            });
        frame_system::Pallet::<T>::assert_has_event(msp_request_sign_up_event.into());

        // Verify that the event of the MSP actually signing up was emitted
        let msp_sign_up_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::MspSignUpSuccess {
                who: user_account.clone(),
                msp_id: msp_id,
                multiaddresses: multiaddresses.clone(),
                capacity: capacity.into(),
                value_prop: ValuePropositionWithId::<T>::build(
                    value_prop_price_per_unit_of_data_per_block.into(),
                    commitment.clone(),
                    value_prop_max_data_limit.into(),
                ),
            });
        frame_system::Pallet::<T>::assert_has_event(msp_sign_up_event.into());

        // Verify that the MSP is now in the providers' storage
        let msp = MainStorageProviders::<T>::get(&msp_id);
        assert!(msp.is_some());

        Ok(())
    }

    #[benchmark]
    fn force_bsp_sign_up() -> Result<(), BenchmarkError> {
        /***********  Setup initial conditions: ***********/
        // Make sure the block number is not 0 so events can be deposited.
        if frame_system::Pallet::<T>::block_number() == Zero::zero() {
            run_to_block::<T>(1u32.into());
        }

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
        let bsp_seed = "benchmark_force_bsp";
        let bsp_id = <<T as crate::Config>::ProviderIdHashing as Hash>::hash(bsp_seed.as_bytes());
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
            RawOrigin::Root,
            user_account.clone(),
            bsp_id,
            capacity.into(),
            multiaddresses.clone(),
            payment_account,
            None,
        );

        /*********** Post-benchmark checks: ***********/
        // Verify that the event of the BSP requesting to sign up was emitted
        let bsp_request_sign_up_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::BspRequestSignUpSuccess {
                who: user_account.clone(),
                capacity: capacity.into(),
                multiaddresses: multiaddresses.clone(),
            });
        frame_system::Pallet::<T>::assert_has_event(bsp_request_sign_up_event.into());

        // Verify that the event of the BSP actually signing up was emitted
        let bsp_sign_up_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::BspSignUpSuccess {
                who: user_account.clone(),
                bsp_id: bsp_id,
                multiaddresses: multiaddresses.clone(),
                capacity: capacity.into(),
                root: T::DefaultMerkleRoot::get(),
            });
        frame_system::Pallet::<T>::assert_has_event(bsp_sign_up_event.into());

        // Verify that the BSP is now in the providers' storage
        let bsp = BackupStorageProviders::<T>::get(&bsp_id);
        assert!(bsp.is_some());

        Ok(())
    }

    #[benchmark]
    fn slash() -> Result<(), BenchmarkError> {
        // TODO: once provider sign off is implemented for providers that run out of stake,
        // add a benchmark to check which is the worst case scenario for this extrinsic
        /***********  Setup initial conditions: ***********/
        // Make sure the block number is not 0 so events can be deposited.
        if frame_system::Pallet::<T>::block_number() == Zero::zero() {
            run_to_block::<T>(1u32.into());
        }

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
        // (we register a BSP since the extrinsic first checks if the account is a MSP, so
        // the worst case scenario is for the provider to be a BSP)
        let initial_capacity = 100000u32;
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
            initial_capacity.into(),
            multiaddresses.clone(),
            payment_account,
        )
        .map_err(|_| BenchmarkError::Stop("Failed to request BSP sign up."))?;

        // Verify that the event of the BSP requesting to sign up was emitted
        let expected_event =
            <T as pallet::Config>::RuntimeEvent::from(Event::<T>::BspRequestSignUpSuccess {
                who: user_account.clone(),
                capacity: initial_capacity.into(),
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

        // Confirm the sign up of the BSP
        Pallet::<T>::confirm_sign_up(RawOrigin::Signed(user_account.clone()).into(), None)
            .map_err(|_| BenchmarkError::Stop("Failed to confirm BSP sign up."))?;

        // Verify that the BSP is now in the providers' storage
        let bsp_id = AccountIdToBackupStorageProviderId::<T>::get(&user_account).unwrap();
        let bsp = BackupStorageProviders::<T>::get(&bsp_id);
        assert!(bsp.is_some());

        // Accrue failed proof submissions for this provider
        <T as crate::Config>::BenchmarkHelpers::set_accrued_failed_proofs(bsp_id.into(), 3);

        // Get the amount to be slashed
        let amount_to_slash = Pallet::<T>::compute_worst_case_scenario_slashable_amount(&bsp_id)
            .map_err(|_| {
                BenchmarkError::Stop("Failed to compute worst case scenario slashable amount.")
            })?;

        // The amount to be slashed should be greater than 0
        assert!(amount_to_slash > Zero::zero());

        // The amount slashed will be the minimum between the amount to slash and the available
        // funds to slash of the provider
        let provider_stake = <T as pallet::Config>::NativeBalance::balance_on_hold(
            &HoldReason::StorageProviderDeposit.into(),
            &user_account,
        );
        let liquid_held_provider_funds =
            <T as pallet::Config>::NativeBalance::reducible_total_balance_on_hold(
                &user_account,
                Fortitude::Polite,
            );
        let amount_to_slash = amount_to_slash
            .min(provider_stake)
            .min(liquid_held_provider_funds);
        assert!(amount_to_slash <= provider_stake);

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(RawOrigin::Signed(user_account.clone()), bsp_id);

        /*********** Post-benchmark checks: ***********/
        // Verify that the event of the provider being slashed was emitted
        let expected_event = <T as pallet::Config>::RuntimeEvent::from(Event::<T>::Slashed {
            provider_id: bsp_id,
            amount: amount_to_slash,
        });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the accrued failed proof submissions have been cleared
        let accrued_failed_proofs =
            <T as crate::Config>::BenchmarkHelpers::get_accrued_failed_proofs(bsp_id.into());
        assert!(accrued_failed_proofs == 0);

        Ok(())
    }

    impl_benchmark_test_suite! {
            Pallet,
            crate::mock::ExtBuilder::build(),
            crate::mock::Test,
    }
}
