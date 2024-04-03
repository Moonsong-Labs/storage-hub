import { test, describe } from "bun:test";
import { createClient as createRawClient } from "@polkadot-api/substrate-client";
import { createClient } from "@polkadot-api/client";
import { WebSocketProvider } from "@polkadot-api/ws-provider/node";
import { start } from "smoldot";
import { getSmProvider } from "@polkadot-api/sm-provider";
import { relaychain } from "@polkadot-api/descriptors";

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
  };
  rawClient.destroy();

  const spec = JSON.stringify(modified);
  const smoldot = start();
  const client = createClient(getSmProvider(smoldot.addChain({ chainSpec: spec })));
  const api = client.getTypedApi(relaychain);
  const runtime = await api.runtime.latest();

  test("Consts check", async () => {
    console.log("Getting the SS58Prefix");
    const blob = api.constants.System.SS58Prefix(runtime);
    console.log(blob);
  });

  test("Test Spec Version", async () => {
    const { spec_name, spec_version } = api.constants.System.Version(runtime);
    console.log(spec_name);
    console.log(spec_version);
  });
});
