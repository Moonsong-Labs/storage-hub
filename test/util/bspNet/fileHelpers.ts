import type { ApiPromise } from "@polkadot/api";
import type { FileMetadata } from "./types";
import { assertEventPresent } from "../asserts";
import { shUser } from "../pjsKeyring";
import * as ShConsts from "./consts";
import { sealBlock } from "./block";
import assert from "node:assert";
import type { HexString } from "@polkadot/util/types";
import type { KeyringPair } from "@polkadot/keyring/types";
import type { AccountId32, H256 } from "@polkadot/types/interfaces";
import { GenericAccountId } from "@polkadot/types";

export const sendNewStorageRequest = async (
  api: ApiPromise,
  source: string,
  location: string,
  bucketId: H256,
  owner?: KeyringPair,
  mspId?: HexString
): Promise<FileMetadata> => {
  const { file_metadata: fileMetadata } = await api.rpc.storagehubclient.loadFileInStorage(
    source,
    location,
    ShConsts.NODE_INFOS.user.AddressId,
    bucketId
  );

  const issueOwner = owner ?? shUser;

  const replicationTarget = {
    Basic: null
  };

  const issueStorageRequestResult = await sealBlock(
    api,
    api.tx.fileSystem.issueStorageRequest(
      bucketId,
      location,
      fileMetadata.fingerprint,
      fileMetadata.file_size,
      mspId ?? ShConsts.DUMMY_MSP_ID,
      [ShConsts.NODE_INFOS.user.expectedPeerId],
      replicationTarget
    ),
    issueOwner
  );

  const accountId: AccountId32 = new GenericAccountId(api.registry, issueOwner.publicKey);

  const newStorageRequestEvent = assertEventPresent(
    api,
    "fileSystem",
    "NewStorageRequest",
    issueStorageRequestResult.events
  );
  const newStorageRequestEventDataBlob =
    api.events.fileSystem.NewStorageRequest.is(newStorageRequestEvent.event) &&
    newStorageRequestEvent.event.data;

  assert(newStorageRequestEventDataBlob, "Event doesn't match Type");

  return {
    fileKey: newStorageRequestEventDataBlob.fileKey.toString(),
    bucketId: bucketId.toString(),
    location: newStorageRequestEventDataBlob.location.toString(),
    owner: accountId.toString(),
    fingerprint: fileMetadata.fingerprint,
    fileSize: fileMetadata.file_size
  };
};

export const createBucketAndSendNewStorageRequest = async (
  api: ApiPromise,
  source: string,
  location: string,
  bucketName: string,
  valuePropId?: HexString | null,
  mspId?: HexString | null,
  owner?: KeyringPair | null,
  replicationTarget?: number | null
): Promise<FileMetadata> => {
  let localValuePropId = valuePropId;
  let localOwner = owner;

  if (!localValuePropId) {
    const valueProps = await api.call.storageProvidersApi.queryValuePropositionsForMsp(
      mspId ?? ShConsts.DUMMY_MSP_ID
    );
    localValuePropId = valueProps[0].id;

    if (!localValuePropId) {
      throw new Error("No value proposition found");
    }
  }

  if (!localOwner) {
    localOwner = shUser;
  }

  const newBucketEventEvent = await createBucket(
    api,
    bucketName,
    localValuePropId,
    mspId ?? ShConsts.DUMMY_MSP_ID,
    localOwner
  );
  const newBucketEventDataBlob =
    api.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

  assert(newBucketEventDataBlob, "Event doesn't match Type");

  const { file_metadata: fileMetadata } = await api.rpc.storagehubclient.loadFileInStorage(
    source,
    location,
    ShConsts.NODE_INFOS.user.AddressId,
    newBucketEventDataBlob.bucketId
  );

  let replicationTargetToUse: { Custom: number } | { Basic: null };
  if (replicationTarget) {
    replicationTargetToUse = {
      Custom: replicationTarget
    };
  } else {
    replicationTargetToUse = {
      Basic: null
    };
  }
  const issueStorageRequestResult = await sealBlock(
    api,
    api.tx.fileSystem.issueStorageRequest(
      newBucketEventDataBlob.bucketId,
      location,
      fileMetadata.fingerprint,
      fileMetadata.file_size,
      mspId ?? ShConsts.DUMMY_MSP_ID,
      [ShConsts.NODE_INFOS.user.expectedPeerId],
      replicationTargetToUse
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

  assert(newStorageRequestEventDataBlob, "Event doesn't match Type");

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
  owner: KeyringPair | null = shUser
) => {
  let localValuePropId = valuePropId;

  if (localValuePropId === undefined) {
    const valueProps = await api.call.storageProvidersApi.queryValuePropositionsForMsp(
      mspId ?? ShConsts.DUMMY_MSP_ID
    );

    localValuePropId = valueProps[0].id;
  }

  if (localValuePropId === undefined || localValuePropId === null) {
    throw new Error("No value proposition found");
  }

  const createBucketResult = await sealBlock(
    api,
    api.tx.fileSystem.createBucket(
      mspId ?? ShConsts.DUMMY_MSP_ID,
      bucketName,
      false,
      localValuePropId
    ),
    owner ?? undefined
  );
  const { event } = assertEventPresent(api, "fileSystem", "NewBucket", createBucketResult.events);

  return event;
};
