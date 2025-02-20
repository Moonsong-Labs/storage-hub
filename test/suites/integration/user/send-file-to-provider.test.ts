import {
  describeMspNet,
  shUser,
  type EnrichedBspApi,
  getContainerIp,
  addMspContainer,
  mspThreeKey,
  createBucket
} from "../../../util";
import { CAPACITY, MAX_STORAGE_CAPACITY } from "../../../util/bspNet/consts.ts";

describeMspNet("User: Send file to provider", ({ before, createUserApi, it }) => {
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

    await userApi.rpc.storagehubclient.loadFileInStorage(
      source,
      destination,
      userApi.shConsts.NODE_INFOS.user.AddressId,
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
      searchString: "Failed to send file",
      containerName: userApi.shConsts.NODE_INFOS.user.containerName
    });
  });

  it("MSP first libp2p multiaddress is wrong and second should be correct. User will be able to connect and send file on the second attempt.", async () => {
    const { containerName, p2pPort, peerId } = await addMspContainer({
      name: "lola1",
      additionalArgs: [
        "--database=rocksdb",
        `--max-storage-capacity=${MAX_STORAGE_CAPACITY}`,
        `--jump-capacity=${CAPACITY[1024]}`,
        "--msp-charging-period=12"
      ]
    });

    //Give it some balance.
    const amount = 10000n * 10n ** 12n;
    await userApi.block.seal({
      calls: [
        userApi.tx.sudo.sudo(userApi.tx.balances.forceSetBalance(mspThreeKey.address, amount))
      ]
    });

    const mspIp = await getContainerIp(containerName);
    const multiAddressMsp = `/ip4/${mspIp}/tcp/${p2pPort}/p2p/${peerId}`;
    await userApi.block.seal({
      calls: [
        userApi.tx.sudo.sudo(
          userApi.tx.providers.forceMspSignUp(
            mspThreeKey.address,
            mspThreeKey.publicKey,
            userApi.shConsts.CAPACITY_512,
            [`/ip4/51.75.30.194/tcp/30350/p2p/${peerId}`, multiAddressMsp],
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

    await userApi.rpc.storagehubclient.loadFileInStorage(
      source,
      destination,
      userApi.shConsts.NODE_INFOS.user.AddressId,
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

    // Fail to connect to the first libp2p address because it is a phony one.
    await userApi.docker.waitForLog({
      searchString: "Failed to upload batch to peer",
      containerName: userApi.shConsts.NODE_INFOS.user.containerName
    });

    // Second libp2p address is the right one so we should successfully send the file through this one.
    await userApi.docker.waitForLog({
      searchString: "File upload complete.",
      containerName: userApi.shConsts.NODE_INFOS.user.containerName
    });
  });
});
