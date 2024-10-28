use crate::storagehub::{configs::SpMinDeposit, Balance, BlockNumber, Runtime, UNIT};
use frame_support::dynamic_params::{dynamic_pallet_params, dynamic_params};
use sp_runtime::Perquintill;

#[dynamic_params(RuntimeParameters, pallet_parameters::Parameters::<Runtime>)]
pub mod dynamic_params {
    use super::*;
    #[dynamic_pallet_params]
    #[codec(index = 0)]
    pub mod runtime_config {
        use super::*;

        #[codec(index = 0)]
        #[allow(non_upper_case_globals)]
        pub static SlashAmountPerMaxFileSize: Balance = 20 * UNIT;

        #[codec(index = 1)]
        #[allow(non_upper_case_globals)]
        // This can be interpreted as "a Provider with 10k UNITs of stake would get the minimum challenge period".
        pub static StakeToChallengePeriod: Balance =
            10_000 * UNIT * Into::<u128>::into(MinChallengePeriod::get()); // 300k UNITs

        #[codec(index = 2)]
        #[allow(non_upper_case_globals)]
        // The CheckpointChallengePeriod is set to be equal to the longest possible challenge period (i.e. the
        // StakeToChallengePeriod divided by the SpMinDeposit).
        pub static CheckpointChallengePeriod: BlockNumber = (StakeToChallengePeriod::get()
            / SpMinDeposit::get()) // 300k UNITs / 100 UNITs = 3k ticks (i.e. 5 hours with 6 seconds per tick)
        .try_into()
        .expect(
            "StakeToChallengePeriod / SpMinDeposit should be a number of ticks that can fit in BlockNumber numerical type",
        );

        #[codec(index = 3)]
        #[allow(non_upper_case_globals)]
        pub static MinChallengePeriod: BlockNumber = 30;

        #[codec(index = 11)]
        #[allow(non_upper_case_globals)]
        /// Ideal utilisation rate of the system
        pub static IdealUtilisationRate: Perquintill = Perquintill::from_percent(85);

        #[codec(index = 12)]
        #[allow(non_upper_case_globals)]
        /// Decay rate of the power of two function that determines the percentage of funds that go to
        /// the treasury for utilisation rates greater than the ideal.
        pub static DecayRate: Perquintill = Perquintill::from_percent(5);

        #[codec(index = 13)]
        #[allow(non_upper_case_globals)]
        /// The minimum treasury cut that can be taken from the amount charged from a payment stream.
        pub static MinimumTreasuryCut: Perquintill = Perquintill::from_percent(1);

        #[codec(index = 14)]
        #[allow(non_upper_case_globals)]
        /// The maximum treasury cut that can be taken from the amount charged from a payment stream.
        pub static MaximumTreasuryCut: Perquintill = Perquintill::from_percent(5);
    }
}

#[cfg(feature = "runtime-benchmarks")]
impl Default for RuntimeParameters {
    fn default() -> Self {
        RuntimeParameters::RuntimeConfig(
            dynamic_params::runtime_config::Parameters::SlashAmountPerMaxFileSize(
                dynamic_params::runtime_config::SlashAmountPerMaxFileSize,
                Some(20 * UNIT),
            ),
        )
    }
}
