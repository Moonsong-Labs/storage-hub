import assert, { strictEqual } from "node:assert";
import { type EnrichedBspApi, describeMspNet } from "../../../util";
import {
  generateMockJWT,
  type PaymentStream,
  type PaymentStreamResponse
} from "../../../util/backend";

await describeMspNet(
  "Backend Payment Streams retrieval",
  {
    initialised: true,
    indexer: true,
    backend: true
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
        containerName: "storage-hub-sh-postgres-1",
        searchString: "database system is ready to accept connections",
        timeout: 10000
      });
    });

    it("Backend service is ready", async () => {
      await userApi.docker.waitForLog({
        containerName: "storage-hub-sh-backend-1",
        searchString: "Server listening on",
        timeout: 10000
      });
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

      const mspNodePeerId = await msp1Api.rpc.system.localPeerId();
      strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp1.expectedPeerId);
    });

    it("Should be able to retrieve current price per giga unit per tick", async () => {
      const current_price =
        await msp1Api.call.paymentStreamsApi.getCurrentPricePerGigaUnitPerTick();

      const current_price_rpc =
        await msp1Api.rpc.storagehubclient.getCurrentPricePerGigaUnitPerTick();

      assert(current_price === current_price_rpc);
    });

    it("Should return payment stream information user", async () => {
      // TODO: Replace with proper flow
      const mockJWT = generateMockJWT(userApi.shConsts.NODE_INFOS.user.AddressId);

      const response = await fetch("http://localhost:8080/payment_streams", {
        headers: {
          Authorization: `Bearer ${mockJWT}`
        }
      });

      const data = (await response.json()) as PaymentStreamsResponse;

      // Check that we have exactly 2 streams
      assert(data.streams.length === 2, "Should have exactly 2 payment streams");

      // Find the MSP and BSP streams
      const mspStream = data.streams.find((s) => s.provider_type === "msp");
      const bspStream = data.streams.find((s) => s.provider_type === "bsp");

      // Verify both stream types exist
      assert(mspStream, "Should have an MSP stream");
      assert(bspStream, "Should have a BSP stream");

      // Verify the MSP provider ID matches DUMMY_MSP_ID
      strictEqual(
        mspStream.provider,
        userApi.shConsts.DUMMY_MSP_ID,
        "MSP provider should match DUMMY_MSP_ID"
      );
    });
  }
);
