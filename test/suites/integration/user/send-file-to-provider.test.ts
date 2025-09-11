import { u8aToHex } from "@polkadot/util";
import { decodeAddress } from "@polkadot/util-crypto";
import {
  addMspContainer,
  createBucket,
  describeMspNet,
  type EnrichedBspApi,
  getContainerIp,
  mspThreeKey,
  shUser
} from "../../../util";
import { CAPACITY, MAX_STORAGE_CAPACITY } from "../../../util/bspNet/consts.ts";

await describeMspNet("User: Send file to provider", ({ before, createUserApi, it }) => {
  let userApi: EnrichedBspApi;

  before(async () => {
    userApi = await createUserApi();
  });

  it("MSP is down and user should show error logs", async () => {
    const source = "res/smile.jpg";
    const destination = "test/smile.jpg";
    const bucketName = "enron";

    const newBucketEventEvent = await userApi.createBucket(bucketName);
    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    const ownerHex1 = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2);
    await userApi.rpc.storagehubclient.loadFileInStorage(
      source,
      destination,
      ownerHex1,
      newBucketEventDataBlob.bucketId
    );

    await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.msp1.containerName);

    await userApi.block.seal({
      calls: [
        userApi.tx.fileSystem.issueStorageRequest(
          newBucketEventDataBlob.bucketId,
          destination,
          userApi.shConsts.TEST_ARTEFACTS["res/smile.jpg"].fingerprint,
          userApi.shConsts.TEST_ARTEFACTS["res/smile.jpg"].size,
          userApi.shConsts.DUMMY_MSP_ID,
          [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
          {
            Basic: null
          }
        )
      ],
      signer: shUser
    });

    await userApi.assert.eventPresent("fileSystem", "NewStorageRequest");

    await userApi.docker.waitForLog({
      searchString: "Unable to upload final batch to peer",
      containerName: userApi.shConsts.NODE_INFOS.user.containerName
    });

    // Resume the MSP container, otherwise the test won't exit correctly as it won't be able to connect to the MSP to clean it up.
    await userApi.docker.resumeContainer({
      containerName: userApi.shConsts.NODE_INFOS.msp1.containerName
    });
  });

  it("MSP first libp2p multiaddress is wrong and second should be correct. User will be able to connect and send file on the second attempt.", async () => {
    const { containerName, p2pPort, peerId } = await addMspContainer({
      name: "storage-hub-sh-msp-3",
      additionalArgs: [
        "--keystore-path=/keystore/msp-three",
        "--database=rocksdb",
        `--max-storage-capacity=${MAX_STORAGE_CAPACITY}`,
        `--jump-capacity=${CAPACITY[1024]}`,
        "--msp-charging-period=12"
      ]
    });

    // Wait until the MSP is up and running.
    await userApi.docker.waitForLog({
      searchString: "Idle",
      containerName: containerName,
      timeout: 30000
    });

    // Give it some balance.
    const amount = 10000n * 10n ** 12n;
    await userApi.block.seal({
      calls: [
        userApi.tx.sudo.sudo(userApi.tx.balances.forceSetBalance(mspThreeKey.address, amount))
      ]
    });

    const mspIp = await getContainerIp(containerName);
    const badMultiAddress = `/ip4/51.75.30.194/tcp/30350/p2p/${peerId}`;
    const multiAddressMsp = `/ip4/${mspIp}/tcp/${p2pPort}/p2p/${peerId}`;
    await userApi.block.seal({
      calls: [
        userApi.tx.sudo.sudo(
          userApi.tx.providers.forceMspSignUp(
            mspThreeKey.address,
            mspThreeKey.publicKey,
            userApi.shConsts.CAPACITY_512,
            [badMultiAddress, multiAddressMsp],
            100 * 1024 * 1024,
            "Terms of Service...",
            9999999,
            mspThreeKey.address
          )
        )
      ]
    });

    const source = "res/smile.jpg";
    const destination = "test/smile.jpg";
    const bucketName = "theranos";

    const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
      mspThreeKey.publicKey
    );

    const localValuePropId = valueProps[0].id;
    const newBucketEventEvent = await createBucket(
      userApi,
      bucketName, // Bucket name
      localValuePropId, // Value proposition ID from MSP
      "0xc0647914b37034d861ddc3f0750ded6defec0823de5c782f3ca7c64ba29a4a2e", // We got with cyberchef
      shUser // Owner (the user)
    );
    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    const ownerHex2 = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2);
    await userApi.rpc.storagehubclient.loadFileInStorage(
      source,
      destination,
      ownerHex2,
      newBucketEventDataBlob.bucketId
    );

    await userApi.block.seal({
      calls: [
        userApi.tx.fileSystem.issueStorageRequest(
          newBucketEventDataBlob.bucketId,
          destination,
          userApi.shConsts.TEST_ARTEFACTS["res/smile.jpg"].fingerprint,
          userApi.shConsts.TEST_ARTEFACTS["res/smile.jpg"].size,
          mspThreeKey.publicKey,
          [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
          {
            Basic: null
          }
        )
      ],
      signer: shUser
    });

    await userApi.assert.eventPresent("fileSystem", "NewStorageRequest");

    // It should have failed to connect to the first libp2p address because it is a phony one, but
    // the second libp2p address is the right one so we should successfully send the file through it.
    await userApi.docker.waitForLog({
      searchString: `File upload complete. Peer PeerId("${peerId}")`,
      containerName: userApi.shConsts.NODE_INFOS.user.containerName,
      timeout: 60000
    });
  });
});
