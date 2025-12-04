import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi, shUser } from "../../../util";

await describeMspNet(
  "Prometheus proof submission metrics",
  {
    initialised: false,
    indexer: true,
    telemetry: true
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

    it("BSP proof submission increments metrics", async () => {
      // First, create files so BSP has something to prove
      const mspId = userApi.shConsts.DUMMY_MSP_ID;
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      // Create files using batchStorageRequests (following fisherman test pattern)
      const batchResult = await userApi.file.batchStorageRequests({
        files: [
          {
            source: "res/smile.jpg",
            destination: "test/proof-metrics-1.jpg",
            bucketIdOrName: "proof-metrics-bucket",
            replicationTarget: 1
          },
          {
            source: "res/whatsup.jpg",
            destination: "test/proof-metrics-2.jpg",
            bucketIdOrName: "proof-metrics-bucket",
            replicationTarget: 1
          }
        ],
        mspId,
        valuePropId,
        owner: shUser,
        bspApi,
        mspApi
      });

      console.log(`Created ${batchResult.fileKeys.length} files for proof submission test`);

      // Verify that storagehub metrics are being exported by the BSP
      const allMetricsResult = await userApi.prometheus.query(
        '{__name__=~"storagehub_.*",job="storagehub-bsp"}'
      );
      console.log(`StorageHub metrics available from BSP: ${allMetricsResult.data.result.length}`);
      if (allMetricsResult.data.result.length > 0) {
        const metricNames = [
          ...new Set(allMetricsResult.data.result.map((r) => r.metric.__name__))
        ];
        console.log(
          `  Metrics: ${metricNames.slice(0, 5).join(", ")}${metricNames.length > 5 ? "..." : ""}`
        );
      }

      // Get initial metric values
      const initialProofsSubmitted = await userApi.prometheus.getMetricValue(
        'storagehub_bsp_proofs_submitted_total{job="storagehub-bsp",status="pending"}'
      );
      const initialProofGenCount = await userApi.prometheus.getMetricValue(
        'storagehub_bsp_proof_generation_seconds_count{job="storagehub-bsp"}'
      );

      console.log(`Initial proofs submitted: ${initialProofsSubmitted}`);
      console.log(`Initial proof generation count: ${initialProofGenCount}`);

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

      // Wait for Prometheus to scrape the updated metrics
      await userApi.prometheus.waitForScrape();

      // Check that metrics have incremented
      const finalProofsSubmitted = await userApi.prometheus.getMetricValue(
        'storagehub_bsp_proofs_submitted_total{job="storagehub-bsp",status="pending"}'
      );
      const finalProofGenCount = await userApi.prometheus.getMetricValue(
        'storagehub_bsp_proof_generation_seconds_count{job="storagehub-bsp"}'
      );
      const proofGenSum = await userApi.prometheus.getMetricValue(
        'storagehub_bsp_proof_generation_seconds_sum{job="storagehub-bsp"}'
      );

      console.log(`Final proofs submitted: ${finalProofsSubmitted}`);
      console.log(`Final proof generation count: ${finalProofGenCount}`);
      console.log(`Proof generation time sum: ${proofGenSum.toFixed(3)}s`);

      // Verify counters incremented
      assert(
        finalProofsSubmitted > initialProofsSubmitted,
        `Expected proofs_submitted_total to increment, got initial=${initialProofsSubmitted}, final=${finalProofsSubmitted}`
      );

      // Verify histogram recorded observations
      assert(
        finalProofGenCount > initialProofGenCount,
        `Expected proof_generation_seconds to record observations, got initial=${initialProofGenCount}, final=${finalProofGenCount}`
      );

      if (finalProofGenCount > 0) {
        const avgProofTime = proofGenSum / finalProofGenCount;
        console.log(`Average proof generation time: ${avgProofTime.toFixed(3)} seconds`);
      }
    });
  }
);
