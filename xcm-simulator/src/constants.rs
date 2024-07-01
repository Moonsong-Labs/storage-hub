//! Collection of useful constants.

// Accounts.
pub const ALICE: sp_runtime::AccountId32 = sp_runtime::AccountId32::new([1u8; 32]);
pub const BOB: sp_runtime::AccountId32 = sp_runtime::AccountId32::new([2u8; 32]);
pub const CHARLIE: sp_runtime::AccountId32 = sp_runtime::AccountId32::new([3u8; 32]);

// Currency units.
pub const UNITS: u128 = 1_000_000_000_000; // 12 decimals.
pub const CENTS: u128 = UNITS / 100; // 100 cents = 1 unit.
pub const INITIAL_BALANCE: u128 = 1 * UNITS;
