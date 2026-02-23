#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

sp_api::decl_runtime_apis! {
    #[api_version(1)]
    pub trait ProofsDealerApi<ProviderId, BlockNumber, Key, RandomnessOutput, CustomChallenge>
    where
        ProviderId: codec::Codec,
        BlockNumber: codec::Codec,
        Key: codec::Codec,
        RandomnessOutput: codec::Codec,
        CustomChallenge: codec::Codec,
    {
        fn get_last_tick_provider_submitted_proof(provider_id: &ProviderId) -> Result<BlockNumber, GetProofSubmissionRecordError>;
        fn get_next_tick_to_submit_proof_for(provider_id: &ProviderId) -> Result<BlockNumber, GetProofSubmissionRecordError>;
        fn get_last_checkpoint_challenge_tick() -> BlockNumber;
        fn get_checkpoint_challenges(
            tick: BlockNumber
        ) -> Result<Vec<CustomChallenge>, GetCheckpointChallengesError>;
        fn get_challenge_seed(tick: BlockNumber) -> Result<RandomnessOutput, GetChallengeSeedError>;
        fn get_challenge_period(provider_id: &ProviderId) -> Result<BlockNumber, GetChallengePeriodError>;
        fn get_checkpoint_challenge_period() -> BlockNumber;
        fn get_challenges_from_seed(seed: &RandomnessOutput, provider_id: &ProviderId, count: u32) -> Vec<Key>;
        fn get_forest_challenges_from_seed(seed: &RandomnessOutput, provider_id: &ProviderId) -> Vec<Key>;
        fn get_current_tick() -> BlockNumber;
        fn get_next_deadline_tick(provider_id: &ProviderId) -> Result<BlockNumber, GetNextDeadlineTickError>;
    }
}

/// Error type for the `get_last_tick_provider_submitted_proof` and `get_next_tick_to_submit_proof_for` runtime API calls.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum GetProofSubmissionRecordError {
    ProviderNotRegistered,
    ProviderNeverSubmittedProof,
    InternalApiError,
}

/// Error type for the `get_checkpoint_challenges` runtime API call.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum GetCheckpointChallengesError {
    TickGreaterThanLastCheckpointTick,
    NoCheckpointChallengesInTick,
    InternalApiError,
}

/// Error type for the `get_challenge_seed` runtime API call.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum GetChallengeSeedError {
    TickBeyondLastSeedStored,
    TickIsInTheFuture,
    InternalApiError,
}

/// Error type for the `get_challenge_period` runtime API call.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum GetChallengePeriodError {
    ProviderNotRegistered,
    InternalApiError,
}

/// Error type for the `get_next_deadline_tick` runtime API call.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum GetNextDeadlineTickError {
    ProviderNotRegistered,
    ProviderNotInitialised,
    ArithmeticOverflow,
    InternalApiError,
}
