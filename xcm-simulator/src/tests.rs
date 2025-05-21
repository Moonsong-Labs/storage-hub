use codec::Encode;
use frame_support::{
    assert_ok,
    traits::{fungible::Inspect, OnFinalize, OnPoll},
    BoundedVec,
};
use pallet_balances;
use pallet_file_system;
use pallet_storage_providers::types::{MaxMultiAddressAmount, MultiAddress};
use shp_traits::{ReadBucketsInterface, ReadProvidersInterface};
use sp_core::H256;
use sp_runtime::bounded_vec;
use sp_weights::WeightMeter;
use xcm::prelude::*;
use xcm_executor::traits::ConvertLocation;
use xcm_simulator::TestExt;
use xcm_simulator::XcmError::UntrustedReserveLocation;

use crate::{
    constants::{ALICE, BOB, CENTS, INITIAL_BALANCE},
    parachain, relay_chain,
    relay_chain::location_converter::LocationConverter,
    sh_sibling_account_id, storagehub,
    storagehub::configs::MaxBatchConfirmStorageRequests,
    system_chain, MockNet, MockParachain, MockSystemChain, Relay, StorageHub, NON_SYS_PARA_ID,
};

fn sh_run_to_block(n: u32) {
    while storagehub::System::block_number() < n {
        storagehub::System::set_block_number(storagehub::System::block_number() + 1);

        // Trigger on_poll hook execution.
        storagehub::ProofsDealer::on_poll(
            storagehub::System::block_number(),
            &mut WeightMeter::new(),
        );

        // Trigger on_finalize hook execution.
        storagehub::ProofsDealer::on_finalize(storagehub::System::block_number());
    }
}

mod relay_token {
    use crate::{child_account_id, SH_PARA_ID};

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

        // BOB starts with INITIAL_BALANCE on StorageHub.
        StorageHub::execute_with(|| {
            assert_eq!(storagehub::Balances::balance(&BOB), INITIAL_BALANCE);
        });

        // ALICE on the Relay Chain sends some Relay Chain native tokens to BOB on StorageHub.
        // The transfer is done with the `transfer_assets` extrinsic in the XCM pallet.
        // The extrinsic figures out it should do a teleport asset transfer.
        Relay::execute_with(|| {
            // The parachain id of StorageHub, defined in `lib.rs`.
            let destination: Location = Parachain(SH_PARA_ID).into();
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
                Box::new(VersionedLocation::V5(destination.clone())),
                Box::new(VersionedLocation::V5(beneficiary.clone())),
                Box::new(VersionedAssets::V5(assets)),
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
            assert_eq!(
                storagehub::Balances::balance(&BOB),
                INITIAL_BALANCE + 50 * CENTS
            );

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
                Box::new(VersionedLocation::V5(destination)),
                Box::new(VersionedLocation::V5(beneficiary)),
                Box::new(VersionedAssets::V5(assets)),
                0,
                WeightLimit::Unlimited,
            ));

            // BOB's balance decreased
            assert_eq!(
                storagehub::Balances::balance(&BOB),
                INITIAL_BALANCE + 25 * CENTS
            );
        });

        Relay::execute_with(|| {
            // ALICE's balance increases
            assert_eq!(
                relay_chain::Balances::balance(&ALICE),
                INITIAL_BALANCE - 50 * CENTS + 25 * CENTS
            );

            // The funds in StorageHub's sovereign account still remain the same
            let parachain: Location = Parachain(SH_PARA_ID).into();
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

        // BOB starts with INITIAL_BALANCE on StorageHub.
        StorageHub::execute_with(|| {
            assert_eq!(storagehub::Balances::balance(&BOB), INITIAL_BALANCE);
        });

        // We check that the sovereign account of StorageHub in the Relay Chain has INITIAL_BALANCE.
        Relay::execute_with(|| {
            let parachain_sovereign_account_in_relay = child_account_id(SH_PARA_ID);
            assert_eq!(
                relay_chain::Balances::balance(&parachain_sovereign_account_in_relay),
                INITIAL_BALANCE
            );
        });

        // ALICE on the system parachain sends some Relay Chain native tokens to BOB on StorageHub.
        // The transfer is done with the `transfer_assets` extrinsic in the XCM pallet.
        // The extrinsic figures out it should do a teleport asset transfer.
        MockSystemChain::execute_with(|| {
            // The location of StorageHub as seen from the system parachain, defined in `lib.rs`.
            let destination: Location = (Parent, Parachain(SH_PARA_ID)).into();
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
                Box::new(VersionedLocation::V5(destination.clone())),
                Box::new(VersionedLocation::V5(beneficiary)),
                Box::new(VersionedAssets::V5(assets)),
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
            let location = Parachain(SH_PARA_ID).into();
            let parachain_sovereign_account =
                LocationConverter::convert_location(&location).unwrap();
            assert_eq!(
                relay_chain::Balances::balance(&parachain_sovereign_account),
                INITIAL_BALANCE
            );
        });

        StorageHub::execute_with(|| {
            // On StorageHub, BOB has received the tokens
            assert_eq!(
                storagehub::Balances::balance(&BOB),
                INITIAL_BALANCE + 50 * CENTS
            );

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
                Box::new(VersionedLocation::V5(destination)),
                Box::new(VersionedLocation::V5(beneficiary)),
                Box::new(VersionedAssets::V5(assets)),
                0,
                WeightLimit::Unlimited,
            ));

            // BOB's balance decreased
            assert_eq!(
                storagehub::Balances::balance(&BOB),
                INITIAL_BALANCE + 25 * CENTS
            );
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
            let parachain: Location = Parachain(SH_PARA_ID).into();
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

        // BOB starts with INITIAL_BALANCE on StorageHub.
        StorageHub::execute_with(|| {
            assert_eq!(storagehub::Balances::balance(&BOB), INITIAL_BALANCE);
        });

        // ALICE on the non-system parachain tries to send some Relay Chain native tokens derivatives to BOB on StorageHub.
        // The transfer is done with the `transfer_assets` extrinsic in the XCM pallet.
        // The extrinsic does a remote reserve transfer (using the Relay Chain as reserve) and then sends an XCM to StorageHub
        // with a ReserveAssetDeposited instruction.
        // StorageHub, since it does not accept reserve transfers (only DOT teleports) will error out, and the assets will be trapped
        // by XCM. The parachain will have to claim them back.
        MockParachain::execute_with(|| {
            // StorageHub's location as seen from the mocked parachain.
            let destination: Location = (Parent, Parachain(SH_PARA_ID)).into();
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
                Box::new(VersionedLocation::V5(destination.clone())),
                Box::new(VersionedLocation::V5(beneficiary)),
                Box::new(VersionedAssets::V5(assets)),
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

        // BOB still has INITIAL_BALANCE on StorageHub and an error should be in the events
        StorageHub::execute_with(|| {
            assert_eq!(storagehub::Balances::balance(&BOB), INITIAL_BALANCE);

            crate::storagehub::System::assert_has_event(crate::storagehub::RuntimeEvent::MsgQueue(
                crate::mock_message_queue::Event::ExecutedDownward {
                    outcome: Outcome::Incomplete {
                        error: UntrustedReserveLocation,
                        used: Weight::zero(),
                    },
                    message_id: [
                        66, 73, 159, 49, 118, 123, 168, 151, 78, 108, 148, 58, 78, 235, 68, 214,
                        87, 145, 94, 202, 107, 210, 243, 162, 142, 23, 124, 119, 119, 156, 199,
                        111,
                    ],
                }
                .into(),
            ));
        });

        // This means the assets were trapped by XCM, and the parachain will have to claim them back.
    }
}

mod root {
    use crate::SH_PARA_ID;

    use super::*;

    #[test]
    fn relay_chain_can_be_root_origin() {
        // Scenario:
        // The Relay Chain (and its executive body) should be able to execute extrinsics as the root origin in StorageHub.

        // We reset storage and messages.
        MockNet::reset();

        // We will set BOB's balance on StorageHub using the Relay Chain.
        let destination: Location = Parachain(SH_PARA_ID).into();

        // We check that BOB's balance on StorageHub is INITIAL_BALANCE.
        StorageHub::execute_with(|| {
            assert_eq!(storagehub::Balances::balance(&BOB), INITIAL_BALANCE);
        });

        // We set BOB's balance to 100 CENTS on StorageHub using the Relay Chain as the root origin.
        Relay::execute_with(|| {
            let call = storagehub::RuntimeCall::Balances(pallet_balances::Call::<
                storagehub::Runtime,
            >::force_set_balance {
                who: sp_runtime::MultiAddress::Id(BOB.clone()),
                new_free: 100 * CENTS,
            });
            let message: Xcm<()> = vec![
                UnpaidExecution {
                    weight_limit: Unlimited,
                    check_origin: Some(Parent.into()),
                },
                Transact {
                    origin_kind: OriginKind::Superuser,
                    fallback_max_weight: None,
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
    use pallet_randomness::LatestOneEpochAgoRandomness;
    use sp_core::H256;
    use sp_runtime::BoundedVec;
    use storagehub::configs::{BspSignUpLockPeriod, SpMinDeposit};

    use crate::{
        sh_sibling_account_id, storagehub::configs::MinBlocksBetweenCapacityChanges,
        NON_SYS_PARA_ID, SH_PARA_ID,
    };

    use super::*;

    #[test]
    fn parachain_should_be_able_to_request_to_register_as_provider() {
        // Scenario:
        // A parachain should be able to request to register as a provider in StorageHub.

        // We reset storage and messages.
        MockNet::reset();

        // We check that the parachain is not a provider in the StorageHub and it has some balance
        StorageHub::execute_with(|| {
            assert!(
                storagehub::Providers::get_provider_id(&sh_sibling_account_id(NON_SYS_PARA_ID))
                    .is_none()
            );
            assert_eq!(
                storagehub::Balances::balance(&sh_sibling_account_id(NON_SYS_PARA_ID)),
                INITIAL_BALANCE
            );
        });

        // The parachain requests to register as a provider in the StorageHub.
        // It has to have balance on StorageHub, which could be easily achieved by teleporting some tokens from the Relay Chain.
        // TODO: Maybe we should allow reserve transfer using the Relay Chain as reserve? It gets a bit messy but would make it easier
        // for parachains to interact with StorageHub
        MockParachain::execute_with(|| {
            let destination: Location = (Parent, Parachain(SH_PARA_ID)).into();
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
                payment_account: sh_sibling_account_id(NON_SYS_PARA_ID),
            });
            // Remember, this message will be executed from the context of StorageHub
            let message: Xcm<()> = vec![
                WithdrawAsset((Parent, 100 * CENTS).into()),
                BuyExecution {
                    fees: (Parent, 100 * CENTS).into(),
                    weight_limit: Unlimited,
                },
                Transact {
                    origin_kind: OriginKind::SovereignAccount,
                    fallback_max_weight: None,
                    call: call.encode().into(),
                },
                RefundSurplus,
                DepositAsset {
                    assets: Wild(All),
                    beneficiary: (Parent, Parachain(NON_SYS_PARA_ID)).into(),
                },
            ]
            .into();
            assert_ok!(parachain::PolkadotXcm::send_xcm(Here, destination, message));
        });

        StorageHub::execute_with(|| {
            // We check that the parachain has correctly requested to sign up as BSP in StorageHub.
            assert!(
                storagehub::Providers::get_sign_up_request(&sh_sibling_account_id(NON_SYS_PARA_ID))
                    .is_ok()
            );

            // And that it's balance has changed
            assert!(
                storagehub::Balances::balance(&sh_sibling_account_id(NON_SYS_PARA_ID))
                    < 10 * SpMinDeposit::get()
            );
        });
    }

    #[test]
    fn parachain_should_be_able_to_confirm_register_as_provider() {
        // Scenario:
        // A parachain should be able to request to register as a provider in StorageHub and the confirm that request
        // after fresh enough randomness is available to get its random Provider ID.

        // We reset storage and messages.
        MockNet::reset();

        // We check that the parachain is not a provider in the StorageHub and it has some balance
        StorageHub::execute_with(|| {
            assert!(
                storagehub::Providers::get_provider_id(&sh_sibling_account_id(NON_SYS_PARA_ID))
                    .is_none()
            );
            assert_eq!(
                storagehub::Balances::balance(&sh_sibling_account_id(NON_SYS_PARA_ID)),
                INITIAL_BALANCE
            );
        });

        // The parachain requests to register as a provider in the StorageHub.
        // It has to have balance on StorageHub, which could be easily achieved by teleporting some tokens from the Relay Chain.
        // TODO: Maybe we should allow reserve transfer using the Relay Chain as reserve? It gets a bit messy but would make it easier
        // for parachains to interact with StorageHub
        MockParachain::execute_with(|| {
            let destination: Location = (Parent, Parachain(SH_PARA_ID)).into();
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
                payment_account: sh_sibling_account_id(NON_SYS_PARA_ID),
            });
            // Remember, this message will be executed from the context of StorageHub
            let message: Xcm<()> = vec![
                WithdrawAsset((Parent, 100 * CENTS).into()),
                BuyExecution {
                    fees: (Parent, 100 * CENTS).into(),
                    weight_limit: Unlimited,
                },
                Transact {
                    origin_kind: OriginKind::SovereignAccount,
                    fallback_max_weight: None,
                    call: call.encode().into(),
                },
                RefundSurplus,
                DepositAsset {
                    assets: Wild(All),
                    beneficiary: (Parent, Parachain(NON_SYS_PARA_ID)).into(),
                },
            ]
            .into();
            assert_ok!(parachain::PolkadotXcm::send_xcm(Here, destination, message));
        });

        StorageHub::execute_with(|| {
            // We check that the parachain has correctly requested to sign up as BSP in StorageHub.
            assert!(
                storagehub::Providers::get_sign_up_request(&sh_sibling_account_id(NON_SYS_PARA_ID))
                    .is_ok()
            );

            // And that it's balance has changed
            assert!(
                storagehub::Balances::balance(&sh_sibling_account_id(NON_SYS_PARA_ID))
                    < 10 * SpMinDeposit::get()
            );

            // We now make randomness available to be able to confirm registration
            let randomness = H256::random();
            let latest_valid_block = 100;
            LatestOneEpochAgoRandomness::<storagehub::Runtime>::put((
                randomness,
                latest_valid_block,
            ));
        });

        // The parachain confirms the registration as a provider in StorageHub.
        MockParachain::execute_with(|| {
            let destination: Location = (Parent, Parachain(SH_PARA_ID)).into();
            let call = storagehub::RuntimeCall::Providers(pallet_storage_providers::Call::<
                storagehub::Runtime,
            >::confirm_sign_up {
                provider_account: None,
            });
            // Remember, this message will be executed from the context of StorageHub
            let message: Xcm<()> = vec![
                WithdrawAsset((Parent, 100 * CENTS).into()),
                BuyExecution {
                    fees: (Parent, 100 * CENTS).into(),
                    weight_limit: Unlimited,
                },
                Transact {
                    origin_kind: OriginKind::SovereignAccount,
                    fallback_max_weight: None,
                    call: call.encode().into(),
                },
                RefundSurplus,
                DepositAsset {
                    assets: Wild(All),
                    beneficiary: (Parent, Parachain(NON_SYS_PARA_ID)).into(),
                },
            ]
            .into();
            assert_ok!(parachain::PolkadotXcm::send_xcm(Here, destination, message));
        });

        StorageHub::execute_with(|| {
            // We check that the parachain is now a provider in StorageHub.
            let parachain_provider_id =
                storagehub::Providers::get_provider_id(&sh_sibling_account_id(NON_SYS_PARA_ID))
                    .unwrap();
            assert!(storagehub::Providers::is_provider(parachain_provider_id),);
        });
    }

    #[test]
    fn parachain_should_be_able_to_cancel_register_as_provider() {
        // Scenario:
        // A parachain should be able to request to register as a provider in StorageHub and then cancel that request.

        // We reset storage and messages.
        MockNet::reset();

        // We check that the parachain is not a provider in the StorageHub and it has some balance
        StorageHub::execute_with(|| {
            assert!(
                storagehub::Providers::get_provider_id(&sh_sibling_account_id(NON_SYS_PARA_ID))
                    .is_none()
            );
            assert_eq!(
                storagehub::Balances::balance(&sh_sibling_account_id(NON_SYS_PARA_ID)),
                INITIAL_BALANCE
            );
        });

        // The parachain requests to register as a provider in the StorageHub.
        // It has to have balance on StorageHub, which could be easily achieved by teleporting some tokens from the Relay Chain.
        // TODO: Maybe we should allow reserve transfer using the Relay Chain as reserve? It gets a bit messy but would make it easier
        // for parachains to interact with StorageHub
        MockParachain::execute_with(|| {
            let destination: Location = (Parent, Parachain(SH_PARA_ID)).into();
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
                payment_account: sh_sibling_account_id(NON_SYS_PARA_ID),
            });
            // Remember, this message will be executed from the context of StorageHub
            let message: Xcm<()> = vec![
                WithdrawAsset((Parent, 100 * CENTS).into()),
                BuyExecution {
                    fees: (Parent, 100 * CENTS).into(),
                    weight_limit: Unlimited,
                },
                Transact {
                    origin_kind: OriginKind::SovereignAccount,
                    fallback_max_weight: None,
                    call: call.encode().into(),
                },
                RefundSurplus,
                DepositAsset {
                    assets: Wild(All),
                    beneficiary: (Parent, Parachain(NON_SYS_PARA_ID)).into(),
                },
            ]
            .into();
            assert_ok!(parachain::PolkadotXcm::send_xcm(Here, destination, message));
        });

        let mut parachain_balance_in_storagehub = 0;
        StorageHub::execute_with(|| {
            // We check that the parachain has correctly requested to sign up as BSP in StorageHub.
            assert!(
                storagehub::Providers::get_sign_up_request(&sh_sibling_account_id(NON_SYS_PARA_ID))
                    .is_ok()
            );

            // And that it's balance has changed
            parachain_balance_in_storagehub =
                storagehub::Balances::balance(&sh_sibling_account_id(NON_SYS_PARA_ID));
            assert!(parachain_balance_in_storagehub < 10 * SpMinDeposit::get());
        });

        // The parachain cancels the registration as a provider in StorageHub.
        MockParachain::execute_with(|| {
            let destination: Location = (Parent, Parachain(SH_PARA_ID)).into();
            let call = storagehub::RuntimeCall::Providers(pallet_storage_providers::Call::<
                storagehub::Runtime,
            >::cancel_sign_up {});
            // Remember, this message will be executed from the context of StorageHub
            let message: Xcm<()> = vec![
                WithdrawAsset((Parent, 100 * CENTS).into()),
                BuyExecution {
                    fees: (Parent, 100 * CENTS).into(),
                    weight_limit: Unlimited,
                },
                Transact {
                    origin_kind: OriginKind::SovereignAccount,
                    fallback_max_weight: None,
                    call: call.encode().into(),
                },
                RefundSurplus,
                DepositAsset {
                    assets: Wild(All),
                    beneficiary: (Parent, Parachain(NON_SYS_PARA_ID)).into(),
                },
            ]
            .into();
            assert_ok!(parachain::PolkadotXcm::send_xcm(Here, destination, message));
        });

        StorageHub::execute_with(|| {
            // We check that the registration request no longer exists and that the deposit has been refunded
            assert!(
                storagehub::Providers::get_sign_up_request(&sh_sibling_account_id(NON_SYS_PARA_ID))
                    .is_err()
            );
            assert!(
                storagehub::Balances::balance(&sh_sibling_account_id(NON_SYS_PARA_ID))
                    > parachain_balance_in_storagehub
            );
        });
    }

    #[test]
    fn parachain_should_be_able_to_sign_off_as_provider() {
        // Scenario:
        // A parachain should be able to sign off as a Provider if its capacity used is 0.

        // We reset storage and messages.
        MockNet::reset();

        // We check that the parachain is not a provider in the StorageHub and it has some balance
        StorageHub::execute_with(|| {
            assert!(
                storagehub::Providers::get_provider_id(&sh_sibling_account_id(NON_SYS_PARA_ID))
                    .is_none()
            );
            assert_eq!(
                storagehub::Balances::balance(&sh_sibling_account_id(NON_SYS_PARA_ID)),
                INITIAL_BALANCE
            );
        });

        // The parachain requests to register as a provider in the StorageHub.
        // It has to have balance on StorageHub, which could be easily achieved by teleporting some tokens from the Relay Chain.
        // TODO: Maybe we should allow reserve transfer using the Relay Chain as reserve? It gets a bit messy but would make it easier
        // for parachains to interact with StorageHub
        MockParachain::execute_with(|| {
            let destination: Location = (Parent, Parachain(SH_PARA_ID)).into();
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
                payment_account: sh_sibling_account_id(NON_SYS_PARA_ID),
            });
            // Remember, this message will be executed from the context of StorageHub
            let message: Xcm<()> = vec![
                WithdrawAsset((Parent, 100 * CENTS).into()),
                BuyExecution {
                    fees: (Parent, 100 * CENTS).into(),
                    weight_limit: Unlimited,
                },
                Transact {
                    origin_kind: OriginKind::SovereignAccount,
                    fallback_max_weight: None,
                    call: call.encode().into(),
                },
                RefundSurplus,
                DepositAsset {
                    assets: Wild(All),
                    beneficiary: (Parent, Parachain(NON_SYS_PARA_ID)).into(),
                },
            ]
            .into();
            assert_ok!(parachain::PolkadotXcm::send_xcm(Here, destination, message));
        });

        StorageHub::execute_with(|| {
            // We check that the parachain has correctly requested to sign up as BSP in StorageHub.
            assert!(
                storagehub::Providers::get_sign_up_request(&sh_sibling_account_id(NON_SYS_PARA_ID))
                    .is_ok()
            );

            // And that it's balance has changed
            assert!(
                storagehub::Balances::balance(&sh_sibling_account_id(NON_SYS_PARA_ID))
                    < 10 * SpMinDeposit::get()
            );

            // We now make randomness available to be able to confirm registration
            let randomness = H256::random();
            let latest_valid_block = 100;
            LatestOneEpochAgoRandomness::<storagehub::Runtime>::put((
                randomness,
                latest_valid_block,
            ));
        });

        // The parachain confirms the registration as a provider in StorageHub.
        MockParachain::execute_with(|| {
            let destination: Location = (Parent, Parachain(SH_PARA_ID)).into();
            let call = storagehub::RuntimeCall::Providers(pallet_storage_providers::Call::<
                storagehub::Runtime,
            >::confirm_sign_up {
                provider_account: None,
            });
            // Remember, this message will be executed from the context of StorageHub
            let message: Xcm<()> = vec![
                WithdrawAsset((Parent, 100 * CENTS).into()),
                BuyExecution {
                    fees: (Parent, 100 * CENTS).into(),
                    weight_limit: Unlimited,
                },
                Transact {
                    origin_kind: OriginKind::SovereignAccount,
                    fallback_max_weight: None,
                    call: call.encode().into(),
                },
                RefundSurplus,
                DepositAsset {
                    assets: Wild(All),
                    beneficiary: (Parent, Parachain(NON_SYS_PARA_ID)).into(),
                },
            ]
            .into();
            assert_ok!(parachain::PolkadotXcm::send_xcm(Here, destination, message));
        });

        let mut parachain_balance_after_deposit = 0;
        StorageHub::execute_with(|| {
            // We check that the parachain is now a provider in StorageHub.
            let parachain_provider_id =
                storagehub::Providers::get_provider_id(&sh_sibling_account_id(NON_SYS_PARA_ID))
                    .unwrap();
            assert!(storagehub::Providers::is_provider(parachain_provider_id),);

            // We check that the parachain has 0 capacity used
            assert_eq!(
                storagehub::Providers::get_used_storage_of_bsp(&parachain_provider_id).unwrap(),
                0
            );

            // And we check its current balance in StorageHub (after deposit)
            parachain_balance_after_deposit =
                storagehub::Balances::balance(&sh_sibling_account_id(NON_SYS_PARA_ID));

            // Advance enough blocks to allow the parachain to sign off as BSP
            sh_run_to_block(storagehub::System::block_number() + BspSignUpLockPeriod::get());
        });

        // The parachain signs off as a provider in StorageHub.
        MockParachain::execute_with(|| {
            let destination: Location = (Parent, Parachain(SH_PARA_ID)).into();
            let call = storagehub::RuntimeCall::Providers(pallet_storage_providers::Call::<
                storagehub::Runtime,
            >::bsp_sign_off {});
            // Remember, this message will be executed from the context of StorageHub
            let message: Xcm<()> = vec![
                WithdrawAsset((Parent, 100 * CENTS).into()),
                BuyExecution {
                    fees: (Parent, 100 * CENTS).into(),
                    weight_limit: Unlimited,
                },
                Transact {
                    origin_kind: OriginKind::SovereignAccount,
                    fallback_max_weight: None,
                    call: call.encode().into(),
                },
                RefundSurplus,
                DepositAsset {
                    assets: Wild(All),
                    beneficiary: (Parent, Parachain(NON_SYS_PARA_ID)).into(),
                },
            ]
            .into();
            assert_ok!(parachain::PolkadotXcm::send_xcm(Here, destination, message));
        });

        StorageHub::execute_with(|| {
            // We check that the parachain is no longer a provider in StorageHub and has been returned its deposit.
            assert!(
                storagehub::Providers::get_provider_id(&sh_sibling_account_id(NON_SYS_PARA_ID))
                    .is_none()
            );
            assert!(
                storagehub::Balances::balance(&sh_sibling_account_id(NON_SYS_PARA_ID))
                    > parachain_balance_after_deposit
            );
        });
    }

    #[test]
    fn parachain_should_be_able_to_change_its_registered_capacity() {
        // Scenario:
        // A parachain should be able to request to register as a provider in StorageHub, confirm that request
        // and after the required cooldown period change its capacity provided.

        // We reset storage and messages.
        MockNet::reset();

        // We check that the parachain is not a provider in the StorageHub and it has some balance
        StorageHub::execute_with(|| {
            assert!(
                storagehub::Providers::get_provider_id(&sh_sibling_account_id(NON_SYS_PARA_ID))
                    .is_none()
            );
            assert_eq!(
                storagehub::Balances::balance(&sh_sibling_account_id(NON_SYS_PARA_ID)),
                INITIAL_BALANCE
            );
        });

        // The parachain requests to register as a provider in the StorageHub.
        // It has to have balance on StorageHub, which could be easily achieved by teleporting some tokens from the Relay Chain.
        // TODO: Maybe we should allow reserve transfer using the Relay Chain as reserve? It gets a bit messy but would make it easier
        // for parachains to interact with StorageHub
        MockParachain::execute_with(|| {
            let destination: Location = (Parent, Parachain(SH_PARA_ID)).into();
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
                payment_account: sh_sibling_account_id(NON_SYS_PARA_ID),
            });
            // Remember, this message will be executed from the context of StorageHub
            let message: Xcm<()> = vec![
                WithdrawAsset((Parent, 100 * CENTS).into()),
                BuyExecution {
                    fees: (Parent, 100 * CENTS).into(),
                    weight_limit: Unlimited,
                },
                Transact {
                    origin_kind: OriginKind::SovereignAccount,
                    fallback_max_weight: None,
                    call: call.encode().into(),
                },
                RefundSurplus,
                DepositAsset {
                    assets: Wild(All),
                    beneficiary: (Parent, Parachain(NON_SYS_PARA_ID)).into(),
                },
            ]
            .into();
            assert_ok!(parachain::PolkadotXcm::send_xcm(Here, destination, message));
        });

        let mut parachain_balance_after_deposit = 0;
        StorageHub::execute_with(|| {
            // We check that the parachain has correctly requested to sign up as BSP in StorageHub.
            assert!(
                storagehub::Providers::get_sign_up_request(&sh_sibling_account_id(NON_SYS_PARA_ID))
                    .is_ok()
            );

            // And that it's balance has changed
            parachain_balance_after_deposit =
                storagehub::Balances::balance(&sh_sibling_account_id(NON_SYS_PARA_ID));
            assert!(parachain_balance_after_deposit < 10 * SpMinDeposit::get());

            // We now make randomness available to be able to confirm registration
            let randomness = H256::random();
            let latest_valid_block = 100;
            LatestOneEpochAgoRandomness::<storagehub::Runtime>::put((
                randomness,
                latest_valid_block,
            ));
        });

        // The parachain confirms the registration as a provider in StorageHub.
        MockParachain::execute_with(|| {
            let destination: Location = (Parent, Parachain(SH_PARA_ID)).into();
            let call = storagehub::RuntimeCall::Providers(pallet_storage_providers::Call::<
                storagehub::Runtime,
            >::confirm_sign_up {
                provider_account: None,
            });
            // Remember, this message will be executed from the context of StorageHub
            let message: Xcm<()> = vec![
                WithdrawAsset((Parent, 100 * CENTS).into()),
                BuyExecution {
                    fees: (Parent, 100 * CENTS).into(),
                    weight_limit: Unlimited,
                },
                Transact {
                    origin_kind: OriginKind::SovereignAccount,
                    fallback_max_weight: None,
                    call: call.encode().into(),
                },
                RefundSurplus,
                DepositAsset {
                    assets: Wild(All),
                    beneficiary: (Parent, Parachain(NON_SYS_PARA_ID)).into(),
                },
            ]
            .into();
            assert_ok!(parachain::PolkadotXcm::send_xcm(Here, destination, message));
        });

        StorageHub::execute_with(|| {
            // We check that the parachain is now a provider in StorageHub.
            let parachain_provider_id =
                storagehub::Providers::get_provider_id(&sh_sibling_account_id(NON_SYS_PARA_ID))
                    .unwrap();
            assert!(storagehub::Providers::is_provider(parachain_provider_id),);

            // Advance enough blocks to allow the parachain to change its provided capacity
            sh_run_to_block(
                storagehub::System::block_number() + MinBlocksBetweenCapacityChanges::get(),
            );
        });

        // The parachain changes its capacity in StorageHub.
        MockParachain::execute_with(|| {
            let destination: Location = (Parent, Parachain(SH_PARA_ID)).into();
            let call = storagehub::RuntimeCall::Providers(pallet_storage_providers::Call::<
                storagehub::Runtime,
            >::change_capacity {
                new_capacity: 20,
            });
            // Remember, this message will be executed from the context of StorageHub
            let message: Xcm<()> = vec![
                WithdrawAsset((Parent, 100 * CENTS).into()),
                BuyExecution {
                    fees: (Parent, 100 * CENTS).into(),
                    weight_limit: Unlimited,
                },
                Transact {
                    origin_kind: OriginKind::SovereignAccount,
                    fallback_max_weight: None,
                    call: call.encode().into(),
                },
                RefundSurplus,
                DepositAsset {
                    assets: Wild(All),
                    beneficiary: (Parent, Parachain(NON_SYS_PARA_ID)).into(),
                },
            ]
            .into();
            assert_ok!(parachain::PolkadotXcm::send_xcm(Here, destination, message));
        });

        StorageHub::execute_with(|| {
            // We check that the parachain has correctly changed its capacity in StorageHub.
            assert_eq!(
                storagehub::Providers::get_total_capacity_of_sp(&sh_sibling_account_id(
                    NON_SYS_PARA_ID
                ))
                .unwrap(),
                20
            );

            // And that its deposit has changed as well
            assert!(
                storagehub::Balances::balance(&sh_sibling_account_id(NON_SYS_PARA_ID))
                    < parachain_balance_after_deposit
            );
        });
    }
}

mod users {

    use crate::{sh_sibling_account_account_id, CHARLIE, SH_PARA_ID};
    use pallet_file_system::types::{
        FileKeyWithProof, MaxFilePathSize, MaxNumberOfPeerIds, MaxPeerIdSize,
        PendingFileDeletionRequest, ReplicationTarget,
    };
    use pallet_storage_providers::types::ValueProposition;
    use sp_trie::CompactProof;
    use storagehub::configs::{BucketNameLimit, SpMinDeposit};

    use super::*;

    #[test]
    fn parachains_should_be_able_to_act_as_users_of_storagehub() {
        // Scenario:
        // A parachain should be able to act as a user of StorageHub, requesting file storage and deletion.

        // We reset storage and messages.
        MockNet::reset();

        // We first register an account as a MSP and another as a BSP in StorageHub.
        let alice_msp_id = H256::random();
        let bob_bsp_id = H256::random();
        let capacity = 100;
        let parachain_account_in_sh = sh_sibling_account_id(NON_SYS_PARA_ID);
        let bucket_name: BoundedVec<u8, BucketNameLimit> =
            "InitialBucket".as_bytes().to_vec().try_into().unwrap();
        let mut bucket_id = H256::default();
        let value_prop = ValueProposition::<storagehub::Runtime>::new(1, bounded_vec![], 10);
        let value_prop_id = value_prop.derive_id();
        StorageHub::execute_with(|| {
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

            // Register Alice as MSP
            assert_ok!(storagehub::Providers::force_msp_sign_up(
                storagehub::RuntimeOrigin::root(),
                ALICE,
                alice_msp_id,
                capacity,
                multiaddresses.clone(),
                1,
                bounded_vec![],
                10,
                ALICE
            ));

            // Register Bob as BSP
            assert_ok!(storagehub::Providers::force_bsp_sign_up(
                storagehub::RuntimeOrigin::root(),
                BOB,
                bob_bsp_id,
                capacity,
                multiaddresses,
                BOB,
                None
            ));
        });

        // We now try to create a bucket with Alice as the parachain
        MockParachain::execute_with(|| {
            let destination: Location = (Parent, Parachain(SH_PARA_ID)).into();
            let bucket_creation_call =
                storagehub::RuntimeCall::FileSystem(pallet_file_system::Call::<
                    storagehub::Runtime,
                >::create_bucket {
                    msp_id: alice_msp_id,
                    name: bucket_name.clone(),
                    private: false,
                    value_prop_id,
                });
            // Remember, this message will be executed from the context of StorageHub
            let message: Xcm<()> = vec![
                WithdrawAsset((Parent, 100 * CENTS).into()),
                BuyExecution {
                    fees: (Parent, 100 * CENTS).into(),
                    weight_limit: Unlimited,
                },
                Transact {
                    origin_kind: OriginKind::SovereignAccount,
                    fallback_max_weight: None,
                    call: bucket_creation_call.encode().into(),
                },
                RefundSurplus,
                DepositAsset {
                    assets: Wild(All),
                    beneficiary: (Parent, Parachain(NON_SYS_PARA_ID)).into(),
                },
            ]
            .into();
            assert_ok!(parachain::PolkadotXcm::send_xcm(Here, destination, message));
        });

        // We check that the bucket was created
        StorageHub::execute_with(|| {
            bucket_id = storagehub::Providers::derive_bucket_id(
                &parachain_account_in_sh,
                bucket_name.clone(),
            );
            assert!(storagehub::Providers::is_bucket_stored_by_msp(
                &alice_msp_id,
                &bucket_id
            ));
            assert!(
                storagehub::Providers::is_bucket_owner(&parachain_account_in_sh, &bucket_id)
                    .unwrap()
            );
        });

        // We now request storing a file as the parachain user
        let file_location: BoundedVec<u8, MaxFilePathSize<storagehub::Runtime>> =
            "file.txt".as_bytes().to_vec().try_into().unwrap();
        let file_fingerprint = H256::random();
        let size = 5;
        let file_key = storagehub::FileSystem::compute_file_key(
            parachain_account_in_sh.clone(),
            bucket_id.clone(),
            file_location.clone(),
            size,
            file_fingerprint.clone(),
        )
        .unwrap();
        MockParachain::execute_with(|| {
            let destination: Location = (Parent, Parachain(SH_PARA_ID)).into();
            let parachain_peer_id: BoundedVec<
                BoundedVec<u8, MaxPeerIdSize<storagehub::Runtime>>,
                MaxNumberOfPeerIds<storagehub::Runtime>,
            > = BoundedVec::new();
            let file_creation_call =
                storagehub::RuntimeCall::FileSystem(pallet_file_system::Call::<
                    storagehub::Runtime,
                >::issue_storage_request {
                    bucket_id: bucket_id.clone(),
                    location: file_location.clone(),
                    fingerprint: file_fingerprint.clone(),
                    size,
                    msp_id: alice_msp_id.clone(),
                    peer_ids: parachain_peer_id,
                    replication_target: ReplicationTarget::Standard,
                });
            // Remember, this message will be executed from the context of StorageHub
            let message: Xcm<()> = vec![
                WithdrawAsset((Parent, 100 * CENTS).into()),
                BuyExecution {
                    fees: (Parent, 100 * CENTS).into(),
                    weight_limit: Unlimited,
                },
                Transact {
                    origin_kind: OriginKind::SovereignAccount,
                    fallback_max_weight: None,
                    call: file_creation_call.encode().into(),
                },
                RefundSurplus,
                DepositAsset {
                    assets: Wild(All),
                    beneficiary: (Parent, Parachain(NON_SYS_PARA_ID)).into(),
                },
            ]
            .into();
            assert_ok!(parachain::PolkadotXcm::send_xcm(Here, destination, message));
        });

        // We check that the storage request exists in StorageHub and volunteer Bob
        StorageHub::execute_with(|| {
            // Check that the storage request exists
            assert!(
                pallet_file_system::StorageRequests::<storagehub::Runtime>::get(file_key.clone())
                    .is_some()
            );

            // Calculate in how many ticks Bob can volunteer for the file
            let current_tick = storagehub::ProofsDealer::get_current_tick();
            let tick_when_bob_can_volunteer =
                storagehub::FileSystem::query_earliest_file_volunteer_tick(bob_bsp_id, file_key)
                    .unwrap();
            if tick_when_bob_can_volunteer > current_tick {
                let ticks_to_advance = tick_when_bob_can_volunteer - current_tick + 1;
                let current_block = storagehub::System::block_number();

                // Advance enough blocks to make sure Bob can volunteer according to the threshold
                sh_run_to_block(current_block + ticks_to_advance);
            }

            // Volunteer Bob
            assert_ok!(storagehub::FileSystem::bsp_volunteer(
                storagehub::RuntimeOrigin::signed(BOB),
                file_key.clone()
            ));

            // And confirm storing the file
            let mut vec_of_key_proofs: BoundedVec<
                FileKeyWithProof<storagehub::Runtime>,
                MaxBatchConfirmStorageRequests,
            > = BoundedVec::new();
            let simulated_proof: CompactProof = CompactProof {
                encoded_nodes: vec![[1u8; 32].to_vec()],
            };
            vec_of_key_proofs.force_push(FileKeyWithProof {
                file_key: file_key.clone(),
                proof: simulated_proof.clone(),
            });
            assert_ok!(storagehub::FileSystem::bsp_confirm_storing(
                storagehub::RuntimeOrigin::signed(BOB),
                simulated_proof.clone(),
                vec_of_key_proofs.clone()
            ));
        });

        // Now request deletion of the file
        MockParachain::execute_with(|| {
            let destination: Location = (Parent, Parachain(SH_PARA_ID)).into();
            let file_deletion_call =
                storagehub::RuntimeCall::FileSystem(pallet_file_system::Call::<
                    storagehub::Runtime,
                >::delete_file {
                    bucket_id: bucket_id.clone(),
                    file_key: file_key.clone(),
                    location: file_location.clone(),
                    size: size,
                    fingerprint: file_fingerprint.clone(),
                    maybe_inclusion_forest_proof: None,
                });
            // Remember, this message will be executed from the context of StorageHub
            let message: Xcm<()> = vec![
                WithdrawAsset((Parent, 100 * CENTS).into()),
                BuyExecution {
                    fees: (Parent, 100 * CENTS).into(),
                    weight_limit: Unlimited,
                },
                Transact {
                    origin_kind: OriginKind::SovereignAccount,
                    fallback_max_weight: None,
                    call: file_deletion_call.encode().into(),
                },
                RefundSurplus,
                DepositAsset {
                    assets: Wild(All),
                    beneficiary: (Parent, Parachain(NON_SYS_PARA_ID)).into(),
                },
            ]
            .into();
            assert_ok!(parachain::PolkadotXcm::send_xcm(Here, destination, message));
        });

        // We check now that there's a pending file deletion request for this file
        StorageHub::execute_with(|| {
            assert_eq!(
                pallet_file_system::PendingFileDeletionRequests::<storagehub::Runtime>::get(
                    parachain_account_in_sh.clone()
                )
                .len(),
                1
            );
            let file_deletion_request_deposit = <storagehub::Runtime as pallet_file_system::Config>::FileDeletionRequestDeposit::get();
            let mut file_deletion_requests_vec: BoundedVec<
                PendingFileDeletionRequest<storagehub::Runtime>,
                <storagehub::Runtime as pallet_file_system::Config>::MaxUserPendingDeletionRequests,
            > = BoundedVec::new();
            let pending_file_deletion_request = PendingFileDeletionRequest {
                user: parachain_account_in_sh.clone(),
                file_key: file_key.clone(),
                file_size: size,
                bucket_id: bucket_id.clone(),
                deposit_paid_for_creation: file_deletion_request_deposit,
                queue_priority_challenge: true,
            };
            file_deletion_requests_vec.force_push(pending_file_deletion_request);
            assert_eq!(
                pallet_file_system::PendingFileDeletionRequests::<storagehub::Runtime>::get(
                    parachain_account_in_sh.clone()
                ),
                file_deletion_requests_vec
            )
        });
    }

    #[test]
    fn users_of_a_parachain_should_be_able_to_act_as_users_of_storagehub() {
        // Scenario:
        // Users of a parachain that allows XCM messaging between it and StorageHub
        // should be able to act as users of StorageHub, requesting file storage and deletion.

        // We reset storage and messages.
        MockNet::reset();

        // We first register an account as a MSP and another as a BSP in StorageHub.
        let alice_msp_id = H256::random();
        let bob_bsp_id = H256::random();
        let capacity = 100;
        let charlie_parachain_account_in_sh =
            sh_sibling_account_account_id(NON_SYS_PARA_ID, CHARLIE);
        let bucket_name: BoundedVec<u8, BucketNameLimit> =
            "InitialBucket".as_bytes().to_vec().try_into().unwrap();
        let mut bucket_id = H256::default();
        let value_prop = ValueProposition::<storagehub::Runtime>::new(1, bounded_vec![], 10);
        let value_prop_id = value_prop.derive_id();
        StorageHub::execute_with(|| {
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

            // Register Alice as MSP
            assert_ok!(storagehub::Providers::force_msp_sign_up(
                storagehub::RuntimeOrigin::root(),
                ALICE,
                alice_msp_id,
                capacity,
                multiaddresses.clone(),
                1,
                bounded_vec![],
                10,
                ALICE
            ));

            // Register Bob as BSP
            assert_ok!(storagehub::Providers::force_bsp_sign_up(
                storagehub::RuntimeOrigin::root(),
                BOB,
                bob_bsp_id,
                capacity,
                multiaddresses,
                BOB,
                None
            ));
        });

        // Charlie has its funds in the parachain, he has to get them to StorageHub.
        // To do that, we initiate a reserve withdraw of the funds from the parachain to the Relay chain and
        // then teleport them to StorageHub
        MockParachain::execute_with(|| {
            // This message will be executed from the context of the parachain (locally)
            // This sends a message to the Relay chain of a reserve asset withdraw, which withdraws
            // funds from the parachain's sovereign account into the holding register, and then sends
            // a teleport of those funds from the Relay chain to StorageHub, which deposits those funds
            // into Charlie's account there
            let message: VersionedXcm<parachain::RuntimeCall> = VersionedXcm::V5(
                vec![
                    WithdrawAsset(
                        (
                            Location {
                                parents: 1,
                                interior: Here.into(),
                            },
                            9 * SpMinDeposit::get(),
                        )
                            .into(),
                    ),
                    BuyExecution {
                        fees: (
                            Location {
                                parents: 1,
                                interior: Here.into(),
                            },
                            100 * CENTS,
                        )
                            .into(),
                        weight_limit: Unlimited,
                    },
                    InitiateReserveWithdraw {
                        assets: Wild(AllOf {
                            id: Location {
                                parents: 1,
                                interior: Here.into(),
                            }
                            .into(),
                            fun: WildFungible,
                        }),
                        reserve: Location {
                            parents: 1,
                            interior: Here.into(),
                        }
                        .into(),
                        xcm: vec![InitiateTeleport {
                            assets: Wild(AllOf {
                                id: Here.into(),
                                fun: WildFungible,
                            }),
                            dest: (Parachain(SH_PARA_ID)).into(),
                            xcm: vec![DepositAsset {
                                assets: Wild(AllOf {
                                    id: Parent.into(),
                                    fun: WildFungible,
                                }),
                                beneficiary: Location {
                                    parents: 1,
                                    interior: (
                                        Parachain(NON_SYS_PARA_ID),
                                        AccountId32 {
                                            network: None,
                                            id: CHARLIE.into(),
                                        },
                                    )
                                        .into(),
                                }
                                .into(),
                            }]
                            .into(),
                        }]
                        .into(),
                    },
                    RefundSurplus,
                    DepositAsset {
                        assets: Wild(All),
                        beneficiary: AccountId32 {
                            network: None,
                            id: CHARLIE.into(),
                        }
                        .into(),
                    },
                ]
                .into(),
            );
            assert_ok!(parachain::PolkadotXcm::execute(
                parachain::RuntimeOrigin::signed(CHARLIE.into()),
                message.into(),
                Weight::MAX
            ));
        });

        // Now, Charlie should have most of his funds in StorageHub, under his sovereign account derived
        // from the parachain which he is user of and his account ID in that parachain.
        // We now try to create a bucket with Alice's MSP as Charlie's account in the parachain
        MockParachain::execute_with(|| {
            let destination: Location = (Parent, Parachain(SH_PARA_ID)).into();
            let bucket_creation_call =
                storagehub::RuntimeCall::FileSystem(pallet_file_system::Call::<
                    storagehub::Runtime,
                >::create_bucket {
                    msp_id: alice_msp_id,
                    name: bucket_name.clone(),
                    private: false,
                    value_prop_id,
                });
            // Remember, this message will be executed from the context of StorageHub
            let message: Xcm<()> = vec![
                DescendOrigin(
                    AccountId32 {
                        network: None,
                        id: CHARLIE.into(),
                    }
                    .into(),
                ),
                WithdrawAsset((Parent, 100 * CENTS).into()),
                BuyExecution {
                    fees: (Parent, 100 * CENTS).into(),
                    weight_limit: Unlimited,
                },
                Transact {
                    origin_kind: OriginKind::SovereignAccount,
                    fallback_max_weight: None,
                    call: bucket_creation_call.encode().into(),
                },
                RefundSurplus,
                DepositAsset {
                    assets: Wild(All),
                    beneficiary: (AccountId32 {
                        network: None,
                        id: CHARLIE.into(),
                    },)
                        .into(),
                },
            ]
            .into();
            assert_ok!(parachain::PolkadotXcm::send_xcm(Here, destination, message));
        });

        // We check that the bucket was created
        StorageHub::execute_with(|| {
            bucket_id = storagehub::Providers::derive_bucket_id(
                &charlie_parachain_account_in_sh,
                bucket_name.clone(),
            );
            assert!(storagehub::Providers::is_bucket_stored_by_msp(
                &alice_msp_id,
                &bucket_id
            ));
            assert!(storagehub::Providers::is_bucket_owner(
                &charlie_parachain_account_in_sh,
                &bucket_id
            )
            .unwrap());
        });

        // We now request storing a file as Charlie from the parachain
        let file_location: BoundedVec<u8, MaxFilePathSize<storagehub::Runtime>> =
            "file.txt".as_bytes().to_vec().try_into().unwrap();
        let file_fingerprint = H256::random();
        let size = 5;
        let file_key = storagehub::FileSystem::compute_file_key(
            charlie_parachain_account_in_sh.clone(),
            bucket_id.clone(),
            file_location.clone(),
            size,
            file_fingerprint.clone(),
        )
        .unwrap();
        MockParachain::execute_with(|| {
            let destination: Location = (Parent, Parachain(SH_PARA_ID)).into();
            let parachain_peer_id: BoundedVec<
                BoundedVec<u8, MaxPeerIdSize<storagehub::Runtime>>,
                MaxNumberOfPeerIds<storagehub::Runtime>,
            > = BoundedVec::new();
            let file_creation_call =
                storagehub::RuntimeCall::FileSystem(pallet_file_system::Call::<
                    storagehub::Runtime,
                >::issue_storage_request {
                    bucket_id: bucket_id.clone(),
                    location: file_location.clone(),
                    fingerprint: file_fingerprint.clone(),
                    size,
                    msp_id: alice_msp_id.clone(),
                    peer_ids: parachain_peer_id,
                    replication_target: ReplicationTarget::Standard,
                });
            // Remember, this message will be executed from the context of StorageHub
            let message: Xcm<()> = vec![
                DescendOrigin(
                    AccountId32 {
                        network: None,
                        id: CHARLIE.into(),
                    }
                    .into(),
                ),
                WithdrawAsset((Parent, 100 * CENTS).into()),
                BuyExecution {
                    fees: (Parent, 100 * CENTS).into(),
                    weight_limit: Unlimited,
                },
                Transact {
                    origin_kind: OriginKind::SovereignAccount,
                    fallback_max_weight: None,
                    call: file_creation_call.encode().into(),
                },
                RefundSurplus,
                DepositAsset {
                    assets: Wild(All),
                    beneficiary: (AccountId32 {
                        network: None,
                        id: CHARLIE.into(),
                    },)
                        .into(),
                },
            ]
            .into();
            assert_ok!(parachain::PolkadotXcm::send_xcm(Here, destination, message));
        });

        // We check that the storage request exists in StorageHub and volunteer Bob
        StorageHub::execute_with(|| {
            // Check that the storage request exists
            assert!(
                pallet_file_system::StorageRequests::<storagehub::Runtime>::get(file_key.clone())
                    .is_some()
            );

            // Calculate in how many ticks Bob can volunteer for the file
            let current_tick = storagehub::ProofsDealer::get_current_tick();
            let tick_when_bob_can_volunteer =
                storagehub::FileSystem::query_earliest_file_volunteer_tick(bob_bsp_id, file_key)
                    .unwrap();
            if tick_when_bob_can_volunteer > current_tick {
                let ticks_to_advance = tick_when_bob_can_volunteer - current_tick + 1;
                let current_block = storagehub::System::block_number();

                // Advance enough blocks to make sure Bob can volunteer according to the threshold
                sh_run_to_block(current_block + ticks_to_advance);
            }

            // Volunteer Bob
            assert_ok!(storagehub::FileSystem::bsp_volunteer(
                storagehub::RuntimeOrigin::signed(BOB),
                file_key.clone()
            ));

            // And confirm storing the file
            let mut vec_of_key_proofs: BoundedVec<
                FileKeyWithProof<storagehub::Runtime>,
                MaxBatchConfirmStorageRequests,
            > = BoundedVec::new();
            let simulated_proof: CompactProof = CompactProof {
                encoded_nodes: vec![[1u8; 32].to_vec()],
            };
            vec_of_key_proofs.force_push(FileKeyWithProof {
                file_key: file_key.clone(),
                proof: simulated_proof.clone(),
            });
            assert_ok!(storagehub::FileSystem::bsp_confirm_storing(
                storagehub::RuntimeOrigin::signed(BOB),
                simulated_proof.clone(),
                vec_of_key_proofs.clone()
            ));
        });

        // Now request deletion of the file
        MockParachain::execute_with(|| {
            let destination: Location = (Parent, Parachain(1)).into();
            let file_deletion_call =
                storagehub::RuntimeCall::FileSystem(pallet_file_system::Call::<
                    storagehub::Runtime,
                >::delete_file {
                    bucket_id: bucket_id.clone(),
                    file_key: file_key.clone(),
                    location: file_location.clone(),
                    size,
                    fingerprint: file_fingerprint.clone(),
                    maybe_inclusion_forest_proof: None,
                });
            // Remember, this message will be executed from the context of StorageHub
            let message: Xcm<()> = vec![
                DescendOrigin(
                    AccountId32 {
                        network: None,
                        id: CHARLIE.into(),
                    }
                    .into(),
                ),
                WithdrawAsset((Parent, 100 * CENTS).into()),
                BuyExecution {
                    fees: (Parent, 100 * CENTS).into(),
                    weight_limit: Unlimited,
                },
                Transact {
                    origin_kind: OriginKind::SovereignAccount,
                    fallback_max_weight: None,
                    call: file_deletion_call.encode().into(),
                },
                RefundSurplus,
                DepositAsset {
                    assets: Wild(All),
                    beneficiary: (AccountId32 {
                        network: None,
                        id: CHARLIE.into(),
                    },)
                        .into(),
                },
            ]
            .into();
            assert_ok!(parachain::PolkadotXcm::send_xcm(Here, destination, message));
        });

        // We check now that there's a pending file deletion request for this file
        StorageHub::execute_with(|| {
            assert_eq!(
                pallet_file_system::PendingFileDeletionRequests::<storagehub::Runtime>::get(
                    charlie_parachain_account_in_sh.clone()
                )
                .len(),
                1
            );
            let file_deletion_request_deposit = <storagehub::Runtime as pallet_file_system::Config>::FileDeletionRequestDeposit::get();

            let mut file_deletion_requests_vec: BoundedVec<
                PendingFileDeletionRequest<storagehub::Runtime>,
                <storagehub::Runtime as pallet_file_system::Config>::MaxUserPendingDeletionRequests,
            > = BoundedVec::new();
            let pending_file_deletion_request = PendingFileDeletionRequest {
                user: charlie_parachain_account_in_sh.clone(),
                file_key: file_key.clone(),
                file_size: size,
                bucket_id: bucket_id.clone(),
                deposit_paid_for_creation: file_deletion_request_deposit,
                queue_priority_challenge: true,
            };
            file_deletion_requests_vec.force_push(pending_file_deletion_request);
            assert_eq!(
                pallet_file_system::PendingFileDeletionRequests::<storagehub::Runtime>::get(
                    charlie_parachain_account_in_sh.clone()
                ),
                file_deletion_requests_vec
            )
        });
    }
}
