import assert from "node:assert";
import { execSync, spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import * as compose from "docker-compose";
import tmp from "tmp";
import yaml from "yaml";
import { JWT_SECRET } from "../backend/consts";
import {
  addBsp,
  BspNetTestApi,
  type EnrichedBspApi,
  type FileMetadata,
  forceSignupBsp,
  getContainerIp,
  getContainerPeerId,
  ShConsts,
  type ToxicInfo,
  waitFor
} from "../bspNet";
import { DUMMY_MSP_ID } from "../bspNet/consts";
import { MILLIUNIT, UNIT } from "../constants";
import { sleep } from "../timer";

export type ShEntity = {
  port: number;
  name: string;
};

export class NetworkLauncher {
  private composeYaml?: any;
  private entities?: ShEntity[];
  private tempFiles: string[] = [];

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

  private async waitForPendingDbReady(serviceName: string, cwd: string, tmpFile: string) {
    // Wait for the Postgres service to be ready by scanning logs for readiness message
    const deadline = Date.now() + 30_000;
    const readyMsg = "database system is ready to accept connections";
    while (Date.now() < deadline) {
      try {
        const logs = await compose.logs(serviceName, {
          cwd,
          config: tmpFile,
          log: false,
          follow: false
        });
        const stdout = typeof logs === "string" ? logs : (logs.out || "") + (logs.err || "");
        if (stdout.includes(readyMsg)) {
          return;
        }
      } catch (_) {
        // ignore transient compose log errors
      }
      await new Promise((r) => setTimeout(r, 500));
    }
    throw new Error(`Timed out waiting for ${serviceName} to be ready`);
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
      delete composeYaml.services.toxiproxy;
    }

    // If runtime is "parachain" there is no need to specify the runtime type, it's the default
    if (this.config.runtimeType === "solochain") {
      // Add the runtime type to the command for user and BSP nodes
      composeYaml.services["sh-bsp"].command.push("--chain=solochain-evm-dev");
      composeYaml.services["sh-user"].command.push("--chain=solochain-evm-dev");

      // Add the runtime type to the command for MSP nodes if we're running fullnet
      if (this.type === "fullnet") {
        composeYaml.services["sh-msp-1"].command.push("--chain=solochain-evm-dev");
        composeYaml.services["sh-msp-2"].command.push("--chain=solochain-evm-dev");
      }

      // Add the runtime type to the command for fisherman if we're running fullnet
      // or simply fisherman is enabled
      if (this.config.fisherman && this.type === "fullnet") {
        composeYaml.services["sh-fisherman"].command.push("--chain=solochain-evm-dev");
      }

      // Add the runtime type to the command for standalone indexer if enabled
      if (this.config.indexer && this.config.standaloneIndexer && this.type === "fullnet") {
        composeYaml.services["sh-indexer"].command.push("--chain=solochain-evm-dev");
      }
    }

    // Configure fisherman service
    if (!this.config.fisherman || this.type !== "fullnet") {
      if (composeYaml.services["sh-fisherman"]) {
        delete composeYaml.services["sh-fisherman"];
      }
    } else {
      // Add fisherman incomplete sync parameters if specified
      if (this.config.fishermanIncompleteSyncMax !== undefined) {
        composeYaml.services["sh-fisherman"].command.push(
          `--fisherman-incomplete-sync-max=${this.config.fishermanIncompleteSyncMax}`
        );
      }
      if (this.config.fishermanIncompleteSyncPageSize !== undefined) {
        composeYaml.services["sh-fisherman"].command.push(
          `--fisherman-incomplete-sync-page-size=${this.config.fishermanIncompleteSyncPageSize}`
        );
      }
    }

    // Remove standalone indexer service if not enabled or not using standalone mode
    if (!this.config.indexer || !this.config.standaloneIndexer || this.type !== "fullnet") {
      if (composeYaml.services["sh-indexer"]) {
        delete composeYaml.services["sh-indexer"];
      }
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

    // Configure log level for all services using -l flag
    if (this.config.logLevel) {
      const logFlag = `-l${this.config.logLevel}`;
      composeYaml.services["sh-bsp"].command.push(logFlag);
      composeYaml.services["sh-user"].command.push(logFlag);

      if (this.type === "fullnet") {
        composeYaml.services["sh-msp-1"].command.push(logFlag);
        composeYaml.services["sh-msp-2"].command.push(logFlag);
      }
      if (this.config.fisherman && this.type === "fullnet") {
        composeYaml.services["sh-fisherman"].command.push(logFlag);
      }
      if (this.config.indexer && this.config.standaloneIndexer && this.type === "fullnet") {
        composeYaml.services["sh-indexer"].command.push(logFlag);
      }
    }

    if (this.config.rocksdb) {
      composeYaml.services["sh-bsp"].command.push("--storage-layer=rocks-db");
      composeYaml.services["sh-bsp"].command.push(
        // biome-ignore lint/suspicious/noTemplateCurlyInString: It's for the yaml file that takes this syntax
        "--storage-path=/tmp/bsp/${BSP_IP:-default_bsp_ip}"
      );
      composeYaml.services["sh-user"].command.push("--storage-layer=rocks-db");
      composeYaml.services["sh-user"].command.push(
        // biome-ignore lint/suspicious/noTemplateCurlyInString: It's for the yaml file that takes this syntax
        "--storage-path=/tmp/bsp/${BSP_IP:-default_bsp_ip}"
      );
      if (this.type === "fullnet") {
        composeYaml.services["sh-msp-1"].command.push("--storage-layer=rocks-db");
        composeYaml.services["sh-msp-1"].command.push(
          // biome-ignore lint/suspicious/noTemplateCurlyInString: It's for the yaml file that takes this syntax
          "--storage-path=/tmp/msp/${MSP_IP:-default_msp_ip}"
        );
        composeYaml.services["sh-msp-2"].command.push("--storage-layer=rocks-db");
        composeYaml.services["sh-msp-2"].command.push(
          // biome-ignore lint/suspicious/noTemplateCurlyInString: It's for the yaml file that takes this syntax
          "--storage-path=/tmp/msp/${MSP_IP:-default_msp_ip}"
        );
      }
    }

    // Configure database access for MSPs when indexer is enabled
    // Only add --msp-database-url when indexer is enabled (MSPs need database for move bucket operations)
    if (this.config.indexer) {
      // Only add for MSPs if they exist in the compose file
      if (composeYaml.services["sh-msp-1"]) {
        composeYaml.services["sh-msp-1"].command.push(
          `--msp-database-url=postgresql://postgres:postgres@${ShConsts.NODE_INFOS.indexerDb.containerName}:5432/storage_hub`
        );
      }
      if (composeYaml.services["sh-msp-2"]) {
        composeYaml.services["sh-msp-2"].command.push(
          `--msp-database-url=postgresql://postgres:postgres@${ShConsts.NODE_INFOS.indexerDb.containerName}:5432/storage_hub`
        );
      }
    }

    if (this.config.indexer) {
      // If using standalone indexer, configure the sh-indexer service
      if (this.config.standaloneIndexer && this.type === "fullnet") {
        // Add indexer mode if specified
        if (this.config.indexerMode) {
          composeYaml.services["sh-indexer"].command.push(
            `--indexer-mode=${this.config.indexerMode}`
          );
        }
      } else {
        // Embedded mode: add indexer flags to sh-user
        composeYaml.services["sh-user"].command.push("--indexer");
        composeYaml.services["sh-user"].environment =
          composeYaml.services["sh-user"].environment ?? {};
        composeYaml.services["sh-user"].environment.SH_INDEXER_DB_AUTO_MIGRATE = "false";
        composeYaml.services["sh-user"].command.push(
          `--indexer-database-url=postgresql://postgres:postgres@${ShConsts.NODE_INFOS.indexerDb.containerName}:5432/storage_hub`
        );
        if (this.config.indexerMode) {
          composeYaml.services["sh-user"].command.push(`--indexer-mode=${this.config.indexerMode}`);
        }
      }
    }

    // If pending DB is enabled, add CLI arg to MSP 1 only
    if (this.config.pendingTxDb && this.type === "fullnet") {
      composeYaml.services["sh-msp-1"].command.push(
        `--pending-db-url=postgresql://postgres:postgres@${ShConsts.NODE_INFOS.pendingDb.containerName}:5432/pending_tx`
      );
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
      name: "storage-hub",
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
    const tmpDir = path.join(cwd, "tmp");
    fs.mkdirSync(tmpDir, { recursive: true });
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

    // Postgres is only needed when indexer is enabled
    if (this.config.indexer) {
      await compose.upOne("sh-indexer-postgres", {
        cwd: cwd,
        config: tmpFile,
        log: verbose
      });

      // Run external migrations when indexer enabled
      // (MSPs and standalone indexer auto-migrate themselves)
      await this.runMigrations();
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

    // Optional: start pending tx Postgres first (so MSP can connect on boot)
    if (this.type === "fullnet" && this.config.pendingTxDb) {
      await compose.upOne("sh-pending-postgres", {
        cwd,
        config: tmpFile,
        log: verbose
      });
      await this.waitForPendingDbReady("sh-pending-postgres", cwd, tmpFile);
    }

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

    // Start backend only if backend flag is enabled (depends on postgres, so requires indexer)
    if (this.config.backend) {
      if (!this.config.indexer) {
        throw new Error("Backend requires indexer to be enabled");
      }
      await compose.upOne("sh-backend", {
        cwd: cwd,
        config: tmpFile,
        log: verbose,
        env: {
          ...process.env,
          JWT_SECRET: JWT_SECRET
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

    // Only start fisherman service if it's enabled and we're using fullnet
    if (this.config.fisherman && this.type === "fullnet") {
      await compose.upOne("sh-fisherman", {
        cwd: cwd,
        config: tmpFile,
        log: verbose,
        env: {
          ...process.env
        }
      });
    }

    // Only start standalone indexer service if it's enabled and we're using fullnet.
    // The indexer itself runs in the `sh-indexer` service, while its Postgres
    // backend is provided by the `sh-indexer-postgres` service started earlier.
    if (this.config.indexer && this.config.standaloneIndexer && this.type === "fullnet") {
      await compose.upOne("sh-indexer", {
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

    return this;
  }

  public async stopNetwork() {
    const services = Object.keys(this.composeYaml.services);
    console.log(services);
  }

  private async runMigrations() {
    // Migrations are needed when indexer is enabled
    assert(this.config.indexer, "Indexer must be enabled to run migrations");

    const dieselCheck = spawnSync("diesel", ["--version"], { stdio: "ignore" });
    assert(
      dieselCheck.status === 0,
      "Error running Diesel CLI. Visit https://diesel.rs/guides/getting-started for install instructions."
    );

    await waitFor({
      lambda: async () => {
        try {
          execSync(
            `docker exec ${ShConsts.NODE_INFOS.indexerDb.containerName} pg_isready -U postgres -h 127.0.0.1 -p 5432 -t 1`,
            {
              stdio: "ignore"
            }
          );
          return true; // exit code 0 => ready
        } catch {
          return false; // non-zero => not ready yet
        }
      }
    });

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
    return BspNetTestApi.create(
      `ws://127.0.0.1:${this.getPort(serviceName)}`,
      this.config.runtimeType ?? "parachain"
    );
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

  public async preFundAccounts(api: EnrichedBspApi) {
    const amount = 10000n * 10n ** 12n;

    const sudo = api.accounts.sudo;
    const signedCalls = [
      api.tx.sudo
        .sudo(api.tx.balances.forceSetBalance(api.accounts.bspKey.address, amount))
        .signAsync(sudo, { nonce: 0 }),
      api.tx.sudo
        .sudo(api.tx.balances.forceSetBalance(api.accounts.shUser.address, amount))
        .signAsync(sudo, { nonce: 1 }),
      api.tx.sudo
        .sudo(api.tx.balances.forceSetBalance(api.accounts.mspKey.address, amount))
        .signAsync(sudo, { nonce: 2 }),
      api.tx.sudo
        .sudo(api.tx.balances.forceSetBalance(api.accounts.mspTwoKey.address, amount))
        .signAsync(sudo, { nonce: 3 }),
      api.tx.sudo
        .sudo(api.tx.balances.forceSetBalance(api.accounts.mspDownKey.address, amount))
        .signAsync(sudo, { nonce: 4 }),
      api.tx.sudo
        .sudo(api.tx.balances.forceSetBalance(api.accounts.fishermanKey.address, amount))
        .signAsync(sudo, { nonce: 5 })
    ];

    const sudoTxns = await Promise.all(signedCalls);

    await api.block.seal({ calls: sudoTxns });
    return;
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

  public async execDemoStorageRequest() {
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
      api.accounts.shUser,
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

  // Same as execDemoStorageRequest but creating a big file on the fly
  public async execBigFileStorageRequest(fileSizeGB: number) {
    const { generateLargeFile } = await import("../fileGeneration");

    await using api = await this.getApi("sh-user");

    // Generate file in docker/resource/ (mounted as /res in container)
    const timestamp = Date.now();
    const fileName = `benchmark-${fileSizeGB}gb-${timestamp}.bin`;
    const hostPath = `../docker/resource/${fileName}`; // Host filesystem path
    const containerPath = `/res/${fileName}`; // Container path (for RPC)

    await generateLargeFile(fileSizeGB, hostPath);

    console.log(`📤 Uploading ${fileSizeGB}GB file to MSP...`);
    const uploadStartTime = Date.now();

    const destination = `test/bigfile-${fileSizeGB}gb.bin`;
    const bucketName = `benchmark-bucket-${timestamp}`;

    const fileMetadata = await api.file.createBucketAndSendNewStorageRequest(
      containerPath, // Use container path for RPC
      destination,
      bucketName,
      null,
      DUMMY_MSP_ID,
      api.accounts.shUser,
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

    const uploadTime = ((Date.now() - uploadStartTime) / 1000).toFixed(2);
    console.log(`✅ Upload completed in ${uploadTime}s`);

    return { fileMetadata, tempFilePath: hostPath, fileSizeGB };
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
    const runtimeTypeArgs =
      this.config.runtimeType === "solochain" ? ["--chain=solochain-evm-dev"] : [];
    const { containerName: bspDownContainerName } = await addBsp(
      api,
      api.accounts.bspDownKey,
      api.accounts.sudo,
      {
        name: "sh-bsp-down",
        rocksdb: this.config.rocksdb,
        bspId: ShConsts.BSP_DOWN_ID,
        bspStartingWeight: this.config.capacity,
        extrinsicRetryTimeout: this.config.extrinsicRetryTimeout,
        additionalArgs: ["--keystore-path=/keystore/bsp-down", ...runtimeTypeArgs]
      }
    );
    const { rpcPort: bspTwoRpcPort } = await addBsp(
      api,
      api.accounts.bspTwoKey,
      api.accounts.sudo,
      {
        name: "sh-bsp-two",
        rocksdb: this.config.rocksdb,
        bspId: ShConsts.BSP_TWO_ID,
        bspStartingWeight: this.config.capacity,
        extrinsicRetryTimeout: this.config.extrinsicRetryTimeout,
        additionalArgs: ["--keystore-path=/keystore/bsp-two", ...runtimeTypeArgs]
      }
    );
    const { rpcPort: bspThreeRpcPort } = await addBsp(
      api,
      api.accounts.bspThreeKey,
      api.accounts.sudo,
      {
        name: "sh-bsp-three",
        rocksdb: this.config.rocksdb,
        bspId: ShConsts.BSP_THREE_ID,
        bspStartingWeight: this.config.capacity,
        extrinsicRetryTimeout: this.config.extrinsicRetryTimeout,
        additionalArgs: ["--keystore-path=/keystore/bsp-three", ...runtimeTypeArgs]
      }
    );

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
      containerName: "storage-hub-sh-bsp-1",
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
      containerName: "storage-hub-sh-user-1",
      searchString: "💤 Idle",
      timeout: 15000
    });

    await launchedNetwork.preFundAccounts(userApi);
    await launchedNetwork.setupBsp(userApi, userApi.accounts.bspKey.address, multiAddressBsp);
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
            ? userApi.accounts.mspKey.address
            : service === "sh-msp-2"
              ? userApi.accounts.mspTwoKey.address
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
      await launchedNetwork.setupMsp(userApi, userApi.accounts.mspKey.address, mockMspMultiAddress);
    }

    if (launchedNetwork.config.initialised === "multi") {
      return await launchedNetwork.initExtraBsps();
    }

    if (launchedNetwork.config.initialised === true) {
      return await launchedNetwork.execDemoStorageRequest();
    }

    if (launchedNetwork.config.big_file) {
      const result = await launchedNetwork.execBigFileStorageRequest(
        launchedNetwork.config.big_file
      );
      launchedNetwork.tempFiles.push(result.tempFilePath);
      return result;
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
   * Generate and upload a large file (in GB) for performance testing.
   * File stored in docker/tmp/ and cleaned up after test.
   */
  big_file?: number;

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
   * Optional parameter to run the indexer as a standalone service.
   * When true, indexer runs in a separate container (sh-indexer) instead of embedded in sh-user.
   * This allows independent control (pause/resume) of the indexer service.
   * Requires indexer to be true and only applies to fullnet.
   */
  standaloneIndexer?: boolean;

  /**
   * Optional parameter to define what toxics to apply to the network.
   * Only applies when `noisy` is set to true.
   */
  toxics?: ToxicInfo[];

  /**
   * Optional parameter to run the fisherman service.
   */
  fisherman?: boolean;

  /**
   * Optional parameter to run the backend service.
   * Requires indexer to be enabled.
   */
  backend?: boolean;

  /**
   * Optional parameter to set the indexer mode when indexer is enabled.
   * 'full' - indexes all events (default)
   * 'lite' - indexes only essential events as defined in LITE_MODE_EVENTS.md
   * 'fishing' - indexes only events related to fishing (fisherman service)
   */
  indexerMode?: "full" | "lite" | "fishing";

  /**
   * Runtime type to use.
   * 'parachain' - Polkadot parachain runtime (default)
   * 'solochain' - Solochain EVM runtime
   */
  runtimeType?: "parachain" | "solochain";

  /**
   * Maximum number of incomplete storage requests to process during initial sync.
   * Must be at least 1.
   */
  fishermanIncompleteSyncMax?: number;

  /**
   * Page size for incomplete storage request pagination.
   * Must be at least 1.
   */
  fishermanIncompleteSyncPageSize?: number;

  /**
   * Optional parameter to set the Rust log level for all nodes.
   * Defaults to 'info' if not specified.
   */
  logLevel?: string;
  /**
   * Optional parameter to enable pending transactions Postgres DB for MSP 1 (and other MSPs if extended).
   * When true, launches a dedicated Postgres container and passes its URL to MSP 1 as a CLI arg.
   */
  pendingTxDb?: boolean;
};
