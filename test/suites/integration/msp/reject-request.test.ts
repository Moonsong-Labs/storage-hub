import { strictEqual } from "assert";
import { describeMspNet, shUser, sleep, type EnrichedBspApi } from "../../../util";

describeMspNet(
    "Single MSP rejecting storage request",
    { initialised: true },
    ({ before, createMspApi, it, createUserApi, getLaunchResponse }) => {
      let userApi: EnrichedBspApi;
      let mspApi: EnrichedBspApi;
  
      before(async () => {
        userApi = await createUserApi();
        const maybeMspApi = await createMspApi();
        if (maybeMspApi) {
          mspApi = maybeMspApi;
        } else {
          throw new Error("MSP API not available");
        }
      });
  
      it("Network launches and can be queried", async () => {
        const userNodePeerId = await userApi.rpc.system.localPeerId();
        strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);
  
        const mspNodePeerId = await mspApi.rpc.system.localPeerId();
        strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp1.expectedPeerId);
      });
  
      it("MSP rejects storage request since it is already being stored", async () => {
        const source = "res/whatsup.jpg";
        const destination = "test/smile.jpg";
        const initialised = await getLaunchResponse();
        const bucketId = initialised?.bucketIds[0];
  
        if (!bucketId) {
          throw new Error("Bucket ID not found");
        }
  
        const local_bucket_root = await mspApi.rpc.storagehubclient.getForestRoot(
          bucketId.toString()
        );
  
        await userApi.sealBlock(
          userApi.tx.fileSystem.issueStorageRequest(
            bucketId,
            destination,
            userApi.shConsts.TEST_ARTEFACTS[source].fingerprint,
            userApi.shConsts.TEST_ARTEFACTS[source].size,
            userApi.shConsts.DUMMY_MSP_ID,
            [userApi.shConsts.NODE_INFOS.user.expectedPeerId]
          ),
          shUser
        );
  
        // Allow time for the MSP to receive and store the file from the user
        await sleep(3000);
  
        const { event } = await userApi.assert.eventPresent("fileSystem", "NewStorageRequest");
  
        const newStorageRequestDataBlob =
          userApi.events.fileSystem.NewStorageRequest.is(event) && event.data;
  
        if (!newStorageRequestDataBlob) {
          throw new Error("Event doesn't match Type");
        }
  
        strictEqual(
          newStorageRequestDataBlob.who.toString(),
          userApi.shConsts.NODE_INFOS.user.AddressId
        );
        strictEqual(newStorageRequestDataBlob.location.toHuman(), destination);
        strictEqual(
          newStorageRequestDataBlob.fingerprint.toString(),
          userApi.shConsts.TEST_ARTEFACTS[source].fingerprint
        );
        strictEqual(
          newStorageRequestDataBlob.size_.toBigInt(),
          userApi.shConsts.TEST_ARTEFACTS[source].size
        );
  
        // Seal block containing the MSP's transaction response to the storage request
        const responses = await userApi.wait.mspResponse();
  
        if (responses.length !== 1) {
          throw new Error(
            "Expected 1 response since there is only a single bucket and should have been accepted"
          );
        }
  
        const response = responses[0].asRejected;
  
        // Allow time for the MSP to update the local forest root
        await sleep(3000);
  
        // Check that the MSP has not updated the local forest root of the bucket
        strictEqual(
          local_bucket_root.toString(),
          (await mspApi.rpc.storagehubclient.getForestRoot(response.bucketId.toString())).toString()
        );
  
        strictEqual(response.bucketId.toString(), bucketId.toString());
  
        strictEqual(response.fileKeys[0][0].toString(), newStorageRequestDataBlob.fileKey.toString());
        strictEqual(response.fileKeys[0][1].toString(), "FileKeyAlreadyStored");
      });
    }
  );
  