use frame_support::assert_noop;
use frame_support::{assert_ok, traits::fungible::Inspect};
use xcm::prelude::*;
use xcm_executor::traits::ConvertLocation;
use xcm_simulator::TestExt;

use crate::relay_chain::location_converter::LocationConverter;
use crate::system_chain;
use crate::{
    constants::{ALICE, BOB, CENTS, INITIAL_BALANCE},
    parachain, relay_chain, storagehub, MockNet, MockParachain, MockSystemChain, Relay, StorageHub,
};
use codec::Encode;
use frame_support::dispatch::GetDispatchInfo;
use pallet_balances;
use pallet_storage_providers::types::MultiAddress;
use shp_traits::ProvidersInterface;

mod relay_token {
    use super::*;

    #[test]
    fn relay_token_transfer_from_and_to_relay_teleports_and_works() {
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
        // The extrinsic figures out it should do a teleport asset transfer.
        Relay::execute_with(|| {
            // The parachain id of StorageHub, defined in `lib.rs`.
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

            // The funds of the sovereign account of StorageHub in the Relay Chain should not increase, since this is a teleport
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
            // StorageHub doesn't have a token of its own, so we always refer to this token,
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

            // The funds in StorageHub's sovereign account still remain the same
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

    #[test]
    fn relay_token_transfer_from_and_to_system_chain_teleports_and_works() {
        // Scenario:
        // ALICE on a system chain holds some of Relay Chain's native tokens.
        // She transfers them to BOB's account on StorageHub using a transfer.
        // BOB receives Relay Chain real native token on StorageHub, since StorageHub
        // is a system chain, it doesn't have its own token, so it uses the Relay Chain token.

        // We reset storage and messages.
        MockNet::reset();

        // ALICE starts with INITIAL_BALANCE on the system chain.
        MockSystemChain::execute_with(|| {
            assert_eq!(system_chain::Balances::balance(&ALICE), INITIAL_BALANCE);
        });

        // BOB starts with 0 on StorageHub.
        StorageHub::execute_with(|| {
            assert_eq!(storagehub::Balances::balance(&BOB), 0);
        });

        // We check that the sovereign account of StorageHub in the Relay Chain has INITIAL_BALANCE.
        Relay::execute_with(|| {
            let location = Parachain(1).into();
            let parachain_sovereign_account =
                LocationConverter::convert_location(&location).unwrap();
            assert_eq!(
                relay_chain::Balances::balance(&parachain_sovereign_account),
                INITIAL_BALANCE
            );
        });

        // ALICE on the system parachain sends some Relay Chain native tokens to BOB on StorageHub.
        // The transfer is done with the `transfer_assets` extrinsic in the XCM pallet.
        // The extrinsic figures out it should do a teleport asset transfer.
        MockSystemChain::execute_with(|| {
            // The location of StorageHub as seen from the system parachain, defined in `lib.rs`.
            let destination: Location = (Parent, Parachain(1)).into();
            let beneficiary: Location = AccountId32 {
                id: BOB.clone().into(),
                network: Some(NetworkId::Polkadot),
            }
            .into();

            // We need to use `u128` here for the conversion to work properly.
            // If we don't specify anything, it will be a `u64`, which the conversion
            // will turn into a non-fungible token instead of a fungible one.
            let assets: Assets = (Parent, 50u128 * CENTS).into();
            assert_ok!(system_chain::PolkadotXcm::transfer_assets(
                system_chain::RuntimeOrigin::signed(ALICE),
                Box::new(VersionedLocation::V4(destination.clone())),
                Box::new(VersionedLocation::V4(beneficiary)),
                Box::new(VersionedAssets::V4(assets)),
                0,
                WeightLimit::Unlimited,
            ));

            // ALICE now has less Relay Chain tokens.
            assert_eq!(
                system_chain::Balances::balance(&ALICE),
                INITIAL_BALANCE - 50 * CENTS
            );
        });

        // The funds of the sovereign account of the parachain should not increase, since this is a teleport
        Relay::execute_with(|| {
            let location = Parachain(1).into();
            let parachain_sovereign_account =
                LocationConverter::convert_location(&location).unwrap();
            assert_eq!(
                relay_chain::Balances::balance(&parachain_sovereign_account),
                INITIAL_BALANCE
            );
        });

        StorageHub::execute_with(|| {
            // On StorageHub, BOB has received the tokens
            assert_eq!(storagehub::Balances::balance(&BOB), 50 * CENTS);

            // BOB gives back half to ALICE on the system chain
            let destination: Location = (Parent, Parachain(2)).into();
            let beneficiary: Location = AccountId32 {
                id: ALICE.clone().into(),
                network: Some(NetworkId::Polkadot),
            }
            .into();
            // We specify `Parent` because we are referencing the Relay Chain token,
            // even though we are transferring between parachains.
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

        MockSystemChain::execute_with(|| {
            // ALICE's balance increases
            assert_eq!(
                system_chain::Balances::balance(&ALICE),
                INITIAL_BALANCE - 50 * CENTS + 25 * CENTS
            );
        });

        // The funds in the parachain's sovereign account still remain the same
        Relay::execute_with(|| {
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

    #[test]
    fn asset_transfer_does_not_work_from_other_non_system_parachain() {
        // Scenario:
        // ALICE on a non-system parachain holds some of Relay Chain's native tokens derivatives.
        // She wants to transfer them to BOB's account on StorageHub using a reserve transfer.
        // StorageHub does not allow this, as it only accepts teleports from the Relay Chain or a sibling system parachain.
        // This means, the transfer should fail.

        // We reset storage and messages.
        MockNet::reset();

        // ALICE starts with INITIAL_BALANCE on the non-system parachain.
        MockParachain::execute_with(|| {
            assert_eq!(parachain::Balances::balance(&ALICE), INITIAL_BALANCE);
        });

        // Which should equal the balance of that parachain's sovereign account in the Relay Chain (ALICE is the sole user)
        Relay::execute_with(|| {
            let location = Parachain(2004).into();
            let parachain_sovereign_account =
                LocationConverter::convert_location(&location).unwrap();
            assert_eq!(
                relay_chain::Balances::balance(&parachain_sovereign_account),
                INITIAL_BALANCE
            );
        });

        // BOB starts with 0 on StorageHub.
        StorageHub::execute_with(|| {
            assert_eq!(storagehub::Balances::balance(&BOB), 0);
        });

        // ALICE on the non-system parachain tries to send some Relay Chain native tokens derivatives to BOB on StorageHub.
        // The transfer is done with the `transfer_assets` extrinsic in the XCM pallet.
        // The extrinsic does a remote reserve transfer (using the Relay Chain as reserve) and then sends an XCM to StorageHub
        // with a ReserveAssetDeposited instruction.
        // StorageHub, since it does not accept reserve transfers (only DOT teleports) will error out, and the assets will be trapped
        // by XCM. The parachain will have to claim them back.
        MockParachain::execute_with(|| {
            // StorageHub's location as seen from the mocked parachain.
            let destination: Location = (Parent, Parachain(1)).into();
            let beneficiary: Location = AccountId32 {
                id: BOB.clone().into(),
                network: Some(NetworkId::Polkadot),
            }
            .into();
            // We need to use `u128` here for the conversion to work properly.
            // If we don't specify anything, it will be a `u64`, which the conversion
            // will turn into a non-fungible token instead of a fungible one.
            let assets: Assets = (Parent, 50u128 * CENTS).into();
            assert_ok!(parachain::PolkadotXcm::transfer_assets(
                parachain::RuntimeOrigin::signed(ALICE),
                Box::new(VersionedLocation::V4(destination.clone())),
                Box::new(VersionedLocation::V4(beneficiary)),
                Box::new(VersionedAssets::V4(assets)),
                0,
                WeightLimit::Unlimited,
            ),);

            // ALICE's balance should have decreased
            assert_eq!(
                parachain::Balances::balance(&ALICE),
                INITIAL_BALANCE - 50 * CENTS
            );
        });

        // The balance of the parachain's sovereign account in the Relay Chain should have decreased
        Relay::execute_with(|| {
            let location = Parachain(2004).into();
            let parachain_sovereign_account =
                LocationConverter::convert_location(&location).unwrap();
            assert_eq!(
                relay_chain::Balances::balance(&parachain_sovereign_account),
                INITIAL_BALANCE - 50 * CENTS
            );
        });

        // BOB still has 0 on StorageHub and an error should be in the events
        StorageHub::execute_with(|| {
            assert_eq!(storagehub::Balances::balance(&BOB), 0);

            crate::storagehub::System::assert_has_event(crate::storagehub::RuntimeEvent::MsgQueue(
                crate::mock_message_queue::Event::ExecutedDownward {
                    outcome: Outcome::Incomplete {
                        error: xcm::v3::Error::UntrustedReserveLocation,
                        used: Weight::zero(),
                    },
                    message_id: [
                        238, 253, 149, 77, 229, 46, 84, 171, 6, 85, 16, 169, 162, 163, 138, 158,
                        29, 209, 2, 194, 93, 45, 215, 46, 93, 126, 153, 94, 133, 129, 49, 2,
                    ],
                }
                .into(),
            ));
        });

        // This means the assets were trapped by XCM, and the parachain will have to claim them back.
    }
}

mod root {
    use super::*;

    #[test]
    fn relay_chain_can_be_root_origin() {
        // Scenario:
        // The Relay Chain (and its executive body) should be able to execute extrinsics as the root origin in StorageHub.

        // We reset storage and messages.
        MockNet::reset();

        // We will set BOB's balance on StorageHub using the Relay Chain.
        let destination: Location = Parachain(1).into();
        let beneficiary: Location = AccountId32 {
            id: BOB.clone().into(),
            network: Some(NetworkId::Polkadot),
        }
        .into();

        // We check that BOB's balance on StorageHub is 0.
        StorageHub::execute_with(|| {
            assert_eq!(storagehub::Balances::balance(&BOB), 0);
        });

        // We set BOB's balance to 100 CENTS on StorageHub using the Relay Chain as the root origin.
        Relay::execute_with(|| {
            let call = storagehub::RuntimeCall::Balances(pallet_balances::Call::<
                storagehub::Runtime,
            >::force_set_balance {
                who: sp_runtime::MultiAddress::Id(BOB.clone()),
                new_free: 100 * CENTS,
            });
            let estimated_weight = call.get_dispatch_info().weight;
            let message: Xcm<()> = vec![
                UnpaidExecution {
                    weight_limit: Unlimited,
                    check_origin: Some(Parent.into()),
                },
                Transact {
                    origin_kind: OriginKind::Superuser,
                    require_weight_at_most: estimated_weight,
                    call: call.encode().into(),
                },
            ]
            .into();
            assert_ok!(relay_chain::XcmPallet::send_xcm(Here, destination, message));
        });

        // We now check that BOB's balance on StorageHub is 100 CENTS.
        StorageHub::execute_with(|| {
            assert_eq!(storagehub::Balances::balance(&BOB), 100 * CENTS);
        });
    }
}
mod providers {
    use pallet_storage_providers::types::MaxMultiAddressAmount;
    use sp_runtime::BoundedVec;

    use crate::sh_sibling_account_id;

    use super::*;

    #[test]
    fn parachain_should_be_able_to_request_to_register_as_provider() {
        // Scenario:
        // A parachain should be able to request to register as a provider in StorageHub.

        // We reset storage and messages.
        MockNet::reset();

        // We check that the parachain is not a provider in the StorageHub and it has some balance
        StorageHub::execute_with(|| {
            assert!(storagehub::Providers::get_provider_id(sh_sibling_account_id(2004)).is_none());
            assert_eq!(
                storagehub::Balances::balance(&sh_sibling_account_id(2004)),
                10 * INITIAL_BALANCE
            );
        });

        // The parachain requests to register as a provider in the StorageHub.
        // It has to have balance on StorageHub, which could be easily achieved by teleporting some tokens from the Relay Chain.
        // TODO: Maybe we should allow reserve transfer using the Relay Chain as reserve? It gets a bit messy but would make it easier
        // for parachains to interact with StorageHub
        MockParachain::execute_with(|| {
            let destination: Location = (Parent, Parachain(1)).into();
            let mut multiaddresses: BoundedVec<
                MultiAddress<storagehub::Runtime>,
                MaxMultiAddressAmount<storagehub::Runtime>,
            > = BoundedVec::new();
            multiaddresses.force_push(
                "/ip4/127.0.0.1/udp/1234"
                    .as_bytes()
                    .to_vec()
                    .try_into()
                    .unwrap(),
            );

            let call = storagehub::RuntimeCall::Providers(pallet_storage_providers::Call::<
                storagehub::Runtime,
            >::request_bsp_sign_up {
                capacity: 10,
                multiaddresses,
                payment_account: sh_sibling_account_id(2004),
            });
            let estimated_weight = call.get_dispatch_info().weight;
            // Remember, this message will be executed from the context of StorageHub
            let message: Xcm<()> = vec![
                WithdrawAsset((Parent, 100 * CENTS).into()),
                BuyExecution {
                    fees: (Parent, 100 * CENTS).into(),
                    weight_limit: Unlimited,
                },
                Transact {
                    origin_kind: OriginKind::SovereignAccount,
                    require_weight_at_most: estimated_weight,
                    call: call.encode().into(),
                },
                RefundSurplus,
                DepositAsset {
                    assets: Wild(All),
                    beneficiary: (Parent, Parachain(2004)).into(),
                },
            ]
            .into();
            assert_ok!(parachain::PolkadotXcm::send_xcm(Here, destination, message));
        });

        StorageHub::execute_with(|| {
            // We check that the parachain has correctly requested to sign up as BSP in StorageHub.
            assert!(
                storagehub::Providers::get_sign_up_request(&sh_sibling_account_id(2004)).is_ok()
            );

            // And that it's balance has changed
            assert!(
                storagehub::Balances::balance(&sh_sibling_account_id(2004)) != 10 * INITIAL_BALANCE
            );
        });
    }
}
