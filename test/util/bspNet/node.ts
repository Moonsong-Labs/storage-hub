import type { ApiPromise } from "@polkadot/api";
import assert from "node:assert";
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
    // Remove all extrinsics from the txPool
    await Promise.all(
      pendingBefore
        .map(({ hash }) => hash.toHex())
        .map((hash) => api.rpc.author.removeExtrinsic([{ Hash: hash }]))
    );
    const pendingAfter = await api.rpc.author.pendingExtrinsics();
    assert(pendingAfter.length === 0, "Not all extrinsics removed from txPool");
  } else if (typeof extrinsic === "object" && "module" in extrinsic && "method" in extrinsic) {
    // Remove extrinsics matching the specified module and method
    const matches = pendingBefore
      .filter(
        ({ method }) => method.section === extrinsic.module && method.method === extrinsic.method
      )
      .map(({ hash }) => hash.toHex());

    assert(
      matches.length > 0,
      `No extrinsics found in txPool matching ${extrinsic.module}:${extrinsic.method}`
    );
    const result = await api.rpc.author.removeExtrinsic(matches.map((hash) => ({ Hash: hash })));
    const pendingAfter = await api.rpc.author.pendingExtrinsics();
    assert(result.length > 0, "No removal confirmation returned by RPC");
    assert(pendingBefore > pendingAfter, "Extrinsic not removed from txPool");
  } else {
    // Remove the extrinsic with the specified hash
    const result = await api.rpc.author.removeExtrinsic([{ Hash: extrinsic }]);
    const pendingAfter = await api.rpc.author.pendingExtrinsics();
    assert(result.length > 0, "No removal confirmation returned by RPC");
    assert(pendingBefore > pendingAfter, "Extrinsic not removed from txPool");
    assert(
      result.find((hash) => hash.toString() === extrinsic),
      "Extrinsic not removed from txPool"
    );
    assert(
      !pendingAfter.find((ext) => ext.hash.toString() === extrinsic),
      "Extrinsic not removed from txPool"
    );
  }

  if (sealAfter) {
    await sealBlock(api);
  }
}
