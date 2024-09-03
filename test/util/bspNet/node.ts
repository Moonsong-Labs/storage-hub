import type { ApiPromise } from "@polkadot/api";
import { strictEqual } from "node:assert";
import type { HexString } from "@polkadot/util/types";
import { sealBlock } from "./block";

/**
 * Drops transaction(s) from the node's transaction pool.
 *
 * @param extrinsic - Optional. Specifies which transaction(s) to drop:
 *                    - If omitted, all transactions in the pool will be cleared.
 *                    - If an object with module and method, it will drop matching transactions.
 *                    - If a hex string, it will drop the transaction with the matching hash.
 * @param sealAfter - Whether to seal a block after dropping the transaction(s). Defaults to false.
 */
export async function dropTransaction(
  api: ApiPromise,
  extrinsic?: { module: string; method: string } | HexString,
  sealAfter = false
) {
  const pendingBefore = await api.rpc.author.pendingExtrinsics();

  if (!extrinsic) {
    await Promise.all(
      pendingBefore
        .map(({ hash }) => hash.toHex())
        .map((hash) => api.rpc.author.removeExtrinsic([{ Hash: hash }]))
    );
    const pendingAfter = await api.rpc.author.pendingExtrinsics();
    strictEqual(pendingAfter.length, 0, "Not all extrinsics removed from txPool");
  } else if (typeof extrinsic === "object" && "module" in extrinsic && "method" in extrinsic) {
    const matches = pendingBefore
      .filter(
        ({ method }) => method.section === extrinsic.module && method.method === extrinsic.method
      )
      .map(({ hash }) => hash.toHex());

    strictEqual(
      matches.length > 0,
      true,
      `No extrinsics found in txPool matching ${extrinsic.module}:${extrinsic.method}`
    );
    const result = await api.rpc.author.removeExtrinsic(matches.map((hash) => ({ Hash: hash })));
    const pendingAfter = await api.rpc.author.pendingExtrinsics();
    strictEqual(result.length > 0, true, "No removal confirmation returned by RPC");
    strictEqual(pendingBefore > pendingAfter, true, "Extrinsic not removed from txPool");
  } else {
    const result = await api.rpc.author.removeExtrinsic([{ Hash: extrinsic }]);
    const pendingAfter = await api.rpc.author.pendingExtrinsics();
    strictEqual(result.length > 0, true, "No removal confirmation returned by RPC");
    strictEqual(pendingBefore > pendingAfter, true, "Extrinsic not removed from txPool");
  }

  if (sealAfter) {
    await sealBlock(api);
  }
}

export namespace NodeBspNet {
  export const dropTxn = dropTransaction;
}
