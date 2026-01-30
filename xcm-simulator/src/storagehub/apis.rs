use crate::storagehub::*;
use crate::*;
use alloc::{collections::BTreeMap, vec::Vec};
use frame_support::{
    genesis_builder_helper::{build_state, get_preset},
    weights::Weight,
};
use pallet_aura::Authorities;
use pallet_file_system::types::StorageRequestMetadata;
use pallet_file_system_runtime_api::*;
use pallet_payment_streams_runtime_api::*;
use pallet_proofs_dealer::types::{
    CustomChallenge, KeyFor, ProviderIdFor as ProofsDealerProviderIdFor, RandomnessOutputFor,
};
use pallet_proofs_dealer_runtime_api::*;
use pallet_storage_providers::types::{
    BackupStorageProvider, BackupStorageProviderId, BucketId, MainStorageProviderId,
    Multiaddresses, ProviderIdFor, StorageDataUnit, StorageProviderId, ValuePropositionWithId,
};
use pallet_storage_providers_runtime_api::*;
use shp_file_metadata::ChunkId;
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata, H256};
use sp_runtime::{
    traits::Block as BlockT,
    transaction_validity::{TransactionSource, TransactionValidity},
    ApplyExtrinsicResult, ExtrinsicInclusionMode,
};
use sp_version::RuntimeVersion;
use xcm::Version;
use xcm_runtime_apis::{
    dry_run::{CallDryRunEffects, Error as XcmDryRunApiError, XcmDryRunEffects},
    fees::Error as XcmPaymentApiError,
};

// Local module imports
use super::{
    AccountId, Balance, Block, Executive, InherentDataExt, Nonce, ParachainSystem, Runtime,
    RuntimeCall, RuntimeGenesisConfig, SessionKeys, System, TransactionPayment, VERSION,
};

impl_runtime_apis! {
    /// Allows the collator client to query its runtime to determine whether it should author a block
    impl cumulus_primitives_aura::AuraUnincludedSegmentApi<Block> for Runtime {
        fn can_build_upon(
            included_hash: <Block as BlockT>::Hash,
            slot: cumulus_primitives_aura::Slot,
        ) -> bool {
            storagehub::configs::ConsensusHook::can_build_upon(included_hash, slot)
        }
    }

    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> sp_consensus_aura::SlotDuration {
            sp_consensus_aura::SlotDuration::from_millis(storagehub::SLOT_DURATION)
        }

        fn authorities() -> Vec<AuraId> {
            Authorities::<Runtime>::get().into_inner()
        }
    }

    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block)
        }

        fn initialize_block(header: &<Block as BlockT>::Header) -> ExtrinsicInclusionMode {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            OpaqueMetadata::new(Runtime::metadata().into())
        }

        fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
            Runtime::metadata_at_version(version)
        }

        fn metadata_versions() -> alloc::vec::Vec<u32> {
            Runtime::metadata_versions()
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(
            block: Block,
            data: sp_inherents::InherentData,
        ) -> sp_inherents::CheckInherentsResult {
            data.check_extrinsics(&block)
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
            block_hash: <Block as BlockT>::Hash,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx, block_hash)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            SessionKeys::generate(seed)
        }

        fn decode_session_keys(
            encoded: Vec<u8>,
        ) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
            SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
        fn account_nonce(account: AccountId) -> Nonce {
            System::account_nonce(account)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
        fn query_info(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }
        fn query_fee_details(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment::FeeDetails<Balance> {
            TransactionPayment::query_fee_details(uxt, len)
        }
        fn query_weight_to_fee(weight: Weight) -> Balance {
            TransactionPayment::weight_to_fee(weight)
        }
        fn query_length_to_fee(length: u32) -> Balance {
            TransactionPayment::length_to_fee(length)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
        for Runtime
    {
        fn query_call_info(
            call: RuntimeCall,
            len: u32,
        ) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_call_info(call, len)
        }
        fn query_call_fee_details(
            call: RuntimeCall,
            len: u32,
        ) -> pallet_transaction_payment::FeeDetails<Balance> {
            TransactionPayment::query_call_fee_details(call, len)
        }
        fn query_weight_to_fee(weight: Weight) -> Balance {
            TransactionPayment::weight_to_fee(weight)
        }
        fn query_length_to_fee(length: u32) -> Balance {
            TransactionPayment::length_to_fee(length)
        }
    }

    impl xcm_runtime_apis::fees::XcmPaymentApi<Block> for Runtime {
        fn query_acceptable_payment_assets(xcm_version: xcm::Version) -> Result<Vec<VersionedAssetId>, XcmPaymentApiError> {

            let acceptable_assets = vec![AssetId(configs::xcm_config::RelayLocation::get())];
            PolkadotXcm::query_acceptable_payment_assets(xcm_version, acceptable_assets)
        }
        fn query_weight_to_asset_fee(weight: Weight, asset: VersionedAssetId) -> Result<u128, XcmPaymentApiError> {
            match asset.try_as::<AssetId>() {
                Ok(asset_id) if asset_id.0 == configs::xcm_config::RelayLocation::get() => {
                    // for native token
                    Ok(<WeightToFee as sp_weights::WeightToFee>::weight_to_fee(&weight))
                },
                Ok(asset_id) => {
                    log::trace!(target: "xcm::xcm_runtime_apis", "query_weight_to_asset_fee - unhandled asset_id: {asset_id:?}!");
                    Err(XcmPaymentApiError::AssetNotFound)
                },
                Err(_) => {
                    log::trace!(target: "xcm::xcm_runtime_apis", "query_weight_to_asset_fee - failed to convert asset: {asset:?}!");
                    Err(XcmPaymentApiError::VersionedConversionFailed)
                }
            }
        }
        fn query_xcm_weight(message: VersionedXcm<()>) -> Result<Weight, XcmPaymentApiError> {
            PolkadotXcm::query_xcm_weight(message)
        }
        fn query_delivery_fees(destination: VersionedLocation, message: VersionedXcm<()>) -> Result<VersionedAssets, XcmPaymentApiError> {
            PolkadotXcm::query_delivery_fees(destination, message)
        }
    }

    impl xcm_runtime_apis::dry_run::DryRunApi<Block, RuntimeCall, RuntimeEvent, OriginCaller> for Runtime {
        fn dry_run_call(origin: OriginCaller, call: RuntimeCall, result_xcms_version: Version) -> Result<CallDryRunEffects<RuntimeEvent>, XcmDryRunApiError> {
            PolkadotXcm::dry_run_call::<Runtime, configs::xcm_config::XcmRouter, OriginCaller, RuntimeCall>(origin, call, result_xcms_version)
        }
        fn dry_run_xcm(origin_location: VersionedLocation, xcm: VersionedXcm<RuntimeCall>) -> Result<XcmDryRunEffects<RuntimeEvent>, XcmDryRunApiError> {
            PolkadotXcm::dry_run_xcm::<Runtime, configs::xcm_config::XcmRouter, RuntimeCall, configs::xcm_config::XcmConfig>(origin_location, xcm)
        }
    }

    impl xcm_runtime_apis::conversions::LocationToAccountApi<Block, AccountId> for Runtime {
        fn convert_location(location: VersionedLocation) -> Result<
            AccountId,
            xcm_runtime_apis::conversions::Error
        > {
            xcm_runtime_apis::conversions::LocationToAccountHelper::<
                AccountId,
                configs::xcm_config::LocationToAccountId,
            >::convert_location(location)
        }
    }

    impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
        fn collect_collation_info(header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
            ParachainSystem::collect_collation_info(header)
        }
    }

    #[cfg(feature = "try-runtime")]
    impl frame_try_runtime::TryRuntime<Block> for Runtime {
        fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
            let weight = Executive::try_runtime_upgrade(checks).unwrap();
            (weight, configs::RuntimeBlockWeights::get().max_block)
        }

        fn execute_block(
            block: Block,
            state_root_check: bool,
            signature_check: bool,
            select: frame_try_runtime::TryStateSelect,
        ) -> Weight {
            // NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
            // have a backtrace here.
            Executive::try_execute_block(block, state_root_check, signature_check, select).unwrap()
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    impl frame_benchmarking::Benchmark<Block> for Runtime {
        fn benchmark_metadata(extra: bool) -> (
            Vec<frame_benchmarking::BenchmarkList>,
            Vec<frame_support::traits::StorageInfo>,
        ) {
            use frame_benchmarking::BenchmarkList;
            use frame_support::traits::StorageInfoTrait;
            use frame_system_benchmarking::Pallet as SystemBench;
            use cumulus_pallet_session_benchmarking::Pallet as SessionBench;

            let mut list = Vec::<BenchmarkList>::new();
            list_benchmarks!(list, extra);

            let storage_info = AllPalletsWithSystem::storage_info();
            (list, storage_info)
        }

        fn dispatch_benchmark(
            config: frame_benchmarking::BenchmarkConfig
        ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, alloc::string::String> {
            use frame_benchmarking::{BenchmarkError, BenchmarkBatch};

            use frame_system_benchmarking::Pallet as SystemBench;
            impl frame_system_benchmarking::Config for Runtime {
                fn setup_set_code_requirements(code: &alloc::vec::Vec<u8>) -> Result<(), BenchmarkError> {
                    ParachainSystem::initialize_for_set_code_benchmark(code.len() as u32);
                    Ok(())
                }

                fn verify_set_code() {
                    System::assert_last_event(cumulus_pallet_parachain_system::Event::<Runtime>::ValidationFunctionStored.into());
                }
            }

            use cumulus_pallet_session_benchmarking::Pallet as SessionBench;
            impl cumulus_pallet_session_benchmarking::Config for Runtime {}

            use frame_support::traits::WhitelistedStorageKeys;
            let whitelist = AllPalletsWithSystem::whitelisted_storage_keys();

            let mut batches = Vec::<BenchmarkBatch>::new();
            let params = (&config, &whitelist);
            add_benchmarks!(params, batches);

            if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
            Ok(batches)
        }
    }

    impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
        fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
            build_state::<RuntimeGenesisConfig>(config)
        }

        fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
            get_preset::<RuntimeGenesisConfig>(id, |_| None)
        }

        fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
            vec![]
        }
    }

    impl pallet_file_system_runtime_api::FileSystemApi<Block, AccountId, BackupStorageProviderId<Runtime>, MainStorageProviderId<Runtime>, H256, BlockNumber, ChunkId, BucketId<Runtime>, StorageRequestMetadata<Runtime>, BucketId<Runtime>, StorageDataUnit<Runtime>, H256> for Runtime {
        fn is_storage_request_open_to_volunteers(file_key: H256) -> Result<bool, IsStorageRequestOpenToVolunteersError> {
            FileSystem::is_storage_request_open_to_volunteers(file_key)
        }

        fn query_earliest_file_volunteer_tick(bsp_id: BackupStorageProviderId<Runtime>, file_key: H256) -> Result<BlockNumber, QueryFileEarliestVolunteerTickError> {
            FileSystem::query_earliest_file_volunteer_tick(bsp_id, file_key)
        }

        fn query_bsp_confirm_chunks_to_prove_for_file(bsp_id: BackupStorageProviderId<Runtime>, file_key: H256) -> Result<Vec<ChunkId>, QueryBspConfirmChunksToProveForFileError> {
            FileSystem::query_bsp_confirm_chunks_to_prove_for_file(bsp_id, file_key)
        }

        fn query_msp_confirm_chunks_to_prove_for_file(msp_id: MainStorageProviderId<Runtime>, file_key: H256) -> Result<Vec<ChunkId>, QueryMspConfirmChunksToProveForFileError> {
            FileSystem::query_msp_confirm_chunks_to_prove_for_file(msp_id, file_key)
        }

        fn query_bsps_volunteered_for_file(file_key: H256) -> Result<Vec<BackupStorageProviderId<Runtime>>, QueryBspsVolunteeredForFileError> {
            FileSystem::query_bsps_volunteered_for_file(file_key)
        }

        fn decode_generic_apply_delta_event_info(encoded_event_info: Vec<u8>) -> Result<BucketId<Runtime>, GenericApplyDeltaEventInfoError> {
            FileSystem::decode_generic_apply_delta_event_info(encoded_event_info)
        }

        fn storage_requests_by_msp(msp_id: MainStorageProviderId<Runtime>) -> BTreeMap<H256, StorageRequestMetadata<Runtime>> {
            FileSystem::storage_requests_by_msp(msp_id)
        }

        fn pending_storage_requests_by_msp(msp_id: MainStorageProviderId<Runtime>) -> BTreeMap<H256, StorageRequestMetadata<Runtime>> {
            FileSystem::pending_storage_requests_by_msp(msp_id)
        }

        fn query_incomplete_storage_request_metadata(file_key: H256) -> Result<pallet_file_system_runtime_api::IncompleteStorageRequestMetadataResponse<AccountId, BucketId<Runtime>, StorageDataUnit<Runtime>, H256, BackupStorageProviderId<Runtime>>, QueryIncompleteStorageRequestMetadataError> {
            FileSystem::query_incomplete_storage_request_metadata(file_key)
        }

        fn list_incomplete_storage_request_keys(start_after: Option<H256>, limit: u32) -> Vec<H256> {
            FileSystem::list_incomplete_storage_request_keys(start_after, limit)
        }

        fn query_pending_bsp_confirm_storage_requests(bsp_id: BackupStorageProviderId<Runtime>, file_keys: Vec<H256>) -> Vec<H256> {
            FileSystem::query_pending_bsp_confirm_storage_requests(bsp_id, file_keys)
        }
    }

    impl pallet_payment_streams_runtime_api::PaymentStreamsApi<Block, ProviderIdFor<Runtime>, Balance, AccountId> for Runtime {
        fn get_users_with_debt_over_threshold(provider_id: &ProviderIdFor<Runtime>, threshold: Balance) -> Result<Vec<AccountId>, GetUsersWithDebtOverThresholdError> {
            PaymentStreams::get_users_with_debt_over_threshold(provider_id, threshold)
        }
        fn get_users_of_payment_streams_of_provider(provider_id: &ProviderIdFor<Runtime>) -> Vec<AccountId> {
            PaymentStreams::get_users_of_payment_streams_of_provider(provider_id)
        }
        fn get_number_of_active_users_of_provider(provider_id: &ProviderIdFor<Runtime>) -> u32 {
            PaymentStreams::get_number_of_active_users_of_provider(provider_id)
        }
        fn get_providers_with_payment_streams_with_user(user_account: &AccountId) -> Vec<ProviderIdFor<Runtime>> {
            PaymentStreams::get_providers_with_payment_streams_with_user(user_account)
        }
        fn get_current_price_per_giga_unit_per_tick() -> Balance {
            PaymentStreams::get_current_price_per_giga_unit_per_tick()
        }
    }

    impl pallet_proofs_dealer_runtime_api::ProofsDealerApi<Block, ProofsDealerProviderIdFor<Runtime>, BlockNumber, KeyFor<Runtime>, RandomnessOutputFor<Runtime>, CustomChallenge<Runtime>> for Runtime {
        fn get_last_tick_provider_submitted_proof(provider_id: &ProofsDealerProviderIdFor<Runtime>) -> Result<BlockNumber, GetProofSubmissionRecordError> {
            ProofsDealer::get_last_tick_provider_submitted_proof(provider_id)
        }

        fn get_next_tick_to_submit_proof_for(provider_id: &ProofsDealerProviderIdFor<Runtime>) -> Result<BlockNumber, GetProofSubmissionRecordError> {
            ProofsDealer::get_next_tick_to_submit_proof_for(provider_id)
        }

        fn get_last_checkpoint_challenge_tick() -> BlockNumber {
            ProofsDealer::get_last_checkpoint_challenge_tick()
        }

        fn get_checkpoint_challenges(
            tick: BlockNumber
        ) -> Result<Vec<CustomChallenge<Runtime>>, GetCheckpointChallengesError> {
            ProofsDealer::get_checkpoint_challenges(tick)
        }

        fn get_challenge_seed(tick: BlockNumber) -> Result<RandomnessOutputFor<Runtime>, GetChallengeSeedError> {
            ProofsDealer::get_challenge_seed(tick)
        }

        fn get_challenge_period(provider_id: &ProofsDealerProviderIdFor<Runtime>) -> Result<BlockNumber, GetChallengePeriodError> {
            ProofsDealer::get_challenge_period(provider_id)
        }

        fn get_checkpoint_challenge_period() -> BlockNumber {
            ProofsDealer::get_checkpoint_challenge_period()
        }

        fn get_challenges_from_seed(seed: &RandomnessOutputFor<Runtime>, provider_id: &ProofsDealerProviderIdFor<Runtime>, count: u32) -> Vec<KeyFor<Runtime>> {
            ProofsDealer::get_challenges_from_seed(seed, provider_id, count)
        }

        fn get_forest_challenges_from_seed(seed: &RandomnessOutputFor<Runtime>, provider_id: &ProofsDealerProviderIdFor<Runtime>) -> Vec<KeyFor<Runtime>> {
            ProofsDealer::get_forest_challenges_from_seed(seed, provider_id)
        }

        fn get_current_tick() -> BlockNumber {
            ProofsDealer::get_current_tick()
        }

        fn get_next_deadline_tick(provider_id: &ProofsDealerProviderIdFor<Runtime>) -> Result<BlockNumber, GetNextDeadlineTickError> {
            ProofsDealer::get_next_deadline_tick(provider_id)
        }
    }

    impl pallet_storage_providers_runtime_api::StorageProvidersApi<Block, BlockNumber, BackupStorageProviderId<Runtime>, BackupStorageProvider<Runtime>, MainStorageProviderId<Runtime>, AccountId, ProviderIdFor<Runtime>, StorageProviderId<Runtime>, StorageDataUnit<Runtime>, Balance, BucketId<Runtime>, Multiaddresses<Runtime>, ValuePropositionWithId<Runtime>, H256> for Runtime {
        fn get_bsp_info(bsp_id: &BackupStorageProviderId<Runtime>) -> Result<BackupStorageProvider<Runtime>, GetBspInfoError> {
            Providers::get_bsp_info(bsp_id)
        }

        fn get_storage_provider_id(who: &AccountId) -> Option<StorageProviderId<Runtime>> {
            Providers::get_storage_provider_id(who)
        }

        fn query_msp_id_of_bucket_id(bucket_id: &BucketId<Runtime>) -> Result<Option<ProviderIdFor<Runtime>>, QueryMspIdOfBucketIdError> {
            Providers::query_msp_id_of_bucket_id(bucket_id)
        }

        fn query_storage_provider_capacity(provider_id: &ProviderIdFor<Runtime>) -> Result<StorageDataUnit<Runtime>, QueryStorageProviderCapacityError> {
            Providers::query_storage_provider_capacity(provider_id)
        }

        fn query_available_storage_capacity(provider_id: &ProviderIdFor<Runtime>) -> Result<StorageDataUnit<Runtime>, QueryAvailableStorageCapacityError> {
            Providers::query_available_storage_capacity(provider_id)
        }

        fn query_earliest_change_capacity_block(provider_id: &BackupStorageProviderId<Runtime>) -> Result<BlockNumber, QueryEarliestChangeCapacityBlockError> {
            Providers::query_earliest_change_capacity_block(provider_id)
        }

        fn get_worst_case_scenario_slashable_amount(provider_id: ProviderIdFor<Runtime>) -> Option<Balance> {
            Providers::get_worst_case_scenario_slashable_amount(&provider_id).ok()
        }

        fn get_slash_amount_per_max_file_size() -> Balance {
            Providers::get_slash_amount_per_max_file_size()
        }

        fn query_provider_multiaddresses(who: &ProviderIdFor<Runtime>) -> Result<Multiaddresses<Runtime>, QueryProviderMultiaddressesError> {
            Providers::query_provider_multiaddresses(who)
        }

        fn query_value_propositions_for_msp(msp_id: &MainStorageProviderId<Runtime>) -> Vec<ValuePropositionWithId<Runtime>> {
            Providers::query_value_propositions_for_msp(msp_id)
        }

        fn get_bsp_stake(bsp_id: &BackupStorageProviderId<Runtime>) -> Result<Balance, GetStakeError> {
            Providers::get_bsp_stake(bsp_id)
        }

        fn can_delete_provider(provider_id: &ProviderIdFor<Runtime>) -> bool {
            Providers::can_delete_provider(provider_id)
        }

        fn query_buckets_for_msp(msp_id: &MainStorageProviderId<Runtime>) -> Result<Vec<BucketId<Runtime>>, QueryBucketsForMspError> {
            Providers::query_buckets_for_msp(msp_id)
        }

        fn query_buckets_of_user_stored_by_msp(msp_id: &ProviderIdFor<Runtime>, user: &AccountId) -> Result<Vec<BucketId<Runtime>>, QueryBucketsOfUserStoredByMspError> {
            Providers::query_buckets_of_user_stored_by_msp(msp_id, user)
        }

        fn query_bucket_root(bucket_id: &BucketId<Runtime>) -> Result<H256, QueryBucketRootError> {
            Providers::query_bucket_root(bucket_id)
        }
    }
}
