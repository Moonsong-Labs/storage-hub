//! Benchmarking setup for pallet-randomness

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;

#[benchmarks(where
	T: crate::Config,
)]
mod benchmarks {
    use __private::traits::Hooks;
    use frame_system::RawOrigin;
    use sp_runtime::Saturating;

    use super::*;
    use crate::{pallet, Call, Config, Event, Pallet};

    #[benchmark]
    fn set_babe_randomness() {
        /***********  Setup initial conditions: ***********/
        // The worst case scenario for the call is when the block producer has to update the
        // epoch randomness as well.
        // Get the current relay epoch index and make sure it's bigger than 0.
        let relay_epoch_index = <<T as pallet::Config>::BabeDataGetter as GetBabeData<
            u64,
            <T as frame_system::Config>::Hash,
        >>::get_epoch_index();
        assert!(relay_epoch_index > 0);

        // Set the last processed relay epoch to one epoch ago.
        RelayEpoch::<T>::set(relay_epoch_index - 1);

        /*********** Call the extrinsic to benchmark: ***********/
        #[extrinsic_call]
        _(RawOrigin::None);

        /*********** Post-benchmark checks: ***********/
        // Get the last epoch randomness
        let epoch_randomness = <<T as pallet::Config>::BabeDataGetter as GetBabeData<
            u64,
            <T as frame_system::Config>::Hash,
        >>::get_epoch_randomness();

        let latest_valid_block_for_randomness =
            LastRelayBlockAndParaBlockValidForNextEpoch::<T>::get()
                .1
                .saturating_sub(sp_runtime::traits::One::one());

        let expected_event = <T as pallet::Config>::RuntimeEvent::from(
            Event::<T>::NewOneEpochAgoRandomnessAvailable {
                randomness_seed: epoch_randomness,
                from_epoch: relay_epoch_index,
                valid_until_block: latest_valid_block_for_randomness,
            },
        );
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());
    }

    #[benchmark]
    fn on_finalize_hook() {
        /***********  Setup initial conditions: ***********/
        // Set the inherent included.
        InherentIncluded::<T>::put(());

        /*********** Call the function to benchmark: ***********/
        #[block]
        {
            Pallet::<T>::on_finalize(frame_system::Pallet::<T>::block_number());
        }

        /*********** Post-benchmark checks: ***********/
        // Check that the inherent included is removed.
        assert!(InherentIncluded::<T>::get().is_none());
    }

    impl_benchmark_test_suite! {
            Pallet,
            crate::mock::ExtBuilder::build(),
            crate::mock::Test,
    }
}
