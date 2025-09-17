import assert, { strictEqual } from "node:assert";
import Docker from "dockerode";
import {
  addBspContainer,
  DOCKER_IMAGE,
  describeBspNet,
  type EnrichedBspApi,
  waitFor
} from "../../../util";
import { CAPACITY, MAX_STORAGE_CAPACITY } from "../../../util/bspNet/consts.ts";

await describeBspNet("BSPNet: Adding new BSPs", ({ before, createUserApi, createApi, it }) => {
  let userApi: EnrichedBspApi;

  before(async () => {
    userApi = await createUserApi();
  });

  it("New BSP can be created", async () => {
    const { containerName, rpcPort, p2pPort, peerId } = await addBspContainer({
      name: "nueva",
      additionalArgs: [
        `--max-storage-capacity=${MAX_STORAGE_CAPACITY}`,
        `--jump-capacity=${CAPACITY[1024]}`
      ]
    });

    await it("is in a running container", async () => {
      const docker = new Docker();
      const {
        State: { Status }
      } = await docker.getContainer(containerName).inspect();
      strictEqual(Status, "running");
    });

    await it("can open new API connection with", async () => {
      console.log(`connecting to rpcPort ${rpcPort}`);
      await using newApi = await createApi(`ws://127.0.0.1:${rpcPort}`);

      await it("has correct reported peerId", async () => {
        const localPeerId = await newApi.rpc.system.localPeerId();
        strictEqual(localPeerId.toString(), peerId);
      });

      await it("is synced with current block", async () => {
        // Give some time to the BSP to catch up
        await userApi.wait.bspCatchUpToChainTip(newApi);

        const syncHeight = (await newApi.rpc.chain.getHeader()).number.toNumber();
        const currentHeight = (await userApi.rpc.chain.getHeader()).number.toNumber();
        const syncHash = (await newApi.rpc.chain.getHeader()).hash.toString();
        const currentHash = (await userApi.rpc.chain.getHeader()).hash.toString();
        strictEqual(syncHeight, currentHeight);
        strictEqual(syncHash, currentHash);
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
    });

    await it("is peer of other nodes", async () => {
      await waitFor({
        lambda: async () => {
          const peers = (await userApi.rpc.system.peers()).map(({ peerId }) => peerId.toString());
          return peers.includes(peerId);
        }
      });
    });
  });

  it("Lots of BSPs can be created", async () => {
    await addBspContainer({
      name: "timbo1",
      additionalArgs: [
        "--database=rocksdb",
        `--max-storage-capacity=${MAX_STORAGE_CAPACITY}`,
        `--jump-capacity=${CAPACITY[1024]}`
      ]
    });
    await addBspContainer({
      name: "timbo2",
      additionalArgs: [
        "--database=paritydb",
        `--max-storage-capacity=${MAX_STORAGE_CAPACITY}`,
        `--jump-capacity=${CAPACITY[1024]}`
      ]
    });
    await addBspContainer({
      name: "timbo3",
      additionalArgs: [
        "--database=auto",
        `--max-storage-capacity=${MAX_STORAGE_CAPACITY}`,
        `--jump-capacity=${CAPACITY[1024]}`
      ]
    });

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
    const { rpcPort } = await addBspContainer({
      name: "insert-keys-container",
      additionalArgs: [
        `--keystore-path=${keystorePath}`,
        `--max-storage-capacity=${MAX_STORAGE_CAPACITY}`,
        `--jump-capacity=${CAPACITY[1024]}`
      ]
    });
    await using insertKeysApi = await createApi(`ws://127.0.0.1:${rpcPort}`);

    const alicePubKey = "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d";
    const bobPubKey = "0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48";
    const bcsvKeyType = "bcsv";
    const bobSeed = "//Bob";

    const hasAliceKey = await insertKeysApi.rpc.author.hasKey(alicePubKey, bcsvKeyType);
    strictEqual(hasAliceKey.isTrue, true);

    let hasBobKey = await insertKeysApi.rpc.author.hasKey(bobPubKey, bcsvKeyType);
    strictEqual(hasBobKey.isTrue, false);

    // Rotate keys and check that Bob's pub key is now in Keystore.
    await insertKeysApi.rpc.storagehubclient.insertBcsvKeys(bobSeed);
    hasBobKey = await insertKeysApi.rpc.author.hasKey(bobPubKey, bcsvKeyType);
    strictEqual(hasBobKey.isTrue, true);
  });

  it("Removes BCSV keys from keystore", async () => {
    const keystore_path = "/tmp/test/remove/keystore";
    const { rpcPort } = await addBspContainer({
      name: "remove-keys-container",
      additionalArgs: [
        `--keystore-path=${keystore_path}`,
        `--max-storage-capacity=${MAX_STORAGE_CAPACITY}`,
        `--jump-capacity=${CAPACITY[1024]}`
      ]
    });
    await using removeKeysApi = await createApi(`ws://127.0.0.1:${rpcPort}`);
    const alicePubKey = "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d";
    const davePubKey = "0x306721211d5404bd9da88e0204360a1a9ab8b87c66c1bc2fcdd37f3c2222cc20";
    const bcsvKeyType = "bcsv";
    const daveSeed = "//Dave";

    let hasAliceKey = await removeKeysApi.rpc.author.hasKey(alicePubKey, bcsvKeyType);
    strictEqual(hasAliceKey.isTrue, true);

    let hasDaveKey = await removeKeysApi.rpc.author.hasKey(davePubKey, bcsvKeyType);
    strictEqual(hasDaveKey.isTrue, false);

    // Rotate keys and check that Dave's pub key is now in Keystore.
    await removeKeysApi.rpc.storagehubclient.insertBcsvKeys(daveSeed);
    hasDaveKey = await removeKeysApi.rpc.author.hasKey(davePubKey, bcsvKeyType);
    strictEqual(hasDaveKey.isTrue, true);

    await removeKeysApi.rpc.storagehubclient.removeBcsvKeys(keystore_path);

    // We still have Alice's key in `--dev` mode because it's inserted into the in-memory Keystore.
    hasAliceKey = await removeKeysApi.rpc.author.hasKey(alicePubKey, bcsvKeyType);
    strictEqual(hasAliceKey.isTrue, true);
    hasDaveKey = await removeKeysApi.rpc.author.hasKey(davePubKey, bcsvKeyType);
    assert(hasDaveKey.isFalse);
  });
});
