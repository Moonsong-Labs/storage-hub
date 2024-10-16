import type { ApiPromise } from "@polkadot/api";
import type { FileMetadata } from "./types";
import { assertEventPresent } from "../asserts";
import { shUser } from "../pjsKeyring";
import * as ShConsts from "./consts";
import { sealBlock } from "./block";
import invariant from "tiny-invariant";
import type { HexString } from "@polkadot/util/types";
import type { KeyringPair } from "@polkadot/keyring/types";
import type { H256 } from "@polkadot/types/interfaces";

export const sendNewStorageRequest = async (
  api: ApiPromise,
  source: string,
  location: string,
  bucketName: string,
  valuePropId?: H256,
  mspId?: HexString,
  owner?: KeyringPair
): Promise<FileMetadata> => {
  if (valuePropId === undefined) {
    const valueProps = await api.call.storageProvidersApi.queryValuePropositionsForMsp(
      mspId ?? ShConsts.DUMMY_MSP_ID
    );

    valuePropId = valueProps[0][0];
  }

  if (valuePropId === undefined) {
    throw new Error("No value proposition found");
  }

  const newBucketEventEvent = await createBucket(api, bucketName, valuePropId, mspId, owner);
  const newBucketEventDataBlob =
    api.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

  invariant(newBucketEventDataBlob, "Event doesn't match Type");

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
      mspId ?? ShConsts.DUMMY_MSP_ID,
      [ShConsts.NODE_INFOS.user.expectedPeerId]
    ),
    owner ?? shUser
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

  invariant(newStorageRequestEventDataBlob, "Event doesn't match Type");

  return {
    fileKey: newStorageRequestEventDataBlob.fileKey.toString(),
    bucketId: newBucketEventDataBlob.bucketId.toString(),
    location: newStorageRequestEventDataBlob.location.toString(),
    owner: newBucketEventDataBlob.who.toString(),
    fingerprint: fileMetadata.fingerprint,
    fileSize: fileMetadata.file_size
  };
};

export const createBucket = async (
  api: ApiPromise,
  bucketName: string,
  valuePropId?: H256,
  mspId: HexString = ShConsts.DUMMY_MSP_ID,
  owner: KeyringPair = shUser
) => {
  const valueProps = await api.call.storageProvidersApi.queryValuePropositionsForMsp(
    mspId ?? ShConsts.DUMMY_MSP_ID
  );

  valuePropId = valueProps[0][0];

  if (valuePropId === undefined) {
    throw new Error("No value proposition found");
  }

  const createBucketResult = await sealBlock(
    api,
    api.tx.fileSystem.createBucket(mspId, bucketName, false, valuePropId),
    owner
  );
  const { event } = assertEventPresent(api, "fileSystem", "NewBucket", createBucketResult.events);

  return event;
};
