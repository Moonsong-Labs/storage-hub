import path from "node:path";
import fs from "node:fs";
import tmp from "tmp";
import * as compose from "docker-compose";
import yaml from "yaml";
import assert from "node:assert";
import {
  addBsp,
  BspNetTestApi,
  forceSignupBsp,
  getContainerIp,
  getContainerPeerId,
  ShConsts,
  type EnrichedBspApi,
  type FileMetadata,
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
  mspDownKey,
  mspKey,
  mspTwoKey,
  shUser
} from "../pjsKeyring";
import { MILLIUNIT, UNIT } from "../constants";
import { sleep } from "../timer";
import { spawn, spawnSync } from "node:child_process";
import { DUMMY_MSP_ID } from "../bspNet/consts";

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

  private loadComposeFile() {
    assert(this.type, "Network type has not been set yet");
    const composeFiles = {
      bspnet: "bspnet-base-template.yml",
      fullnet: "fullnet-base-template.yml"
    } as const;

    // if (this.config.noisy && this.type === "fullnet") {
    //   assert(false, "Noisy fullnet not supported");
    // }

    const file = this.type === "fullnet" ? composeFiles.fullnet : composeFiles.bspnet;

    assert(file, `Compose file not found for ${this.type} network`);

    const composeFilePath = path.resolve(process.cwd(), "..", "docker", file);
    const composeFile = fs.readFileSync(composeFilePath, "utf8");
    const composeYaml = yaml.parse(composeFile);

    this.composeYaml = composeYaml;
    return this;
  }

  public async getPeerId(serviceName: string) {
    assert(this.entities, "Entities have not been populated yet, run populateEntities() first");
    assert(
      Object.values(this.entities)
        .map(({ name }) => name)
        .includes(serviceName),
      `Service ${serviceName} not found in compose file`
    );

    const port = this.entities.find((entity) => entity.name === serviceName)?.port;
    assert(port, `Port for service ${serviceName} not found in compose file`);
    return getContainerPeerId(`http://127.0.0.1:${port}`);
  }

  private populateEntities() {
    assert(this.composeYaml, "Compose file has not been selected yet, run loadComposeFile() first");
    const shServices: ShEntity[] = Object.entries(this.composeYaml.services)
      .filter(([_serviceName, service]: [string, any]) => service.image === "storage-hub:local")
      .map(([serviceName, _service]: [string, any]) => ({
        port: this.getPort(serviceName),
        name: serviceName
      }));
    assert(shServices.length > 0, "No storage-hub services found in compose file");
    this.entities = shServices;
    return this;
  }

  // TODO: Turn this into a submodule system with separate handlers for each option
  private remapComposeYaml() {
    assert(this.composeYaml, "Compose file has not been selected yet, run loadComposeFile() first");

    const composeYaml = this.composeYaml;

    if (this.config.noisy) {
      for (const svcName of Object.keys(composeYaml.services)) {
        if (svcName === "toxiproxy") {
          continue;
        }
        composeYaml.services[`${svcName}`].ports = composeYaml.services[`${svcName}`].ports.filter(
          (portMapping: `${string}:${string}`) =>
            !portMapping
              .split(":")
              .some((port: string) => port.startsWith("30") && port.length === 5)
        );
        composeYaml.services[`${svcName}`].networks = {
          "storage-hub-network": { aliases: [`${svcName}`] }
        };
      }
    } else {
      // biome-ignore lint/performance/noDelete: to ensure compose file is valid
      delete composeYaml.services.toxiproxy;
    }

    if (this.config.extrinsicRetryTimeout) {
      composeYaml.services["sh-bsp"].command.push(
        `--extrinsic-retry-timeout=${this.config.extrinsicRetryTimeout}`
      );
      composeYaml.services["sh-user"].command.push(
        `--extrinsic-retry-timeout=${this.config.extrinsicRetryTimeout}`
      );
      if (this.type === "fullnet") {
        composeYaml.services["sh-msp-1"].command.push(
          `--extrinsic-retry-timeout=${this.config.extrinsicRetryTimeout}`
        );
      }
    }

    if (this.config.rocksdb) {
      composeYaml.services["sh-bsp"].command.push("--storage-layer=rocks-db");
      composeYaml.services["sh-bsp"].command.push(
        "--storage-path=/tmp/bsp/${BSP_IP:-default_bsp_ip}"
      );
      composeYaml.services["sh-user"].command.push("--storage-layer=rocks-db");
      composeYaml.services["sh-user"].command.push(
        "--storage-path=/tmp/bsp/${BSP_IP:-default_bsp_ip}"
      );
    }

    if (this.config.indexer) {
      composeYaml.services["sh-user"].command.push("--indexer");
      composeYaml.services["sh-user"].command.push(
        "--database-url=postgresql://postgres:postgres@docker-sh-postgres-1:5432/storage_hub"
      );
      if (this.config.userIndexerMode) {
        composeYaml.services["sh-user"].command.push(
          `--indexer-mode=${this.config.userIndexerMode}`
        );
      }
      if (this.type === "fullnet") {
        composeYaml.services["sh-msp-1"].command.push(
          "--database-url=postgresql://postgres:postgres@docker-sh-postgres-1:5432/storage_hub"
        );
        composeYaml.services["sh-msp-2"].command.push(
          "--database-url=postgresql://postgres:postgres@docker-sh-postgres-1:5432/storage_hub"
        );
      }
    }

    // Add fisherman node if enabled
    if (this.config.fisherman && this.type === "fullnet") {
      // Fisherman node is already in the docker-compose template, just need to enable it
      if (this.config.fishermanIndexerMode) {
        composeYaml.services["sh-fisherman"].command.push(
          `--indexer-mode=${this.config.fishermanIndexerMode}`
        );
      }
    }

    const cwd = path.resolve(process.cwd(), "..", "docker");
    const entries = Object.entries(composeYaml.services).map(([key, value]: any) => {
      let remappedValue: any;
      if ("volumes" in value) {
        remappedValue = {
          ...value,
          volumes: value.volumes.map((volume: any) => volume.replace("./", `${cwd}/`))
        };
      }
      return { node: key, spec: remappedValue ?? value };
    });

    const remappedYamlContents = entries.reduce(
      (acc, curr) => ({ ...acc, [curr.node]: curr.spec }),
      {}
    );

    let composeContents = {
      name: "docker",
      services: remappedYamlContents
    };

    if (this.config.noisy) {
      composeContents = Object.assign(composeContents, {
        networks: {
          "storage-hub-network": { driver: "bridge" }
        }
      });
    }

    const updatedCompose = yaml.stringify(composeContents, {
      collectionStyle: "flow",
      defaultStringType: "QUOTE_DOUBLE",
      doubleQuotedAsJSON: true,
      flowCollectionPadding: true
    });
    fs.mkdirSync(path.join(cwd, "tmp"), { recursive: true });
    const tmpFile = tmp.fileSync({ postfix: ".yml" });
    fs.writeFileSync(tmpFile.name, updatedCompose);
    return tmpFile.name;
  }

  private async startNetwork(verbose = false) {
    const cwd = path.resolve(process.cwd(), "..", "docker");
    const tmpFile = this.remapComposeYaml();

    if (this.config.noisy) {
      await compose.upOne("toxiproxy", {
        cwd: cwd,
        config: tmpFile,
        log: verbose
      });
    }

    await compose.upOne("sh-bsp", {
      cwd: cwd,
      config: tmpFile,
      log: verbose
    });

    const bspIp = await getContainerIp(
      this.config.noisy ? "toxiproxy" : ShConsts.NODE_INFOS.bsp.containerName
    );

    if (verbose && this.config.noisy) {
      console.log(`toxiproxy IP: ${bspIp}`);
    } else {
      console.log(`sh-bsp IP: ${bspIp}`);
    }

    const bspPeerId = await getContainerPeerId(
      `http://127.0.0.1:${ShConsts.NODE_INFOS.bsp.port}`,
      true
    );
    verbose && console.log(`sh-bsp Peer ID: ${bspPeerId}`);

    process.env.BSP_IP = bspIp;
    process.env.BSP_PEER_ID = bspPeerId;

    if (this.type === "fullnet") {
      const mspServices = Object.keys(this.composeYaml.services).filter((service) =>
        service.includes("sh-msp")
      );

      for (const mspService of mspServices) {
        const nodeKey =
          mspService === "sh-msp-1"
            ? ShConsts.NODE_INFOS.msp1.nodeKey
            : mspService === "sh-msp-2"
              ? ShConsts.NODE_INFOS.msp2.nodeKey
              : undefined;
        assert(
          nodeKey,
          `Service ${mspService} not msp-1/2, either add to hardcoded list or make this dynamic`
        );

        const mspId =
          mspService === "sh-msp-1"
            ? ShConsts.DUMMY_MSP_ID
            : mspService === "sh-msp-2"
              ? ShConsts.DUMMY_MSP_ID_2
              : undefined;
        assert(
          mspId,
          `Service ${mspService} not msp-1/2, either add to hardcoded list or make this dynamic`
        );

        await compose.upOne(mspService, {
          cwd: cwd,
          config: tmpFile,
          log: verbose,
          env: {
            ...process.env,
            NODE_KEY: nodeKey,
            BSP_IP: bspIp,
            BSP_PEER_ID: bspPeerId,
            MSP_ID: mspId
          }
        });
      }
    }

    if (this.config.indexer || this.config.fisherman) {
      await compose.upOne("sh-postgres", {
        cwd: cwd,
        config: tmpFile,
        log: verbose
      });

      await this.runMigrations();
    }

    // Start fisherman node if enabled
    if (this.config.fisherman && this.type === "fullnet") {
      await compose.upOne("sh-fisherman", {
        cwd: cwd,
        config: tmpFile,
        log: verbose,
        env: {
          ...process.env,
          BSP_IP: bspIp,
          BSP_PEER_ID: bspPeerId
        }
      });
    }

    await compose.upOne("sh-user", {
      cwd: cwd,
      config: tmpFile,
      log: verbose,
      env: {
        ...process.env,
        BSP_IP: bspIp,
        BSP_PEER_ID: bspPeerId
      }
    });

    return this;
  }

  public async stopNetwork() {
    const services = Object.keys(this.composeYaml.services);
    console.log(services);
  }

  private async runMigrations() {
    assert(
      this.config.indexer || this.config.fisherman,
      "Indexer or fisherman must be enabled to run migrations"
    );

    const dieselCheck = spawnSync("diesel", ["--version"], { stdio: "ignore" });
    assert(
      dieselCheck.status === 0,
      "Error running Diesel CLI. Visit https://diesel.rs/guides/getting-started for install instructions."
    );
    // TODO Poll this with a ping
    await sleep(2000);

    const cwd = path.resolve(process.cwd(), "..", "client", "indexer-db");

    const result = await new Promise((resolve, reject) => {
      const env = {
        ...process.env,
        DATABASE_URL: "postgresql://postgres:postgres@localhost:5432/storage_hub"
      };

      const diesel = spawn("diesel", ["migration", "run"], {
        cwd,
        env,
        stdio: "inherit"
      });

      diesel.on("close", (code) => {
        if (code === 0) {
          resolve(true);
        } else {
          reject(new Error(`Diesel migrations failed with code ${code}`));
        }
      });
    });

    return result;
  }

  private getPort(serviceName: string) {
    assert(this.composeYaml, "Compose file has not been selected yet, run loadComposeFile() first");
    const service = this.composeYaml.services[serviceName];
    assert(service, `Service ${serviceName} not found in compose file`);

    const ports = service.ports;
    assert(Array.isArray(ports), `Ports for service ${serviceName} is in unexpected format.`);

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
    const maxReplicationTargetRuntimeParameter = {
      RuntimeConfig: {
        MaxReplicationTarget: [null, 10]
      }
    };
    const tickRangeToMaximumThresholdRuntimeParameter = {
      RuntimeConfig: {
        TickRangeToMaximumThreshold: [null, 1]
      }
    };

    const signedCalls = [
      api.tx.sudo
        .sudo(api.tx.balances.forceSetBalance(bspKey.address, amount))
        .signAsync(alice, { nonce: 0 }),
      api.tx.sudo
        .sudo(api.tx.balances.forceSetBalance(shUser.address, amount))
        .signAsync(alice, { nonce: 1 }),
      api.tx.sudo
        .sudo(api.tx.balances.forceSetBalance(mspKey.address, amount))
        .signAsync(alice, { nonce: 2 }),
      api.tx.sudo
        .sudo(api.tx.balances.forceSetBalance(mspTwoKey.address, amount))
        .signAsync(alice, { nonce: 3 }),
      api.tx.sudo
        .sudo(api.tx.balances.forceSetBalance(mspDownKey.address, amount))
        .signAsync(alice, { nonce: 4 }),
      api.tx.sudo
        .sudo(api.tx.parameters.setParameter(maxReplicationTargetRuntimeParameter))
        .signAsync(alice, { nonce: 5 }),
      api.tx.sudo
        .sudo(api.tx.parameters.setParameter(tickRangeToMaximumThresholdRuntimeParameter))
        .signAsync(alice, { nonce: 6 })
    ];

    const sudoTxns = await Promise.all(signedCalls);

    return api.block.seal({ calls: sudoTxns });
  }

  public async setupMsp(api: EnrichedBspApi, who: string, multiAddressMsp: string, mspId?: string) {
    await api.block.seal({
      calls: [
        api.tx.sudo.sudo(
          api.tx.providers.forceMspSignUp(
            who,
            mspId ?? ShConsts.DUMMY_MSP_ID,
            this.config.capacity || ShConsts.CAPACITY_512,
            // The peer ID has to be different from the BSP's since the user now attempts to send files to MSPs when new storage requests arrive.
            [multiAddressMsp],
            // The MSP will charge 100 UNITS per GigaUnit of data per block.
            100 * 1024 * 1024,
            "Terms of Service...",
            9999999,
            who
          )
        )
      ]
    });
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
    //     SystemUtilisationLowerThresholdPercentage: [null, {VALUE_YOU_WANT}],
    //     SystemUtilisationUpperThresholdPercentage: [null, {VALUE_YOU_WANT}],
    //     MostlyStablePrice: [null, {VALUE_YOU_WANT}],
    //     MaxPrice: [null, {VALUE_YOU_WANT}],
    //     MinPrice: [null, {VALUE_YOU_WANT}],
    //     UpperExponentFactor: [null, {VALUE_YOU_WANT}],
    //     LowerExponentFactor: [null, {VALUE_YOU_WANT}],
    //     ZeroSizeBucketFixedRate: [null, {VALUE_YOU_WANT}],
    //     IdealUtilisationRate: [null, {VALUE_YOU_WANT}],
    //     DecayRate: [null, {VALUE_YOU_WANT}],
    //     MinimumTreasuryCut: [null, {VALUE_YOU_WANT}],
    //     MaximumTreasuryCut: [null, {VALUE_YOU_WANT}],
    //     BspStopStoringFilePenalty: [null, {VALUE_YOU_WANT}],
    //     ProviderTopUpTtl: [null, {VALUE_YOU_WANT}],
    //     BasicReplicationTarget: [null, {VALUE_YOU_WANT}],
    //     StandardReplicationTarget: [null, {VALUE_YOU_WANT}],
    //     HighSecurityReplicationTarget: [null, {VALUE_YOU_WANT}],
    //     SuperHighSecurityReplicationTarget: [null, {VALUE_YOU_WANT}],
    //     UltraHighSecurityReplicationTarget: [null, {VALUE_YOU_WANT}],
    //     MaxReplicationTarget: [null, {VALUE_YOU_WANT}],
    //     TickRangeToMaximumThreshold: [null, {VALUE_YOU_WANT}],
    //     StorageRequestTtl: [null, {VALUE_YOU_WANT}],
    //     MinWaitForStopStoring: [null, {VALUE_YOU_WANT}],
    //     MinSeedPeriod: [null, {VALUE_YOU_WANT}],
    //     StakeToSeedPeriod: [null, {VALUE_YOU_WANT}],
    //   }
    // }
    const slashAmountPerMaxFileSizeRuntimeParameter = {
      RuntimeConfig: {
        SlashAmountPerMaxFileSize: [null, 20n * MILLIUNIT]
      }
    };
    await api.block.seal({
      calls: [
        api.tx.sudo.sudo(api.tx.parameters.setParameter(slashAmountPerMaxFileSizeRuntimeParameter))
      ]
    });
    const stakeToChallengePeriodRuntimeParameter = {
      RuntimeConfig: {
        StakeToChallengePeriod: [null, 1000n * UNIT]
      }
    };
    await api.block.seal({
      calls: [
        api.tx.sudo.sudo(api.tx.parameters.setParameter(stakeToChallengePeriodRuntimeParameter))
      ]
    });
    const checkpointChallengePeriodRuntimeParameter = {
      RuntimeConfig: {
        CheckpointChallengePeriod: [null, 10]
      }
    };
    await api.block.seal({
      calls: [
        api.tx.sudo.sudo(api.tx.parameters.setParameter(checkpointChallengePeriodRuntimeParameter))
      ]
    });
    const minChallengePeriodRuntimeParameter = {
      RuntimeConfig: {
        MinChallengePeriod: [null, 5]
      }
    };
    await api.block.seal({
      calls: [api.tx.sudo.sudo(api.tx.parameters.setParameter(minChallengePeriodRuntimeParameter))]
    });
    const basicReplicationTargetRuntimeParameter = {
      RuntimeConfig: {
        BasicReplicationTarget: [null, 3]
      }
    };
    await api.block.seal({
      calls: [
        api.tx.sudo.sudo(api.tx.parameters.setParameter(basicReplicationTargetRuntimeParameter))
      ]
    });
    const maxReplicationTargetRuntimeParameter = {
      RuntimeConfig: {
        MaxReplicationTarget: [null, 9]
      }
    };
    await api.block.seal({
      calls: [
        api.tx.sudo.sudo(api.tx.parameters.setParameter(maxReplicationTargetRuntimeParameter))
      ]
    });
    const tickRangeToMaximumThresholdRuntimeParameter = {
      RuntimeConfig: {
        TickRangeToMaximumThreshold: [null, 10]
      }
    };
    await api.block.seal({
      calls: [
        api.tx.sudo.sudo(
          api.tx.parameters.setParameter(tickRangeToMaximumThresholdRuntimeParameter)
        )
      ]
    });
    const minWaitForStopStoringRuntimeParameter = {
      RuntimeConfig: {
        MinWaitForStopStoring: [null, 15]
      }
    };
    await api.block.seal({
      calls: [
        api.tx.sudo.sudo(api.tx.parameters.setParameter(minWaitForStopStoringRuntimeParameter))
      ]
    });
    const storageRequestTtlRuntimeParameter = {
      RuntimeConfig: {
        StorageRequestTtl: [null, 20]
      }
    };
    await api.block.seal({
      calls: [api.tx.sudo.sudo(api.tx.parameters.setParameter(storageRequestTtlRuntimeParameter))]
    });
  }

  public async execDemoTransfer() {
    await using api = await this.getApi("sh-user");

    const source = "res/whatsup.jpg";
    const destination = "test/smile.jpg";
    const bucketName = "nothingmuch-1";
    const fileMetadata = await api.file.createBucketAndSendNewStorageRequest(
      source,
      destination,
      bucketName,
      null,
      DUMMY_MSP_ID,
      shUser,
      1
    );

    if (this.type === "bspnet") {
      await api.wait.bspVolunteer();
      await api.wait.bspStored();
    }

    if (this.type === "fullnet") {
      // This will advance the block which also contains the BSP volunteer tx.
      // Hence why we can wait for the BSP to confirm storing.
      await api.wait.mspResponseInTxPool();
      await api.wait.bspVolunteerInTxPool();
      await api.block.seal();
      await api.wait.bspStored();
    }

    return { fileMetadata };
  }

  public async initExtraBsps() {
    await using api = await this.getApi("sh-user");

    const basicReplicationTargetRuntimeParameter = {
      RuntimeConfig: {
        BasicReplicationTarget: [null, 4]
      }
    };
    await api.block.seal({
      calls: [
        api.tx.sudo.sudo(api.tx.parameters.setParameter(basicReplicationTargetRuntimeParameter))
      ]
    });

    const tickToMaximumThresholdRuntimeParameter = {
      RuntimeConfig: {
        TickRangeToMaximumThreshold: [null, 1]
      }
    };
    await api.block.seal({
      calls: [
        api.tx.sudo.sudo(api.tx.parameters.setParameter(tickToMaximumThresholdRuntimeParameter))
      ]
    });

    // Add more BSPs to the network.
    // One BSP will be down, two more will be up.
    const { containerName: bspDownContainerName } = await addBsp(api, bspDownKey, {
      name: "sh-bsp-down",
      rocksdb: this.config.rocksdb,
      bspKeySeed: bspDownSeed,
      bspId: ShConsts.BSP_DOWN_ID,
      bspStartingWeight: this.config.capacity,
      extrinsicRetryTimeout: this.config.extrinsicRetryTimeout,
      additionalArgs: ["--keystore-path=/keystore/bsp-down"]
    });
    const { rpcPort: bspTwoRpcPort } = await addBsp(api, bspTwoKey, {
      name: "sh-bsp-two",
      rocksdb: this.config.rocksdb,
      bspKeySeed: bspTwoSeed,
      bspId: ShConsts.BSP_TWO_ID,
      bspStartingWeight: this.config.capacity,
      extrinsicRetryTimeout: this.config.extrinsicRetryTimeout,
      additionalArgs: ["--keystore-path=/keystore/bsp-two"]
    });
    const { rpcPort: bspThreeRpcPort } = await addBsp(api, bspThreeKey, {
      name: "sh-bsp-three",
      rocksdb: this.config.rocksdb,
      bspKeySeed: bspThreeSeed,
      bspId: ShConsts.BSP_THREE_ID,
      bspStartingWeight: this.config.capacity,
      extrinsicRetryTimeout: this.config.extrinsicRetryTimeout,
      additionalArgs: ["--keystore-path=/keystore/bsp-three"]
    });

    const source = "res/whatsup.jpg";
    const location = "test/smile.jpg";
    const bucketName = "nothingmuch-1";

    // Wait for a few seconds for all BSPs to be synced
    await sleep(5000);

    const fileMetadata = await api.file.createBucketAndSendNewStorageRequest(
      source,
      location,
      bucketName,
      null,
      null
    );
    await api.wait.bspVolunteer(4);
    await api.wait.bspStored({ expectedExts: 4 });

    // Stop BSP that is supposed to be down
    await api.docker.stopContainer(bspDownContainerName);

    // Attempt to debounce and stabilise
    await sleep(1500);

    return {
      bspTwoRpcPort,
      bspThreeRpcPort,
      fileMetadata: {
        fileKey: fileMetadata.fileKey,
        bucketId: fileMetadata.bucketId,
        location: location,
        owner: fileMetadata.owner,
        fingerprint: fileMetadata.fingerprint,
        fileSize: fileMetadata.fileSize
      }
    };
  }

  public static async create(
    type: NetworkType,
    config: NetLaunchConfig
  ): Promise<
    | { fileMetadata: FileMetadata }
    | { bspTwoRpcPort: number; bspThreeRpcPort: number; fileMetadata: FileMetadata }
    | undefined
  > {
    console.log("\n=== Launching network config ===");
    console.table({ config });
    const launchedNetwork = await new NetworkLauncher(type, config)
      .loadComposeFile()
      .populateEntities()
      .startNetwork();

    await using bspApi = await launchedNetwork.getApi("sh-bsp");

    // Wait for network to be in sync
    await bspApi.docker.waitForLog({
      containerName: "docker-sh-bsp-1",
      searchString: "💤 Idle",
      timeout: 15000
    });

    const userPeerId = await launchedNetwork.getPeerId("sh-user");
    console.log(`sh-user Peer ID: ${userPeerId}`);

    const bspContainerName = launchedNetwork.composeYaml.services["sh-bsp"].container_name;
    assert(bspContainerName, "BSP container name not found in compose file");
    const bspIp = await getContainerIp(
      launchedNetwork.config.noisy ? "toxiproxy" : bspContainerName
    );

    const bspPeerId = await launchedNetwork.getPeerId("sh-bsp");
    const multiAddressBsp = `/ip4/${bspIp}/tcp/30350/p2p/${bspPeerId}`;

    await using userApi = await launchedNetwork.getApi("sh-user");

    await userApi.docker.waitForLog({
      containerName: "docker-sh-user-1",
      searchString: "💤 Idle",
      timeout: 15000
    });

    await launchedNetwork.setupGlobal(userApi);
    await launchedNetwork.setupBsp(userApi, bspKey.address, multiAddressBsp);
    await launchedNetwork.setupRuntimeParams(userApi);
    await userApi.block.seal();

    if (launchedNetwork.type === "fullnet") {
      const mspServices = Object.keys(launchedNetwork.composeYaml.services).filter((service) =>
        service.includes("sh-msp")
      );
      for (const service of mspServices) {
        const mspContainerName = launchedNetwork.composeYaml.services[service].container_name;
        assert(mspContainerName, "MSP container name not found in compose file");
        const mspIp = await getContainerIp(mspContainerName);
        const mspPeerId = await launchedNetwork.getPeerId(service);
        const multiAddressMsp = `/ip4/${mspIp}/tcp/30350/p2p/${mspPeerId}`;

        // TODO: As we add more MSPs make this more dynamic
        const mspAddress =
          service === "sh-msp-1"
            ? mspKey.address
            : service === "sh-msp-2"
              ? mspTwoKey.address
              : undefined;
        assert(
          mspAddress,
          `Service ${service} not msp-1/2, either add to hardcoded list or make this dynamic`
        );

        const mspId =
          service === "sh-msp-1"
            ? ShConsts.DUMMY_MSP_ID
            : service === "sh-msp-2"
              ? ShConsts.DUMMY_MSP_ID_2
              : undefined;
        assert(
          mspId,
          `Service ${service} not msp-1/2, either add to hardcoded list or make this dynamic`
        );
        console.log(`Adding msp ${service} with address ${multiAddressMsp} and id ${mspId}`);
        await launchedNetwork.setupMsp(userApi, mspAddress, multiAddressMsp, mspId);
      }
    }

    if (launchedNetwork.type === "bspnet") {
      const mockMspMultiAddress = `/ip4/${bspIp}/tcp/30350/p2p/${ShConsts.DUMMY_MSP_PEER_ID}`;
      await launchedNetwork.setupMsp(userApi, mspKey.address, mockMspMultiAddress);
    }

    if (launchedNetwork.config.initialised === "multi") {
      return await launchedNetwork.initExtraBsps();
    }

    if (launchedNetwork.config.initialised === true) {
      return await launchedNetwork.execDemoTransfer();
    }

    // Attempt to debounce and stabilise
    await sleep(1500);
    return undefined;
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
   * Optional parameter to set whether to enable indexer service on the user node.
   * This will also launch the environment with an attached postgres db
   */
  indexer?: boolean;

  /**
   * Optional parameter to set the indexer mode for the user node when indexer is enabled.
   * 'full' - indexes all events (default)
   * 'lite' - indexes only essential events
   * 'fishing' - indexes only events relevant to fisherman monitoring (file tracking)
   */
  userIndexerMode?: "full" | "lite" | "fishing";

  /**
   * If true, runs a dedicated fisherman node with indexer service.
   */
  fisherman?: boolean;

  /**
   * Optional parameter to set the indexer mode for the fisherman node when fisherman is enabled.
   * 'full' - indexes all events (default)
   * 'lite' - indexes only essential events
   * 'fishing' - indexes only events relevant to fisherman monitoring (file tracking)
   */
  fishermanIndexerMode?: "full" | "lite" | "fishing";

  /**
   * Optional parameter to define what toxics to apply to the network.
   * Only applies when `noisy` is set to true.
   */
  toxics?: ToxicInfo[];
};
