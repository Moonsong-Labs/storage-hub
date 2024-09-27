import path from "node:path";
import fs from "node:fs";
import { v2 as compose } from "docker-compose";
import { parse, stringify } from "yaml";
import invariant from "tiny-invariant";
import { BspNetTestApi, forceSignupBsp, getContainerIp, getContainerPeerId, ShConsts, type EnrichedBspApi, type ToxicInfo } from "../bspNet";
import { alice, bspKey, shUser } from "../pjsKeyring";

export type ShEntity = {
    port: number;
    name: string;
};

export class NetworkLauncher {
    private readonly config: NetLaunchConfig;
    private composeYaml?: any;
    private composeContents?: string;
    private entities?: ShEntity[];

    private constructor(config: NetLaunchConfig) {
        this.config = config;
    }

    private selectComposeFile() {
        let file = "local-dev-bsp-compose.yml";
        if (this.config.rocksdb) {
            file = "local-dev-bsp-rocksdb-compose.yml";
        }
        if (this.config.noisy) {
            file = "noisy-bsp-compose.yml";
        }

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
        }
        this.composeYaml = composeYaml;
        this.composeContents = stringify(composeYaml);
        return this;
    }

    public async getPeerId(serviceName: string) {
        invariant(this.entities, "Entities have not been populated yet, run populateEntities() first");
        invariant(
            Object.values(this.entities).map(({name})=> name).includes(serviceName),
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
            this.composeContents,
            "Compose file has not been selected yet, run selectComposeFile() first"
        );
        const cwd = path.resolve(process.cwd(), "..", "docker");

        if (this.config.noisy) {
            await compose.upOne("toxiproxy", {
                cwd: cwd,
                configAsString: this.composeContents,
                log: true
            });
        }

        await compose.upOne("sh-bsp", { cwd: cwd, configAsString: this.composeContents, log: true });

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

        await compose.upOne("sh-user", {
            cwd: cwd,
            configAsString: this.composeContents,
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

        const sudoTxns = await Promise.all([
            api.tx.sudo.sudo(api.tx.balances.forceSetBalance(bspKey.address, amount)).signAsync(alice, { nonce: 0 }),
            api.tx.sudo.sudo(api.tx.balances.forceSetBalance(shUser.address, amount)).signAsync(alice, { nonce: 1 }),
            api.tx.sudo.sudo(api.tx.fileSystem.setGlobalParameters(1, 1)).signAsync(alice, { nonce: 2 })
        ])

        return api.sealBlock(sudoTxns)
    }

    public async setupMsp(api: EnrichedBspApi, who: string, multiAddressMsp: string) {
        await api.sealBlock(
            api.tx.sudo.sudo(
                api.tx.providers.forceMspSignUp(
                    who,
                    ShConsts.DUMMY_MSP_ID,
                    this.config.capacity || ShConsts.CAPACITY_512,
                    [multiAddressMsp],
                    {
                        identifier: ShConsts.VALUE_PROP,
                        dataLimit: 500,
                        protocols: ["https", "ssh", "telnet"]
                    },
                    who
                )
            )
        );
        return this
    }

    public static async create(type: NetworkType, config: NetLaunchConfig) {
        console.log(
            `Launching network config ${config.noisy ? "with" : "without"} noise and ${config.rocksdb ? "with" : "without"} RocksDB for ${type} network`
        );
        const launchedNetwork = await new NetworkLauncher(config)
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
        await launchedNetwork.setupMsp(userApi, alice.address, multiAddressBsp);

        return launchedNetwork;
    }
}

export type NetworkType = "bspnet" | "fullnet";

/**
 * Configuration options for the BSP network.
 * These settings determine the behavior and characteristics of the network during tests.
 */
export type NetLaunchConfig = {
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
