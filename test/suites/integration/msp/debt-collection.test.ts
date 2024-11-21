import { strictEqual } from "node:assert";
import { describeMspNet, shUser, sleep, type EnrichedBspApi } from "../../../util";
import invariant from "tiny-invariant";

describeMspNet(
    "Single MSP collecting debt",
    ({ before, createMspApi, it, createUserApi }) => {
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

        it("MSP receives files from user after issued storage requests", async () => {
            const source = ["res/whatsup.jpg", "res/adolphus.jpg", "res/smile.jpg"];
            const destination = ["test/whatsup.jpg", "test/adolphus.jpg", "test/smile.jpg"];
            const bucketName = "nothingmuch-3";

            const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
                userApi.shConsts.DUMMY_MSP_ID
            );

            const valuePropId = valueProps[0].id;

            const newBucketEventEvent = await userApi.createBucket(bucketName, valuePropId);
            const newBucketEventDataBlob =
                userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

            if (!newBucketEventDataBlob) {
                throw new Error("NewBucket event data does not match expected type");
            }

            const txs = [];
            for (let i = 0; i < source.length; i++) {
                const { fingerprint, file_size, location } =
                    await userApi.rpc.storagehubclient.loadFileInStorage(
                        source[i],
                        destination[i],
                        userApi.shConsts.NODE_INFOS.user.AddressId,
                        newBucketEventDataBlob.bucketId
                    );

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

            // Allow time for the MSP to receive and store the file from the user
            await sleep(3000);

            const events = await userApi.assert.eventMany("fileSystem", "NewStorageRequest");

            const matchedEvents = events.filter((e) =>
                userApi.events.fileSystem.NewStorageRequest.is(e.event)
            );

            if (matchedEvents.length !== source.length) {
                throw new Error(`Expected ${source.length} NewStorageRequest events`);
            }

            const issuedFileKeys = [];
            for (const e of matchedEvents) {
                const newStorageRequestDataBlob =
                    userApi.events.fileSystem.NewStorageRequest.is(e.event) && e.event.data;

                if (!newStorageRequestDataBlob) {
                    throw new Error("Event doesn't match NewStorageRequest type");
                }

                const result = await mspApi.rpc.storagehubclient.isFileInFileStorage(
                    newStorageRequestDataBlob.fileKey
                );

                if (!result.isFileFound) {
                    throw new Error(
                        `File not found in storage for ${newStorageRequestDataBlob.location.toHuman()}`
                    );
                }

                issuedFileKeys.push(newStorageRequestDataBlob.fileKey);
            }

            // Seal block containing the MSP's transaction response to the storage request
            await userApi.wait.mspResponseInTxPool();
            await userApi.sealBlock();

            let mspAcceptedStorageRequestDataBlob: any = undefined;
            let storageRequestFulfilledDataBlob: any = undefined;

            try {
                const { event: mspAcceptedStorageRequestEvent } = await userApi.assert.eventPresent(
                    "fileSystem",
                    "MspAcceptedStorageRequest"
                );
                mspAcceptedStorageRequestDataBlob =
                    userApi.events.fileSystem.MspAcceptedStorageRequest.is(mspAcceptedStorageRequestEvent) &&
                    mspAcceptedStorageRequestEvent.data;
            } catch {
                // Event not found, continue
            }

            try {
                const { event: storageRequestFulfilledEvent } = await userApi.assert.eventPresent(
                    "fileSystem",
                    "StorageRequestFulfilled"
                );
                storageRequestFulfilledDataBlob =
                    userApi.events.fileSystem.StorageRequestFulfilled.is(storageRequestFulfilledEvent) &&
                    storageRequestFulfilledEvent.data;
            } catch {
                // Event not found, continue
            }

            let acceptedFileKey: string | null = null;
            // We expect either the MSP accepted the storage request or the storage request was fulfilled
            if (mspAcceptedStorageRequestDataBlob) {
                acceptedFileKey = mspAcceptedStorageRequestDataBlob.fileKey.toString();
            } else if (storageRequestFulfilledDataBlob) {
                acceptedFileKey = storageRequestFulfilledDataBlob.fileKey.toString();
            }

            if (!acceptedFileKey) {
                throw new Error(
                    "Neither MspAcceptedStorageRequest nor StorageRequestFulfilled events were found"
                );
            }

            // There is only a single key being accepted since it is the first file key to be processed and there is nothing to batch.
            strictEqual(
                issuedFileKeys.some((key) => key.toString() === acceptedFileKey),
                true
            );

            // Allow time for the MSP to update the local forest root
            await sleep(3000);

            const local_bucket_root = await mspApi.rpc.storagehubclient.getForestRoot(
                newBucketEventDataBlob.bucketId.toString()
            );

            const { event: bucketRootChangedEvent } = await userApi.assert.eventPresent(
                "providers",
                "BucketRootChanged"
            );

            const bucketRootChangedDataBlob =
                userApi.events.providers.BucketRootChanged.is(bucketRootChangedEvent) &&
                bucketRootChangedEvent.data;

            if (!bucketRootChangedDataBlob) {
                throw new Error("Expected BucketRootChanged event but received event of different type");
            }

            strictEqual(bucketRootChangedDataBlob.newRoot.toString(), local_bucket_root.toString());

            const isFileInForest = await mspApi.rpc.storagehubclient.isFileInForest(
                newBucketEventDataBlob.bucketId.toString(),
                acceptedFileKey
            );

            invariant(isFileInForest.isTrue, "File is not in forest");

            // Seal block containing the MSP's transaction response to the storage request
            await userApi.wait.mspResponseInTxPool();
            await userApi.sealBlock();

            const fileKeys2: string[] = [];

            const mspAcceptedStorageRequestEvents = await userApi.assert.eventMany(
                "fileSystem",
                "MspAcceptedStorageRequest"
            );

            for (const e of mspAcceptedStorageRequestEvents) {
                const mspAcceptedStorageRequestDataBlob =
                    userApi.events.fileSystem.MspAcceptedStorageRequest.is(e.event) && e.event.data;
                if (mspAcceptedStorageRequestDataBlob) {
                    fileKeys2.push(mspAcceptedStorageRequestDataBlob.fileKey.toString());
                }
            }

            invariant(fileKeys2.length === 2, "Expected 2 file keys");

            // Allow time for the MSP to update the local forest root
            await sleep(3000);

            const local_bucket_root2 = await mspApi.rpc.storagehubclient.getForestRoot(
                newBucketEventDataBlob.bucketId.toString()
            );

            const { event: bucketRootChangedEvent2 } = await userApi.assert.eventPresent(
                "providers",
                "BucketRootChanged"
            );

            const bucketRootChangedDataBlob2 =
                userApi.events.providers.BucketRootChanged.is(bucketRootChangedEvent2) &&
                bucketRootChangedEvent2.data;

            if (!bucketRootChangedDataBlob2) {
                throw new Error("Expected BucketRootChanged event but received event of different type");
            }

            strictEqual(bucketRootChangedDataBlob2.newRoot.toString(), local_bucket_root2.toString());

            for (const fileKey of fileKeys2) {
                const isFileInForest = await mspApi.rpc.storagehubclient.isFileInForest(
                    newBucketEventDataBlob.bucketId.toString(),
                    fileKey
                );
                invariant(isFileInForest.isTrue, "File is not in forest");
            }
        });

        it("MSP is charging user", async () => {
            // Calculate how many blocks to advance until next challenge tick.
            const currentBlock = await userApi.rpc.chain.getBlock();
            const currentBlockNumber = currentBlock.block.header.number.toNumber();

            const blocksToAdvance = currentBlockNumber + (12 - (currentBlockNumber % 12)) + 1; // 12 is the msp_charging_freq that we setup
            if (blocksToAdvance > currentBlockNumber) {
                await userApi.advanceToBlock(blocksToAdvance);
            }

            // Verify that the MSP charged the users after the notified
            await userApi.assert.eventPresent("paymentStreams", "PaymentStreamCharged");
        });
    }
);
