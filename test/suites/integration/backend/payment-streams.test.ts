import assert, { strictEqual } from "node:assert";
import type { u64, u128 } from "@polkadot/types";
import type { H256 } from "@polkadot/types/interfaces";
import { describeMspNet, type EnrichedBspApi } from "../../../util";
import { fetchJwtToken, type PaymentStreamsResponse } from "../../../util/backend";
import { SH_EVM_SOLOCHAIN_CHAIN_ID } from "../../../util/evmNet/consts";
import { ETH_SH_USER_ADDRESS, ETH_SH_USER_PRIVATE_KEY } from "../../../util/evmNet/keyring";

type OnChainPaymentStream = { provider: string; user: `0x${string}` } & (
  | { type: "fixed"; rate: u128 }
  | { type: "dynamic"; amountProvided: u64 }
);

await describeMspNet(
  "Backend Payment Streams retrieval",
  {
    initialised: true,
    indexer: true,
    backend: true,
    runtimeType: "solochain"
  },
  ({ before, createMsp1Api, createUserApi, it }) => {
    let userApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;
    const chainPaymentStreams: OnChainPaymentStream[] = [];

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
        containerName: "storage-hub-sh-indexer-postgres-1",
        searchString: "database system is ready to accept connections",
        timeout: 10000
      });
    });

    it("Backend service is ready", async () => {
      await userApi.docker.waitForLog({
        containerName: "storage-hub-sh-backend-1",
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

    it("Should be able to retrieve current price per giga unit per tick", async () => {
      const current_price =
        await msp1Api.call.paymentStreamsApi.getCurrentPricePerGigaUnitPerTick();

      const current_price_rpc =
        await msp1Api.rpc.storagehubclient.getCurrentPricePerGigaUnitPerTick();

      strictEqual(
        current_price.toString(),
        current_price_rpc.toString(),
        "Runtime API and RPC should have the same value"
      );
    });

    it("Should fetch payment streams from chain", async () => {
      const userAddress = ETH_SH_USER_ADDRESS;

      // Get providers with payment streams for the user
      const providersWithPaymentStreams = (
        await userApi.call.paymentStreamsApi.getProvidersWithPaymentStreamsWithUser(userAddress)
      ).map((provider) => provider as H256);

      // Fetch both fixed and dynamic rate payment streams for each provider
      for (const provider of providersWithPaymentStreams) {
        const fixedStream = await userApi.query.paymentStreams.fixedRatePaymentStreams(
          provider,
          userAddress
        );
        const dynamicStream = await userApi.query.paymentStreams.dynamicRatePaymentStreams(
          provider,
          userAddress
        );

        if (fixedStream.isSome) {
          chainPaymentStreams.push({
            provider: provider.toString(),
            user: userAddress,
            type: "fixed",
            rate: fixedStream.unwrap().rate
          });
        }

        if (dynamicStream.isSome) {
          chainPaymentStreams.push({
            provider: provider.toString(),
            user: userAddress,
            type: "dynamic",
            amountProvided: dynamicStream.unwrap().amountProvided
          });
        }
      }

      // Verify we have payment streams on chain
      assert(chainPaymentStreams.length > 0, "Should have at least one payment stream on chain");
    });

    it("Should return payment stream information user", async () => {
      assert(chainPaymentStreams, "On-chain payment stream data is initialized");

      const userJWT = await fetchJwtToken(ETH_SH_USER_PRIVATE_KEY, SH_EVM_SOLOCHAIN_CHAIN_ID);

      const response = await fetch("http://localhost:8080/payment_streams", {
        headers: {
          Authorization: `Bearer ${userJWT}`
        }
      });

      const data = (await response.json()) as PaymentStreamsResponse;

      // Find the MSP and BSP streams
      const apiMspStreams = data.streams.filter((s) => s.providerType === "msp");
      const apiBspStreams = data.streams.filter((s) => s.providerType === "bsp");

      // Verify both stream types exist
      assert(apiMspStreams.length > 0, "Should have an MSP stream");
      assert(apiBspStreams.length > 0, "Should have a BSP stream");

      // Verify the MSP provider ID matches DUMMY_MSP_ID
      for (const stream of apiMspStreams) {
        strictEqual(
          stream.provider,
          userApi.shConsts.DUMMY_MSP_ID,
          "MSP provider should match DUMMY_MSP_ID"
        );
      }

      // Verify that the API data matches what's on chain
      // Count fixed streams (MSP = fixed) and dynamic streams (BSP = dynamic) from chain
      const chainFixedStreams = chainPaymentStreams.filter((s) => s.type === "fixed");
      const chainDynamicStreams = chainPaymentStreams.filter((s) => s.type === "dynamic");

      // Verify counts match
      strictEqual(
        apiMspStreams.length,
        chainFixedStreams.length,
        "Backend API MSP streams count should match on chain data"
      );

      strictEqual(
        apiBspStreams.length,
        chainDynamicStreams.length,
        "Backend API BSP streams count should match on chain data"
      );

      // Verify each API stream has a matching chain stream with correct type
      for (const apiStream of data.streams) {
        const expectedType = apiStream.providerType === "msp" ? "fixed" : "dynamic";
        const matchingChainStream = chainPaymentStreams.find(
          (s) => s.provider === apiStream.provider && s.type === expectedType
        );

        const costPerTick = BigInt(apiStream.costPerTick);
        strictEqual(
          costPerTick.toString(),
          apiStream.costPerTick,
          "costPerTick should be parseable as BigInt"
        );

        assert(
          matchingChainStream,
          `Stream for provider ${apiStream.provider} with type ${expectedType} should exist on chain`
        );
      }
    });
  }
);
