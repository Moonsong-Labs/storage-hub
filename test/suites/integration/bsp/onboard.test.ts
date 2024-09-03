import Docker from "dockerode";
import { strictEqual } from "node:assert";
import { addBspContainer, describeBspNet, DOCKER_IMAGE, type EnrichedBspApi } from "../../../util";

describeBspNet("BSPNet: Adding new BSPs", ({ before, createBspApi, createApi, it }) => {
  let api: EnrichedBspApi;

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
      const newApi = await createApi(`ws://127.0.0.1:${rpcPort}`);

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

  it("Rotates the blockchain service keys (bcsv)", async () => {
    const alice_pub_key = "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d";
    const bob_pub_key = "0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48";
    const bcsv_key_type = "bcsv";
    const bob_seed = "//Bob";

    const has_alice_key = await api.rpc.author.hasKey(alice_pub_key, bcsv_key_type);
    strictEqual(has_alice_key.toHuman().valueOf(), true);

    let has_bob_key = await api.rpc.author.hasKey(bob_pub_key, bcsv_key_type);
    strictEqual(has_bob_key.toHuman().valueOf(), false);

    // Rotate keys and check that Bob's pub key is now in Keystore.
    await api.rpc.storagehubclient.rotateBcsvKeys(bob_seed);
    has_bob_key = await api.rpc.author.hasKey(bob_pub_key, bcsv_key_type);
    strictEqual(has_bob_key.toHuman().valueOf(), true);
  });
});
