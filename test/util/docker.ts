import Docker from "dockerode";
import { DOCKER_IMAGE } from "./constants";

export const checkNetworkRunning = async () => {
  const docker = new Docker();

  const containers = await docker.listContainers({
    filters: { ancestor: [DOCKER_IMAGE] }
  });

  return containers.length > 0;
};
