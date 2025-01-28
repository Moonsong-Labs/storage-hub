use crate::storagehub::{
    configs::{ChallengeTicksTolerance, ReplicationTargetType, SpMinDeposit},
    Balance, BlockNumber, Runtime, UNIT,
};
use frame_support::dynamic_params::{dynamic_pallet_params, dynamic_params};
use sp_runtime::Perbill;

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
        // 300k UNITs / 100 UNITs + 50 + 1 = ~3k ticks (i.e. ~5 hours with 6 seconds per tick)
        pub static CheckpointChallengePeriod: BlockNumber = (StakeToChallengePeriod::get()
            / SpMinDeposit::get()).saturating_add(ChallengeTicksTolerance::get() as u128).saturating_add(1)
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
        pub static IdealUtilisationRate: Perbill = Perbill::from_percent(85);

        #[codec(index = 12)]
        #[allow(non_upper_case_globals)]
        /// Decay rate of the power of two function that determines the percentage of funds that go to
        /// the treasury for utilisation rates greater than the ideal.
        pub static DecayRate: Perbill = Perbill::from_percent(5);

        #[codec(index = 13)]
        #[allow(non_upper_case_globals)]
        /// The minimum treasury cut that can be taken from the amount charged from a payment stream.
        pub static MinimumTreasuryCut: Perbill = Perbill::from_percent(1);

        #[codec(index = 14)]
        #[allow(non_upper_case_globals)]
        /// The maximum treasury cut that can be taken from the amount charged from a payment stream.
        pub static MaximumTreasuryCut: Perbill = Perbill::from_percent(5);

        /// The minimum amount of ticks between a stop storing request from a BSP and that BSP being able to
        /// confirm to stop storing that file key.
        ///
        /// It's a function of the checkpoint challenge period since this makes it so BSPs can't avoid checkpoint
        /// challenges by stopping storing a file key right before the challenge period ends in case they lost it.
        #[codec(index = 15)]
        #[allow(non_upper_case_globals)]
        pub static MinWaitForStopStoring: BlockNumber = CheckpointChallengePeriod::get()
            .saturating_mul(110)
            .saturating_div(100);

        /// The amount of BSPs that a basic security storage request should use as the replication target.
        ///
        /// This must be the lowest amount of BSPs that guarantee that the probability that a malicious
        /// actor controlling 1/3 of the BSPs can hold the file hostage by controlling all its
        /// volunteered BSPs is ~1%.
        #[codec(index = 18)]
        #[allow(non_upper_case_globals)]
        pub static BasicReplicationTarget: ReplicationTargetType = 7;

        /// The amount of BSPs that a standard security storage request should use as the replication target.
        ///
        /// This must be the lowest amount of BSPs that guarantee that the probability that a malicious
        /// actor controlling 1/3 of the BSPs can hold the file hostage by controlling all its
        /// volunteered BSPs is ~0.1%.
        #[codec(index = 19)]
        #[allow(non_upper_case_globals)]
        pub static StandardReplicationTarget: ReplicationTargetType = 12;

        /// The amount of BSPs that a high security storage request should use as the replication target.
        ///
        /// This must be the lowest amount of BSPs that guarantee that the probability that a malicious
        /// actor controlling 1/3 of the BSPs can hold the file hostage by controlling all its
        /// volunteered BSPs is ~0.01%.
        #[codec(index = 20)]
        #[allow(non_upper_case_globals)]
        pub static HighSecurityReplicationTarget: ReplicationTargetType = 17;

        /// The amount of BSPs that a super high security storage request should use as the replication target.
        ///
        /// This must be the lowest amount of BSPs that guarantee that the probability that a malicious
        /// actor controlling 1/3 of the BSPs can hold the file hostage by controlling all its
        /// volunteered BSPs is ~0.001%.
        #[codec(index = 21)]
        #[allow(non_upper_case_globals)]
        pub static SuperHighSecurityReplicationTarget: ReplicationTargetType = 22;

        /// The amount of BSPs that an ultra high security storage request should use as the replication target.
        ///
        /// This must be the lowest amount of BSPs that guarantee that the probability that a malicious
        /// actor controlling 1/3 of the BSPs can hold the file hostage by controlling all its
        /// volunteered BSPs is ~0.0001%.
        #[codec(index = 22)]
        #[allow(non_upper_case_globals)]
        pub static UltraHighSecurityReplicationTarget: ReplicationTargetType = 26;

        /// The maximum amount of BSPs that a user can require a storage request to use as the replication target.
        ///
        /// This is a safety measure to prevent users from issuing storage requests that are too large and would
        /// require a large number of BSPs to store the file.
        #[codec(index = 23)]
        #[allow(non_upper_case_globals)]
        pub static MaxReplicationTarget: ReplicationTargetType =
            UltraHighSecurityReplicationTarget::get()
                .saturating_mul(150)
                .saturating_div(100);

        /// The amount of ticks that have to pass for the threshold to volunteer for a specific storage request
        /// to arrive at its maximum value.
        ///
        /// This is big enough so volunteering for a storage request is not open to everyone inmediatly, preventing
        /// a select few BSPs from taking all the requests, while small enough so that storage requests don't take
        /// too long to be filled.
        #[codec(index = 24)]
        #[allow(non_upper_case_globals)]
        pub static TickRangeToMaximumThreshold: BlockNumber = 3600; // 6 hours with a 6 second block time

        /// The amount of ticks after which a storage request is considered expired and can be removed from storage.
        ///
        /// It's a function of the TickRangeToMaximumThreshold since it does not make sense for a storage request to
        /// expire before arriving at its maximum threshold for volunteering.
        #[codec(index = 25)]
        #[allow(non_upper_case_globals)]
        pub static StorageRequestTtl: BlockNumber = TickRangeToMaximumThreshold::get()
            .saturating_mul(110)
            .saturating_div(100);

        #[codec(index = 26)]
        #[allow(non_upper_case_globals)]
        /// 20 ticks, or 2 minutes with 6 seconds per tick.
        pub static MinSeedPeriod: BlockNumber = 20;

        #[codec(index = 27)]
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
