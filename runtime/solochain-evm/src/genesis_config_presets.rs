//! # StorageHub Runtime genesis config presets

use crate::*;
use alloc::{format, vec, vec::Vec};
use configs::{ExistentialDeposit, TreasuryAccount};
use cumulus_primitives_core::ParaId;
use serde_json::Value;
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Get, Pair, Public};
use sp_genesis_builder::PresetId;
use sp_runtime::traits::{IdentifyAccount, Verify};

type AccountPublic = <Signature as Verify>::Signer;

const STORAGEHUB_ED: Balance = ExistentialDeposit::get();

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Generate collator keys from seed.
///
/// This function's return type must always match the session keys of the chain in tuple format.
pub fn get_collator_keys_from_seed(seed: &str) -> AuraId {
    get_from_seed::<AuraId>(seed)
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn template_session_keys(keys: AuraId) -> SessionKeys {
    SessionKeys { aura: keys }
}

fn storagehub_genesis(
    invulnerables: Vec<(AccountId, AuraId)>,
    endowed_accounts: Vec<AccountId>,
    endowment: Balance,
    root: Option<AccountId>,
    id: ParaId,
) -> Value {
    let config = RuntimeGenesisConfig {
        balances: BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, endowment))
                .collect(),
        },
        parachain_info: ParachainInfoConfig {
            parachain_id: id,
            ..Default::default()
        },
        collator_selection: CollatorSelectionConfig {
            invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
            candidacy_bond: STORAGEHUB_ED * 16,
            ..Default::default()
        },
        session: SessionConfig {
            keys: invulnerables
                .into_iter()
                .map(|(acc, aura)| {
                    (
                        acc.clone(),                 // account id
                        acc,                         // validator id
                        template_session_keys(aura), // session keys
                    )
                })
                .collect(),
            ..Default::default()
        },
        polkadot_xcm: PolkadotXcmConfig {
            safe_xcm_version: Some(SAFE_XCM_VERSION),
            ..Default::default()
        },
        sudo: SudoConfig { key: root },
        ..Default::default()
    };

    serde_json::to_value(config).expect("Could not build genesis config.")
}

/// Encapsulates names of predefined genesis config presets.
mod preset_names {
    pub const PRESET_GENESIS: &str = "genesis";
}

fn local_testnet_genesis() -> Value {
    storagehub_genesis(
        // initial collators.
        vec![
            (
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                get_collator_keys_from_seed("Alice"),
            ),
            (
                get_account_id_from_seed::<sr25519::Public>("Bob"),
                get_collator_keys_from_seed("Bob"),
            ),
        ],
        vec![
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            get_account_id_from_seed::<sr25519::Public>("Bob"),
            get_account_id_from_seed::<sr25519::Public>("Charlie"),
            get_account_id_from_seed::<sr25519::Public>("Dave"),
            get_account_id_from_seed::<sr25519::Public>("Eve"),
            get_account_id_from_seed::<sr25519::Public>("Ferdie"),
            get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
            get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
            get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
            get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
            get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
            get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
            TreasuryAccount::get(),
        ],
        1u128 << 60,
        Some(get_account_id_from_seed::<sr25519::Public>("Alice")),
        1000.into(),
    )
}

fn development_config_genesis() -> Value {
    storagehub_genesis(
        // initial collators.
        vec![
            (
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                get_collator_keys_from_seed("Alice"),
            ),
            (
                get_account_id_from_seed::<sr25519::Public>("Bob"),
                get_collator_keys_from_seed("Bob"),
            ),
        ],
        vec![
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            get_account_id_from_seed::<sr25519::Public>("Bob"),
            get_account_id_from_seed::<sr25519::Public>("Charlie"),
            get_account_id_from_seed::<sr25519::Public>("Dave"),
            get_account_id_from_seed::<sr25519::Public>("Eve"),
            get_account_id_from_seed::<sr25519::Public>("Ferdie"),
            get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
            get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
            get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
            get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
            get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
            get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
            TreasuryAccount::get(),
        ],
        1u128 << 60,
        Some(get_account_id_from_seed::<sr25519::Public>("Alice")),
        1000.into(),
    )
}

// TODO: Replace this genesis config with the actual production config
fn genesis_config() -> Value {
    storagehub_genesis(
        // initial collators.
        vec![
            (
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                get_collator_keys_from_seed("Alice"),
            ),
            (
                get_account_id_from_seed::<sr25519::Public>("Bob"),
                get_collator_keys_from_seed("Bob"),
            ),
        ],
        vec![
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            get_account_id_from_seed::<sr25519::Public>("Bob"),
            get_account_id_from_seed::<sr25519::Public>("Charlie"),
            get_account_id_from_seed::<sr25519::Public>("Dave"),
            get_account_id_from_seed::<sr25519::Public>("Eve"),
            get_account_id_from_seed::<sr25519::Public>("Ferdie"),
            get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
            get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
            get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
            get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
            get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
            get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
            TreasuryAccount::get(),
        ],
        1u128 << 60,
        Some(get_account_id_from_seed::<sr25519::Public>("Alice")),
        1000.into(),
    )
}

/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &PresetId) -> Option<vec::Vec<u8>> {
    use preset_names::*;
    let patch = match id.as_str() {
        PRESET_GENESIS => genesis_config(),
        sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET => local_testnet_genesis(),
        sp_genesis_builder::DEV_RUNTIME_PRESET => development_config_genesis(),
        _ => return None,
    };
    Some(
        serde_json::to_string(&patch)
            .expect("serialization to json is expected to work. qed.")
            .into_bytes(),
    )
}

/// List of supported presets.
pub fn preset_names() -> Vec<PresetId> {
    use preset_names::*;
    vec![
        PresetId::from(sp_genesis_builder::DEV_RUNTIME_PRESET),
        PresetId::from(sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET),
        PresetId::from(PRESET_GENESIS),
    ]
}

pub fn alith() -> AccountId {
    AccountId::from(hex!("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac"))
}

pub fn baltathar() -> AccountId {
    AccountId::from(hex!("3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"))
}

pub fn charleth() -> AccountId {
    AccountId::from(hex!("798d4Ba9baf0064Ec19eB4F0a1a45785ae9D6DFc"))
}

pub fn dorothy() -> AccountId {
    AccountId::from(hex!("773539d4Ac0e786233D90A233654ccEE26a613D9"))
}

pub fn ethan() -> AccountId {
    AccountId::from(hex!("Ff64d3F6efE2317EE2807d2235B1ac2AA69d9E87"))
}

pub fn frank() -> AccountId {
    AccountId::from(hex!("C0F0f4ab324C46e55D02D0033343B4Be8A55532d"))
}
