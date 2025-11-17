import assert from "node:assert";
import { execSync } from "node:child_process";
import net from "node:net";
import path from "node:path";
import { PassThrough, type Readable } from "node:stream";
import Docker from "dockerode";
import { DOCKER_IMAGE } from "../constants";
import { sendCustomRpc } from "../rpc";
import { sleep } from "../timer";
import * as NodeBspNet from "./node";
import { BspNetTestApi } from "./test-api";
import { waitFor } from "./waits";

export const addCopypartyContainer = async (options?: { name?: string }) => {
  const docker = new Docker();
  const containerName = options?.name || "storage-hub-sh-copyparty";
  const imageName = "copyparty/min:1.19.8";

  // Remove any existing container with same name
  try {
    const oldContainer = docker.getContainer(containerName);
    await oldContainer.remove({ force: true });
  } catch (_e) {
    // Container doesn't exist, that's fine
  }

  // Check if image exists, pull if it doesn't
  try {
    await docker.getImage(imageName).inspect();
  } catch (_e) {
    // Image doesn't exist, pull it
    console.log(`Pulling ${imageName}...`);
    const stream = await docker.pull(imageName);
    await new Promise<void>((resolve, reject) => {
      docker.modem.followProgress(stream, (err: any) => {
        if (err) {
          reject(err);
        } else {
          resolve();
        }
      });
    });
  }

  const container = await docker.createContainer({
    Image: imageName,
    name: containerName,
    Labels: {
      "com.docker.compose.project": "storage-hub",
      "com.docker.compose.service": containerName,
      "com.docker.compose.container-number": "1",
      "com.docker.compose.oneoff": "False"
    },
    Cmd: [
      "--ftp",
      "3921", // Enable FTP on port 3921
      "-v",
      "/res:res:r", // Read-only access to resources at /res path
      "-v",
      "/uploads:uploads:rw" // Read-write for uploads at /uploads path
    ],
    NetworkingConfig: {
      EndpointsConfig: {
        "storage-hub_default": {}
      }
    },
    ExposedPorts: {
      "3923/tcp": {},
      "3921/tcp": {}
    },
    HostConfig: {
      PortBindings: {
        "3923/tcp": [{ HostPort: "0" }], // Random available port
        "3921/tcp": [{ HostPort: "0" }] // Random available port
      },
      Binds: [`${process.cwd()}/../docker/resource:/res:ro`]
    }
  });

  await container.start();

  // Get container info
  const containerInfo = await container.inspect();
  const containerIp = containerInfo.NetworkSettings.Networks["storage-hub_default"]?.IPAddress;

  // Also get the mapped ports
  const httpHostPort = containerInfo.NetworkSettings.Ports["3923/tcp"]?.[0]?.HostPort || "3923";
  const ftpHostPort = containerInfo.NetworkSettings.Ports["3921/tcp"]?.[0]?.HostPort || "3921";

  // Wait for server to be ready by checking both HTTP and FTP endpoints
  const waitForServer = async (maxRetries = 50, delayMs = 300): Promise<void> => {
    let httpReady = false;
    let ftpReady = false;

    const checkHttp = async (): Promise<boolean> => {
      try {
        const response = await fetch(`http://localhost:${httpHostPort}/`);
        if (response.ok || response.status === 403) {
          console.log(`Copyparty HTTP server ready on http://localhost:${httpHostPort}`);
          return true;
        }
      } catch (_e) {
        // HTTP not ready yet
      }
      return false;
    };

    const checkFtp = async (): Promise<boolean> => {
      return new Promise<boolean>((resolve) => {
        let resolved = false;
        const client: net.Socket = net.createConnection(
          { port: Number(ftpHostPort), host: "localhost" },
          () => {
            // Wait for FTP 220 greeting
            client.once("data", (data) => {
              if (!resolved) {
                const response = data.toString();
                if (response.includes("220")) {
                  // Got 220 greeting, now verify files are accessible by sending LIST command
                  client.write("USER anonymous\r\n");

                  client.once("data", (loginResponse) => {
                    if (loginResponse.toString().includes("331")) {
                      // Need password
                      client.write("PASS \r\n");

                      client.once("data", (passResponse) => {
                        if (passResponse.toString().includes("230")) {
                          // Logged in, try to list /res directory to verify mount is ready
                          client.write("CWD /res\r\n");

                          client.once("data", (cwdResponse) => {
                            if (!resolved) {
                              resolved = true;
                              clearTimeout(timeoutId);
                              if (cwdResponse.toString().includes("250")) {
                                // Successfully changed to /res directory, files are accessible
                                console.log(
                                  `Copyparty FTP server ready with files accessible on ftp://localhost:${ftpHostPort}`
                                );
                                client.write("QUIT\r\n");
                                client.end();
                                resolve(true);
                              } else {
                                // Could not access /res, files not ready yet
                                client.destroy();
                                resolve(false);
                              }
                            }
                          });
                        } else {
                          // Login failed
                          if (!resolved) {
                            resolved = true;
                            clearTimeout(timeoutId);
                            client.destroy();
                            resolve(false);
                          }
                        }
                      });
                    } else if (loginResponse.toString().includes("230")) {
                      // Already logged in (no password needed)
                      client.write("CWD /res\r\n");

                      client.once("data", (cwdResponse) => {
                        if (!resolved) {
                          resolved = true;
                          clearTimeout(timeoutId);
                          if (cwdResponse.toString().includes("250")) {
                            console.log(
                              `Copyparty FTP server ready with files accessible on ftp://localhost:${ftpHostPort}`
                            );
                            client.write("QUIT\r\n");
                            client.end();
                            resolve(true);
                          } else {
                            client.destroy();
                            resolve(false);
                          }
                        }
                      });
                    } else {
                      // Unexpected response
                      if (!resolved) {
                        resolved = true;
                        clearTimeout(timeoutId);
                        client.destroy();
                        resolve(false);
                      }
                    }
                  });
                } else {
                  // Got data but not a 220 greeting, server not ready
                  if (!resolved) {
                    resolved = true;
                    clearTimeout(timeoutId);
                    client.destroy();
                    resolve(false);
                  }
                }
              }
            });
          }
        );

        const timeoutId = setTimeout(() => {
          if (!resolved) {
            resolved = true;
            client.destroy();
            resolve(false);
          }
        }, 250);

        client.on("error", () => {
          if (!resolved) {
            resolved = true;
            clearTimeout(timeoutId);
            resolve(false);
          }
        });

        client.on("close", () => {
          if (!resolved) {
            resolved = true;
            clearTimeout(timeoutId);
            resolve(false);
          }
        });
      });
    };

    // Poll with iterations and delay between checks
    for (let i = 0; i < maxRetries; i++) {
      // Check both endpoints concurrently
      const results: [boolean, boolean] = await Promise.all([
        httpReady ? Promise.resolve(true) : checkHttp(),
        ftpReady ? Promise.resolve(true) : checkFtp()
      ]);

      httpReady = results[0];
      ftpReady = results[1];

      if (httpReady && ftpReady) {
        return;
      }

      // Wait before next iteration (except on the last iteration)
      if (i < maxRetries - 1) {
        await new Promise((resolve) => setTimeout(resolve, delayMs));
      }
    }

    throw new Error(
      `Copyparty server failed to start after ${maxRetries} attempts (HTTP: ${httpReady}, FTP: ${ftpReady})`
    );
  };

  await waitForServer();

  return {
    container,
    containerName,
    containerIp,
    httpPort: 3923,
    ftpPort: 3921,
    httpHostPort: Number.parseInt(httpHostPort, 10),
    ftpHostPort: Number.parseInt(ftpHostPort, 10)
  };
};

export const checkBspForFile = async (filePath: string, options?: { containerName?: string }) => {
  const containerId = options?.containerName || "storage-hub-sh-bsp-1";
  const loc = path.join("/storage", filePath);

  for (let i = 0; i < 100; i++) {
    try {
      // TODO: Replace with dockerode
      execSync(`docker exec ${containerId} ls ${loc}`, { stdio: "ignore" });
      return;
    } catch {
      await sleep(100);
    }
  }
  throw `File not found: ${loc} in ${containerId}`;
};

export const checkFileChecksum = async (filePath: string, options?: { containerName?: string }) => {
  const containerId = options?.containerName || "storage-hub-sh-bsp-1";
  const loc = path.join("/storage", filePath);
  const output = execSync(`docker exec ${containerId} sha256sum ${loc}`);
  return output.toString().split(" ")[0];
};

export const showContainers = () => {
  try {
    // TODO: Replace with dockerode
    execSync("docker ps -a", { stdio: "inherit" });
  } catch (e) {
    console.log(e);
    console.log("Error displaying docker containers");
  }
};

export const addBspContainer = async (options?: {
  name?: string;
  additionalArgs?: string[];
  pendingDbUrl?: string;
}) => {
  const additionalArgs = options?.additionalArgs ?? [];
  if (options?.pendingDbUrl) {
    additionalArgs.push(`--pending-db-url=${options.pendingDbUrl}`);
  }
  return addContainer("bsp", { name: options?.name, additionalArgs });
};

export const addMspContainer = async (options?: {
  name?: string;
  additionalArgs?: string[];
  pendingDbUrl?: string;
}) => {
  const additionalArgs = options?.additionalArgs ?? [];
  if (options?.pendingDbUrl) {
    additionalArgs.push(`--pending-db-url=${options.pendingDbUrl}`);
  }
  return addContainer("msp", { name: options?.name, additionalArgs });
};

const addContainer = async (
  providerType: "bsp" | "msp",
  options?: {
    name?: string;
    additionalArgs?: string[];
  }
) => {
  const docker = new Docker();
  const existingContainers = (
    await docker.listContainers({
      filters: { ancestor: [DOCKER_IMAGE] }
    })
  )
    .flatMap(({ Command }) => Command)
    .filter((cmd) => cmd.includes(`--provider-type=${providerType}`));

  const containerCount = existingContainers.length;

  assert(containerCount > 0, `No existing ${providerType.toUpperCase()} containers`);

  const allContainersCount = (
    await docker.listContainers({
      filters: { ancestor: [DOCKER_IMAGE] }
    })
  ).flatMap(({ Command }) => Command).length;

  // Use allContainersCount for p2p port to avoid conflicts between BSPs and MSPs
  const p2pPort = 30350 + allContainersCount;
  const rpcPort = 9888 + allContainersCount * 7;
  const containerName = options?.name || `storage-hub-sh-${providerType}-${containerCount + 1}`;

  // Get bootnode from docker args
  const { Args } = await docker.getContainer("storage-hub-sh-user-1").inspect();
  const bootNodeArg = Args.find((arg) => arg.includes("--bootnodes="));

  assert(bootNodeArg, "No bootnode found in docker args");

  const keystoreArg = Args.find((arg) => arg.includes("--keystore-path="));
  const keystorePath = keystoreArg ? keystoreArg.split("=")[1] : "/keystore";

  // Check if postgres container exists (indicates indexer is enabled)
  let indexerEnabled = false;
  try {
    await docker.getContainer("storage-hub-sh-indexer-postgres-1").inspect();
    indexerEnabled = true;
  } catch {
    // Postgres container doesn't exist, indexer is not enabled
    indexerEnabled = false;
  }

  const container = await docker.createContainer({
    Image: DOCKER_IMAGE,
    name: containerName,
    platform: "linux/amd64",
    Labels: {
      "com.docker.compose.project": "storage-hub",
      "com.docker.compose.service": containerName,
      "com.docker.compose.container-number": (containerCount + 1).toString(),
      "com.docker.compose.oneoff": "False"
    },
    NetworkingConfig: {
      EndpointsConfig: {
        "storage-hub_default": {}
      }
    },
    HostConfig: {
      PortBindings: {
        "9944/tcp": [{ HostPort: rpcPort.toString() }],
        [`${p2pPort}/tcp`]: [{ HostPort: p2pPort.toString() }]
      },
      Binds: [`${process.cwd()}/../docker/dev-keystores:${keystorePath}:rw`]
    },
    Cmd: [
      "--dev",
      "--sealing=manual",
      "--provider",
      `--provider-type=${providerType}`,
      `--name=${containerName}`,
      "--no-hardware-benchmarks",
      "--unsafe-rpc-external",
      "--rpc-methods=unsafe",
      "--rpc-cors=all",
      `--port=${p2pPort}`,
      "--base-path=/data",
      // Only add database URL for MSP containers when indexer is enabled (MSP-only parameter)
      ...(providerType === "msp" && indexerEnabled
        ? [
            "--msp-database-url=postgresql://postgres:postgres@storage-hub-sh-indexer-postgres-1:5432/storage_hub"
          ]
        : []),
      bootNodeArg,
      ...(options?.additionalArgs || [])
    ]
  });

  await container.start();

  // Wait for container to be truly ready using deterministic checks
  await waitFor({
    lambda: async () => {
      try {
        // Check if container is still running
        const containerInfo = await container.inspect();
        if (!containerInfo.State.Running) {
          console.log(`Container not running: ${containerInfo.State.Status}`);
          return false;
        }

        // Check if RPC endpoint is responding
        const peerId = await sendCustomRpc(`http://127.0.0.1:${rpcPort}`, "system_localPeerId");
        return !!peerId;
      } catch (_error) {
        // RPC not ready yet, continue waiting
        return false;
      }
    },
    iterations: 150, // 30 seconds total (150 * 200ms)
    delay: 200
  });

  // Get peerId from the now-ready container
  const peerId = await sendCustomRpc(`http://127.0.0.1:${rpcPort}`, "system_localPeerId");
  assert(peerId, `Failed to get peerId from container on port ${rpcPort}`);

  const api = await BspNetTestApi.create(`ws://127.0.0.1:${rpcPort}`);
  const chainName = api.consts.system.version.specName.toString();
  const supportedChains = ["sh-parachain-runtime", "sh-solochain-evm"];

  assert(
    supportedChains.includes(chainName),
    `Error connecting to ${providerType.toUpperCase()} via api ${containerName}`
  );

  await api.disconnect();

  console.log(
    `▶️ ${providerType.toUpperCase()} container started with name: ${containerName}, rpc port: ${rpcPort}, p2p port: ${p2pPort}, peerId: ${peerId}`
  );

  return { containerName, rpcPort, p2pPort, peerId };
};

// Make this a rusty style OO function with api contexts
export const pauseContainer = async (containerName: string) => {
  const docker = new Docker();
  const container = docker.getContainer(containerName);
  await container.pause();

  await waitFor({
    lambda: async () => {
      try {
        const info = await container.inspect();
        return info?.State?.Paused === true;
      } catch {
        return false;
      }
    }
  });
};

export const stopContainer = async (containerName: string) => {
  const docker = new Docker();

  try {
    const containersToStop = await docker.listContainers({
      filters: { name: [containerName] }
    });

    if (containersToStop.length === 0) {
      console.log(`Container ${containerName} not found, already stopped/removed`);
      return;
    }

    const container = docker.getContainer(containersToStop[0].Id);

    // Stop the container
    try {
      await container.stop({ t: 10 }); // 10 second timeout
    } catch (e) {
      console.log(`Container ${containerName} already stopped or error stopping: ${e}`);
    }

    // Remove the container
    await container.remove({ force: true });

    // Give the system time to release ports
    await sleep(100);

    console.log(`Container ${containerName} stopped and removed successfully`);
  } catch (e) {
    console.error(`Error stopping container ${containerName}:`, e);
    throw e;
  }
};

export const startContainer = async (options: { containerName: string }) => {
  const docker = new Docker();
  const container = docker.getContainer(options.containerName);
  await container.start();
};

export const restartContainer = async (options: { containerName: string }) => {
  const docker = new Docker();
  const container = docker.getContainer(options.containerName);
  await container.restart();
};

export const clearLogs = async (options: { containerName: string }) => {
  const docker = new Docker();
  const container = docker.getContainer(options.containerName);
  const exec = await container.exec({
    AttachStdout: true,
    AttachStderr: true,
    Cmd: ["sh", "-c", `> /var/lib/docker/containers/${options.containerName}/*.log`]
  });

  await exec.start({});
  console.log(`Logs cleared for container ${options.containerName}`);
};

export const resumeContainer = async (options: { containerName: string }) => {
  const docker = new Docker();
  const container = docker.getContainer(options.containerName);
  await container.unpause();
};

export const dropAllTransactionsGlobally = async () => {
  const docker = new Docker();

  const containersToStop = await docker.listContainers({
    filters: { ancestor: ["storage-hub:local"] }
  });

  for (const container of containersToStop) {
    const publicPort = container.Ports.filter(
      ({ IP, PrivatePort }) => IP === "0.0.0.0" && PrivatePort === 9944
    )[0].PublicPort;
    const endpoint: `ws://${string}` = `ws://127.0.0.1:${publicPort}`;
    await using api = await BspNetTestApi.connect(endpoint);
    try {
      await NodeBspNet.dropTransaction(api);
    } catch {
      console.log(`Error dropping txn from ${container.Id}, continuing...`);
    }
  }
};

export const dropTransactionGlobally = async (options: { module: string; method: string }) => {
  const docker = new Docker();

  const containersToStop = await docker.listContainers({
    filters: { ancestor: ["storage-hub:local"] }
  });

  for (const container of containersToStop) {
    const publicPort = container.Ports.filter(
      ({ IP, PrivatePort }) => IP === "0.0.0.0" && PrivatePort === 9944
    )[0].PublicPort;
    const endpoint: `ws://${string}` = `ws://127.0.0.1:${publicPort}`;
    await using api = await BspNetTestApi.connect(endpoint);
    await NodeBspNet.dropTransaction(api, { module: options.module, method: options.method });
  }
};

export const waitForLog = async (options: {
  searchString: string;
  containerName: string;
  timeout?: number;
  tail?: number;
}): Promise<string> => {
  return new Promise((resolve, reject) => {
    const docker = new Docker();
    const container = docker.getContainer(options.containerName);

    container.logs(
      { follow: true, stdout: true, stderr: true, tail: options.tail, timestamps: false }, // set tail default to 10 to get the 10 last lines of logs printed
      (err, stream) => {
        if (err) {
          return reject(err);
        }

        if (stream === undefined) {
          return reject(new Error("No stream returned."));
        }

        const stdout = new PassThrough();
        const stderr = new PassThrough();

        docker.modem.demuxStream(stream, stdout, stderr);

        let timeoutHandle: ReturnType<typeof setTimeout> | undefined;

        const cleanup = () => {
          (stream as Readable).destroy();
          stdout.destroy();
          stderr.destroy();
          if (timeoutHandle) {
            clearTimeout(timeoutHandle);
          }
        };

        const onData = (chunk: Buffer) => {
          const log = chunk.toString("utf8");
          if (log.includes(options.searchString)) {
            cleanup();
            resolve(log);
          }
        };

        stdout.on("data", onData);
        stderr.on("data", onData);

        stream.on("error", (err) => {
          cleanup();
          reject(err);
        });

        if (options.timeout) {
          timeoutHandle = setTimeout(() => {
            cleanup();
            reject(
              new Error(
                `Timeout of ${options.timeout}ms exceeded while waiting for log ${options.searchString}`
              )
            );
          }, options.timeout);
        }
      }
    );
  });
};

export const waitForAnyLog = async (options: {
  searchStrings: string[];
  containerName: string;
  timeout?: number;
  tail?: number;
}): Promise<{ log: string; matchedString: string }> => {
  return new Promise((resolve, reject) => {
    const docker = new Docker();
    const container = docker.getContainer(options.containerName);

    container.logs(
      { follow: true, stdout: true, stderr: true, tail: options.tail, timestamps: false },
      (err, stream) => {
        if (err) {
          return reject(err);
        }

        if (stream === undefined) {
          return reject(new Error("No stream returned."));
        }

        const stdout = new PassThrough();
        const stderr = new PassThrough();

        docker.modem.demuxStream(stream, stdout, stderr);

        let timeoutHandle: ReturnType<typeof setTimeout> | undefined;

        const cleanup = () => {
          (stream as Readable).destroy();
          stdout.destroy();
          stderr.destroy();
          if (timeoutHandle) {
            clearTimeout(timeoutHandle);
          }
        };

        const onData = (chunk: Buffer) => {
          const log = chunk.toString("utf8");

          // Check if any of the search strings match
          for (const searchString of options.searchStrings) {
            if (log.includes(searchString)) {
              cleanup();
              resolve({ log, matchedString: searchString });
              return;
            }
          }
        };

        stdout.on("data", onData);
        stderr.on("data", onData);

        stream.on("error", (err) => {
          cleanup();
          reject(err);
        });

        if (options.timeout) {
          timeoutHandle = setTimeout(() => {
            cleanup();
            reject(
              new Error(
                `Timeout of ${options.timeout}ms exceeded while waiting for any of these logs: ${options.searchStrings.join(", ")}`
              )
            );
          }, options.timeout);
        }
      }
    );
  });
};
