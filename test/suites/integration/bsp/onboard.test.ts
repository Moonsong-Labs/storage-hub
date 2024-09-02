import "@storagehub/api-augment";
import Docker from "dockerode";
import { strictEqual } from "node:assert";
import {
  addBspContainer,
  type BspNetApi,
  createApiObject,
  describeBspNet,
  DOCKER_IMAGE,
  stopBspContainer
} from "../../../util";

describeBspNet("BSPNet: Adding new BSPs", ({ before, createBspApi, it }) => {
  let api: BspNetApi;

  before(async () => {
    api = await createBspApi();
  });

  it("New BSP can be created", async () => {
    const { containerName, rpcPort, p2pPort, peerId } = await addBspContainer({ name: "nueva" });

    await it("is in a running container", async () => {
      const docker = new Docker();
      const {
        State: { Status }
      } = await docker.getContainer(containerName).inspect();
      strictEqual(Status, "running");
    });

    await it("can open new API connection with", async () => {
      console.log(`connecting to rpcPort ${rpcPort}`);
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
      strictEqual(peers.includes(peerId), true, `PeerId ${peerId} not found in ${peers}`);
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

  it("Inserts new blockchain service keys (BCSV)", async () => {
    const keystorePath = "/tmp/test/insert/keystore";
    const { containerName, rpcPort } = await addBspContainer({
      name: "insert-keys-container",
      additionalArgs: [`--keystore-path=${keystorePath}`]
    });
    const insertKeysApi = await createApiObject(`ws://127.0.0.1:${rpcPort}`);

    const alicePubKey = "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d";
    const bobPubKey = "0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48";
    const bcsvKeyType = "bcsv";
    const bobSeed = "//Bob";

    const hasAliceKey = await insertKeysApi.rpc.author.hasKey(alicePubKey, bcsvKeyType);
    strictEqual(hasAliceKey.toHuman().valueOf(), true);

    let hasBobKey = await insertKeysApi.rpc.author.hasKey(bobPubKey, bcsvKeyType);
    strictEqual(hasBobKey.toHuman().valueOf(), false);

    // Rotate keys and check that Bob's pub key is now in Keystore.
    await insertKeysApi.rpc.storagehubclient.insertBcsvKeys(bobSeed);
    hasBobKey = await insertKeysApi.rpc.author.hasKey(bobPubKey, bcsvKeyType);
    strictEqual(hasBobKey.toHuman().valueOf(), true);

    // We remove again the keys added in this test.
    await insertKeysApi.rpc.storagehubclient.removeBcsvKeys(keystorePath);

    stopBspContainer({ containerName, api: insertKeysApi });
  });

  it("Removes BCSV keys from keystore", async () => {
    const keystore_path = "/tmp/test/remove/keystore";
    const { containerName, rpcPort } = await addBspContainer({
      name: "remove-keys-container",
      additionalArgs: [`--keystore-path=${keystore_path}`]
    });
    const removeKeysApi = await createApiObject(`ws://127.0.0.1:${rpcPort}`);

    const alicePubKey = "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d";
    const davePubKey = "0x306721211d5404bd9da88e0204360a1a9ab8b87c66c1bc2fcdd37f3c2222cc20";
    const bcsvKeyType = "bcsv";
    const daveSeed = "//Dave";

    let hasAliceKey = await removeKeysApi.rpc.author.hasKey(alicePubKey, bcsvKeyType);
    strictEqual(hasAliceKey.toHuman().valueOf(), true);

    let hasDaveKey = await removeKeysApi.rpc.author.hasKey(davePubKey, bcsvKeyType);
    strictEqual(hasDaveKey.toHuman().valueOf(), false);

    // Rotate keys and check that Dave's pub key is now in Keystore.
    await removeKeysApi.rpc.storagehubclient.insertBcsvKeys(daveSeed);
    hasDaveKey = await removeKeysApi.rpc.author.hasKey(davePubKey, bcsvKeyType);
    strictEqual(hasDaveKey.toHuman().valueOf(), true);

    await removeKeysApi.rpc.storagehubclient.removeBcsvKeys(keystore_path);

    // We still have Alice's key in `--dev` mode because it's inserted into the in-memory Keystore.
    hasAliceKey = await removeKeysApi.rpc.author.hasKey(alicePubKey, bcsvKeyType);
    strictEqual(hasAliceKey.toHuman().valueOf(), true);
    hasDaveKey = await removeKeysApi.rpc.author.hasKey(davePubKey, bcsvKeyType);
    strictEqual(hasDaveKey.toHuman().valueOf(), false);

    stopBspContainer({ containerName, api: removeKeysApi });
  });
});
