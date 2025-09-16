import assert from "node:assert";
import type { ApiPromise } from "@polkadot/api";
import type { KeyringPair } from "@polkadot/keyring/types";
import { GenericAccountId } from "@polkadot/types";
import type { AccountId32, H256 } from "@polkadot/types/interfaces";
import { u8aToHex } from "@polkadot/util";
import type { HexString } from "@polkadot/util/types";
import { decodeAddress } from "@polkadot/util-crypto";
import { assertEventPresent } from "../asserts";
import { shUser } from "../pjsKeyring";
import { sealBlock } from "./block";
import * as ShConsts from "./consts";
import type { FileMetadata } from "./types";

export const sendNewStorageRequest = async (
  api: ApiPromise,
  source: string,
  location: string,
  bucketId: H256,
  owner: KeyringPair,
  mspId?: HexString
): Promise<FileMetadata> => {
  const ownerHexString = u8aToHex(decodeAddress(ShConsts.NODE_INFOS.user.AddressId));
  const { file_metadata: fileMetadata } = await api.rpc.storagehubclient.loadFileInStorage(
    source,
    location,
    ownerHexString.slice(2),
    bucketId
  );

  const issueOwner = owner;

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
    fingerprint: fileMetadata.fingerprint.toHex(),
    fileSize: fileMetadata.file_size.toNumber()
  };
};

export const createBucketAndSendNewStorageRequest = async (
  api: ApiPromise,
  source: string,
  location: string,
  bucketName: string,
  owner: KeyringPair,
  valuePropId?: HexString | null,
  mspId?: HexString | null,
  replicationTarget?: number | null,
  finalizeBlock = true
): Promise<FileMetadata> => {
  let localValuePropId = valuePropId;
  const localOwner = owner;

  if (!localValuePropId) {
    const valueProps = await api.call.storageProvidersApi.queryValuePropositionsForMsp(
      mspId ?? ShConsts.DUMMY_MSP_ID
    );
    localValuePropId = valueProps[0].id.toHex() as HexString;

    if (!localValuePropId) {
      throw new Error("No value proposition found");
    }
  }

  const newBucketEventEvent = await createBucket(
    api,
    bucketName,
    localOwner,
    localValuePropId,
    mspId ?? ShConsts.DUMMY_MSP_ID
  );
  const newBucketEventDataBlob =
    api.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

  assert(newBucketEventDataBlob, "Event doesn't match Type");

  const ownerHexString = u8aToHex(decodeAddress(localOwner.address));
  const { file_metadata: fileMetadata } = await api.rpc.storagehubclient.loadFileInStorage(
    source,
    location,
    ownerHexString.slice(2),
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
    owner ?? shUser,
    undefined,
    undefined,
    finalizeBlock
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
    fingerprint: fileMetadata.fingerprint.toHex(),
    fileSize: fileMetadata.file_size.toNumber()
  };
};

export const createBucket = async (
  api: ApiPromise,
  bucketName: string,
  owner: KeyringPair,
  valuePropId?: HexString | null,
  mspId: HexString | null = ShConsts.DUMMY_MSP_ID
) => {
  let localValuePropId = valuePropId;

  if (localValuePropId === undefined) {
    const valueProps = await api.call.storageProvidersApi.queryValuePropositionsForMsp(
      mspId ?? ShConsts.DUMMY_MSP_ID
    );

    localValuePropId = valueProps[0].id.toHex() as HexString;
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
    owner
  );
  const { event } = assertEventPresent(api, "fileSystem", "NewBucket", createBucketResult.events);

  return event;
};
