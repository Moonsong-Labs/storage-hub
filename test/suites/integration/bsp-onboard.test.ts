import "@storagehub/api-augment";
import { after, before, describe, it } from "node:test";
import {
  addBspContainer,
  type BspNetApi,
  cleardownTest,
  createApiObject,
  DOCKER_IMAGE,
  NODE_INFOS,
  runBspNet
} from "../../util";
import Docker from "dockerode";
import { strictEqual } from "node:assert";

const bspNetConfigCases = [
  { noisy: false, rocksdb: false },
  { noisy: false, rocksdb: true },
  { noisy: true, rocksdb: false }
];

for (const { noisy, rocksdb } of bspNetConfigCases) {
  describe("BSPNet: Adding new BSPs", () => {
    let api: BspNetApi;

    before(async () => {
      await runBspNet(noisy, rocksdb);
      api = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.bsp.port}`);
    });

    after(async () => {
      await cleardownTest(api);
    });

    it("New BSP can be created", async () => {
      const { containerName, rpcPort, p2pPort, peerId } = await addBspContainer();

      await it("is in a running container", async () => {
        const docker = new Docker();
        const {
          State: { Status }
        } = await docker.getContainer(containerName).inspect();
        strictEqual(Status, "running");
      });

      await it("can open new API connection with", async () => {
        const newApi = await createApiObject(`ws://127.0.0.1:${rpcPort}`);

        await it("has correct reported peerId", async () => {
          const localPeerId = await newApi.rpc.system.localPeerId();
          strictEqual(localPeerId.toString(), peerId);
        });

        await it("is synced with current block", async () => {
          const syncHeight = (await newApi.rpc.chain.getHeader()).number.toNumber();
          const currentHeight = (await api.rpc.chain.getHeader()).number.toNumber();
          strictEqual(syncHeight, currentHeight);
        });

        await it("is listening on the correct P2P port", async () => {
          const listenAddresses = (await newApi.rpc.system.localListenAddresses()).map((address) =>
            address.toString()
          );
          const matchingAddress = listenAddresses.filter((address) =>
            address.includes(`/tcp/${p2pPort}/p2p/`)
          );
          strictEqual(matchingAddress.length > 1, true);
        });

        await newApi.disconnect();
      });

      await it("is peer of other nodes", async () => {
        const peers = (await api.rpc.system.peers()).map(({ peerId }) => peerId.toString());
        strictEqual(peers.includes(peerId), true);
      });
    });

    it("Lots of BSPs can be created", async () => {
      await addBspContainer({ name: "timbo1", additionalArgs: ["--database=rocksdb"] });
      await addBspContainer({ name: "timbo2", additionalArgs: ["--database=paritydb"] });
      await addBspContainer({ name: "timbo3", additionalArgs: ["--database=auto"] });

      const docker = new Docker();
      const sh_nodes = (
        await docker.listContainers({
          filters: { ancestor: [DOCKER_IMAGE] }
        })
      ).flatMap(({ Names }) => Names);

      strictEqual(sh_nodes.length > 3, true);
    });
  });
}
