use crate::{configs::SpMinDeposit, Balance, BlockNumber, Perbill, Runtime, PICOUNIT, UNIT};
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
        /// 48 [`PICOUNIT`]s is the price per MB of data, per tick.
        ///
        /// With 6 seconds per tick, this means that over a month, the price of 1 GB is:
        /// 48e-12 * 10 ticks/min * 60 min/h * 24 h/day * 30 days/month * 1024 MB/GB = 21.23e-3 [`UNIT`]s
        pub static MostlyStablePrice: Balance = 48 * PICOUNIT;

        #[codec(index = 7)]
        #[allow(non_upper_case_globals)]
        /// [`MostlyStablePrice`] * 10 = 480 [`PICOUNIT`]s
        pub static MaxPrice: Balance = MostlyStablePrice::get() * 10;

        #[codec(index = 8)]
        #[allow(non_upper_case_globals)]
        /// [`MostlyStablePrice`] / 5 = 9 [`PICOUNIT`]s
        pub static MinPrice: Balance = MostlyStablePrice::get() / 5;

        #[codec(index = 9)]
        #[allow(non_upper_case_globals)]
        /// u = [`UpperExponentFactor`]
        /// system_utilisation = 1
        ///
        /// [`MaxPrice`] = [`MostlyStablePrice`] + u * e ^ ( 1 - [`SystemUtilisationUpperThresholdPercentage`] )
        ///
        /// 480 = 48 + u * (e ^ (1 - 0.95) - 1)
        /// u = (480 - 48) / (e ^ (1 - 0.95) - 1) ≈ 8426
        pub static UpperExponentFactor: u32 = 8426;

        #[codec(index = 10)]
        #[allow(non_upper_case_globals)]
        /// l = [`LowerExponentFactor`]
        /// system_utilisation = 0
        ///
        /// [`MinPrice`] = [`MostlyStablePrice`] - u * e ^ ( [`SystemUtilisationLowerThresholdPercentage`] - 0 )
        ///
        /// 9 = 48 - l * (e ^ (0.3 - 0) - 1)
        /// l = (48 - 9) / (e ^ (0.3 - 0) - 1) ≈ 111
        pub static LowerExponentFactor: u32 = 111;
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
