import { expect, test, describe, beforeAll } from "bun:test";
import { createClient as createRawClient } from "@polkadot-api/substrate-client";
import { createClient } from "@polkadot-api/client";
import { WebSocketProvider } from "@polkadot-api/ws-provider/node";
import { start } from "smoldot";
import { getChain } from "@polkadot-api/node-polkadot-provider";
import { getSmProvider } from "@polkadot-api/sm-provider";
import relayTypes from "../../typegen/relaychain";
import rawspec from "../../rawSpec.json" assert { type: "json" };

describe("Simple zombieTest", async () => {
  // biome-ignore lint/complexity/noBannedTypes: WIP
  const getChainspec = async (count = 1): Promise<{}> => {
    try {
      return await rawClient.request("sync_state_genSyncSpec", [true]);
    } catch (e) {
      if (count === 20) throw e;
      await new Promise((res) => setTimeout(res, 3_000));
      return getChainspec(count + 1);
    }
  };

  const rawClient = createRawClient(WebSocketProvider("ws://127.0.0.1:39459/"));
  const modified = {
    ...(await getChainspec()),
    bootNodes: [
      "/ip4/127.0.0.1/tcp/36123/ws/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp",
    ],
  };

  const spec = JSON.stringify(modified);
  const smoldot = start();
  const client = createClient(
    getChain({ provider: getSmProvider(smoldot, { chainSpec: spec }), keyring: [] })
  );
  const api = client.getTypedApi(relayTypes);

  console.log("getting the latest runtime");
  const runtime = await api.runtime.latest();

  test("Consts check", async () => {
    const blob = api.constants.System.SS58Prefix(runtime);
    console.log(blob);
  });
});
