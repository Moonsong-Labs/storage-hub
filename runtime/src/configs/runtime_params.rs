use crate::{
    configs::{ChallengeTicksTolerance, ReplicationTargetType, SpMinDeposit},
    Balance, BlockNumber, Perbill, Runtime, NANOUNIT, UNIT,
};
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
        // 300k UNITs / 100 UNITs + 50 + 1 = ~3k ticks (i.e. ~5 hours with 6 seconds per tick)
        pub static CheckpointChallengePeriod: BlockNumber = (StakeToChallengePeriod::get()
            / SpMinDeposit::get()).saturating_add(ChallengeTicksTolerance::get() as u128).saturating_add(1)
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

        /// Time-to-live for a provider to top up their deposit to cover a capacity deficit.
        /// Set to 14_400 relay blocks = 1 day with 6 second timeslots.
        #[codec(index = 17)]
        #[allow(non_upper_case_globals)]
        pub static ProviderTopUpTtl: BlockNumber = 14_400;

        /// The default number of BSPs that a storage request targets when issued by the file system pallet.
        ///
        /// This number is a safe default so users that do not have specific requirements for replication
        /// can still have their files stored in a way that is resilient to failures, while being small enough
        /// to not be a burden on the network.
        #[codec(index = 18)]
        #[allow(non_upper_case_globals)]
        pub static DefaultReplicationTarget: ReplicationTargetType = 7;

        /// The maximum amount of BSPs that a user can require a storage request to use as the replication target.
        ///
        /// This is a safety measure to prevent users from issuing storage requests that are too large and would
        /// require a large number of BSPs to store the file.
        #[codec(index = 19)]
        #[allow(non_upper_case_globals)]
        pub static MaxReplicationTarget: ReplicationTargetType =
            3 * DefaultReplicationTarget::get();

        /// The amount of ticks that have to pass for the threshold to volunteer for a specific storage request
        /// to arrive at its maximum value.
        ///
        /// This is big enough so volunteering for a storage request is not open to everyone inmediatly, preventing
        /// a select few BSPs from taking all the requests, while small enough so that storage requests don't take
        /// too long to be filled.
        #[codec(index = 20)]
        #[allow(non_upper_case_globals)]
        pub static TickRangeToMaximumThreshold: BlockNumber = 3600; // 6 hours with a 6 second block time

        /// The amount of ticks after which a storage request is considered expired and can be removed from storage.
        ///
        /// It's a function of the TickRangeToMaximumThreshold since it does not make sense for a storage request to
        /// expire before arriving at its maximum threshold for volunteering.
        #[codec(index = 21)]
        #[allow(non_upper_case_globals)]
        pub static StorageRequestTtl: BlockNumber = TickRangeToMaximumThreshold::get()
            .saturating_mul(110)
            .saturating_div(100);

        /// The minimum amount of ticks between a stop storing request from a BSP and that BSP being able to
        /// confirm to stop storing that file key.
        ///
        /// It's a function of the checkpoint challenge period since this makes it so BSPs can't avoid checkpoint
        /// challenges by stopping storing a file key right before the challenge period ends in case they lost it.
        #[codec(index = 22)]
        #[allow(non_upper_case_globals)]
        pub static MinWaitForStopStoring: BlockNumber = CheckpointChallengePeriod::get()
            .saturating_mul(110)
            .saturating_div(100);

        #[codec(index = 23)]
        #[allow(non_upper_case_globals)]
        /// 20 ticks, or 2 minutes with 6 seconds per tick.
        pub static MinSeedPeriod: BlockNumber = 20;

        #[codec(index = 24)]
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
