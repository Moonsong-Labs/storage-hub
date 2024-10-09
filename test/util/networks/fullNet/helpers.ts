import "@storagehub/api-augment";
import { v2 as compose } from "docker-compose";
import path from "node:path";
import { bspKey, mspKey, shUser } from "../../pjsKeyring.ts";
import type { BspNetConfig } from "../bspNet/types";
import * as ShConsts from "../consts.ts";
import { ShTestApi, type EnrichedBspApi } from "../bspNet/test-api.ts";
import invariant from "tiny-invariant";
import * as fs from "node:fs";
import { parse, stringify } from "yaml";
import { forceSignupBsp, getContainerIp, getContainerPeerId } from "../helpers.ts";

export const runFullNet = async (bspNetConfig: BspNetConfig) => {
  let userApi: EnrichedBspApi | undefined;
  try {
    console.log(`SH user id: ${shUser.address}`);
    console.log(`SH BSP id: ${bspKey.address}`);
    console.log(`SH MSP id: ${mspKey.address}`);

    let file = "local-dev-full-compose.yml";
    if (bspNetConfig.rocksdb) {
      file = "local-dev-full-rocksdb-compose.yml";
    }

    const composeFilePath = path.resolve(process.cwd(), "..", "docker", file);
    const cwd = path.resolve(process.cwd(), "..", "docker");
    const composeFile = fs.readFileSync(composeFilePath, "utf8");
    const composeYaml = parse(composeFile);
    if (bspNetConfig.extrinsicRetryTimeout) {
      composeYaml.services["sh-bsp"].command.push(
        `--extrinsic-retry-timeout=${bspNetConfig.extrinsicRetryTimeout}`
      );
      composeYaml.services["sh-msp"].command.push(
        `--extrinsic-retry-timeout=${bspNetConfig.extrinsicRetryTimeout}`
      );
      composeYaml.services["sh-user"].command.push(
        `--extrinsic-retry-timeout=${bspNetConfig.extrinsicRetryTimeout}`
      );
    }

    const updatedCompose = stringify(composeYaml);

    if (bspNetConfig.noisy) {
      await compose.upOne("toxiproxy", {
        cwd: cwd,
        configAsString: updatedCompose,
        log: true
      });
    }

    await compose.upOne("sh-bsp", {
      cwd: cwd,
      configAsString: updatedCompose,
      log: true
    });

    const bspIp = await getContainerIp(
      bspNetConfig.noisy ? "toxiproxy" : ShConsts.NODE_INFOS.bsp.containerName
    );

    if (bspNetConfig.noisy) {
      console.log(`toxiproxy IP: ${bspIp}`);
    } else {
      console.log(`sh-bsp IP: ${bspIp}`);
    }

    const bspPeerId = await getContainerPeerId(
      `http://127.0.0.1:${ShConsts.NODE_INFOS.bsp.port}`,
      true
    );
    console.log(`sh-bsp Peer ID: ${bspPeerId}`);

    process.env.BSP_IP = bspIp;
    process.env.BSP_PEER_ID = bspPeerId;

    await compose.upOne("sh-msp", {
      cwd: cwd,
      configAsString: updatedCompose,
      log: true,
      env: {
        ...process.env,
        NODE_KEY: ShConsts.NODE_INFOS.msp.nodeKey,
        BSP_IP: bspIp,
        BSP_PEER_ID: bspPeerId,
        MSP_ID: ShConsts.DUMMY_MSP_ID
      }
    });

    const mspId = await getContainerIp(
      bspNetConfig.noisy ? "toxiproxy" : ShConsts.NODE_INFOS.msp.containerName
    );

    const mspPeerId = await getContainerPeerId(`http://127.0.0.1:${ShConsts.NODE_INFOS.msp.port}`);
    console.log(`sh-msp Peer ID: ${mspPeerId}`);

    const multiAddressMsp = `/ip4/${mspId}/tcp/30350/p2p/${mspPeerId}`;

    await compose.upOne("sh-user", {
      cwd: cwd,
      configAsString: updatedCompose,
      log: true,
      env: {
        ...process.env,
        BSP_IP: bspIp,
        BSP_PEER_ID: bspPeerId
      }
    });

    const peerIDUser = await getContainerPeerId(
      `http://127.0.0.1:${ShConsts.NODE_INFOS.user.port}`
    );
    console.log(`sh-user Peer ID: ${peerIDUser}`);

    const multiAddressBsp = `/ip4/${bspIp}/tcp/30350/p2p/${bspPeerId}`;

    // Create Connection API Object to User Node
    userApi = await ShTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.user.port}`);

    // Give Balances
    const amount = 10000n * 10n ** 12n;
    await userApi.sealBlock(
      userApi.tx.sudo.sudo(userApi.tx.balances.forceSetBalance(bspKey.address, amount))
    );
    await userApi.sealBlock(
      userApi.tx.sudo.sudo(userApi.tx.balances.forceSetBalance(mspKey.address, amount))
    );
    await userApi.sealBlock(
      userApi.tx.sudo.sudo(userApi.tx.balances.forceSetBalance(shUser.address, amount))
    );

    await userApi.sealBlock(userApi.tx.sudo.sudo(userApi.tx.fileSystem.setGlobalParameters(1, 1)));

    // Make BSP
    await forceSignupBsp({
      api: userApi,
      who: bspKey.address,
      multiaddress: multiAddressBsp,
      bspId: ShConsts.DUMMY_BSP_ID,
      capacity: bspNetConfig.capacity || ShConsts.CAPACITY_512,
      weight: bspNetConfig.bspStartingWeight
    });

    // Sign up MSP
    await userApi.sealBlock(
      userApi.tx.sudo.sudo(
        userApi.tx.providers.forceMspSignUp(
          mspKey.address,
          ShConsts.DUMMY_MSP_ID,
          bspNetConfig.capacity || ShConsts.CAPACITY_512,
          [multiAddressMsp],
          {
            identifier: ShConsts.VALUE_PROP,
            dataLimit: 500,
            protocols: ["https", "ssh", "telnet"]
          },
          mspKey.address
        )
      )
    );
  } catch (e) {
    console.error("Error ", e);
  } finally {
    userApi?.disconnect();
  }
};

export const runInitialisedFullNet = async (bspNetConfig: BspNetConfig) => {
  await runFullNet(bspNetConfig);

  let userApi: EnrichedBspApi | undefined;
  try {
    userApi = await ShTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.user.port}`);

    /**** CREATE BUCKET AND ISSUE STORAGE REQUEST ****/
    const source = "res/whatsup.jpg";
    const destination = "test/smile.jpg";
    const bucketName = "nothingmuch-1";

    const newBucketEventEvent = await userApi.createBucket(bucketName);
    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    invariant(newBucketEventDataBlob, "Event doesn't match Type");

    const { fingerprint, file_size, location } =
      await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        destination,
        ShConsts.NODE_INFOS.user.AddressId,
        newBucketEventDataBlob.bucketId
      );

    await userApi.sealBlock(
      userApi.tx.fileSystem.issueStorageRequest(
        newBucketEventDataBlob.bucketId,
        location,
        fingerprint,
        file_size,
        ShConsts.DUMMY_MSP_ID,
        [ShConsts.NODE_INFOS.user.expectedPeerId]
      ),
      shUser
    );

    await userApi.wait.bspVolunteer();
    await userApi.wait.bspStored();
  } catch (e) {
    console.error("Error ", e);
  } finally {
    userApi?.disconnect();
  }
};
