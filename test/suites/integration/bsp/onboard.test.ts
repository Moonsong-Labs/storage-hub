import "@storagehub/api-augment";
import { after, before, describe, it } from "node:test";
import {
  addBspContainer,
  type BspNetApi,
  type BspNetConfig,
  cleardownTest,
  createApiObject,
  DOCKER_IMAGE,
  NODE_INFOS,
  runSimpleBspNet
} from "../../../util";
import Docker from "dockerode";
import { strictEqual } from "node:assert";

const bspNetConfigCases: BspNetConfig[] = [
  { noisy: false, rocksdb: false },
  { noisy: false, rocksdb: true }
];

for (const bspNetConfig of bspNetConfigCases) {
  describe("BSPNet: Adding new BSPs", () => {
    let api: BspNetApi;

    before(async () => {
      await runSimpleBspNet(bspNetConfig);
      api = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.bsp.port}`);
    });

    after(async () => {
      await cleardownTest({ api });
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

    it("Inserts new blockchain service keys (bcsv)", async () => {
      const { containerName, rpcPort, p2pPort, peerId } = await addBspContainer({ keystorePath: "/tmp/test/insert/keystore" });
      const newApi = await createApiObject(`ws://127.0.0.1:${rpcPort}`);

      const alice_pub_key = "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d";
      const bob_pub_key = "0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48";
      const bcsv_key_type = "bcsv";
      const bob_seed = "//Bob";

      const has_alice_key = await newApi.rpc.author.hasKey(alice_pub_key, bcsv_key_type);
      strictEqual(has_alice_key.toHuman().valueOf(), true);

      let has_bob_key = await newApi.rpc.author.hasKey(bob_pub_key, bcsv_key_type);
      strictEqual(has_bob_key.toHuman().valueOf(), false);

      // Rotate keys and check that Bob's pub key is now in Keystore.
      await newApi.rpc.storagehubclient.insertBcsvKeys(bob_seed);
      has_bob_key = await newApi.rpc.author.hasKey(bob_pub_key, bcsv_key_type);
      strictEqual(has_bob_key.toHuman().valueOf(), true);

      const keystore_path = "/tmp/test/insert/keystore";
      await newApi.rpc.storagehubclient.removeBcsvKeys(keystore_path);

    });

    it("Removes keys from keystore", async () => {
      const { containerName, rpcPort, p2pPort, peerId } = await addBspContainer({ keystorePath: "/tmp/test/remove/keystore" });
      const newApi = await createApiObject(`ws://127.0.0.1:${rpcPort}`);

      const alice_pub_key = "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d";
      const dave_pub_key = "0x306721211d5404bd9da88e0204360a1a9ab8b87c66c1bc2fcdd37f3c2222cc20";
      const bcsv_key_type = "bcsv";
      const dave_seed = "//Dave";
      const keystore_path = "/tmp/test/remove/keystore";

      let has_alice_key = await newApi.rpc.author.hasKey(alice_pub_key, bcsv_key_type);
      strictEqual(has_alice_key.toHuman().valueOf(), true);

      let has_dave_key = await newApi.rpc.author.hasKey(dave_pub_key, bcsv_key_type);
      strictEqual(has_dave_key.toHuman().valueOf(), false);

      // Rotate keys and check that Dave's pub key is now in Keystore.
      await newApi.rpc.storagehubclient.insertBcsvKeys(dave_seed);
      has_dave_key = await newApi.rpc.author.hasKey(dave_pub_key, bcsv_key_type);
      strictEqual(has_dave_key.toHuman().valueOf(), true);

      await newApi.rpc.storagehubclient.removeBcsvKeys(keystore_path);

      // We still have Alice's key in `--dev` mode because it's inserted into the in-memory Keystore.
      has_alice_key = await newApi.rpc.author.hasKey(alice_pub_key, bcsv_key_type);
      strictEqual(has_alice_key.toHuman().valueOf(), true);
      has_dave_key = await newApi.rpc.author.hasKey(dave_pub_key, bcsv_key_type);
      strictEqual(has_dave_key.toHuman().valueOf(), false);
    });
  });
}
