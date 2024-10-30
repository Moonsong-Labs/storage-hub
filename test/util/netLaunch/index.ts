import path from "node:path";
import fs from "node:fs";
import * as compose from "docker-compose";
import { parse, stringify } from "yaml";
import invariant from "tiny-invariant";
import {
  addBsp,
  BspNetTestApi,
  forceSignupBsp,
  getContainerIp,
  getContainerPeerId,
  ShConsts,
  type EnrichedBspApi,
  type ToxicInfo
} from "../bspNet";
import {
  alice,
  bspDownKey,
  bspDownSeed,
  bspKey,
  bspThreeKey,
  bspThreeSeed,
  bspTwoKey,
  bspTwoSeed,
  mspKey,
  shUser
} from "../pjsKeyring";
import { MILLIUNIT, UNIT } from "../constants";

export type ShEntity = {
  port: number;
  name: string;
};

export class NetworkLauncher {
  private composeYaml?: any;
  private entities?: ShEntity[];

  constructor(
    private readonly type: NetworkType,
    private readonly config: NetLaunchConfig
  ) {}

  private selectComposeFile() {
    invariant(this.type, "Network type has not been set yet");

    // TODO: Add noisy fullnet
    const composeFiles = {
      bspnet: "local-dev-bsp-compose.yml",
      fullnet: "local-dev-full-rocksdb-compose.yml",
      rocksdb: "local-dev-bsp-rocksdb-compose.yml",
      noisy: "noisy-bsp-compose.yml"
    } as const;

    if (this.config.noisy && this.type === "fullnet") {
      invariant(false, "Noisy fullnet not supported");
    }

    const file = this.config.noisy
      ? composeFiles.noisy
      : this.config.rocksdb && this.type === "bspnet"
        ? composeFiles.rocksdb
        : composeFiles[this.type];

    invariant(file, "Compose file not found for network type");

    const composeFilePath = path.resolve(process.cwd(), "..", "docker", file);
    const composeFile = fs.readFileSync(composeFilePath, "utf8");
    const composeYaml = parse(composeFile);
    if (this.config.extrinsicRetryTimeout) {
      composeYaml.services["sh-bsp"].command.push(
        `--extrinsic-retry-timeout=${this.config.extrinsicRetryTimeout}`
      );
      composeYaml.services["sh-user"].command.push(
        `--extrinsic-retry-timeout=${this.config.extrinsicRetryTimeout}`
      );
      if (this.type === "fullnet") {
        composeYaml.services["sh-msp"].command.push(
          `--extrinsic-retry-timeout=${this.config.extrinsicRetryTimeout}`
        );
      }
    }
    this.composeYaml = composeYaml;
    return this;
  }

  public async getPeerId(serviceName: string) {
    invariant(this.entities, "Entities have not been populated yet, run populateEntities() first");
    invariant(
      Object.values(this.entities)
        .map(({ name }) => name)
        .includes(serviceName),
      `Service ${serviceName} not found in compose file`
    );

    const port = this.entities.find((entity) => entity.name === serviceName)?.port;
    invariant(port, `Port for service ${serviceName} not found in compose file`);
    return getContainerPeerId(`http://127.0.0.1:${port}`);
  }

  private populateEntities() {
    invariant(
      this.composeYaml,
      "Compose file has not been selected yet, run selectComposeFile() first"
    );
    const sHservices: ShEntity[] = Object.entries(this.composeYaml.services)
      .filter(([_serviceName, service]: [string, any]) => service.image === "storage-hub:local")
      .map(([serviceName, _service]: [string, any]) => ({
        port: this.getPort(serviceName),
        name: serviceName
      }));
    invariant(sHservices.length > 0, "No storage-hub services found in compose file");
    this.entities = sHservices;
    return this;
  }

  private async startNetwork() {
    invariant(
      this.composeYaml,
      "Compose file has not been selected yet, run selectComposeFile() first"
    );
    const cwd = path.resolve(process.cwd(), "..", "docker");
    const composeContents = stringify(this.composeYaml);
    if (this.config.noisy) {
      await compose.upOne("toxiproxy", {
        cwd: cwd,
        configAsString: composeContents,
        log: true
      });
    }

    await compose.upOne("sh-bsp", { cwd: cwd, configAsString: composeContents, log: true });

    const bspIp = await getContainerIp(
      this.config.noisy ? "toxiproxy" : ShConsts.NODE_INFOS.bsp.containerName
    );

    if (this.config.noisy) {
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

    if (this.type === "fullnet") {
      await compose.upOne("sh-msp", {
        cwd: cwd,
        configAsString: composeContents,
        log: true,
        env: {
          ...process.env,
          NODE_KEY: ShConsts.NODE_INFOS.msp.nodeKey,
          BSP_IP: bspIp,
          BSP_PEER_ID: bspPeerId
        }
      });
    }

    await compose.upOne("sh-user", {
      cwd: cwd,
      configAsString: composeContents,
      log: true,
      env: {
        ...process.env,
        BSP_IP: bspIp,
        BSP_PEER_ID: bspPeerId
      }
    });

    return this;
  }

  private getPort(serviceName: string) {
    invariant(
      this.composeYaml,
      "Compose file has not been selected yet, run selectComposeFile() first"
    );
    const service = this.composeYaml.services[serviceName];
    invariant(service, `Service ${serviceName} not found in compose file`);

    const ports = service.ports;
    invariant(Array.isArray(ports), `Ports for service ${serviceName} is in unexpected format.`);

    for (const portMapping of ports) {
      const [external, internal] = portMapping.split(":");
      if (internal === "9944") {
        return Number.parseInt(external, 10);
      }
    }

    throw new Error(`No port mapping to 9944 found for service ${serviceName}`);
  }

  public async getApi(serviceName = "sh-user") {
    return BspNetTestApi.create(`ws://127.0.0.1:${await this.getPort(serviceName)}`);
  }

  public async setupBsp(api: EnrichedBspApi, who: string, multiaddress: string, bspId?: string) {
    await forceSignupBsp({
      api: api,
      who,
      multiaddress,
      bspId: bspId ?? ShConsts.DUMMY_BSP_ID,
      capacity: this.config.capacity ?? ShConsts.CAPACITY_512,
      weight: this.config.bspStartingWeight
    });
    return this;
  }

  public async setupGlobal(api: EnrichedBspApi) {
    const amount = 10000n * 10n ** 12n;
    const signedCalls = [
      api.tx.sudo
        .sudo(api.tx.balances.forceSetBalance(bspKey.address, amount))
        .signAsync(alice, { nonce: 0 }),
      api.tx.sudo
        .sudo(api.tx.balances.forceSetBalance(shUser.address, amount))
        .signAsync(alice, { nonce: 1 }),
      api.tx.sudo.sudo(api.tx.fileSystem.setGlobalParameters(1, 1)).signAsync(alice, { nonce: 2 })
    ];

    if (this.type === "fullnet") {
      signedCalls.push(
        api.tx.sudo
          .sudo(api.tx.balances.forceSetBalance(mspKey.address, amount))
          .signAsync(alice, { nonce: 3 })
      );
    }

    const sudoTxns = await Promise.all(signedCalls);

    return api.sealBlock(sudoTxns);
  }

  public async setupMsp(api: EnrichedBspApi, who: string, multiAddressMsp: string) {
    await api.sealBlock(
      api.tx.sudo.sudo(
        api.tx.providers.forceMspSignUp(
          who,
          ShConsts.DUMMY_MSP_ID,
          this.config.capacity || ShConsts.CAPACITY_512,
          // The peer ID has to be different from the BSP's since the user now attempts to send files to MSPs when new storage requests arrive.
          [multiAddressMsp],
          1,
          "Terms of Service...",
          500,
          who
        )
      )
    );
    return this;
  }

  public async setupRuntimeParams(api: EnrichedBspApi) {
    // Adjusting runtime parameters...
    // The `set_parameter` extrinsic receives an object like this:
    // {
    //   RuntimeConfig: Enum {
    //     SlashAmountPerMaxFileSize: [null, {VALUE_YOU_WANT}],
    //     StakeToChallengePeriod: [null, {VALUE_YOU_WANT}],
    //     CheckpointChallengePeriod: [null, {VALUE_YOU_WANT}],
    //     MinChallengePeriod: [null, {VALUE_YOU_WANT}],
    //   }
    // }
    const slashAmountPerMaxFileSizeRuntimeParameter = {
      RuntimeConfig: {
        SlashAmountPerMaxFileSize: [null, 20n * MILLIUNIT]
      }
    };
    await api.sealBlock(
      api.tx.sudo.sudo(api.tx.parameters.setParameter(slashAmountPerMaxFileSizeRuntimeParameter))
    );
    const stakeToChallengePeriodRuntimeParameter = {
      RuntimeConfig: {
        StakeToChallengePeriod: [null, 1000n * UNIT]
      }
    };
    await api.sealBlock(
      api.tx.sudo.sudo(api.tx.parameters.setParameter(stakeToChallengePeriodRuntimeParameter))
    );
    const checkpointChallengePeriodRuntimeParameter = {
      RuntimeConfig: {
        CheckpointChallengePeriod: [null, 10]
      }
    };
    await api.sealBlock(
      api.tx.sudo.sudo(api.tx.parameters.setParameter(checkpointChallengePeriodRuntimeParameter))
    );
    const minChallengePeriodRuntimeParameter = {
      RuntimeConfig: {
        MinChallengePeriod: [null, 5]
      }
    };
    await api.sealBlock(
      api.tx.sudo.sudo(api.tx.parameters.setParameter(minChallengePeriodRuntimeParameter))
    );
  }

  public async execDemoTransfer() {
    await using api = await this.getApi("sh-user");

    const source = "res/whatsup.jpg";
    const destination = "test/smile.jpg";
    const bucketName = "nothingmuch-1";

    await api.file.newStorageRequest(source, destination, bucketName);
    await api.wait.bspVolunteer();
    await api.wait.bspStored();

    return this;
  }

  public async initExtraBsps() {
    await using api = await this.getApi("sh-user");

    await api.sealBlock(api.tx.sudo.sudo(api.tx.fileSystem.setGlobalParameters(5, 1)));

    // Add more BSPs to the network.
    // One BSP will be down, two more will be up.
    const { containerName: bspDownContainerName } = await addBsp(api, bspDownKey, {
      name: "sh-bsp-down",
      rocksdb: this.config.rocksdb,
      bspKeySeed: bspDownSeed,
      bspId: ShConsts.BSP_DOWN_ID,
      bspStartingWeight: this.config.capacity,
      additionalArgs: ["--keystore-path=/keystore/bsp-down"]
    });
    const { rpcPort: bspTwoRpcPort } = await addBsp(api, bspTwoKey, {
      name: "sh-bsp-two",
      rocksdb: this.config.rocksdb,
      bspKeySeed: bspTwoSeed,
      bspId: ShConsts.BSP_TWO_ID,
      bspStartingWeight: this.config.capacity,
      additionalArgs: ["--keystore-path=/keystore/bsp-two"]
    });
    const { rpcPort: bspThreeRpcPort } = await addBsp(api, bspThreeKey, {
      name: "sh-bsp-three",
      rocksdb: this.config.rocksdb,
      bspKeySeed: bspThreeSeed,
      bspId: ShConsts.BSP_THREE_ID,
      bspStartingWeight: this.config.capacity,
      additionalArgs: ["--keystore-path=/keystore/bsp-three"]
    });

    const source = "res/whatsup.jpg";
    const location = "test/smile.jpg";
    const bucketName = "nothingmuch-1";

    const fileMetadata = await api.file.newStorageRequest(source, location, bucketName);
    await api.wait.bspVolunteer();
    await api.wait.bspStored();

    await api.docker.stopBspContainer(bspDownContainerName);

    return {
      bspTwoRpcPort,
      bspThreeRpcPort,
      fileData: {
        fileKey: fileMetadata.fileKey,
        bucketId: fileMetadata.bucketId,
        location: location,
        owner: fileMetadata.owner,
        fingerprint: fileMetadata.fingerprint,
        fileSize: fileMetadata.fileSize
      }
    };
  }

  public static async create(type: NetworkType, config: NetLaunchConfig) {
    console.log(
      `Launching network config ${config.noisy ? "with" : "without"} noise and ${config.rocksdb ? "with" : "without"} RocksDB for ${type} network`
    );
    const launchedNetwork = await new NetworkLauncher(type, config)
      .selectComposeFile()
      .populateEntities()
      .startNetwork();

    const peerIDUser = await launchedNetwork.getPeerId("sh-user");
    console.log(`sh-user Peer ID: ${peerIDUser}`);

    const bspContainerName = launchedNetwork.composeYaml.services["sh-bsp"].container_name;
    invariant(bspContainerName, "BSP container name not found in compose file");
    const bspIp = await getContainerIp(
      launchedNetwork.config.noisy ? "toxiproxy" : bspContainerName
    );

    const bspPeerId = await launchedNetwork.getPeerId("sh-bsp");
    const multiAddressBsp = `/ip4/${bspIp}/tcp/30350/p2p/${bspPeerId}`;

    await using userApi = await launchedNetwork.getApi("sh-user");

    await launchedNetwork.setupGlobal(userApi);
    await launchedNetwork.setupBsp(userApi, bspKey.address, multiAddressBsp);
    await launchedNetwork.setupRuntimeParams(userApi);

    if (launchedNetwork.type === "fullnet") {
      const mspContainerName = launchedNetwork.composeYaml.services["sh-msp"].container_name;
      invariant(mspContainerName, "MSP container name not found in compose file");
      const mspId = await getContainerIp(mspContainerName);
      const mspPeerId = await launchedNetwork.getPeerId("sh-msp");
      const multiAddressMsp = `/ip4/${mspId}/tcp/30350/p2p/${mspPeerId}`;
      await launchedNetwork.setupMsp(userApi, mspKey.address, multiAddressMsp);
    } else {
      await launchedNetwork.setupMsp(userApi, mspKey.address, multiAddressBsp);
    }

    if (launchedNetwork.config.initialised === "multi") {
      return await launchedNetwork.initExtraBsps();
    }

    if (launchedNetwork.config.initialised === true) {
      await launchedNetwork.execDemoTransfer();
    }
  }
}

export type NetworkType = "bspnet" | "fullnet";

/**
 * Configuration options for the BSP network.
 * These settings determine the behavior and characteristics of the network during tests.
 */
export type NetLaunchConfig = {
  /**
   * Optional parameter to set the network to be initialised with a pre-existing state.
   */
  initialised?: boolean | "multi";

  /**
   * If true, simulates a noisy network environment with added latency and bandwidth limitations.
   * Useful for testing network resilience and performance under suboptimal conditions.
   */
  noisy: boolean;

  /**
   * If true, uses RocksDB as the storage backend instead of the default in-memory database.
   */
  rocksdb: boolean;

  /**
   * Optional parameter to set the storage capacity of the BSP.
   * Measured in bytes.
   */
  capacity?: bigint;

  /**
   * Optional parameter to set the timeout interval for submit extrinsic retries.
   */
  extrinsicRetryTimeout?: number;

  /**
   * Optional parameter to set the weight of the BSP.
   * Measured in bytes.
   */
  bspStartingWeight?: bigint;

  /**
   * Optional parameter to define what toxics to apply to the network.
   * Only applies when `noisy` is set to true.
   */
  toxics?: ToxicInfo[];
};
