//! Precompile to interact with the pallet_file_system instance through the EVM.

#![cfg_attr(not(feature = "std"), no_std)]

use fp_account::{AccountId20, EthereumSignature};
use fp_evm::{Log, PrecompileHandle};
use frame_support::dispatch::{GetDispatchInfo, PostDispatchInfo};
use pallet_evm::AddressMapping;
use pallet_file_system::{
    types::{
        BucketNameFor, FileLocation, FileOperation, FileOperationIntention, Fingerprint,
        MerkleHash, PeerIds, ProviderIdFor, ReplicationTarget, StorageDataUnit, ValuePropId,
    },
    Call as FileSystemCall,
};
use precompile_utils::prelude::*;
use shp_traits::ReadBucketsInterface;
use sp_core::{ConstU32, H160, H256, U256};
use sp_runtime::{traits::Dispatchable, BoundedVec};
use sp_std::{marker::PhantomData, vec::Vec};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub const SELECTOR_LOG_BUCKET_CREATED: [u8; 32] =
    keccak256!("BucketCreated(address,bytes32,bytes32)");
pub const SELECTOR_LOG_BUCKET_MOVE_REQUESTED: [u8; 32] =
    keccak256!("BucketMoveRequested(address,bytes32,bytes32)");
pub const SELECTOR_LOG_BUCKET_PRIVACY_UPDATED: [u8; 32] =
    keccak256!("BucketPrivacyUpdated(address,bytes32,bool)");
pub const SELECTOR_LOG_COLLECTION_CREATED: [u8; 32] =
    keccak256!("CollectionCreated(address,bytes32,bytes32)");
pub const SELECTOR_LOG_BUCKET_DELETED: [u8; 32] = keccak256!("BucketDeleted(address,bytes32)");
pub const SELECTOR_LOG_STORAGE_REQUEST_ISSUED: [u8; 32] =
    keccak256!("StorageRequestIssued(address,bytes32,bytes32)");
pub const SELECTOR_LOG_STORAGE_REQUEST_REVOKED: [u8; 32] =
    keccak256!("StorageRequestRevoked(bytes32)");
pub const SELECTOR_LOG_FILE_DELETION_REQUESTED: [u8; 32] =
    keccak256!("FileDeletionRequested(bytes32,address)");

pub fn log_bucket_created(
    address: impl Into<H160>,
    who: impl Into<H160>,
    bucket_id: H256,
    msp_id: H256,
) -> Log {
    log4(
        address.into(),
        SELECTOR_LOG_BUCKET_CREATED,
        who.into(),
        bucket_id,
        msp_id,
        Vec::new(),
    )
}

pub fn log_bucket_move_requested(
    address: impl Into<H160>,
    who: impl Into<H160>,
    bucket_id: H256,
    new_msp_id: H256,
) -> Log {
    log4(
        address.into(),
        SELECTOR_LOG_BUCKET_MOVE_REQUESTED,
        who.into(),
        bucket_id,
        new_msp_id,
        Vec::new(),
    )
}

pub fn log_bucket_privacy_updated(
    address: impl Into<H160>,
    who: impl Into<H160>,
    bucket_id: H256,
    private: bool,
) -> Log {
    log4(
        address.into(),
        SELECTOR_LOG_BUCKET_PRIVACY_UPDATED,
        who.into(),
        bucket_id,
        H256::from_low_u64_be(if private { 1 } else { 0 }),
        Vec::new(),
    )
}

pub fn log_collection_created(
    address: impl Into<H160>,
    who: impl Into<H160>,
    bucket_id: H256,
    collection_id: U256,
) -> Log {
    log4(
        address.into(),
        SELECTOR_LOG_COLLECTION_CREATED,
        who.into(),
        bucket_id,
        H256::from_slice(&collection_id.to_big_endian()),
        Vec::new(),
    )
}

pub fn log_bucket_deleted(address: impl Into<H160>, who: impl Into<H160>, bucket_id: H256) -> Log {
    log3(
        address.into(),
        SELECTOR_LOG_BUCKET_DELETED,
        who.into(),
        bucket_id,
        Vec::new(),
    )
}

pub fn log_storage_request_issued(
    address: impl Into<H160>,
    who: impl Into<H160>,
    file_key: H256,
    bucket_id: H256,
) -> Log {
    log4(
        address.into(),
        SELECTOR_LOG_STORAGE_REQUEST_ISSUED,
        who.into(),
        file_key,
        bucket_id,
        Vec::new(),
    )
}

pub fn log_storage_request_revoked(address: impl Into<H160>, file_key: H256) -> Log {
    log2(
        address.into(),
        SELECTOR_LOG_STORAGE_REQUEST_REVOKED,
        file_key,
        Vec::new(),
    )
}

pub fn log_file_deletion_requested(
    address: impl Into<H160>,
    file_key: H256,
    owner: impl Into<H160>,
) -> Log {
    log3(
        address.into(),
        SELECTOR_LOG_FILE_DELETION_REQUESTED,
        file_key,
        owner.into(),
        Vec::new(),
    )
}

/// FileSystem precompile.
///
/// Provides EVM-compatible interface to the file system pallet functionality.
#[derive(Debug, Clone)]
pub struct FileSystemPrecompile<Runtime>(PhantomData<Runtime>);

// TODO: Change all concrete types (AccountId20, EthereumSignature, etc.) to `storage_hub_runtime::` types once the EVM-compatible SH runtime is ready
#[precompile_utils::precompile]
#[precompile::test_concrete_types(mock::Test)]
impl<Runtime> FileSystemPrecompile<Runtime>
where
    Runtime: pallet_file_system::Config
        + pallet_evm::Config
        + frame_system::Config<AccountId = AccountId20>,
    Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
    Runtime::RuntimeCall: From<FileSystemCall<Runtime>>,
    <Runtime as pallet_evm::Config>::AddressMapping: AddressMapping<Runtime::AccountId>,
    <Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
    // Runtime types (we should update this bounds with the correct types once the EVM-compatible SH runtime is ready):
    ProviderIdFor<Runtime>: From<H256> + Into<H256>,
    ValuePropId<Runtime>: From<H256> + Into<H256>,
    StorageDataUnit<Runtime>: From<u64>,
    MerkleHash<Runtime>: From<H256> + Into<H256>,
	<<Runtime as pallet_file_system::Config>::Nfts as frame_support::traits::nonfungibles_v2::Inspect<AccountId20>>::CollectionId: Into<U256>,
    Fingerprint<Runtime>: From<H256> + Into<H256>,
    <Runtime as pallet_file_system::Config>::OffchainSignature: From<EthereumSignature>,


{
    #[precompile::public("createBucket(bytes32,bytes,bool,bytes32)")]
    fn create_bucket(
        handle: &mut impl PrecompileHandle,
        msp_id: H256,
        name: BoundedBytes<ConstU32<100>>,
        private: bool,
        value_prop_id: H256,
    ) -> EvmResult {
        // Record gas cost for the operation
        // TODO: Add actual gas cost from benchmark
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let name: BucketNameFor<Runtime> =
            BoundedVec::try_from(name.as_bytes().to_vec()).map_err(|_| RevertReason::custom("Bucket name too long"))?;
        let value_prop_id = value_prop_id.into();

        // Calculate bucket_id deterministically (same as pallet logic)
        let bucket_id = <Runtime as pallet_file_system::Config>::Providers::derive_bucket_id(&origin, name.clone());
        let bucket_id_h256: H256 = H256::from_slice(bucket_id.as_ref());

        let call = FileSystemCall::<Runtime>::create_bucket {
            msp_id: msp_id.clone().into(),
            name,
            private,
            value_prop_id,
        };

        // TODO: Consult about what storage growth argument is
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

        // Emit EVM log for BucketCreated event
        // Event signature: BucketCreated(address indexed who, bytes32 indexed bucketId, bytes32 indexed mspId)
        let log = log_bucket_created(
            handle.context().address,
            handle.context().caller,
            bucket_id_h256,
            msp_id,
        );
        handle.record_log_costs(&[&log])?;
        log.record(handle)?;

        Ok(())
    }

    #[precompile::public("requestMoveBucket(bytes32,bytes32,bytes32)")]
    fn request_move_bucket(
        handle: &mut impl PrecompileHandle,
        bucket_id: H256,
        new_msp_id: H256,
        new_value_prop_id: H256,
    ) -> EvmResult {
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let new_value_prop_id = new_value_prop_id.into();

        let call = FileSystemCall::<Runtime>::request_move_bucket {
            bucket_id: bucket_id.clone().into(),
            new_msp_id: new_msp_id.clone().into(),
            new_value_prop_id,
        };

        // TODO: Consult about what storage growth argument is
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

        // Emit EVM log for BucketMoveRequested event
        // Event signature: BucketMoveRequested(address indexed who, bytes32 indexed bucketId, bytes32 indexed newMspId)
        let log = log_bucket_move_requested(
            handle.context().address,
            handle.context().caller,
            bucket_id.into(),
            new_msp_id.into(),
        );
        handle.record_log_costs(&[&log])?;
        log.record(handle)?;

        Ok(())
    }


    #[precompile::public("updateBucketPrivacy(bytes32,bool)")]
    fn update_bucket_privacy(
        handle: &mut impl PrecompileHandle,
        bucket_id: H256,
        private: bool,
    ) -> EvmResult {
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);

        let call = FileSystemCall::<Runtime>::update_bucket_privacy { bucket_id: bucket_id.clone().into(), private };

        // TODO: Consult about what storage growth argument is
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

        // Emit EVM log for BucketPrivacyUpdated event
        // Event signature: BucketPrivacyUpdated(address indexed who, bytes32 indexed bucketId, bool _private)
        let log = log_bucket_privacy_updated(
            handle.context().address,
            handle.context().caller,
            bucket_id,
            private,
        );
        handle.record_log_costs(&[&log])?;
        log.record(handle)?;

        Ok(())
    }

    #[precompile::public("createAndAssociateCollectionWithBucket(bytes32)")]
    fn create_and_associate_collection_with_bucket(
        handle: &mut impl PrecompileHandle,
        bucket_id: H256,
    ) -> EvmResult {
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);

        let call =
            FileSystemCall::<Runtime>::create_and_associate_collection_with_bucket { bucket_id: bucket_id.clone().into() };

        // TODO: Consult about what storage growth argument is
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

        // Get the collection_id that was just created and associated with the bucket
        let collection_id = <Runtime as pallet_file_system::Config>::Providers::get_read_access_group_id_of_bucket(&bucket_id.clone().into())
            .map_err(|_| RevertReason::custom("Failed to get collection ID"))?
            .ok_or(RevertReason::custom("Collection ID not found"))?; // Should exist after successful dispatch

        // Emit EVM log for CollectionCreated event
        // Event signature: CollectionCreated(address indexed who, bytes32 indexed bucketId, bytes32 indexed collectionId)
        let log = log_collection_created(
            handle.context().address,
            handle.context().caller,
            bucket_id,
            collection_id.into(),
        );
        handle.record_log_costs(&[&log])?;
        log.record(handle)?;

        Ok(())
    }

    #[precompile::public("deleteBucket(bytes32)")]
    fn delete_bucket(handle: &mut impl PrecompileHandle, bucket_id: H256) -> EvmResult {
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);

        let call = FileSystemCall::<Runtime>::delete_bucket { bucket_id: bucket_id.clone().into() };

        // TODO: Consult about what storage growth argument is
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

        // Emit EVM log for BucketDeleted event
        // Event signature: BucketDeleted(address indexed who, bytes32 indexed bucketId)
        let log = log_bucket_deleted(
            handle.context().address,
            handle.context().caller,
            bucket_id,
        );
        handle.record_log_costs(&[&log])?;
        log.record(handle)?;

        Ok(())
    }

    #[precompile::public(
        "issueStorageRequest(bytes32,bytes,bytes32,uint64,bytes32,bytes[],uint8,uint32)"
    )]
    fn issue_storage_request(
        handle: &mut impl PrecompileHandle,
        bucket_id: H256,
        location: BoundedBytes<ConstU32<512>>,
        fingerprint: H256,
        size: u64,
        msp_id: H256,
        peer_ids: Vec<BoundedBytes<ConstU32<100>>>,
        replication_target: u8,
        custom_replication_target: u32,
    ) -> EvmResult {
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let bucket_id_runtime = bucket_id.clone().into();
        let location: FileLocation<Runtime> =
            BoundedVec::try_from(location.as_bytes().to_vec()).map_err(|_| RevertReason::custom("Location path too long"))?;
        let fingerprint = fingerprint.into();
        let size = size.into();
        let msp_id = msp_id.into();

        // Convert peer_ids
        let peer_ids: Result<PeerIds<Runtime>, _> = peer_ids
            .into_iter()
            .map(|peer_id| BoundedVec::try_from(peer_id.as_bytes().to_vec()).map_err(|_| RevertReason::custom("Peer ID too long")))
            .collect::<Result<Vec<_>, _>>()?
            .try_into();
        let peer_ids = peer_ids.map_err(|_| RevertReason::custom("Too many peer IDs"))?;

        let replication_target = match replication_target {
            0 => ReplicationTarget::Basic,
            1 => ReplicationTarget::Standard,
            2 => ReplicationTarget::HighSecurity,
            3 => ReplicationTarget::SuperHighSecurity,
            4 => ReplicationTarget::UltraHighSecurity,
            5 => ReplicationTarget::Custom(custom_replication_target.into()),
            _ => {
                return Err(RevertReason::custom("Invalid replication target").into());
            }
        };

		// Calculate the file_key (same logic as pallet)
        let file_key = pallet_file_system::Pallet::<Runtime>::compute_file_key(
            origin.clone(),
            bucket_id_runtime,
            location.clone(),
            size,
            fingerprint,
        ).map_err(|_| RevertReason::custom("Failed to compute file key"))?;
        let file_key_h256: H256 = file_key.into();

        let call = FileSystemCall::<Runtime>::issue_storage_request {
            bucket_id: bucket_id_runtime,
            location,
            fingerprint,
            size,
            msp_id,
            peer_ids,
            replication_target,
        };

        // TODO: Consult about what storage growth argument is
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

        // Emit EVM log for StorageRequestIssued event
        // Event signature: StorageRequestIssued(address indexed who, bytes32 indexed fileKey, bytes32 indexed bucketId)
        let log = log_storage_request_issued(
            handle.context().address,
            handle.context().caller,
            file_key_h256,
            bucket_id,
        );
        handle.record_log_costs(&[&log])?;
        log.record(handle)?;

        Ok(())
    }

    #[precompile::public("revokeStorageRequest(bytes32)")]
    fn revoke_storage_request(handle: &mut impl PrecompileHandle, file_key: H256) -> EvmResult {
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);

        let call = FileSystemCall::<Runtime>::revoke_storage_request { file_key: file_key.clone().into() };

        // TODO: Consult about what storage growth argument is
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

        // Emit EVM log for StorageRequestRevoked event
        // Event signature: StorageRequestRevoked(bytes32 indexed fileKey)
        let log = log_storage_request_revoked(
            handle.context().address,
            file_key,
        );
        handle.record_log_costs(&[&log])?;
        log.record(handle)?;

        Ok(())
    }

    #[precompile::public("requestDeleteFile((bytes32,uint8),bytes,bytes32,bytes,uint64,bytes32)")]
    fn request_delete_file(
        handle: &mut impl PrecompileHandle,
        signed_intention: (H256, u8),
        signature: UnboundedBytes,
        bucket_id: H256,
        location: BoundedBytes<ConstU32<512>>,
        size: u64,
        fingerprint: H256,
    ) -> EvmResult {
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let file_key = signed_intention.0.into();
        let file_key_h256 = signed_intention.0; // Store the original H256 for log emission
        let operation = match signed_intention.1 {
            0 => FileOperation::Delete,
            _ => {
                return Err(RevertReason::custom("Invalid file operation").into());
            }
        };

        let signed_intention = FileOperationIntention {
            file_key,
            operation,
        };

        let signature_bytes: Vec<u8> = signature.into();
        let bucket_id = bucket_id.into();
        let location: FileLocation<Runtime> =
            BoundedVec::try_from(location.as_bytes().to_vec()).map_err(|_| RevertReason::custom("Location path too long"))?;
        let size = size.into();
        let fingerprint = fingerprint.into();

        // Convert signature bytes to the expected signature type
        // TODO: This is temporary as a PoC. Once Frontier is integrated and our signature type is `EthereumSignature`, correctly integrate signature parsing using `storage_hub_runtime::Signature`
        let signature = if signature_bytes.len() == 65 {
            use sp_core::ecdsa;
            let mut sig_array = [0u8; 65];
            sig_array.copy_from_slice(&signature_bytes);
            EthereumSignature::new(ecdsa::Signature::from_raw(sig_array))
        } else {
            return Err(RevertReason::custom("Invalid signature length").into());
        };

        let call = FileSystemCall::<Runtime>::request_delete_file {
            signed_intention,
            signature: signature.into(),
            bucket_id,
            location,
            size,
            fingerprint,
        };

        // TODO: Consult about what storage growth argument is
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

        // Emit EVM log for FileDeletionRequested event
        // Event signature: FileDeletionRequested(bytes32 indexed fileKey, address indexed owner)
        let log = log_file_deletion_requested(
            handle.context().address,
            file_key_h256,
            handle.context().caller,
        );

        // Record gas costs automatically based on the log structure
        handle.record_log_costs(&[&log])?;

        // Emit the log
        log.record(handle)?;

        Ok(())
    }

	/// Get pending file deletion requests for a user
	#[precompile::public("getPendingFileDeletionRequestsCount(address)")]
    #[precompile::view]
    fn get_pending_file_deletion_requests_count(
        handle: &mut impl PrecompileHandle,
        user_address: Address,
    ) -> EvmResult<u32> {
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

        let user_account = Runtime::AddressMapping::into_account_id(user_address.0);

        let pending_requests = pallet_file_system::PendingFileDeletionRequests::<Runtime>::get(&user_account);
        let count = pending_requests.len() as u32;

        Ok(count)
    }

    /// Derive a bucket ID from owner and bucket name
	#[precompile::public("deriveBucketId(address,bytes)")]
    #[precompile::view]
    fn derive_bucket_id(
        handle: &mut impl PrecompileHandle,
        owner: Address,
        name: BoundedBytes<ConstU32<100>>,
    ) -> EvmResult<H256> {
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

        let owner_account = Runtime::AddressMapping::into_account_id(owner.0);
        let name_vec = name.as_bytes().to_vec();
        let name_bounded: BucketNameFor<Runtime> =
            BoundedVec::try_from(name_vec).map_err(|_| RevertReason::custom("Bucket name too long"))?;

        let bucket_id = <<Runtime as pallet_file_system::Config>::Providers as shp_traits::ReadBucketsInterface>::derive_bucket_id(
            &owner_account,
            name_bounded,
        );

        let bucket_id_h256: H256 = bucket_id.into();

        Ok(bucket_id_h256)
    }
}
