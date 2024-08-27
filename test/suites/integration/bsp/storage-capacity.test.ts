import "@storagehub/api-augment";
import assert from "node:assert";
import { after, before, describe, it } from "node:test";
import {
  bspKey,
  type BspNetApi,
  type BspNetConfig,
  CAPACITY,
  CAPACITY_512,
  cleardownTest,
  createApiObject,
  DUMMY_BSP_ID,
  DUMMY_MSP_ID,
  ferdie,
  fetchEventData,
  NODE_INFOS,
  runSimpleBspNet,
  shUser,
  skipBlocks,
  TEST_ARTEFACTS
} from "../../../util";
import { setTimeout } from "node:timers/promises";

const bspNetConfigCases: BspNetConfig[] = [
  { noisy: false, rocksdb: false }
  // { noisy: false, rocksdb: true }
];

for (const bspNetConfig of bspNetConfigCases) {
  describe("BSPNet: Validating max storage", () => {
    let api: BspNetApi;

    before(async () => {
      await runSimpleBspNet(bspNetConfig);
      api = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.user.port}`);
    });

    after(async () => {
      await cleardownTest({ api });
    });

    it("Unregistered accounts fail when changing capacities", async () => {
      const totalCapacityBefore = await api.query.providers.totalBspsCapacity();
      const bspCapacityBefore = await api.query.providers.backupStorageProviders(DUMMY_BSP_ID);
      assert.ok(bspCapacityBefore.unwrap().capacity.eq(totalCapacityBefore));
      const { events, extSuccess } = await api.sealBlock(
        api.tx.providers.changeCapacity(CAPACITY[1024]),
        ferdie
      );
      assert.strictEqual(extSuccess, false);

      await skipBlocks(api, 20);
      const [eventInfo, _eventError] = fetchEventData(api.events.system.ExtrinsicFailed, events);
      assert.strictEqual(eventInfo.asModule.index.toNumber(), 40); // providers
      assert.strictEqual(eventInfo.asModule.error.toHex(), "0x0f000000"); // NotRegistered

      const totalCapacityAfter = await api.query.providers.totalBspsCapacity();
      const bspCapacityAfter = await api.query.providers.backupStorageProviders(DUMMY_BSP_ID);
      assert.ok(bspCapacityAfter.unwrap().capacity.eq(totalCapacityBefore));
      assert.ok(totalCapacityAfter.eq(totalCapacityBefore));
    });

    it.skip("Maxed out storages not volunteered", async () => {
      // const tim = await addBsp(api, bspTwoKey, {
      //   name: "sh-bsp-two",
      //   rocksdb: bspNetConfig.rocksdb,
      //   bspKeySeed: bspTwoSeed,
      //   bspId: BSP_TWO_ID,
      //   additionalArgs: ["--keystore-path=/keystore/bsp-two"],
      //   // connectApi: true
      // });
      // console.log(tim);
    });

    it("Total capacity updated when single BSP capacity updated", async () => {
      const newCapacity = BigInt(Math.floor(Math.random() * 1000 * 1024 * 1024)) + CAPACITY_512;

      await api.sealBlock(api.tx.providers.changeCapacity(newCapacity), bspKey);

      const totalCapacityAfter = await api.query.providers.totalBspsCapacity();
      const bspCapacityAfter = await api.query.providers.backupStorageProviders(DUMMY_BSP_ID);
      assert.strictEqual(bspCapacityAfter.unwrap().capacity.toBigInt(), newCapacity);
      assert.strictEqual(totalCapacityAfter.toBigInt(), newCapacity);
    });

    it("BSP storage size can be exceeded", async () => {
      //TODO
    });

    it.skip("Test BSP storage size can be increased", async () => {
      //TODO
    });

    it("Test BSP storage size can not be decreased below used", async () => {
      const source = "res/whatsup.jpg";
      const destination = "test/whatsup.jpg";
      const bucketName = "nothingmuch-2";
      const newBucketEventEvent = await api.createBucket(bucketName);
      const newBucketEventDataBlob =
        api.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

      if (!newBucketEventDataBlob) {
        throw new Error("Event doesn't match Type");
      }

      // TODO derive how many blocks to skip
      await skipBlocks(api, 20);

      const { fingerprint, file_size, location } = await api.rpc.storagehubclient.loadFileInStorage(
        source,
        destination,
        NODE_INFOS.user.AddressId,
        newBucketEventDataBlob.bucketId
      );

      await api.sealBlock(
        api.tx.fileSystem.issueStorageRequest(
          newBucketEventDataBlob.bucketId,
          location,
          fingerprint,
          file_size,
          DUMMY_MSP_ID,
          [NODE_INFOS.user.expectedPeerId]
        ),
        shUser
      );
      await api.sealBlock();

      await setTimeout(5000);
      await api.sealBlock();
      const { events, extSuccess } = await api.sealBlock(
        api.tx.providers.changeCapacity(2n),
        bspKey
      );
      assert.strictEqual(extSuccess, false);
      const [eventInfo, _eventError] = fetchEventData(api.events.system.ExtrinsicFailed, events);
      assert.strictEqual(eventInfo.asModule.index.toNumber(), 40); // providers
      assert.strictEqual(eventInfo.asModule.error.toHex(), "0x0b000000"); // NewCapacityLessThanUsedStorage

      //TODO
    });

    // it("New BSP can be created", async () => {
    //   const { containerName, rpcPort, p2pPort, peerId } = await addBspContainer();

    //   await it("is in a running container", async () => {
    //     const docker = new Docker();
    //     const {
    //       State: { Status }
    //     } = await docker.getContainer(containerName).inspect();
    //     strictEqual(Status, "running");
    //   });

    //   await it("can open new API connection with", async () => {
    //     const newApi = await createApiObject(`ws://127.0.0.1:${rpcPort}`);

    //     await it("has correct reported peerId", async () => {
    //       const localPeerId = await newApi.rpc.system.localPeerId();
    //       strictEqual(localPeerId.toString(), peerId);
    //     });

    //     await it("is synced with current block", async () => {
    //       const syncHeight = (await newApi.rpc.chain.getHeader()).number.toNumber();
    //       const currentHeight = (await api.rpc.chain.getHeader()).number.toNumber();
    //       strictEqual(syncHeight, currentHeight);
    //     });

    //     await it("is listening on the correct P2P port", async () => {
    //       const listenAddresses = (await newApi.rpc.system.localListenAddresses()).map((address) =>
    //         address.toString()
    //       );
    //       const matchingAddress = listenAddresses.filter((address) =>
    //         address.includes(`/tcp/${p2pPort}/p2p/`)
    //       );
    //       strictEqual(matchingAddress.length > 1, true);
    //     });

    //     await newApi.disconnect();
    //   });

    //   await it("is peer of other nodes", async () => {
    //     const peers = (await api.rpc.system.peers()).map(({ peerId }) => peerId.toString());
    //     strictEqual(peers.includes(peerId), true);
    //   });
    // });

    // it.only("Lots of BSPs can be created", async () => {
    //   await addBspContainer({ name: "timbo1", additionalArgs: ["--database=rocksdb"] });
    //   await addBspContainer({ name: "timbo2", additionalArgs: ["--database=paritydb"] });
    //   await addBspContainer({ name: "timbo3", additionalArgs: ["--database=auto"] });

    //   const docker = new Docker();
    //   const sh_nodes = (
    //     await docker.listContainers({
    //       filters: { ancestor: [DOCKER_IMAGE] }
    //     })
    //   ).flatMap(({ Names }) => Names);

    //   strictEqual(sh_nodes.length > 3, true);
    // });

    // it("Rotates the blockchain service keys (bcsv)", async () => {
    //   const alice_pub_key = "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d";
    //   const bob_pub_key = "0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48";
    //   const bcsv_key_type = "bcsv";
    //   const bob_seed = "//Bob";

    //   const has_alice_key = await api.rpc.author.hasKey(alice_pub_key, bcsv_key_type);
    //   strictEqual(has_alice_key.toHuman().valueOf(), true);

    //   let has_bob_key = await api.rpc.author.hasKey(bob_pub_key, bcsv_key_type);
    //   strictEqual(has_bob_key.toHuman().valueOf(), false);

    //   // Rotate keys and check that Bob's pub key is now in Keystore.
    //   await api.rpc.storagehubclient.rotateBcsvKeys(bob_seed);
    //   has_bob_key = await api.rpc.author.hasKey(bob_pub_key, bcsv_key_type);
    //   strictEqual(has_bob_key.toHuman().valueOf(), true);
    // });
  });
}
