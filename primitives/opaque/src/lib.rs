#![cfg_attr(not(feature = "std"), no_std)]

use frame_system::Config;
use parachains_common::BlockNumber;
use sp_runtime::traits::IdentifyAccount;
use sp_runtime::traits::Verify;
use sp_runtime::{
    generic,
    traits::{BlakeTwo256, Hash as HashT},
};
use sp_runtime::{MultiAddress, MultiSignature};

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub use sp_runtime::OpaqueExtrinsic;
/// Opaque block header type.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Opaque block type.
pub type Block = generic::Block<Header, OpaqueExtrinsic>;
/// Opaque block identifier type.
pub type BlockId = generic::BlockId<Block>;
/// Opaque block hash type.
pub type Hash = <BlakeTwo256 as HashT>::Output;

/////// Storage Hub Runtime abstraction

/// This is redundant with what we have in the Runtime
pub type Signature = MultiSignature;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Address = MultiAddress<AccountId, ()>;

/// The SignedExtension to the basic transaction logic.
pub type SignedExtra<Runtime> = (
    frame_system::CheckNonZeroSender<Runtime>,
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
    cumulus_primitives_storage_weight_reclaim::StorageWeightReclaim<Runtime>,
    frame_metadata_hash_extension::CheckMetadataHash<Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic<Runtime> = generic::UncheckedExtrinsic<
    Address,
    <Runtime as Config>::RuntimeCall,
    Signature,
    SignedExtra<Runtime>,
>;
