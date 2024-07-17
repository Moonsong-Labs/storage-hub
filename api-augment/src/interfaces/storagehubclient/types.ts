// Auto-generated via `yarn polkadot-types-from-defs`, do not edit
/* eslint-disable */

import type { Bytes, Struct, U8aFixed, u64 } from "@polkadot/types-codec";

/** @name FileMetadata */
export interface FileMetadata extends Struct {
  readonly owner: Bytes;
  readonly bucket_id: Bytes;
  readonly location: Bytes;
  readonly file_size: u64;
  readonly fingerprint: U8aFixed;
}

export type PHANTOM_STORAGEHUBCLIENT = "storagehubclient";
