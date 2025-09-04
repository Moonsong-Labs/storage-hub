#![cfg_attr(not(feature = "std"), no_std)]
// `frame_support::runtime` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub mod apis;
pub mod configs;
mod genesis_config_presets;
mod weights;

extern crate alloc;

use fp_account::EthereumSignature;
use frame_support::weights::{
    constants::WEIGHT_REF_TIME_PER_SECOND, Weight, WeightToFeeCoefficient, WeightToFeeCoefficients,
    WeightToFeePolynomial,
};
pub use parachains_common::BlockNumber;
use smallvec::smallvec;
use sp_runtime::{
    generic, impl_opaque_keys,
    traits::{
        BlakeTwo256, DispatchInfoOf, Dispatchable, IdentifyAccount, PostDispatchInfoOf, Verify,
    },
    transaction_validity::{TransactionValidity, TransactionValidityError},
    Perbill,
};
use sp_std::prelude::{Vec, *};
use sp_version::RuntimeVersion;
use weights::ExtrinsicBaseWeight;

#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

impl_opaque_keys! {
    pub struct SessionKeys {
        pub babe: Babe,
        pub grandpa: Grandpa,
    }
}

pub mod currency {
    use super::Balance;

    // Provide a common factor between runtimes based on a supply of 10_000_000 tokens.
    pub const SUPPLY_FACTOR: Balance = 1;

    pub const WEI: Balance = 1;
    pub const KILOWEI: Balance = 1_000;
    pub const MEGAWEI: Balance = 1_000_000;
    pub const GIGAWEI: Balance = 1_000_000_000;
    pub const MICROHAVE: Balance = 1_000_000_000_000;
    pub const MILLIHAVE: Balance = 1_000_000_000_000_000;
    pub const HAVE: Balance = 1_000_000_000_000_000_000;
    pub const KILOHAVE: Balance = 1_000_000_000_000_000_000_000;

    pub const TRANSACTION_BYTE_FEE: Balance = 1 * GIGAWEI * SUPPLY_FACTOR;
    pub const STORAGE_BYTE_FEE: Balance = 100 * MICROHAVE * SUPPLY_FACTOR;
    pub const WEIGHT_FEE: Balance = 50 * KILOWEI * SUPPLY_FACTOR / 4;

    pub const fn deposit(items: u32, bytes: u32) -> Balance {
        items as Balance * 1 * HAVE * SUPPLY_FACTOR + (bytes as Balance) * STORAGE_BYTE_FEE
    }
}

pub mod gas {
    use frame_support::weights::constants::WEIGHT_REF_TIME_PER_SECOND;

    /// Current approximation of the gas/s consumption considering
    /// EVM execution over compiled WASM (on 4.4Ghz CPU).
    /// Given the 1000ms Weight, from which 75% only are used for transactions,
    /// the total EVM execution gas limit is: GAS_PER_SECOND * 1 * 0.75 ~= 30_000_000.
    pub const GAS_PER_SECOND: u64 = 40_000_000;

    /// Approximate ratio of the amount of Weight per Gas.
    /// u64 works for approximations because Weight is a very small unit compared to gas.
    pub const WEIGHT_PER_GAS: u64 = WEIGHT_REF_TIME_PER_SECOND / GAS_PER_SECOND;

    /// The highest amount of new storage that can be created in a block (160KB).
    pub const BLOCK_STORAGE_LIMIT: u64 = 160 * 1024;
}

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = EthereumSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// The address format for describing accounts.
pub type Address = AccountId;

/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;

/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;

/// The TransactionExtension to the basic transaction logic.
pub type TxExtension = (
    frame_system::CheckNonZeroSender<Runtime>,
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
    frame_metadata_hash_extension::CheckMetadataHash<Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
    fp_self_contained::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
>;

/// Handles converting a weight scalar to a fee value, based on the scale and granularity of the
/// node's balance type.
///
/// This should typically create a mapping between the following ranges:
///   - `[0, MAXIMUM_BLOCK_WEIGHT]`
///   - `[Balance::min, Balance::max]`
///
/// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
///   - Setting it to `0` will essentially disable the weight fee.
///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
    type Balance = Balance;
    fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
        // in Rococo, extrinsic base weight (smallest non-zero weight) is mapped to 1 MILLIUNIT:
        // in our template, we map to 1/10 of that, or 1/10 MILLIUNIT
        let p = MILLIUNIT / 10;
        let q = 100 * Balance::from(ExtrinsicBaseWeight::get().ref_time());
        smallvec![WeightToFeeCoefficient {
            degree: 1,
            negative: false,
            coeff_frac: Perbill::from_rational(p % q, q),
            coeff_integer: p / q,
        }]
    }
}

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: alloc::borrow::Cow::Borrowed("sh-solochain-evm"),
    impl_name: alloc::borrow::Cow::Borrowed("sh-solochain-evm"),
    authoring_version: 1,
    spec_version: 1,
    impl_version: 0,
    apis: apis::RUNTIME_API_VERSIONS,
    transaction_version: 1,
    system_version: 1,
};

/// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
/// up by `pallet_aura` to implement `fn slot_duration()`.
///
/// Change this to adjust the block time.
pub const MILLISECS_PER_BLOCK: u64 = 6000;
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

// Unit = the base number of indivisible units for balances
pub const UNIT: Balance = 1_000_000_000_000;
pub const CENTS: Balance = UNIT / 100;
pub const MILLIUNIT: Balance = 1_000_000_000;
pub const MICROUNIT: Balance = 1_000_000;
pub const NANOUNIT: Balance = 1_000;
pub const PICOUNIT: Balance = 1;

/// The existential deposit. Set to 1/10 of the Connected Relay Chain.
pub const EXISTENTIAL_DEPOSIT: Balance = 0;

/// We assume that ~5% of the block weight is consumed by `on_initialize` handlers. This is
/// used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(5);

/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used by
/// `Operational` extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

/// We allow for 2 seconds of compute with a 6 second average block.
pub const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
    WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2),
    cumulus_primitives_core::relay_chain::MAX_POV_SIZE as u64,
);

// Create the runtime by composing the FRAME pallets that were previously configured.
#[frame_support::runtime]
mod runtime {
    #[runtime::runtime]
    #[runtime::derive(
        RuntimeCall,
        RuntimeEvent,
        RuntimeError,
        RuntimeOrigin,
        RuntimeFreezeReason,
        RuntimeHoldReason,
        RuntimeSlashReason,
        RuntimeLockId,
        RuntimeTask
    )]
    pub struct Runtime;

    // ╔══════════════════ System and Consensus Pallets ═════════════════╗
    #[runtime::pallet_index(0)]
    pub type System = frame_system;

    // Babe must be before session.
    #[runtime::pallet_index(1)]
    pub type Babe = pallet_babe;

    #[runtime::pallet_index(2)]
    pub type Timestamp = pallet_timestamp;

    #[runtime::pallet_index(3)]
    pub type Balances = pallet_balances;

    // Consensus support.
    // Authorship must be before session in order to note author in the correct session and era.
    #[runtime::pallet_index(4)]
    pub type Authorship = pallet_authorship;

    #[runtime::pallet_index(5)]
    pub type Offences = pallet_offences;

    #[runtime::pallet_index(6)]
    pub type Historical = pallet_session::historical;

    #[runtime::pallet_index(8)]
    pub type Session = pallet_session;

    #[runtime::pallet_index(10)]
    pub type Grandpa = pallet_grandpa;

    #[runtime::pallet_index(11)]
    pub type TransactionPayment = pallet_transaction_payment;
    // ╚═════════════════ System and Consensus Pallets ══════════════════╝

    // ╔═════════════════ Polkadot SDK Utility Pallets ══════════════════╗

    #[runtime::pallet_index(35)]
    pub type Parameters = pallet_parameters;

    #[runtime::pallet_index(36)]
    pub type Sudo = pallet_sudo;

    #[runtime::pallet_index(90)]
    pub type Nfts = pallet_nfts;
    // ╚═════════════════ Polkadot SDK Utility Pallets ══════════════════╝

    // ╔════════════════════ Frontier (EVM) Pallets ═════════════════════╗
    #[runtime::pallet_index(50)]
    pub type Ethereum = pallet_ethereum;

    #[runtime::pallet_index(51)]
    pub type Evm = pallet_evm;

    #[runtime::pallet_index(52)]
    pub type EvmChainId = pallet_evm_chain_id;
    // ╚════════════════════ Frontier (EVM) Pallets ═════════════════════╝

    // ╔══════════════════════ StorageHub Pallets ═══════════════════════╗
    // Start with index 80
    #[runtime::pallet_index(80)]
    pub type Providers = pallet_storage_providers;

    #[runtime::pallet_index(81)]
    pub type FileSystem = pallet_file_system;

    #[runtime::pallet_index(82)]
    pub type ProofsDealer = pallet_proofs_dealer;

    #[runtime::pallet_index(83)]
    pub type Randomness = pallet_randomness;

    #[runtime::pallet_index(84)]
    pub type PaymentStreams = pallet_payment_streams;

    #[runtime::pallet_index(85)]
    pub type BucketNfts = pallet_bucket_nfts;

    // TODO: Add `pallet_cr_randomness` to the runtime when it's ready.
    // #[runtime::pallet_index(46)]
    // pub type CrRandomness = pallet_cr_randomness;
    // ╚══════════════════════ StorageHub Pallets ═══════════════════════╝
}

#[cfg(feature = "runtime-benchmarks")]
mod benches {
    frame_benchmarking::define_benchmarks!(
        [frame_system, SystemBench::<Runtime>]
        [pallet_balances, Balances]
        [pallet_timestamp, Timestamp]
        [pallet_sudo, Sudo]
        [pallet_nfts, Nfts]
        [pallet_parameters, Parameters]
        [pallet_payment_streams, PaymentStreams]
        [pallet_proofs_dealer, ProofsDealer]
        [pallet_storage_providers, Providers]
        [pallet_randomness, Randomness]
        [pallet_file_system, FileSystem]
        [pallet_bucket_nfts, BucketNfts]
    );
}

impl fp_self_contained::SelfContainedCall for RuntimeCall {
    type SignedInfo = sp_core::H160;

    fn is_self_contained(&self) -> bool {
        match self {
            RuntimeCall::Ethereum(call) => call.is_self_contained(),
            _ => false,
        }
    }

    fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
        match self {
            RuntimeCall::Ethereum(call) => call.check_self_contained(),
            _ => None,
        }
    }

    fn validate_self_contained(
        &self,
        info: &Self::SignedInfo,
        dispatch_info: &DispatchInfoOf<Self>,
        len: usize,
    ) -> Option<TransactionValidity> {
        match self {
            RuntimeCall::Ethereum(call) => call.validate_self_contained(info, dispatch_info, len),
            _ => None,
        }
    }

    fn pre_dispatch_self_contained(
        &self,
        info: &Self::SignedInfo,
        dispatch_info: &DispatchInfoOf<Self>,
        len: usize,
    ) -> Option<Result<(), TransactionValidityError>> {
        match self {
            RuntimeCall::Ethereum(call) => {
                call.pre_dispatch_self_contained(info, dispatch_info, len)
            }
            _ => None,
        }
    }

    fn apply_self_contained(
        self,
        _info: Self::SignedInfo,
    ) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
        match self {
            call @ RuntimeCall::Ethereum(_) => Some(call.dispatch(RuntimeOrigin::none())),
            _ => None,
        }
    }
}
