import assert, { strictEqual } from "node:assert";
import { bspKey, describeBspNet, type EnrichedBspApi, shUser, waitFor } from "../../../util";

await describeBspNet(
  "BSPNet: Stop storing file and other BSPs taking the relay",
  { initialised: "multi", networkConfig: "standard" },
  ({ before, createUserApi, after, it, createApi, createBspApi, getLaunchResponse }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let bspTwoApi: EnrichedBspApi;
    let bspThreeApi: EnrichedBspApi;

    before(async () => {
      const launchResponse = await getLaunchResponse();
      assert(
        launchResponse && "bspTwoRpcPort" in launchResponse && "bspThreeRpcPort" in launchResponse,
        "BSPNet failed to initialise with required ports"
      );
      userApi = await createUserApi();
      bspApi = await createBspApi();
      bspTwoApi = await createApi(`ws://127.0.0.1:${launchResponse.bspTwoRpcPort}`);
      bspThreeApi = await createApi(`ws://127.0.0.1:${launchResponse.bspThreeRpcPort}`);
    });

    after(async () => {
      await bspTwoApi.disconnect();
      await bspThreeApi.disconnect();
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);
      const bspNodePeerId = await bspApi.rpc.system.localPeerId();
      strictEqual(bspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.bsp.expectedPeerId);
    });

    it("bsp one stop storing and bsp three volunteer", async () => {
      const source = "res/whatsup.jpg";
      const destination = "test/whatsup.jpg";
      const bucketName = "tastytest";

      // Pause BSP-Three.
      await userApi.docker.pauseContainer("sh-bsp-three");

      const { fileKey, location, fingerprint, fileSize, bucketId } =
        await userApi.file.createBucketAndSendNewStorageRequest(source, destination, bucketName);

      // Wait for the two BSP to volunteer
      await userApi.wait.bspVolunteer(2);
      await userApi.wait.bspStored({ expectedExts: 2 });

      // Revoke the storage request otherwise the new storage request event is not being triggered
      await userApi.block.seal({
        calls: [userApi.tx.fileSystem.revokeStorageRequest(fileKey)],
        signer: shUser
      });

      await userApi.assert.eventPresent("fileSystem", "StorageRequestRevoked");

      // Unpause BSP Three
      await userApi.docker.resumeContainer({
        containerName: "sh-bsp-three"
      });
      await userApi.wait.bspCatchUpToChainTip(bspThreeApi);

      // TODO: create an RPC to automatically execute everything below
      // TODO: everything below should be removed and replaced with other testing logic

      // Wait for BSP to update its local Forest root before starting to generate the inclusion proofs
      await waitFor({
        lambda: async () => {
          const isFileInForest = await bspApi.rpc.storagehubclient.isFileInForest(null, fileKey);
          return isFileInForest.isTrue;
        }
      });

      // Add the file key to the exclude list
      bspApi.rpc.storagehubclient.addToExcludeList(fileKey, "file");

      // Request to stop storing a file with Dummy BSP
      const inclusionForestProof = await bspApi.rpc.storagehubclient.generateForestProof(null, [
        fileKey
      ]);
      await userApi.wait.waitForAvailabilityToSendTx(bspKey.address.toString());
      await userApi.block.seal({
        calls: [
          bspApi.tx.fileSystem.bspRequestStopStoring(
            fileKey,
            bucketId,
            location,
            userApi.shConsts.NODE_INFOS.user.AddressId,
            fingerprint,
            fileSize,
            false,
            inclusionForestProof.toString()
          )
        ],
        signer: bspKey
      });

      await userApi.assert.eventPresent("fileSystem", "BspRequestedToStopStoring");

      // When requested to stop storing a file we should also receive an event new storage request
      // to replace the bsp leaving
      // TODO: add rpc to add user to blacklisted users to skip any storage request from them
      // TODO: add rpc to add bucket to blacklisted buckets to skip any storage request from them
      await userApi.assert.eventPresent("fileSystem", "NewStorageRequest");

      // Wait for the right moment to confirm stop storing
      const currentBlock = await userApi.rpc.chain.getBlock();
      const currentBlockNumber = currentBlock.block.header.number.toNumber();
      const minWaitForStopStoring = (
        await userApi.query.parameters.parameters({
          RuntimeConfig: {
            MinWaitForStopStoring: null
          }
        })
      )
        .unwrap()
        .asRuntimeConfig.asMinWaitForStopStoring.toNumber();
      const cooldown = currentBlockNumber + minWaitForStopStoring;

      // New storage request does not get fulfilled and therefore gets cleaned up and we enqueue a checkpoint challenge remove mutation
      // Which then the bsp responds to and has the file key get removed from the forest
      // Once we send the bspConfirmStopStoring the extrinsic fails because the runtime forest does not match the local
      await userApi.block.skipTo(cooldown);

      // TODO: commented out since this extrinsic will inevitably fail. The reason being is that the revoke storage request executed above
      // TODO: created a checkpoint challenge remove mutation which the bsp responded to and removed the file key from the forest before executing this extrinsic
      // TODO: we should remove this entirely and implement a task or rpc to handle automatic stop storing
      // await userApi.block.seal({
      //   calls: [
      //     userApi.tx.fileSystem.bspConfirmStopStoring(
      //       fileKey,
      //       inclusionForestProof
      //     )
      //   ],
      //   signer: bspKey
      // });

      // await userApi.assert.eventPresent(
      //   "fileSystem",
      //   "BspConfirmStoppedStoring"
      // );
    });
  }
);
