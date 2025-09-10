// import assert, { strictEqual } from "node:assert";
// import { describeMspNet, shUser, waitFor, type EnrichedBspApi } from "../../../util";

// TODO: Skipping this test suite until new file deletion flow is implemented.
// await describeMspNet(
//   "Single MSP deleting multiple files",
//   ({ before, createMsp1Api, it, createUserApi }) => {
//     let userApi: EnrichedBspApi;
//     let mspApi: EnrichedBspApi;
//     const source = ["res/whatsup.jpg", "res/adolphus.jpg", "res/cloud.jpg"];
//     const destination = ["test/whatsup.jpg", "test/adolphus.jpg", "test/cloud.jpg"];
//     const bucketName = "delete-bucket-1";
//     let bucketId: string;
//     let fileKeys: string[] = [];

//     before(async () => {
//       userApi = await createUserApi();
//       const maybeMspApi = await createMsp1Api();
//       assert(maybeMspApi, "MSP API not available");
//       mspApi = maybeMspApi;
//     });

//     it("Network launches and can be queried", async () => {
//       const userNodePeerId = await userApi.rpc.system.localPeerId();
//       strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

//       const mspNodePeerId = await mspApi.rpc.system.localPeerId();
//       strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp1.expectedPeerId);
//     });

//     it("User submits multiple storage requests and MSP accepts them", async () => {
//       // Get value propositions from the MSP to use
//       const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
//         userApi.shConsts.DUMMY_MSP_ID
//       );
//       const valuePropId = valueProps[0].id;

//       // Create a new bucket where all the files will be stored
//       const newBucketEventEvent = await userApi.createBucket(bucketName, valuePropId);
//       const newBucketEventDataBlob =
//         userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

//       assert(newBucketEventDataBlob, "NewBucket event data does not match expected type");
//       bucketId = newBucketEventDataBlob.bucketId.toString();

//       // Submit storage requests for all files
//       const files = [];
//       const txs = [];
//       for (let i = 0; i < source.length; i++) {
//         const {
//           file_metadata: { location, fingerprint, file_size }
//         } = await userApi.rpc.storagehubclient.loadFileInStorage(
//           source[i],
//           destination[i],
//           userApi.shConsts.NODE_INFOS.user.AddressId,
//           bucketId
//         );

//         files.push({ fingerprint, file_size, location });
//         txs.push(
//           userApi.tx.fileSystem.issueStorageRequest(
//             bucketId,
//             location,
//             fingerprint,
//             file_size,
//             userApi.shConsts.DUMMY_MSP_ID,
//             [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
//             {
//               Basic: null
//             }
//           )
//         );
//       }
//       await userApi.block.seal({ calls: txs, signer: shUser });

//       // Get the storage request events and verify file keys
//       const storageRequestEvents = await userApi.assert.eventMany(
//         "fileSystem",
//         "NewStorageRequest"
//       );
//       strictEqual(storageRequestEvents.length, source.length);

//       fileKeys = storageRequestEvents.map((event) => {
//         const dataBlob =
//           userApi.events.fileSystem.NewStorageRequest.is(event.event) && event.event.data;
//         assert(dataBlob, "Event doesn't match NewStorageRequest type");
//         return dataBlob.fileKey.toString();
//       });

//       // Wait for MSP to store all files
//       for (const fileKey of fileKeys) {
//         await mspApi.wait.fileStorageComplete(fileKey);
//       }

//       // Wait for MSP response and verify acceptance
//       await userApi.wait.mspResponseInTxPool();
//       await userApi.block.seal();

//       // Verify that the MSP a single storage request (MSP will accept the first one and avoid batching)
//       await userApi.assert.eventPresent("fileSystem", "MspAcceptedStorageRequest");

//       // Wait for the 2 remaining storage requests to be accepted
//       await userApi.wait.mspResponseInTxPool();
//       await userApi.block.seal();

//       // Verify that the MSP accepted the remaining 2 storage requests
//       const mspAcceptedEvents = await userApi.assert.eventMany(
//         "fileSystem",
//         "MspAcceptedStorageRequest"
//       );
//       strictEqual(mspAcceptedEvents.length, 2);

//       // Verify files are in MSP's forest storage
//       await waitFor({
//         lambda: async () => {
//           let allFilesInForest = true;
//           for (const fileKey of fileKeys) {
//             const isFileInForest = await mspApi.rpc.storagehubclient.isFileInForest(
//               bucketId,
//               fileKey
//             );
//             allFilesInForest = allFilesInForest && isFileInForest.isTrue;
//           }
//           return allFilesInForest;
//         }
//       });
//     });

//     it("User requests file deletions and MSP processes them", async () => {
//       // Submit deletion requests for all files
//       const bucketOption = userApi.createType("Option<H256>", bucketId);
//       const deletionTxs = await Promise.all(
//         fileKeys.map(async (fileKey) => {
//           const fileMetadata = (
//             await mspApi.rpc.storagehubclient.getFileMetadata(bucketOption, fileKey)
//           ).unwrap();

//           return userApi.tx.fileSystem.deleteFile(
//             bucketId,
//             fileKey,
//             fileMetadata.location,
//             fileMetadata.file_size,
//             fileMetadata.fingerprint,
//             null
//           );
//         })
//       );

//       await userApi.block.seal({ calls: deletionTxs, signer: shUser });

//       // Process deletion requests one by one since the runtime extrinsic does not support batching
//       for (const _ of fileKeys) {
//         // Wait for MSP to process this deletion request
//         await userApi.wait.mspPendingFileDeletionRequestSubmitProof(1);
//         await userApi.block.seal();

//         // Verify deletion request was finalized
//         await userApi.assert.eventPresent(
//           "fileSystem",
//           "ProofSubmittedForPendingFileDeletionRequest"
//         );

//         const proofSubmittedEvents = await userApi.assert.eventMany(
//           "fileSystem",
//           "ProofSubmittedForPendingFileDeletionRequest"
//         );
//         strictEqual(proofSubmittedEvents.length, 1);
//       }

//       // Wait for MSP to process the finalized deletions and verify files are removed
//       await waitFor({
//         lambda: async () => {
//           let allFilesDeleted = true;
//           for (const fileKey of fileKeys) {
//             const isFileInForest = await mspApi.rpc.storagehubclient.isFileInForest(
//               bucketId,
//               fileKey
//             );
//             allFilesDeleted = allFilesDeleted && isFileInForest.isFalse;
//           }
//           return allFilesDeleted;
//         }
//       });
//     });
//   }
// );
