import { strictEqual } from "node:assert";
import {
  addMspContainer,
  assertEventPresent,
  bspThreeKey,
  bspTwoKey,
  describeMspNet,
  type EnrichedBspApi,
  getContainerIp,
  mspThreeKey,
  ShConsts,
  shUser
} from "../../../util";

await describeMspNet(
  "MSP rejects bucket move requests due to low capacity",
  { initialised: false, indexer: true },
  ({ before, after, createMsp1Api, it, createUserApi, createApi }) => {
    let userApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;
    let msp3Api: EnrichedBspApi;

    const source = ["res/cloud.jpg", "res/smile.jpg", "res/whatsup.jpg"];
    const destination = ["test/cloud.jpg", "test/smile.jpg", "test/whatsup.jpg"];
    const bucketName = "move-bucket";
    let bucketId: string;
    const allBucketFiles: string[] = [];

    before(async () => {
      userApi = await createUserApi();
      const maybeMspApi = await createMsp1Api();
      if (!maybeMspApi) {
        throw new Error("Failed to create MSP API");
      }
      mspApi = maybeMspApi;
    });

    after(async () => {
      msp3Api.disconnect();
    });

    it("postgres DB is ready", async () => {
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.indexerDb.containerName,
        searchString: "database system is ready to accept connections",
        timeout: 5000
      });
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

      const mspNodePeerId = await mspApi.rpc.system.localPeerId();
      strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp1.expectedPeerId);
    });

    it("Add 2 more BSPs (3 total) and set the replication target to 2", async () => {
      // Replicate to 2 BSPs, 5 blocks to maxthreshold
      const newRuntimeParameter = {
        RuntimeConfig: {
          MaxReplicationTarget: [null, 2],
          TickRangeToMaximumThreshold: [null, 5]
        }
      };
      await userApi.block.seal({
        calls: [userApi.tx.sudo.sudo(userApi.tx.parameters.setParameter(newRuntimeParameter))]
      });

      await userApi.docker.onboardBsp({
        bspSigner: bspTwoKey,
        name: "sh-bsp-two",
        bspId: ShConsts.BSP_TWO_ID,
        additionalArgs: ["--keystore-path=/keystore/bsp-two"],
        waitForIdle: true
      });

      await userApi.docker.onboardBsp({
        bspSigner: bspThreeKey,
        name: "sh-bsp-three",
        bspId: ShConsts.BSP_THREE_ID,
        additionalArgs: ["--keystore-path=/keystore/bsp-three"],
        waitForIdle: true
      });
    });

    it("Add new MSP with low capacity", async () => {
      const { containerName, p2pPort, peerId, rpcPort } = await addMspContainer({
        name: "storage-hub-sh-msp-sleepy",
        additionalArgs: [
          "--keystore-path=/keystore/msp-three",
          `--max-storage-capacity=${1024 * 1024}`,
          `--jump-capacity=${1024 * 1024}`,
          "--msp-charging-period=12"
        ]
      });

      await userApi.docker.waitForLog({
        containerName: "storage-hub-sh-msp-sleepy",
        searchString: "💤 Idle",
        timeout: 15000
      });

      msp3Api = await createApi(`ws://127.0.0.1:${rpcPort}`);

      // Give it some balance.
      const amount = 10000n * 10n ** 12n;
      await userApi.block.seal({
        calls: [
          userApi.tx.sudo.sudo(userApi.tx.balances.forceSetBalance(mspThreeKey.address, amount))
        ]
      });

      const mspIp = await getContainerIp(containerName);
      const multiAddressMsp = `/ip4/${mspIp}/tcp/${p2pPort}/p2p/${peerId}`;
      await userApi.block.seal({
        calls: [
          userApi.tx.sudo.sudo(
            userApi.tx.providers.forceMspSignUp(
              mspThreeKey.address,
              mspThreeKey.publicKey,
              userApi.shConsts.CAPACITY_512,
              [multiAddressMsp],
              100 * 1024 * 1024,
              "Terms of Service...",
              9999999,
              mspThreeKey.address
            )
          )
        ]
      });
    });

    it("User submits 3 storage requests in the same bucket for first MSP", async () => {
      // Get value propositions from the MSP to use, and use the first one (can be any).
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
        userApi.shConsts.DUMMY_MSP_ID
      );
      const valuePropId = valueProps[0].id;

      // Use batchStorageRequests helper to create bucket and submit storage requests
      const batchResult = await userApi.file.batchStorageRequests({
        files: source.map((src, i) => ({
          source: src,
          destination: destination[i],
          bucketIdOrName: bucketName,
          replicationTarget: 2
        })),
        mspId: userApi.shConsts.DUMMY_MSP_ID,
        valuePropId,
        owner: shUser,
        bspApi: undefined, // No BSP needed for this test
        mspApi
      });

      // Extract bucket ID and file keys from the batch result
      bucketId = batchResult.bucketIds[0];
      allBucketFiles.push(...batchResult.fileKeys);
    });

    it("New MSP rejects move request due to low capacity", async () => {
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
        mspThreeKey.publicKey
      );
      const valuePropId = valueProps[0].id;

      // User requests to move bucket to second MSP
      const requestMoveBucketResult = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.requestMoveBucket(bucketId, mspThreeKey.publicKey, valuePropId)
        ],
        signer: shUser
      });

      assertEventPresent(
        userApi,
        "fileSystem",
        "MoveBucketRequested",
        requestMoveBucketResult.events
      );

      // Finalising the block in the BSP node as well, to trigger the reorg in the BSP node too.
      const finalisedBlockHash = await userApi.rpc.chain.getFinalizedHead();

      // Wait for BSP node to have imported the finalised block built by the user node.
      await msp3Api.wait.blockImported(finalisedBlockHash.toString());
      await msp3Api.block.finaliseBlock(finalisedBlockHash.toString());

      // Wait for the rejection response from Sleepy
      await userApi.wait.waitForTxInPool({
        module: "fileSystem",
        method: "mspRespondMoveBucketRequest"
      });

      const { events } = await userApi.block.seal();

      // Verify that the move request was rejected
      assertEventPresent(userApi, "fileSystem", "MoveBucketRejected", events);
    });
  }
);
