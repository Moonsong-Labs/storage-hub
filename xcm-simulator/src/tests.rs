use frame_support::{assert_ok, traits::fungible::Inspect};
use xcm::prelude::*;
use xcm_executor::traits::ConvertLocation;
use xcm_simulator::TestExt;

use crate::relay_chain::location_converter::LocationConverter;
use crate::{
    constants::{ALICE, BOB, CENTS, INITIAL_BALANCE},
    parachain, relay_chain, storagehub, MockNet, Relay, StorageHub,
};

#[test]
fn asset_transfer_from_relay_teleports_and_works() {
    // Scenario:
    // ALICE on the relay chain holds some of Relay Chain's native tokens.
    // She transfers them to BOB's account on StorageHub using a transfer.
    // BOB receives Relay Chain real native token on StorageHub, since StorageHub
    // is a system chain, it doesn't have its own token, so it uses the Relay Chain token.

    // We reset storage and messages.
    MockNet::reset();

    // ALICE starts with INITIAL_BALANCE on the relay chain.
    Relay::execute_with(|| {
        assert_eq!(relay_chain::Balances::balance(&ALICE), INITIAL_BALANCE);
    });

    // BOB starts with 0 on StorageHub.
    StorageHub::execute_with(|| {
        assert_eq!(storagehub::Balances::balance(&BOB), 0);
    });

    // ALICE on the Relay Chain sends some Relay Chain native tokens to BOB on StorageHub.
    // The transfer is done with the `transfer_assets` extrinsic in the XCM pallet.
    // The extrinsic figures out it should do a reserve asset transfer
    // with the local chain as reserve.
    Relay::execute_with(|| {
        // The parachain id of `ParaA`, defined in `lib.rs`.
        let destination: Location = Parachain(1).into();
        let beneficiary: Location = AccountId32 {
            id: BOB.clone().into(),
            network: Some(NetworkId::Polkadot),
        }
        .into();

        // We check that the sovereign account of the parachain has INITIAL_BALANCE.
        let parachains_sovereign_account =
            LocationConverter::convert_location(&destination).unwrap();
        assert_eq!(
            relay_chain::Balances::balance(&parachains_sovereign_account),
            INITIAL_BALANCE
        );

        // We need to use `u128` here for the conversion to work properly.
        // If we don't specify anything, it will be a `u64`, which the conversion
        // will turn into a non-fungible token instead of a fungible one.
        let assets: Assets = (Here, 50u128 * CENTS).into();
        assert_ok!(relay_chain::XcmPallet::transfer_assets(
            relay_chain::RuntimeOrigin::signed(ALICE),
            Box::new(VersionedLocation::V4(destination.clone())),
            Box::new(VersionedLocation::V4(beneficiary)),
            Box::new(VersionedAssets::V4(assets)),
            0,
            WeightLimit::Unlimited,
        ));

        // ALICE now has less Relay Chain tokens.
        assert_eq!(
            relay_chain::Balances::balance(&ALICE),
            INITIAL_BALANCE - 50 * CENTS
        );

        // The funds of the sovereign account of the parachain should not increase, since this is a teleport
        assert_eq!(
            relay_chain::Balances::balance(&parachains_sovereign_account),
            INITIAL_BALANCE
        );
    });

    StorageHub::execute_with(|| {
        // On StorageHub, BOB has received the tokens
        assert_eq!(storagehub::Balances::balance(&BOB), 50 * CENTS);

        // BOB gives back half to ALICE on the relay chain
        let destination: Location = Parent.into();
        let beneficiary: Location = AccountId32 {
            id: ALICE.clone().into(),
            network: Some(NetworkId::Polkadot),
        }
        .into();
        // We specify `Parent` because we are referencing the Relay Chain token.
        // This chain doesn't have a token of its own, so we always refer to this token,
        // and we do so by the Location of the Relay Chain.
        let assets: Assets = (Parent, 25u128 * CENTS).into();
        assert_ok!(storagehub::PolkadotXcm::transfer_assets(
            storagehub::RuntimeOrigin::signed(BOB),
            Box::new(VersionedLocation::V4(destination)),
            Box::new(VersionedLocation::V4(beneficiary)),
            Box::new(VersionedAssets::V4(assets)),
            0,
            WeightLimit::Unlimited,
        ));

        // BOB's balance decreased
        assert_eq!(storagehub::Balances::balance(&BOB), 25 * CENTS);
    });

    Relay::execute_with(|| {
        // ALICE's balance increases
        assert_eq!(
            relay_chain::Balances::balance(&ALICE),
            INITIAL_BALANCE - 50 * CENTS + 25 * CENTS
        );

        // The funds in the parachain's sovereign account still remain the same
        let parachain: Location = Parachain(1).into();
        let parachains_sovereign_account =
            relay_chain::location_converter::LocationConverter::convert_location(&parachain)
                .unwrap();
        assert_eq!(
            relay_chain::Balances::balance(&parachains_sovereign_account),
            INITIAL_BALANCE
        );
    });
}
