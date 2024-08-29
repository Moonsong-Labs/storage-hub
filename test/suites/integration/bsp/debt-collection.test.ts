import "@storagehub/api-augment";
import assert, { strictEqual } from "node:assert";
import { after, before, describe, it } from "node:test";
import {
    NODE_INFOS,
    createApiObject,
    type BspNetApi,
    type BspNetConfig,
    closeSimpleBspNet,
    DUMMY_BSP_ID,
    sleep,
    assertEventMany,
    runSimpleBspNet,
    runMultipleInitialisedBspsNet,
    assertEventPresent
} from "../../../util";

const bspNetConfigCases: BspNetConfig[] = [
    { noisy: false, rocksdb: false },
    { noisy: false, rocksdb: true }
];

for (const bspNetConfig of bspNetConfigCases) {
    describe.only(`BSPNet: Users's debt collection (${bspNetConfig.noisy ? "Noisy" : "Noiseless"} and ${bspNetConfig.rocksdb ? "RocksDB" : "MemoryDB"})`, () => {
        let userApi: BspNetApi;
        let bspApi: BspNetApi;
        let bspTwoApi: BspNetApi;
        let bspThreeApi: BspNetApi;
        let fileData: {
            fileKey: string;
            bucketId: string;
            location: string;
            owner: string;
            fingerprint: string;
            fileSize: number;
        };

        before(async () => {
            const bspNetInfo = await runMultipleInitialisedBspsNet(bspNetConfig);
            userApi = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.user.port}`);
            bspApi = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.bsp.port}`);
            bspTwoApi = await createApiObject(`ws://127.0.0.1:${bspNetInfo?.bspTwoRpcPort}`);
            bspThreeApi = await createApiObject(`ws://127.0.0.1:${bspNetInfo?.bspThreeRpcPort}`);

            assert(bspNetInfo, "BSPNet failed to initialise");
            fileData = bspNetInfo?.fileData;
        });

        after(async () => {
            await userApi.disconnect();
            await bspApi.disconnect();
            await bspTwoApi.disconnect();
            await bspThreeApi.disconnect();
            await closeSimpleBspNet();
        });

        it.only("BSP Charging Task reacts to ProofAccepted and charges users", async () => {
            // create payment stream.
            let alice = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY";
            let result = await userApi.sealBlock(userApi.tx.sudo.sudo(userApi.tx.paymentStreams.createDynamicRatePaymentStream(DUMMY_BSP_ID, alice, 100, 1, 1)));
            await sleep(500);

            // Seal one more block with the pending extrinsics.
            const dynamicStreamEvents = assertEventMany(
                bspApi,
                "paymentStreams",
                "DynamicRatePaymentStreamCreated",
                result.events
            );
            strictEqual(dynamicStreamEvents.length, 1, "There should be one dynamic stream event");

            // Seal one more block with the pending extrinsics.
            result = await userApi.sealBlock();

            // Calculate the next challenge tick for the BSPs. It should be the same for all BSPs,
            // since they all have the same file they were initialised with, and responded to it at
            // the same time.
            // We first get the last tick for which the BSP submitted a proof.
            const lastTickResult =
                await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(DUMMY_BSP_ID);
            assert(lastTickResult.isOk);
            const lastTickBspSubmittedProof = lastTickResult.asOk.toNumber();
            // Then we get the challenge period for the BSP.
            const challengePeriodResult =
                await userApi.call.proofsDealerApi.getChallengePeriod(DUMMY_BSP_ID);
            assert(challengePeriodResult.isOk);
            const challengePeriod = challengePeriodResult.asOk.toNumber();
            // Then we calculate the next challenge tick.
            const nextChallengeTick = lastTickBspSubmittedProof + challengePeriod;

            // Calculate how many blocks to advance until next challenge tick.
            const currentBlock = await userApi.rpc.chain.getBlock();
            const currentBlockNumber = currentBlock.block.header.number.toNumber();
            const blocksToAdvance = nextChallengeTick - currentBlockNumber;

            // Advance blocksToAdvance blocks.
            for (let i = 0; i < blocksToAdvance; i++) {
                await userApi.sealBlock();
            }

            // Wait for tasks to execute and for the BSPs to submit proofs.
            await sleep(500);
            // Check that there are 3 pending extrinsics from BSPs (proof submission).
            const submitProofPending = await userApi.rpc.author.pendingExtrinsics();
            strictEqual(
                submitProofPending.length,
                3,
                "There should be three pending extrinsics from BSPs (proof submission)"
            );

            // Seal one more block with the pending extrinsics.
            let blockResult = await userApi.sealBlock();

            // Assert for the the event of the proof successfully submitted and verified.
            const proofAcceptedEvents = assertEventMany(
                userApi,
                "proofsDealer",
                "ProofAccepted",
                blockResult.events
            );
            strictEqual(proofAcceptedEvents.length, 3, "There should be three proofs accepted events");

            await sleep(500);
            // Assert for the the event of the proof successfully submitted and verified.
            assertEventPresent(
                userApi,
                "paymentStreams",
                "PaymentStreamCharged",
                blockResult.events
            );
            // TODO: check users' balances
        }
        );
    });
}