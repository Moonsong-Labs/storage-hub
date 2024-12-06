import assert, { notEqual, strictEqual } from "node:assert";
import { bspKey, describeBspNet, shUser, sleep, type EnrichedBspApi } from "../../../util";

describeBspNet("Single BSP Volunteering", { only: true }, ({ before, createBspApi, it, createUserApi }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;

    before(async () => {
        userApi = await createUserApi();
        bspApi = await createBspApi();
    });

    it("Network launches and can be queried", async () => {
        const userNodePeerId = await userApi.rpc.system.localPeerId();
        strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

        const bspNodePeerId = await bspApi.rpc.system.localPeerId();
        strictEqual(bspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.bsp.expectedPeerId);
    });

    it("Volunteer for multiple files and delete them", async () => {
        const source = ["res/whatsup.jpg", "res/adolphus.jpg", "res/cloud.jpg"];
        const destination = ["test/whatsup.jpg", "test/adolphus.jpg", "test/cloud.jpg"];
        const bucketName = "something-3";

        const newBucketEventEvent = await userApi.createBucket(bucketName);
        const newBucketEventDataBlob =
            userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

        assert(newBucketEventDataBlob, "Event doesn't match Type");

        let files = [];
        const txs = [];
        for (let i = 0; i < source.length; i++) {
            const { fingerprint, file_size, location } =
                await userApi.rpc.storagehubclient.loadFileInStorage(
                    source[i],
                    destination[i],
                    userApi.shConsts.NODE_INFOS.user.AddressId,
                    newBucketEventDataBlob.bucketId
                );

            files.push({ fingerprint, file_size, location });
            txs.push(
                userApi.tx.fileSystem.issueStorageRequest(
                    newBucketEventDataBlob.bucketId,
                    location,
                    fingerprint,
                    file_size,
                    userApi.shConsts.DUMMY_MSP_ID,
                    [userApi.shConsts.NODE_INFOS.user.expectedPeerId]
                )
            );
        }

        await userApi.sealBlock(txs, shUser);

        // Get the new storage request events, making sure we have 3
        const storageRequestEvents = await userApi.assert.eventMany("fileSystem", "NewStorageRequest");
        strictEqual(storageRequestEvents.length, 3);

        // Get the file keys from the storage request events
        const fileKeys = storageRequestEvents.map((event) => {
            const dataBlob =
                userApi.events.fileSystem.NewStorageRequest.is(event.event) && event.event.data;
            if (!dataBlob) {
                throw new Error("Event doesn't match Type");
            }
            return dataBlob.fileKey;
        });

        // Wait for the BSP to volunteer
        await userApi.wait.bspVolunteer(source.length);
        for (const fileKey of fileKeys) {
            await bspApi.wait.bspFileStorageComplete(fileKey);
        }

        // Waiting for a confirmation of the first file to be stored
        await sleep(500);
        await userApi.wait.bspStored(1);

        // Here we expect the 2 others files to be batched
        await sleep(500);
        await userApi.wait.bspStored(1);

        console.log(fileKeys.length);

        for (let i = 0; i < fileKeys.length; i++) {
            const inclusionForestProof = await bspApi.rpc.storagehubclient.generateForestProof(null, [
                fileKeys[i],
            ]);
            await userApi.sealBlock(
                userApi.tx.fileSystem.bspRequestStopStoring(
                    fileKeys[i],
                    newBucketEventDataBlob.bucketId,
                    files[i].location,
                    userApi.shConsts.NODE_INFOS.user.AddressId,
                    files[i].fingerprint,
                    files[i].file_size,
                    false,
                    inclusionForestProof.toString()
                ),
                bspKey
            );
        }

        await sleep(500);
        const BspRequestedToStopStoringEvents = await userApi.assert.eventMany("fileSystem", "BspRequestedToStopStoring");

        strictEqual(
            BspRequestedToStopStoringEvents.length,
            3,
            "Should request to stop storing 3 files"
        );
    });

});