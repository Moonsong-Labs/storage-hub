#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
    #[api_version(1)]
    pub trait ProofsDealerApi<ProviderId, BlockNumber, Key, RandomnessOutput, TrieRemoveMutation>
    where
        ProviderId: codec::Codec,
        BlockNumber: codec::Codec,
        Key: codec::Codec,
        RandomnessOutput: codec::Codec,
        TrieRemoveMutation: codec::Codec,
    {
        fn get_last_tick_provider_submitted_proof(provider_id: &ProviderId) -> Result<BlockNumber, GetLastTickProviderSubmittedProofError>;
        fn get_last_checkpoint_challenge_tick() -> BlockNumber;
        fn get_checkpoint_challenges(
            tick: BlockNumber
        ) -> Result<Vec<(Key, Option<TrieRemoveMutation>)>, GetCheckpointChallengesError>;
        fn get_challenge_period(provider_id: &ProviderId) -> Result<BlockNumber, GetChallengePeriodError>;
        fn get_checkpoint_challenge_period() -> BlockNumber;
        fn get_challenges_from_seed(seed: &RandomnessOutput, provider_id: &ProviderId, count: u32) -> Vec<Key>;
        fn get_forest_challenges_from_seed(seed: &RandomnessOutput, provider_id: &ProviderId) -> Vec<Key>;
        fn get_current_tick() -> BlockNumber;
    }
}

/// Error type for the `get_last_tick_provider_submitted_proof` runtime API call.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum GetLastTickProviderSubmittedProofError {
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

/// Error type for the `get_challenge_period` runtime API call.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum GetChallengePeriodError {
    ProviderNotRegistered,
}
