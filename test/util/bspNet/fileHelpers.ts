import type { ApiPromise } from "@polkadot/api";
import type { FileMetadata } from "./types";
import { assertEventPresent } from "../asserts";
import { shUser } from "../pjsKeyring";
import { ShConsts } from "./consts";
import { sealBlock } from "./block";

export const sendNewStorageRequest = async (
  api: ApiPromise,
  source: string,
  location: string,
  bucketName: string
): Promise<FileMetadata> => {
  const newBucketEventEvent = await createBucket(api, bucketName);
  const newBucketEventDataBlob =
    api.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

  if (!newBucketEventDataBlob) {
    throw new Error("Event doesn't match Type");
  }

  const fileMetadata = await api.rpc.storagehubclient.loadFileInStorage(
    source,
    location,
    ShConsts.NODE_INFOS.user.AddressId,
    newBucketEventDataBlob.bucketId
  );

  const issueStorageRequestResult = await sealBlock(
    api,
    api.tx.fileSystem.issueStorageRequest(
      newBucketEventDataBlob.bucketId,
      location,
      fileMetadata.fingerprint,
      fileMetadata.file_size,
      ShConsts.DUMMY_MSP_ID,
      [ShConsts.NODE_INFOS.user.expectedPeerId]
    ),
    shUser
  );

  const newStorageRequestEvent = assertEventPresent(
    api,
    "fileSystem",
    "NewStorageRequest",
    issueStorageRequestResult.events
  );
  const newStorageRequestEventDataBlob =
    api.events.fileSystem.NewStorageRequest.is(newStorageRequestEvent.event) &&
    newStorageRequestEvent.event.data;

  if (!newStorageRequestEventDataBlob) {
    throw new Error("Event doesn't match Type");
  }

  return {
    fileKey: newStorageRequestEventDataBlob.fileKey.toString(),
    bucketId: newBucketEventDataBlob.bucketId.toString(),
    location: newStorageRequestEventDataBlob.location.toString(),
    owner: newBucketEventDataBlob.who.toString(),
    fingerprint: fileMetadata.fingerprint,
    fileSize: fileMetadata.file_size
  };
};

export const createBucket = async (api: ApiPromise, bucketName: string) => {
  const createBucketResult = await sealBlock(
    api,
    api.tx.fileSystem.createBucket(ShConsts.DUMMY_MSP_ID, bucketName, false),
    shUser
  );
  const { event } = assertEventPresent(api, "fileSystem", "NewBucket", createBucketResult.events);

  return event;
};

export namespace Files {
  export const newStorageRequest = sendNewStorageRequest;
  export const newBucket = createBucket;
}