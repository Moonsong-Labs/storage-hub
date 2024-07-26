//! Collection of useful constants.

use crate::storagehub::configs::SpMinDeposit;

// Accounts.
pub const ALICE: sp_runtime::AccountId32 = sp_runtime::AccountId32::new([1u8; 32]);
pub const BOB: sp_runtime::AccountId32 = sp_runtime::AccountId32::new([2u8; 32]);
pub const CHARLIE: sp_runtime::AccountId32 = sp_runtime::AccountId32::new([3u8; 32]);

// Currency units.
pub const UNITS: u128 = 1_000_000_000_000; // 12 decimals.
pub const CENTS: u128 = UNITS / 100; // 100 cents = 1 unit.
pub const INITIAL_BALANCE: u128 = 10 * SpMinDeposit::get();

// Para IDs
pub const SH_PARA_ID: u32 = 1;
pub const SYS_PARA_ID: u32 = 2;
pub const NON_SYS_PARA_ID: u32 = 2004;
