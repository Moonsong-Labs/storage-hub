// Auto-generated via `yarn polkadot-types-from-defs`, do not edit
/* eslint-disable */

import type { Bytes, Struct, U8aFixed } from "@polkadot/types-codec";

/** @name FileMetadata */
export interface FileMetadata extends Struct {
  readonly owner: Bytes;
  readonly bucket_id: Bytes;
  readonly location: Bytes;
  readonly size: number;
  readonly fingerprint: U8aFixed;
}

export type PHANTOM_STORAGEHUBCLIENT = "storagehubclient";
