//! Precompile to interact with the pallet_file_system instance through the EVM.

#![cfg_attr(not(feature = "std"), no_std)]

use evm::ExitSucceed;
use fp_evm::{Log, PrecompileHandle, PrecompileOutput};
use frame_support::{
    dispatch::{GetDispatchInfo, PostDispatchInfo},
    traits::Get,
};
use pallet_evm::AddressMapping;
use pallet_file_system::{
    types::{
        BucketMoveRequestResponse, FileOperation, FileOperationIntention, MaxNumberOfPeerIds,
        MaxPeerIdSize, ReplicationTarget,
    },
    Call as FileSystemCall,
};
use precompile_utils::prelude::*;
use sp_core::{crypto::UncheckedFrom, ConstU32, H256, U256};
use sp_runtime::{
    traits::{Dispatchable, Saturating},
    BoundedVec,
};
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
pub const SELECTOR_LOG_BSP_VOLUNTEERED: [u8; 32] = keccak256!("BspVolunteered(bytes32,bytes32)");
pub const SELECTOR_LOG_BSP_CONFIRMED_STORING: [u8; 32] =
    keccak256!("BspConfirmedStoring(bytes32,bytes32[],bytes32)");
pub const SELECTOR_LOG_BSP_STOP_STORING_REQUESTED: [u8; 32] =
    keccak256!("BspStopStoringRequested(bytes32,bytes32)");
pub const SELECTOR_LOG_BSP_STOP_STORING_CONFIRMED: [u8; 32] =
    keccak256!("BspStopStoringConfirmed(bytes32,bytes32)");
pub const SELECTOR_LOG_FILE_DELETION_REQUESTED: [u8; 32] =
    keccak256!("FileDeletionRequested(bytes32,address)");

/// FileSystem precompile.
///
/// Provides EVM-compatible interface to the file system pallet functionality.
#[derive(Debug, Clone)]
pub struct FileSystemPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> FileSystemPrecompile<Runtime>
where
    Runtime: pallet_file_system::Config
        + pallet_evm::Config
        + frame_system::Config<AccountId = storage_hub_runtime::AccountId>,
    Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
    Runtime::RuntimeCall: From<FileSystemCall<Runtime>>,
    <Runtime as pallet_evm::Config>::AddressMapping: AddressMapping<Runtime::AccountId>,
    <Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
    // Runtime types (we should update this bounds with the correct types once the EVM-compatible SH runtime is ready):
    <<Runtime as pallet_file_system::Config>::Providers as shp_traits::ReadProvidersInterface>::ProviderId: From<H256>,
    <<Runtime as pallet_file_system::Config>::Providers as shp_traits::ReadStorageProvidersInterface>::ValuePropId: From<H256>,
    <<Runtime as pallet_file_system::Config>::Providers as shp_traits::ReadStorageProvidersInterface>::StorageDataUnit: From<u64>,
    <<Runtime as pallet_file_system::Config>::Providers as shp_traits::ReadProvidersInterface>::MerkleHash: From<H256>,
    <Runtime as pallet_file_system::Config>::Fingerprint: From<H256>,
    BoundedVec<u8, <Runtime as pallet_file_system::Config>::MaxPeerIdSize>: From<BoundedBytes<ConstU32<100>>>,
    BoundedVec<u8, <Runtime as pallet_file_system::Config>::MaxFilePathSize>: From<BoundedBytes<ConstU32<512>>>,
    BoundedVec<u8, <<Runtime as pallet_file_system::Config>::Providers as shp_traits::ReadBucketsInterface>::BucketNameLimit>: From<BoundedBytes<ConstU32<100>>>,
{
    // =======
    // Getters
    // =======

    // TODO: Add view functions for reading bucket info, storage requests, etc.
    // These would query the pallet storage directly without dispatching calls

    // ================
    // Public functions
    // ================
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
        let msp_id = msp_id.into();
        let name = name.into();
        let value_prop_id = value_prop_id.into();

        let call = FileSystemCall::<Runtime>::create_bucket {
            msp_id,
            name,
            private,
            value_prop_id,
        };

        // TODO: Consult about what storage growth argument is
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

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
        let bucket_id = bucket_id.into();
        let new_msp_id = new_msp_id.into();
        let new_value_prop_id = new_value_prop_id.into();

        let call = FileSystemCall::<Runtime>::request_move_bucket {
            bucket_id,
            new_msp_id,
            new_value_prop_id,
        };

        // TODO: Consult about what storage growth argument is
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

        Ok(())
    }

    #[precompile::public("mspRespondMoveBucketRequest(bytes32,uint8)")]
    fn msp_respond_move_bucket_request(
        handle: &mut impl PrecompileHandle,
        bucket_id: H256,
        response: u8,
    ) -> EvmResult {
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let bucket_id = bucket_id.into();
        let response = match response {
            0 => BucketMoveRequestResponse::Accepted,
            1 => BucketMoveRequestResponse::Rejected,
            _ => {
                return Err(RevertReason::custom("Invalid bucket move response").into());
            }
        };

        let call = FileSystemCall::<Runtime>::msp_respond_move_bucket_request {
            bucket_id,
            response,
        };

        // TODO: Consult about what storage growth argument is
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

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
        let bucket_id = bucket_id.into();

        let call = FileSystemCall::<Runtime>::update_bucket_privacy { bucket_id, private };

        // TODO: Consult about what storage growth argument is
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

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
        let bucket_id = bucket_id.into();

        let call =
            FileSystemCall::<Runtime>::create_and_associate_collection_with_bucket { bucket_id };

        // TODO: Consult about what storage growth argument is
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

        Ok(())
    }

    #[precompile::public("deleteBucket(bytes32)")]
    fn delete_bucket(handle: &mut impl PrecompileHandle, bucket_id: H256) -> EvmResult {
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let bucket_id = bucket_id.into();

        let call = FileSystemCall::<Runtime>::delete_bucket { bucket_id };

        // TODO: Consult about what storage growth argument is
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

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
        let bucket_id = bucket_id.into();
        let location = location.into();
        let fingerprint = fingerprint.into();
        let size = size.into();
        let msp_id = msp_id.into();

        // Convert peer_ids
        let peer_ids: Result<BoundedVec<BoundedVec<u8, MaxPeerIdSize<Runtime>>, MaxNumberOfPeerIds<Runtime>>, _> = peer_ids
            .into_iter()
            .map(|peer_id| peer_id.into())
            .collect::<Vec<_>>()
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

        let call = FileSystemCall::<Runtime>::issue_storage_request {
            bucket_id,
            location,
            fingerprint,
            size,
            msp_id,
            peer_ids,
            replication_target,
        };

        // TODO: Consult about what storage growth argument is
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

        Ok(())
    }

    #[precompile::public("revokeStorageRequest(bytes32)")]
    fn revoke_storage_request(handle: &mut impl PrecompileHandle, file_key: H256) -> EvmResult {
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let file_key = file_key.into();

        let call = FileSystemCall::<Runtime>::revoke_storage_request { file_key };

        // TODO: Consult about what storage growth argument is
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

        Ok(())
    }

    #[precompile::public("mspStopStoringBucket(bytes32)")]
    fn msp_stop_storing_bucket(handle: &mut impl PrecompileHandle, bucket_id: H256) -> EvmResult {
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let bucket_id = bucket_id.into();

        let call = FileSystemCall::<Runtime>::msp_stop_storing_bucket { bucket_id };

        // TODO: Consult about what storage growth argument is
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

        Ok(())
    }

    #[precompile::public("bspVolunteer(bytes32)")]
    fn bsp_volunteer(handle: &mut impl PrecompileHandle, file_key: H256) -> EvmResult {
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let file_key = file_key.into();

        let call = FileSystemCall::<Runtime>::bsp_volunteer { file_key };

        // TODO: Consult about what storage growth argument is
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

        Ok(())
    }

    #[precompile::public(
        "bspRequestStopStoring(bytes32,bytes32,bytes,address,bytes32,uint64,bool)"
    )]
    fn bsp_request_stop_storing(
        handle: &mut impl PrecompileHandle,
        file_key: H256,
        bucket_id: H256,
        location: BoundedBytes<ConstU32<512>>,
        owner: Address,
        fingerprint: H256,
        size: u64,
        can_serve: bool,
    ) -> EvmResult {
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let file_key = file_key.into();
        let bucket_id = bucket_id.into();
        let location = location.into();
        let owner = Runtime::AddressMapping::into_account_id(owner.into());
        let fingerprint = fingerprint.into();
        let size = size.into();

        let call = FileSystemCall::<Runtime>::bsp_request_stop_storing {
            file_key,
            bucket_id,
            location,
            owner,
            fingerprint,
            size,
            can_serve,
            // Note: inclusion_forest_proof parameter would need to be handled
            // in a production implementation with proper proof parsing
            inclusion_forest_proof: Default::default(),
        };

        // TODO: Consult about what storage growth argument is
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

        Ok(())
    }

    #[precompile::public("bspConfirmStopStoring(bytes32)")]
    fn bsp_confirm_stop_storing(handle: &mut impl PrecompileHandle, file_key: H256) -> EvmResult {
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let file_key = file_key.into();

        let call = FileSystemCall::<Runtime>::bsp_confirm_stop_storing {
            file_key,
            // Note: inclusion_forest_proof parameter would need to be handled
            // in a production implementation with proper proof parsing
            inclusion_forest_proof: Default::default(),
        };

        // TODO: Consult about what storage growth argument is
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

        Ok(())
    }

    #[precompile::public(
        "stopStoringForInsolventUser(bytes32,bytes32,bytes,address,bytes32,uint64)"
    )]
    fn stop_storing_for_insolvent_user(
        handle: &mut impl PrecompileHandle,
        file_key: H256,
        bucket_id: H256,
        location: BoundedBytes<ConstU32<1024>>,
        owner: Address,
        fingerprint: H256,
        size: u64,
    ) -> EvmResult {
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let file_key = file_key.into();
        let bucket_id = bucket_id.into();
        let location = location.into();
        let owner = Runtime::AddressMapping::into_account_id(owner.into());
        let fingerprint = fingerprint.into();
        let size = size.into();

        let call = FileSystemCall::<Runtime>::stop_storing_for_insolvent_user {
            file_key,
            bucket_id,
            location,
            owner,
            fingerprint,
            size,
            // Note: inclusion_forest_proof parameter would need to be handled
            // in a production implementation with proper proof parsing
            inclusion_forest_proof: Default::default(),
        };

        // TODO: Consult about what storage growth argument is
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

        Ok(())
    }

    #[precompile::public("mspStopStoringBucketForInsolventUser(bytes32)")]
    fn msp_stop_storing_bucket_for_insolvent_user(
        handle: &mut impl PrecompileHandle,
        bucket_id: H256,
    ) -> EvmResult {
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let bucket_id = bucket_id.into();

        let call =
            FileSystemCall::<Runtime>::msp_stop_storing_bucket_for_insolvent_user { bucket_id };

        // TODO: Consult about what storage growth argument is
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

        Ok(())
    }

    #[precompile::public("requestDeleteFile((bytes32,uint8),bytes,bytes32,bytes,uint64,bytes32)")]
    fn request_delete_file(
        handle: &mut impl PrecompileHandle,
        signed_intention: (H256, u8),
        signature: UnboundedBytes,
        bucket_id: H256,
        location: BoundedBytes<ConstU32<1024>>,
        size: u64,
        fingerprint: H256,
    ) -> EvmResult {
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let file_key = signed_intention.0.into();
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

        let signature = signature.into();
        let bucket_id = bucket_id.into();
        let location = location.into();
        let size = size.into();
        let fingerprint = fingerprint.into();

        let call = FileSystemCall::<Runtime>::request_delete_file {
            signed_intention,
            signature: signature
                .try_into()
                .map_err(|_| RevertReason::custom("Invalid signature format"))?,
            bucket_id,
            location,
            size,
            fingerprint,
        };

        // TODO: Consult about what storage growth argument is
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

        Ok(())
    }
}
