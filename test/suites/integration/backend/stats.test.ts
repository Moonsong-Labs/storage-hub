import assert, { strictEqual } from "node:assert";
import { describeMspNet, type EnrichedBspApi } from "../../../util";
import type { StatsResponse } from "../../../util/backend";

await describeMspNet(
  "Backend MSP Stats retrieval",
  {
    initialised: true,
    indexer: true,
    backend: true,
    runtimeType: "solochain"
  },
  ({ before, createMsp1Api, createUserApi, it }) => {
    let userApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
      const maybeMsp1Api = await createMsp1Api();
      if (maybeMsp1Api) {
        msp1Api = maybeMsp1Api;
      } else {
        throw new Error("MSP API for first MSP not available");
      }
    });

    it("Postgres DB is ready", async () => {
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.indexerDb.containerName,
        searchString: "database system is ready to accept connections",
        timeout: 10000
      });
    });

    it("Backend service is ready", async () => {
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.backend.containerName,
        searchString: "Server listening",
        timeout: 10000
      });
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

      const mspNodePeerId = await msp1Api.rpc.system.localPeerId();
      strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp1.expectedPeerId);
    });

    it("Should return MSP stats from backend matching on-chain data", async () => {
      // Get MSP info from chain
      const mspId = userApi.shConsts.DUMMY_MSP_ID;
      const mspInfoOption = await userApi.query.providers.mainStorageProviders(mspId);
      assert(mspInfoOption.isSome, "MSP should exist on chain");
      const mspInfo = mspInfoOption.unwrap();

      // Get active users count via runtime API
      const activeUsersList =
        await userApi.call.paymentStreamsApi.getUsersOfPaymentStreamsOfProvider(mspId);
      const activeUsersCount = activeUsersList.length;

      // Get stats from backend
      const response = await fetch("http://localhost:8080/stats");
      assert(response.ok, `Backend stats request failed with status ${response.status}`);

      const stats = (await response.json()) as StatsResponse;

      // Verify capacity values match chain data
      strictEqual(
        stats.capacity.totalBytes,
        mspInfo.capacity.toString(),
        "Total capacity should match on-chain data"
      );

      const expectedUsed = mspInfo.capacityUsed.toString();
      strictEqual(
        stats.capacity.usedBytes,
        expectedUsed,
        "Used capacity should match on-chain data"
      );

      const expectedAvailable = (
        mspInfo.capacity.toBigInt() - mspInfo.capacityUsed.toBigInt()
      ).toString();
      strictEqual(
        stats.capacity.availableBytes,
        expectedAvailable,
        "Available capacity should match calculated value (total - used)"
      );

      // Verify buckets amount matches on-chain data
      strictEqual(
        stats.bucketsAmount,
        mspInfo.amountOfBuckets.toString(),
        "Buckets amount should match on-chain data"
      );

      // Verify active users count matches runtime API
      strictEqual(
        stats.activeUsers,
        activeUsersCount,
        "Active users count should match runtime API data"
      );

      // Verify other stats fields match chain data
      strictEqual(
        stats.lastCapacityChange,
        mspInfo.lastCapacityChange.toString(),
        "Last capacity change should match on-chain data"
      );

      strictEqual(
        stats.valuePropsAmount,
        mspInfo.amountOfValueProps.toString(),
        "Value props amount should match on-chain data"
      );
    });
  }
);
