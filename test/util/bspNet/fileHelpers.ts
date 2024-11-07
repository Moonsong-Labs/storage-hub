import type { ApiPromise } from "@polkadot/api";
import type { FileMetadata } from "./types";
import { assertEventPresent } from "../asserts";
import { shUser } from "../pjsKeyring";
import * as ShConsts from "./consts";
import { sealBlock } from "./block";
import invariant from "tiny-invariant";
import type { HexString } from "@polkadot/util/types";
import type { KeyringPair } from "@polkadot/keyring/types";

export const sendNewStorageRequest = async (
  api: ApiPromise,
  source: string,
  location: string,
  bucketName: string,
  valuePropId?: HexString | null,
  mspId?: HexString | null,
  owner?: KeyringPair
): Promise<FileMetadata> => {
  let localValuePropId = valuePropId;

  if (mspId === undefined) {
    mspId = ShConsts.DUMMY_MSP_ID;
  }

  if (localValuePropId === undefined) {
    const valueProps = await api.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);

    localValuePropId = valueProps[0].id;
  }

  if (localValuePropId === undefined) {
    throw new Error("No value proposition found");
  }

  const newBucketEventEvent = await createBucket(api, bucketName, localValuePropId, mspId, owner);
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
      mspId,
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
  valuePropId?: HexString | null,
  mspId: HexString | null = ShConsts.DUMMY_MSP_ID,
  owner: KeyringPair = shUser
) => {
  let localValuePropId = valuePropId;

  if (localValuePropId === undefined) {
    const valueProps = await api.call.storageProvidersApi.queryValuePropositionsForMsp(
      mspId ?? ShConsts.DUMMY_MSP_ID
    );

    localValuePropId = valueProps[0].id;
  }

  if (localValuePropId === undefined) {
    throw new Error("No value proposition found");
  }

  const createBucketResult = await sealBlock(
    api,
    api.tx.fileSystem.createBucket(mspId, bucketName, false, localValuePropId),
    owner
  );
  const { event } = assertEventPresent(api, "fileSystem", "NewBucket", createBucketResult.events);

  return event;
};
