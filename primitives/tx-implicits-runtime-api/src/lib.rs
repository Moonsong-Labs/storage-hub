#![cfg_attr(not(feature = "std"), no_std)]

use sp_api::decl_runtime_apis;
use sp_runtime::{generic::Era, transaction_validity::TransactionValidityError};
use sp_std::vec::Vec;

decl_runtime_apis! {
    pub trait TxImplicitsApi {
        /// Compute the implicit tuple for the runtime's SignedExtra, given the provided `era`
        /// and whether metadata hash checking is enabled.
        ///
        /// Returns SCALE-encoded bytes of `Runtime::Extension::Implicit`.
        fn compute_signed_extra_implicit(era: Era, enable_metadata: bool) -> Result<Vec<u8>, TransactionValidityError>;
    }
}
