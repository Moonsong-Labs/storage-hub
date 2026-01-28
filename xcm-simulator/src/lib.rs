#![recursion_limit = "256"]

extern crate alloc;

use sp_core::Get;
use sp_runtime::BuildStorage;
use sp_tracing;
use storagehub::configs::{ExistentialDeposit, TreasuryAccount};
use xcm::prelude::*;
use xcm_executor::traits::ConvertLocation;
use xcm_simulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain, TestExt};

mod constants;
mod mock_message_queue;
mod parachain;
mod relay_chain;
mod storagehub;
mod system_chain;

#[cfg(test)]
mod tests;

use constants::*;

decl_test_parachain! {
    pub struct StorageHub {
        Runtime = storagehub::Runtime,
        XcmpMessageHandler = storagehub::MsgQueue,
        DmpMessageHandler = storagehub::MsgQueue,
        new_ext = sh_ext(),
    }
}

decl_test_parachain! {
    pub struct MockSystemChain {
        Runtime = system_chain::Runtime,
        XcmpMessageHandler = system_chain::MsgQueue,
        DmpMessageHandler = system_chain::MsgQueue,
        new_ext = sys_ext(2),
    }
}

decl_test_parachain! {
    pub struct MockParachain {
        Runtime = parachain::Runtime,
        XcmpMessageHandler = parachain::MsgQueue,
        DmpMessageHandler = parachain::MsgQueue,
        new_ext = para_ext(2004),
    }
}

decl_test_relay_chain! {
    pub struct Relay {
        Runtime = relay_chain::Runtime,
        RuntimeCall = relay_chain::RuntimeCall,
        RuntimeEvent = relay_chain::RuntimeEvent,
        XcmConfig = relay_chain::XcmConfig,
        MessageQueue = relay_chain::MessageQueue,
        System = relay_chain::System,
        new_ext = relay_ext(),
    }
}

decl_test_network! {
    pub struct MockNet {
        relay_chain = Relay,
        parachains = vec![
            (1, StorageHub),
            (2, MockSystemChain),
            (2004, MockParachain),
        ],
    }
}

pub fn parent_account_id() -> parachain::AccountId {
    let location = (Parent,);
    parachain::location_converter::LocationConverter::convert_location(&location.into()).unwrap()
}

pub fn sys_parent_account_id() -> system_chain::AccountId {
    let location = (Parent,);
    system_chain::location_converter::LocationConverter::convert_location(&location.into()).unwrap()
}

pub fn sh_parent_account_id() -> storagehub::AccountId {
    let location = (Parent,);
    storagehub::configs::xcm_config::LocationToAccountId::convert_location(&location.into())
        .unwrap()
}

pub fn child_account_id(para: u32) -> relay_chain::AccountId {
    let location = (Parachain(para),);
    relay_chain::location_converter::LocationConverter::convert_location(&location.into()).unwrap()
}

pub fn child_account_account_id(para: u32, who: sp_runtime::AccountId32) -> relay_chain::AccountId {
    let location = (
        Parachain(para),
        AccountId32 {
            network: None,
            id: who.into(),
        },
    );
    relay_chain::location_converter::LocationConverter::convert_location(&location.into()).unwrap()
}

pub fn sibling_account_id(para: u32) -> parachain::AccountId {
    let location = (Parent, Parachain(para));
    parachain::location_converter::LocationConverter::convert_location(&location.into()).unwrap()
}

pub fn sys_sibling_account_id(para: u32) -> system_chain::AccountId {
    let location = (Parent, Parachain(para));
    system_chain::location_converter::LocationConverter::convert_location(&location.into()).unwrap()
}

pub fn sh_sibling_account_id(para: u32) -> storagehub::AccountId {
    let location = (Parent, Parachain(para));
    storagehub::configs::xcm_config::LocationToAccountId::convert_location(&location.into())
        .unwrap()
}

pub fn sibling_account_account_id(para: u32, who: sp_runtime::AccountId32) -> parachain::AccountId {
    let location = (
        Parent,
        Parachain(para),
        AccountId32 {
            network: None,
            id: who.into(),
        },
    );
    parachain::location_converter::LocationConverter::convert_location(&location.into()).unwrap()
}

pub fn sys_sibling_account_account_id(
    para: u32,
    who: sp_runtime::AccountId32,
) -> system_chain::AccountId {
    let location = (
        Parent,
        Parachain(para),
        AccountId32 {
            network: None,
            id: who.into(),
        },
    );
    system_chain::location_converter::LocationConverter::convert_location(&location.into()).unwrap()
}

pub fn sh_sibling_account_account_id(
    para: u32,
    who: sp_runtime::AccountId32,
) -> storagehub::AccountId {
    let location = (
        Parent,
        Parachain(para),
        AccountId32 {
            network: None,
            id: who.into(),
        },
    );
    storagehub::configs::xcm_config::LocationToAccountId::convert_location(&location.into())
        .unwrap()
}

pub fn parent_account_account_id(who: sp_runtime::AccountId32) -> parachain::AccountId {
    let location = (
        Parent,
        AccountId32 {
            network: None,
            id: who.into(),
        },
    );
    parachain::location_converter::LocationConverter::convert_location(&location.into()).unwrap()
}

pub fn sys_parent_account_account_id(who: sp_runtime::AccountId32) -> system_chain::AccountId {
    let location = (
        Parent,
        AccountId32 {
            network: None,
            id: who.into(),
        },
    );
    system_chain::location_converter::LocationConverter::convert_location(&location.into()).unwrap()
}

pub fn sh_parent_account_account_id(who: sp_runtime::AccountId32) -> storagehub::AccountId {
    let location = (
        Parent,
        AccountId32 {
            network: None,
            id: who.into(),
        },
    );
    storagehub::configs::xcm_config::LocationToAccountId::convert_location(&location.into())
        .unwrap()
}

pub fn para_ext(para_id: u32) -> sp_io::TestExternalities {
    use parachain::{MsgQueue, Runtime, System};

    let mut t = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Runtime> {
        balances: vec![
            (ALICE, INITIAL_BALANCE),
            (CHARLIE, INITIAL_BALANCE),
            (parent_account_id(), INITIAL_BALANCE),
            (sibling_account_id(SH_PARA_ID), 10 * INITIAL_BALANCE),
            (sibling_account_id(SYS_PARA_ID), 10 * INITIAL_BALANCE),
        ],
        dev_accounts: None,
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {
        sp_tracing::try_init_simple();
        System::set_block_number(1);
        MsgQueue::set_para_id(para_id.into());
    });
    ext
}

pub fn sys_ext(para_id: u32) -> sp_io::TestExternalities {
    use system_chain::{MsgQueue, Runtime, System};

    let mut t = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Runtime> {
        balances: vec![
            (ALICE, INITIAL_BALANCE),
            (sys_parent_account_id(), INITIAL_BALANCE),
            (
                sys_sibling_account_id(NON_SYS_PARA_ID),
                10 * INITIAL_BALANCE,
            ),
        ],
        dev_accounts: None,
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {
        sp_tracing::try_init_simple();
        System::set_block_number(1);
        MsgQueue::set_para_id(para_id.into());
    });
    ext
}

pub fn sh_ext() -> sp_io::TestExternalities {
    use storagehub::{MsgQueue, Runtime, System};

    let mut t = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Runtime> {
        balances: vec![
            (ALICE, INITIAL_BALANCE),
            (BOB, INITIAL_BALANCE),
            (sh_parent_account_id(), INITIAL_BALANCE),
            (sh_sibling_account_id(NON_SYS_PARA_ID), INITIAL_BALANCE),
            (TreasuryAccount::get(), ExistentialDeposit::get()),
        ],
        dev_accounts: None,
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {
        sp_tracing::try_init_simple();
        System::set_block_number(1);
        MsgQueue::set_para_id(SH_PARA_ID.into());
    });
    ext
}

pub fn relay_ext() -> sp_io::TestExternalities {
    use relay_chain::{Runtime, RuntimeOrigin, System, Uniques};

    let mut t = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Runtime> {
        balances: vec![
            (ALICE, INITIAL_BALANCE),
            (child_account_id(SH_PARA_ID), INITIAL_BALANCE),
            (child_account_id(SYS_PARA_ID), INITIAL_BALANCE),
            (child_account_id(NON_SYS_PARA_ID), INITIAL_BALANCE),
        ],
        dev_accounts: None,
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {
        System::set_block_number(1);
        assert_eq!(
            Uniques::force_create(RuntimeOrigin::root(), 1, ALICE, true),
            Ok(())
        );
        assert_eq!(
            Uniques::mint(
                RuntimeOrigin::signed(ALICE),
                1,
                42,
                child_account_id(SH_PARA_ID)
            ),
            Ok(())
        );
    });
    ext
}

pub type RelayChainPalletXcm = pallet_xcm::Pallet<relay_chain::Runtime>;
pub type StorageHubPalletXcm = pallet_xcm::Pallet<storagehub::Runtime>;
pub type ParachainPalletXcm = pallet_xcm::Pallet<parachain::Runtime>;
pub type SystemChainPalletXcm = pallet_xcm::Pallet<system_chain::Runtime>;
