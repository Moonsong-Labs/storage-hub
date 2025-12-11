#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::{
    generic,
    traits::{BlakeTwo256, Hash as HashT},
};

/// Block number type used by the runtime.
pub type BlockNumber = u32;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;
/// Opaque block header type.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Opaque block type.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// Opaque block identifier type.
pub type BlockId = generic::BlockId<Block>;
/// Opaque block hash type.
pub type Hash = <BlakeTwo256 as HashT>::Output;
