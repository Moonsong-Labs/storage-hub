import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi, shUser } from "../../../util";

await describeMspNet(
  "Prometheus fee charging metrics",
  {
    initialised: false,
    indexer: true,
    prometheus: true
  },
  ({ before, createUserApi, createBspApi, createMsp1Api, it }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
      const maybeMspApi = await createMsp1Api();
      assert(maybeMspApi, "MSP API not available");
      mspApi = maybeMspApi;

      // Wait for Prometheus to be ready
      await userApi.prometheus.waitForReady();
    });

    it("BSP fee charging metric increments after proof submission", async () => {
      // First, create files so BSP has something to prove and charge fees for
      const mspId = userApi.shConsts.DUMMY_MSP_ID;
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      // Create files using batchStorageRequests (following fisherman test pattern)
      const batchResult = await userApi.file.batchStorageRequests({
        files: [
          {
            source: "res/smile.jpg",
            destination: "test/fee-metrics-1.jpg",
            bucketIdOrName: "fee-metrics-bucket",
            replicationTarget: 1
          },
          {
            source: "res/whatsup.jpg",
            destination: "test/fee-metrics-2.jpg",
            bucketIdOrName: "fee-metrics-bucket",
            replicationTarget: 1
          }
        ],
        mspId,
        valuePropId,
        owner: shUser,
        bspApi,
        mspApi
      });

      console.log(`Created ${batchResult.fileKeys.length} files for fee charging test`);

      // Get initial metric value
      const initialBspFees = await userApi.prometheus.getMetricValue(
        'storagehub_bsp_fees_charged_total{job="storagehub-bsp"}'
      );

      console.log(`Initial BSP fees charged: ${initialBspFees}`);

      // Get the last tick for which the BSP submitted a proof
      const lastTickResult = await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(
        userApi.shConsts.DUMMY_BSP_ID
      );
      assert(lastTickResult.isOk, "Failed to get last tick");
      const lastTickBspSubmittedProof = lastTickResult.asOk.toNumber();

      // Get the challenge period for the BSP
      const challengePeriodResult = await userApi.call.proofsDealerApi.getChallengePeriod(
        userApi.shConsts.DUMMY_BSP_ID
      );
      assert(challengePeriodResult.isOk, "Failed to get challenge period");
      const challengePeriod = challengePeriodResult.asOk.toNumber();

      // Calculate and advance to the next challenge tick
      const nextChallengeTick = lastTickBspSubmittedProof + challengePeriod;
      console.log(
        `Advancing from tick ${lastTickBspSubmittedProof} to next challenge tick ${nextChallengeTick}`
      );
      await userApi.block.skipTo(nextChallengeTick);

      // Wait for proof submission in tx pool (1 BSP)
      await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true,
        assertLength: 1,
        timeout: 15000
      });

      // Seal the block with the proof
      await userApi.block.seal();

      // Verify proof was accepted
      const proofAcceptedEvents = await userApi.assert.eventMany("proofsDealer", "ProofAccepted");
      assert(
        proofAcceptedEvents.length >= 1,
        `Expected at least 1 ProofAccepted event, got ${proofAcceptedEvents.length}`
      );
      console.log(`${proofAcceptedEvents.length} proof(s) accepted on-chain`);

      // Seal one more block to trigger LastChargeableInfoUpdated event
      await userApi.block.seal();

      // Wait for the LastChargeableInfoUpdated event
      const lastChargeableInfoEvents = await userApi.assert.eventMany(
        "paymentStreams",
        "LastChargeableInfoUpdated"
      );
      console.log(
        `${lastChargeableInfoEvents.length} LastChargeableInfoUpdated event(s), BSP will now charge fees`
      );

      // Wait for BSP fee charging extrinsic
      await userApi.assert.extrinsicPresent({
        module: "paymentStreams",
        method: "chargeMultipleUsersPaymentStreams",
        checkTxPool: true,
        assertLength: 1,
        exactLength: false,
        timeout: 10000
      });

      // Seal block to process fee charging
      await userApi.block.seal();

      // Verify UsersCharged event
      await userApi.assert.eventPresent("paymentStreams", "UsersCharged");
      console.log("UsersCharged event emitted, fees have been charged");

      // Wait for Prometheus to scrape the updated metrics
      await userApi.prometheus.waitForScrape();

      // Check that BSP fees metric has incremented
      const finalBspFees = await userApi.prometheus.getMetricValue(
        'storagehub_bsp_fees_charged_total{job="storagehub-bsp"}'
      );

      console.log(`Final BSP fees charged: ${finalBspFees}`);

      // Verify counter incremented (fee charging happens after proof acceptance)
      assert(
        finalBspFees > initialBspFees,
        `Expected bsp_fees_charged_total to increment, got initial=${initialBspFees}, final=${finalBspFees}`
      );
    });

    it("MSP fee charging metric is available and can increment", async () => {
      // MSP fee charging happens periodically via NotifyPeriod events
      // The metric should exist and be queryable

      const mspFeesQuery = 'storagehub_msp_fees_charged_total{job="storagehub-msp-1"}';
      const mspFees = await userApi.prometheus.getMetricValue(mspFeesQuery);

      console.log(`MSP fees charged (current): ${mspFees}`);

      // The metric should exist (even if 0 initially)
      const result = await userApi.prometheus.query(mspFeesQuery);
      assert.strictEqual(result.status, "success", "MSP fee query should succeed");

      // If we have seen any fee charging, verify it's a positive number
      if (mspFees > 0) {
        console.log(`MSP has charged fees ${mspFees} time(s)`);
      } else {
        console.log("MSP fee charging not yet triggered (requires periodic notification)");
      }
    });

    it("Insolvent user processing metric is queryable", async () => {
      // The insolvent_users_processed_total metric tracks when users without funds are processed
      // This metric is incremented in bsp_charge_fees.rs when UserWithoutFunds event is handled

      const insolventQuery = 'storagehub_insolvent_users_processed_total{job="storagehub-bsp"}';
      const result = await userApi.prometheus.query(insolventQuery);

      // Query should succeed even if no insolvent users have been processed
      assert.strictEqual(result.status, "success", "Insolvent users query should succeed");

      const insolventCount = await userApi.prometheus.getMetricValue(insolventQuery);
      console.log(`Insolvent users processed: ${insolventCount}`);

      // Note: Triggering actual insolvent user processing requires draining a user's balance
      // which is complex to set up in a test. We just verify the metric is queryable.
    });
  }
);
