use crate::{configs::SpMinDeposit, Balance, BlockNumber, Perbill, Runtime, NANOUNIT, UNIT};
use frame_support::dynamic_params::{dynamic_pallet_params, dynamic_params};

#[dynamic_params(RuntimeParameters, pallet_parameters::Parameters::<Runtime>)]
pub mod dynamic_params {
    use super::*;
    #[dynamic_pallet_params]
    #[codec(index = 0)]
    pub mod runtime_config {

        use super::*;

        #[codec(index = 0)]
        #[allow(non_upper_case_globals)]
        /// 20 UNITs
        pub static SlashAmountPerMaxFileSize: Balance = 20 * UNIT;

        #[codec(index = 1)]
        #[allow(non_upper_case_globals)]
        /// 10k UNITs * [`MinChallengePeriod`] = 10k UNITs * 30 = 300k UNITs
        ///
        ///  This can be interpreted as "a Provider with 10k UNITs of stake would get the minimum challenge period".
        pub static StakeToChallengePeriod: Balance =
            10_000 * UNIT * Into::<u128>::into(MinChallengePeriod::get());

        #[codec(index = 2)]
        #[allow(non_upper_case_globals)]
        /// The [`CheckpointChallengePeriod`] is set to be equal to the longest possible challenge period
        /// (i.e. the [`StakeToChallengePeriod`] divided by the [`SpMinDeposit`]).
        ///
        /// 300k UNITs / 100 UNITs = 3k ticks (i.e. 5 hours with 6 seconds per tick)
        pub static CheckpointChallengePeriod: BlockNumber = (StakeToChallengePeriod::get()
            / SpMinDeposit::get())
        .try_into()
        .expect(
            "StakeToChallengePeriod / SpMinDeposit should be a number of ticks that can fit in BlockNumber numerical type",
        );

        #[codec(index = 3)]
        #[allow(non_upper_case_globals)]
        /// 30 ticks, or 3 minutes with 6 seconds per tick.
        pub static MinChallengePeriod: BlockNumber = 30;

        #[codec(index = 4)]
        #[allow(non_upper_case_globals)]
        /// Price decreases when system utilisation is below 30%.
        pub static SystemUtilisationLowerThresholdPercentage: Perbill = Perbill::from_percent(30);

        #[codec(index = 5)]
        #[allow(non_upper_case_globals)]
        /// Price increases when system utilisation is above 95%.
        pub static SystemUtilisationUpperThresholdPercentage: Perbill = Perbill::from_percent(95);

        #[codec(index = 6)]
        #[allow(non_upper_case_globals)]
        /// 50 [`NANOUNIT`]s is the price per GB of data, per tick.
        ///
        /// With 6 seconds per tick, this means that over a month, the price of 1 GB is:
        /// 50e-9 [`UNIT`]s * 10 ticks/min * 60 min/h * 24 h/day * 30 days/month = 21.6e-3 [`UNIT`]s
        pub static MostlyStablePrice: Balance = 50 * NANOUNIT;

        #[codec(index = 7)]
        #[allow(non_upper_case_globals)]
        /// [`MostlyStablePrice`] * 10 = 500 [`NANOUNIT`]s
        pub static MaxPrice: Balance = MostlyStablePrice::get() * 10;

        #[codec(index = 8)]
        #[allow(non_upper_case_globals)]
        /// [`MostlyStablePrice`] / 5 = 10 [`NANOUNIT`]s
        pub static MinPrice: Balance = MostlyStablePrice::get() / 5;

        #[codec(index = 9)]
        #[allow(non_upper_case_globals)]
        /// u = [`UpperExponentFactor`]
        /// system_utilisation = 1
        ///
        /// [`MaxPrice`] = [`MostlyStablePrice`] + u * e ^ ( 1 - [`SystemUtilisationUpperThresholdPercentage`] )
        ///
        /// 500 = 50 + u * (e ^ (1 - 0.95) - 1)
        /// u = (500 - 50) / (e ^ (1 - 0.95) - 1) ≈ 8777
        pub static UpperExponentFactor: u32 = 8777;

        #[codec(index = 10)]
        #[allow(non_upper_case_globals)]
        /// l = [`LowerExponentFactor`]
        /// system_utilisation = 0
        ///
        /// [`MinPrice`] = [`MostlyStablePrice`] - u * e ^ ( [`SystemUtilisationLowerThresholdPercentage`] - 0 )
        ///
        /// 10 = 50 - l * (e ^ (0.3 - 0) - 1)
        /// l = (50 - 10) / (e ^ (0.3 - 0) - 1) ≈ 114
        pub static LowerExponentFactor: u32 = 114;

        #[codec(index = 11)]
        #[allow(non_upper_case_globals)]
        /// 0-size bucket fixed rate payment stream representing the price for 1 GB of data.
        ///
        /// Base rate for a new fixed payment stream established between an MSP and a user.
        pub static ZeroSizeBucketFixedRate: Balance = 50 * NANOUNIT;

        #[codec(index = 12)]
        #[allow(non_upper_case_globals)]
        /// Ideal utilisation rate of the system
        pub static IdealUtilisationRate: Perbill = Perbill::from_percent(85);

        #[codec(index = 13)]
        #[allow(non_upper_case_globals)]
        /// Decay rate of the power of two function that determines the percentage of funds that go to
        /// the treasury for utilisation rates greater than the ideal.
        pub static DecayRate: Perbill = Perbill::from_percent(5);

        #[codec(index = 14)]
        #[allow(non_upper_case_globals)]
        /// The minimum treasury cut that can be taken from the amount charged from a payment stream.
        pub static MinimumTreasuryCut: Perbill = Perbill::from_percent(1);

        #[codec(index = 15)]
        #[allow(non_upper_case_globals)]
        /// The maximum treasury cut that can be taken from the amount charged from a payment stream.
        pub static MaximumTreasuryCut: Perbill = Perbill::from_percent(5);

        #[codec(index = 16)]
        #[allow(non_upper_case_globals)]
        /// The penalty a BSP must pay when they forcefully stop storing a file.
        /// We set this to be half of the `SlashAmountPerMaxFileSize` with the rationale that
        /// for a BSP that has lost this file, it should be more convenient to voluntarily
        /// show up and pay this penalty in good faith, rather than risking being slashed for
        /// being unable to submit a proof that should include this file.
        pub static BspStopStoringFilePenalty: Balance = SlashAmountPerMaxFileSize::get() / 2;

        #[codec(index = 17)]
        #[allow(non_upper_case_globals)]
        /// 20 ticks, or 2 minutes with 6 seconds per tick.
        pub static MinSeedPeriod: BlockNumber = 20;

        #[codec(index = 18)]
        #[allow(non_upper_case_globals)]
        /// 10k UNITs * [`MinSeedPeriod`] = 10k UNITs * 20 = 200k UNITs
        ///
        ///  This can be interpreted as "a Provider with 10k UNITs of stake would get the minimum seed period".
        pub static StakeToSeedPeriod: Balance =
            10_000 * UNIT * Into::<u128>::into(MinSeedPeriod::get());
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
