import assert, { strictEqual } from "node:assert";
import { describeMspNet, shUser, type EnrichedBspApi, sleep } from "../../../util";

describeMspNet(
    "MSP catching up with chain and volunteering for storage request",
    { initialised: true, only: true },
    ({ before, createMsp1Api, it, createUserApi, getLaunchResponse }) => {
        let userApi: EnrichedBspApi;
        let mspApi: EnrichedBspApi;

        before(async () => {
            userApi = await createUserApi();
            const maybeMspApi = await createMsp1Api();

            assert(maybeMspApi, "MSP API not available");
            mspApi = maybeMspApi;
        });

        it("Network launches and can be queried", async () => {
            const userNodePeerId = await userApi.rpc.system.localPeerId();
            strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

            const mspNodePeerId = await mspApi.rpc.system.localPeerId();
            strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp1.expectedPeerId);
        });

        it("MSP accepts subsequent storage request for the same file key", async () => {
            const source = "res/whatsup.jpg";
            const destination = "test/smile.jpg";
            const initialised = await getLaunchResponse();
            const bucketId = initialised?.fileMetadata.bucketId;

            assert(bucketId, "Bucket ID not found");

            const localBucketRoot = await mspApi.rpc.storagehubclient.getForestRoot(bucketId.toString());

            await userApi.docker.pauseBspContainer("docker-sh-msp-1");

            await userApi.block.seal({
                calls: [
                    userApi.tx.fileSystem.issueStorageRequest(
                        bucketId,
                        destination,
                        userApi.shConsts.TEST_ARTEFACTS[source].fingerprint,
                        userApi.shConsts.TEST_ARTEFACTS[source].size,
                        userApi.shConsts.DUMMY_MSP_ID,
                        [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
                        null
                    )
                ],
                signer: shUser
            });

            await userApi.assert.eventPresent("fileSystem", "NewStorageRequest");

            // Advancing 10 blocks to see if MSP catchup
            await userApi.block.skip(50);

            await userApi.docker.restartBspContainer({ containerName: "docker-sh-msp-1" });

            await sleep(50000);

            await userApi.assert.eventPresent("fileSystem", "MspAcceptedStorageRequest");

        });
    }
);
