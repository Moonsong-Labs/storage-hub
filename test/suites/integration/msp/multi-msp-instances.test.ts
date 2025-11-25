import assert, { strictEqual } from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  getContainerPeerId,
  restartContainer,
  type SqlClient,
  shUser,
  waitFor
} from "../../../util";
import { MSP_CHARGING_PERIOD } from "../../../util/bspNet/consts";

await describeMspNet(
  "MSP 1 runs with multiple instances, only one being the leader, persisting pending transactions between instances",
  {
    initialised: true,
    pendingTxDb: true,
    networkConfig: [{ noisy: false, rocksdb: true }]
  },
  ({ before, after, createUserApi, createMsp1Api, createApi, it }) => {
    let userApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;
    let sql: SqlClient;
    let mspPendingNonce: bigint | undefined;
    let chargeInvalidNonce: bigint | undefined;

    before(async () => {
      userApi = await createUserApi();
      const maybeMspApi = await createMsp1Api();

      assert(maybeMspApi, "MSP API not available");
      mspApi = maybeMspApi;

      sql = userApi.pendingDb.createClient();
    });

    after(async () => {
      mspApi.disconnect();
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

      const mspNodePeerId = await mspApi.rpc.system.localPeerId();
      strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp1.expectedPeerId);
    });

    it("There are no pending transactions in the database", async () => {
      // First ensure MSP has no pending extrinsics in its tx pool by building and finalising blocks as needed.
      const mspAddress = userApi.accounts.mspKey.address;
      await mspApi.wait.waitForAvailabilityToSendTx(mspAddress);

      // Finalise the last block for the MSP node, to make sure it updates the pending transactions DB as "finalized".
      const latestFinalisedBlockHash = await userApi.rpc.chain.getFinalizedHead();
      await userApi.wait.nodeCatchUpToChainTip(mspApi);
      await mspApi.block.finaliseBlock(latestFinalisedBlockHash.toString());

      // Once there are no pending extrinsics for MSP and the last block has been finalised,
      // verify that the pending transactions DB contains only terminal-state entries for this account.
      const accountId = userApi.pendingDb.accountIdFromAddress(mspAddress);

      try {
        // We do this in a waitFor, to give time to the MSP node to update the pending transactions DB,
        // after we just finalised the last block.
        await waitFor({
          lambda: async () => {
            const activeCount = await userApi.pendingDb.countActive({ sql, accountId });
            return activeCount === 0n;
          }
        });
      } catch (error) {
        // If we time out waiting for active transactions to clear, fetch full account state
        const rows = await userApi.pendingDb.getAllByAccount({ sql, accountId });
        const activeStates = new Set(["future", "ready", "broadcast", "in_block", "retracted"]);
        const activeRows = rows.filter((row) => activeStates.has(row.state));

        // Log detailed information about any remaining active pending transactions
        // to help debug why they were not cleared.
        // eslint-disable-next-line no-console
        console.error(
          "[multi-msp-instances] Timed out waiting for MSP pending transactions to clear",
          {
            mspAddress,
            activeCount: activeRows.length,
            activeRows
          }
        );

        throw error;
      }
    });

    it("User sends a storage request, MSP 1 should respond accepting and the transaction should be persisted in the DB as ready/broadcasted", async () => {
      const mspAddress = userApi.accounts.mspKey.address;

      // Ensure MSP is free of pending extrinsics before starting this scenario.
      await mspApi.wait.waitForAvailabilityToSendTx(mspAddress);

      // Issue a new storage request from the user so MSP 1 will respond.
      const source = "res/whatsup.jpg";
      const destination = "test/multi-msp-pending-1.jpg";
      const bucketName = "multi-msp-pending-bucket-1";

      await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        destination,
        bucketName,
        null,
        null,
        shUser,
        1,
        false
      );

      // Wait for MSP 1 to submit its acceptance extrinsic into the tx pool.
      await userApi.wait.mspResponseInTxPool(1);

      const accountId = userApi.pendingDb.accountIdFromAddress(mspAddress);

      // Wait until at least one pending DB row for MSP is in a "ready" or "broadcast" state.
      await waitFor({
        lambda: async () => {
          const rows = await userApi.pendingDb.getAllByAccount({ sql, accountId });
          return rows.some((row) => row.state === "ready" || row.state === "broadcast");
        }
      });

      const rows = await userApi.pendingDb.getAllByAccount({ sql, accountId });
      const candidateRows = rows.filter(
        (row) => row.state === "ready" || row.state === "broadcast"
      );

      assert(
        candidateRows.length > 0,
        "Expected at least one pending transaction in 'ready' or 'broadcast' state for MSP account"
      );

      // Track the latest nonce so we can follow this transaction in subsequent tests.
      const latestRow = candidateRows.reduce((acc, row) =>
        BigInt(row.nonce) > BigInt(acc.nonce) ? row : acc
      );
      mspPendingNonce = BigInt(latestRow.nonce);
    });

    it("Build block, transaction should change to in_block and then cleared once finalised", async () => {
      const mspAddress = userApi.accounts.mspKey.address;
      const accountId = userApi.pendingDb.accountIdFromAddress(mspAddress);

      assert(mspPendingNonce !== undefined, "MSP pending nonce not set from previous test");
      const nonce = mspPendingNonce;

      // 1) Build a block including the MSP acceptance transaction (but do not finalise yet).
      await userApi.block.seal({ finaliseBlock: false });

      // Wait for the transaction to be marked as in_block in the pending DB.
      await userApi.pendingDb.waitForState({
        sql,
        accountId,
        nonce,
        state: "in_block"
      });

      // 2) Finalise the block on both the producer (user) and the MSP node.
      const latestHeader = await userApi.rpc.chain.getHeader();
      const latestBlockHash = latestHeader.hash.toString();

      // Finalise on producer node.
      await userApi.block.finaliseBlock(latestBlockHash);

      // Ensure MSP imports and finalises the same block.
      await mspApi.wait.blockImported(latestBlockHash);
      await mspApi.block.finaliseBlock(latestBlockHash);

      // Wait for the transaction to reach the "finalized" state in the pending DB.
      await userApi.pendingDb.waitForState({
        sql,
        accountId,
        nonce,
        state: "finalized"
      });

      // 3) Build and finalise one more block to allow nonce-based cleanup to remove the row.
      await userApi.block.seal({ finaliseBlock: true });
      const nextFinalisedHash = await userApi.rpc.chain.getFinalizedHead();

      await mspApi.wait.blockImported(nextFinalisedHash.toString());
      await mspApi.block.finaliseBlock(nextFinalisedHash.toString());

      // Finally, wait until the specific (account, nonce) row is cleared from the DB.
      try {
        await waitFor({
          lambda: async () => {
            const row = await userApi.pendingDb.getByNonce({ sql, accountId, nonce });
            return row === null;
          }
        });
      } catch (error) {
        // If we time out waiting for the row to be cleared, log all remaining pending txs for this account
        const rows = await userApi.pendingDb.getAllByAccount({ sql, accountId });
        // eslint-disable-next-line no-console
        console.error(
          "[multi-msp-instances] Pending transactions still present after MSP restart cleanup",
          {
            accountId: mspAddress,
            rows
          }
        );
        throw error;
      }
    });

    it("Wait for charge user tx, drop it and check update in DB", async () => {
      const mspAddress = userApi.accounts.mspKey.address;
      const accountId = userApi.pendingDb.accountIdFromAddress(mspAddress);

      // Advance to the next MSP charging period so that a chargeMultipleUsersPaymentStreams
      // extrinsic is submitted to the tx pool.
      const currentHeader = await userApi.rpc.chain.getHeader();
      const currentBlockNumber = currentHeader.number.toNumber();
      const blocksToAdvance =
        MSP_CHARGING_PERIOD - (currentBlockNumber % MSP_CHARGING_PERIOD || MSP_CHARGING_PERIOD);
      await userApi.block.skipTo(currentBlockNumber + blocksToAdvance);

      // Wait until the MSP tries to charge the user (tx in the pool on the user node).
      await userApi.assert.extrinsicPresent({
        module: "paymentStreams",
        method: "chargeMultipleUsersPaymentStreams",
        checkTxPool: true
      });

      // Snapshot number of invalid pending transactions before dropping the tx.
      const beforeRows = await userApi.pendingDb.getAllByAccount({ sql, accountId });
      const invalidBefore = beforeRows.filter((row) => row.state === "invalid").length;

      // Drop the charge user transaction from the tx pool.
      await userApi.node.dropTxn({
        module: "paymentStreams",
        method: "chargeMultipleUsersPaymentStreams"
      });
      await mspApi.node.dropTxn({
        module: "paymentStreams",
        method: "chargeMultipleUsersPaymentStreams"
      });

      // Wait until the pending DB reflects a new 'invalid' state for the MSP account.
      let latestInvalidNonce: bigint | undefined;
      try {
        await waitFor({
          lambda: async () => {
            const rows = await userApi.pendingDb.getAllByAccount({ sql, accountId });
            const invalidRows = rows.filter((row) => row.state === "invalid");
            if (invalidRows.length <= invalidBefore) {
              return false;
            }

            latestInvalidNonce = invalidRows.reduce<bigint>((acc, row) => {
              const n = BigInt(row.nonce);
              return n > acc ? n : acc;
            }, BigInt(invalidRows[0].nonce));

            return true;
          }
        });
      } catch (error) {
        const rows = await userApi.pendingDb.getAllByAccount({ sql, accountId });
        // eslint-disable-next-line no-console
        console.error(
          "[multi-msp-instances] Failed while waiting for 'invalid' state for charge tx",
          {
            accountId: mspAddress,
            beforeInvalidCount: invalidBefore,
            rows
          }
        );
        throw error;
      }

      assert(latestInvalidNonce !== undefined, "Expected at least one newly invalid pending tx");
      chargeInvalidNonce = latestInvalidNonce;

      // Finalise a block and ensure the invalid transaction is NOT cleared yet from the DB.
      await userApi.block.seal({ finaliseBlock: true });
      const finalisedHash = await userApi.rpc.chain.getFinalizedHead();

      await mspApi.wait.blockImported(finalisedHash.toString());
      await mspApi.block.finaliseBlock(finalisedHash.toString());

      const rowAfterFinalise = await userApi.pendingDb.getByNonce({
        sql,
        accountId,
        nonce: chargeInvalidNonce
      });
      assert(
        rowAfterFinalise,
        "Invalid pending tx row should still exist after finalising a block"
      );
      strictEqual(
        rowAfterFinalise.state,
        "invalid",
        "Invalid pending tx should remain in 'invalid' state after finalisation"
      );

      // Build and finalise one more block; nonce for MSP has not advanced, so the invalid tx
      // should still not be cleared from the DB.
      await userApi.block.seal({ finaliseBlock: true });
      const nextFinalisedHash = await userApi.rpc.chain.getFinalizedHead();

      await mspApi.wait.blockImported(nextFinalisedHash.toString());
      await mspApi.block.finaliseBlock(nextFinalisedHash.toString());

      const rowAfterSecondFinalise = await userApi.pendingDb.getByNonce({
        sql,
        accountId,
        nonce: chargeInvalidNonce
      });
      assert(
        rowAfterSecondFinalise,
        "Invalid pending tx row should still exist after additional finalised block"
      );
      strictEqual(
        rowAfterSecondFinalise.state,
        "invalid",
        "Invalid pending tx should still be in 'invalid' state after additional finalised block"
      );
    });

    it("Wait for next charge user tx, build a block and check update in DB", async () => {
      const mspAddress = userApi.accounts.mspKey.address;
      const accountId = userApi.pendingDb.accountIdFromAddress(mspAddress);

      assert(chargeInvalidNonce !== undefined, "chargeInvalidNonce not set from previous test");
      const nonce = chargeInvalidNonce;

      // Capture previous row (should be in 'invalid' state) and its hash.
      const previousRow = await userApi.pendingDb.getByNonce({ sql, accountId, nonce });
      assert(previousRow, "Expected invalid pending tx row to exist before next charge cycle");
      strictEqual(
        previousRow.state,
        "invalid",
        "Expected previous charge tx to be in 'invalid' state before next cycle"
      );
      const previousHashHex = Buffer.from(previousRow.hash).toString("hex");

      // Advance to the next MSP charging period.
      // Either a new chargeMultipleUsersPaymentStreams extrinsic or a remark transaction
      // acting as gap-filling will be submitted to the tx pool, reusing the same nonce.
      const currentHeader = await userApi.rpc.chain.getHeader();
      const currentBlockNumber = currentHeader.number.toNumber();
      const blocksToAdvance =
        MSP_CHARGING_PERIOD - (currentBlockNumber % MSP_CHARGING_PERIOD || MSP_CHARGING_PERIOD);
      await userApi.block.skipTo(currentBlockNumber + blocksToAdvance);

      await userApi.assert.extrinsicPresent({
        module: "paymentStreams",
        method: "chargeMultipleUsersPaymentStreams",
        checkTxPool: true
      });

      // Wait until the pending DB row for this nonce is upserted with a new hash and non-invalid state.
      // This could be a new chargeMultipleUsersPaymentStreams extrinsic or a remark transaction
      // acting as gap-filling.
      try {
        await waitFor({
          lambda: async () => {
            const row = await userApi.pendingDb.getByNonce({ sql, accountId, nonce });
            if (!row) {
              return false;
            }

            return row.state !== "invalid";
          }
        });
      } catch (error) {
        const rows = await userApi.pendingDb.getAllByAccount({ sql, accountId });
        // eslint-disable-next-line no-console
        console.error(
          `[multi-msp-instances] Failed while waiting for upserted state for charge tx with nonce ${nonce}`,
          {
            accountId: mspAddress,
            previousHashHex,
            rows
          }
        );
        throw error;
      }

      // Build a block including the charge tx but do not finalise yet, then wait for "in_block".
      await userApi.block.seal({ finaliseBlock: false });
      await userApi.pendingDb.waitForState({
        sql,
        accountId,
        nonce,
        state: "in_block"
      });

      // Finalise the block on user and MSP nodes, then wait for "finalized".
      const latestHeader = await userApi.rpc.chain.getHeader();
      const latestBlockHash = latestHeader.hash.toString();

      await userApi.block.finaliseBlock(latestBlockHash);
      await mspApi.wait.blockImported(latestBlockHash);
      await mspApi.block.finaliseBlock(latestBlockHash);

      await userApi.pendingDb.waitForState({
        sql,
        accountId,
        nonce,
        state: "finalized"
      });

      // Build and finalise one more block and wait for the row to be cleared.
      await userApi.block.seal({ finaliseBlock: true });
      const nextFinalisedHash = await userApi.rpc.chain.getFinalizedHead();

      await mspApi.wait.blockImported(nextFinalisedHash.toString());
      await mspApi.block.finaliseBlock(nextFinalisedHash.toString());

      await waitFor({
        lambda: async () => {
          const row = await userApi.pendingDb.getByNonce({ sql, accountId, nonce });
          return row === null;
        }
      });
    });

    it("Turn off MSP 1, turn it back on and check persisted transactions are still there", async () => {
      const mspAddress = userApi.accounts.mspKey.address;
      const accountId = userApi.pendingDb.accountIdFromAddress(mspAddress);

      // Ensure MSP is free of pending extrinsics before starting this scenario.
      await mspApi.wait.waitForAvailabilityToSendTx(mspAddress);

      // Advance to the next MSP charging period so that a new chargeMultipleUsersPaymentStreams
      // extrinsic is submitted to the tx pool.
      const currentHeader = await userApi.rpc.chain.getHeader();
      const currentBlockNumber = currentHeader.number.toNumber();
      const blocksToAdvance =
        MSP_CHARGING_PERIOD - (currentBlockNumber % MSP_CHARGING_PERIOD || MSP_CHARGING_PERIOD);
      await userApi.block.skipTo(currentBlockNumber + blocksToAdvance);

      await userApi.assert.extrinsicPresent({
        module: "paymentStreams",
        method: "chargeMultipleUsersPaymentStreams",
        checkTxPool: true
      });

      // Wait until a pending DB row exists for this MSP account in a non-terminal state and capture its nonce.
      let restartNonce: bigint | undefined;
      try {
        await waitFor({
          lambda: async () => {
            const rows = await userApi.pendingDb.getAllByAccount({ sql, accountId });
            const nonTerminal = rows.filter(
              (row) => row.state === "future" || row.state === "ready" || row.state === "broadcast"
            );
            if (nonTerminal.length === 0) {
              return false;
            }
            restartNonce = nonTerminal.reduce<bigint>((acc, row) => {
              const n = BigInt(row.nonce);
              return n > acc ? n : acc;
            }, BigInt(nonTerminal[0].nonce));
            return true;
          }
        });
      } catch (error) {
        const rows = await userApi.pendingDb.getAllByAccount({ sql, accountId });
        // eslint-disable-next-line no-console
        console.error(
          "[multi-msp-instances] Failed while waiting for non-terminal charge tx before MSP restart",
          {
            accountId: mspAddress,
            rows
          }
        );
        throw error;
      }

      assert(
        restartNonce !== undefined,
        "Expected at least one non-terminal pending tx before MSP restart"
      );

      // Restart MSP 1 container.
      await mspApi.disconnect();
      await restartContainer({ containerName: userApi.shConsts.NODE_INFOS.msp1.containerName });

      // Wait for MSP RPC to be back up.
      await getContainerPeerId(`http://127.0.0.1:${userApi.shConsts.NODE_INFOS.msp1.port}`, true);

      // Wait for MSP service to be idle again.
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName,
        searchString: "💤 Idle",
        timeout: 20000,
        tail: 50
      });

      // Recreate MSP API and ensure it catches up to the chain tip.
      const newMspApiMaybe = await createApi(
        `ws://127.0.0.1:${userApi.shConsts.NODE_INFOS.msp1.port}`
      );
      assert(newMspApiMaybe, "Failed to recreate MSP API after restart");
      mspApi = newMspApiMaybe;
      await userApi.wait.nodeCatchUpToChainTip(mspApi);

      const nonce = restartNonce;

      // Build a block including the charge tx but do not finalise yet, then wait for "in_block".
      await userApi.block.seal({ finaliseBlock: false });
      try {
        await userApi.pendingDb.waitForState({
          sql,
          accountId,
          nonce,
          state: "in_block"
        });
      } catch (error) {
        const rows = await userApi.pendingDb.getAllByAccount({ sql, accountId });
        // eslint-disable-next-line no-console
        console.error(
          "[multi-msp-instances] Failed while waiting for non-terminal charge tx before MSP restart",
          {
            accountId: mspAddress,
            rows
          }
        );
        throw error;
      }

      // Finalise the block on user and MSP nodes, then wait for "finalized".
      const latestHeader = await userApi.rpc.chain.getHeader();
      const latestBlockHash = latestHeader.hash.toString();

      await userApi.block.finaliseBlock(latestBlockHash);
      await mspApi.wait.blockImported(latestBlockHash);
      await mspApi.block.finaliseBlock(latestBlockHash);

      await userApi.pendingDb.waitForState({
        sql,
        accountId,
        nonce,
        state: "finalized"
      });

      // Build and finalise one more block and wait for the row to be cleared.
      await userApi.block.seal({ finaliseBlock: true });
      const nextFinalisedHash = await userApi.rpc.chain.getFinalizedHead();

      await mspApi.wait.blockImported(nextFinalisedHash.toString());
      await mspApi.block.finaliseBlock(nextFinalisedHash.toString());

      try {
        await waitFor({
          lambda: async () => {
            const row = await userApi.pendingDb.getByNonce({ sql, accountId, nonce });
            return row === null;
          }
        });
      } catch (error) {
        const rows = await userApi.pendingDb.getAllByAccount({ sql, accountId });
        // eslint-disable-next-line no-console
        console.error(
          "[multi-msp-instances] Pending transactions still present after MSP restart final cleanup",
          {
            accountId: mspAddress,
            rows
          }
        );
        throw error;
      }
    });

    it("Turn off MSP 1, build block with pending transaction, turn it back on and check transaction is not watched anymore", async () => {
      const mspAddress = userApi.accounts.mspKey.address;
      const accountId = userApi.pendingDb.accountIdFromAddress(mspAddress);

      // Ensure MSP is free of pending extrinsics before starting this scenario.
      await mspApi.wait.waitForAvailabilityToSendTx(mspAddress);

      // Advance to the next MSP charging period so that a new chargeMultipleUsersPaymentStreams
      // extrinsic is submitted to the tx pool.
      const currentHeader = await userApi.rpc.chain.getHeader();
      const currentBlockNumber = currentHeader.number.toNumber();
      const blocksToAdvance =
        MSP_CHARGING_PERIOD - (currentBlockNumber % MSP_CHARGING_PERIOD || MSP_CHARGING_PERIOD);
      await userApi.block.skipTo(currentBlockNumber + blocksToAdvance);

      await userApi.assert.extrinsicPresent({
        module: "paymentStreams",
        method: "chargeMultipleUsersPaymentStreams",
        checkTxPool: true
      });

      // Wait until a pending DB row exists for this MSP account in a non-terminal state and capture its nonce.
      let restartNonce: bigint | undefined;
      let prePauseState: string | undefined;
      try {
        await waitFor({
          lambda: async () => {
            const rows = await userApi.pendingDb.getAllByAccount({ sql, accountId });
            const nonTerminal = rows.filter(
              (row) => row.state === "future" || row.state === "ready" || row.state === "broadcast"
            );
            if (nonTerminal.length === 0) {
              return false;
            }
            // Pick the highest nonce non-terminal row
            const latest = nonTerminal.reduce((acc, row) => (row.nonce > acc.nonce ? row : acc));
            restartNonce = BigInt(latest.nonce);
            prePauseState = latest.state;
            return true;
          }
        });
      } catch (error) {
        const rows = await userApi.pendingDb.getAllByAccount({ sql, accountId });
        // eslint-disable-next-line no-console
        console.error(
          "[multi-msp-instances] Failed while waiting for non-terminal charge tx before MSP restart",
          {
            accountId: mspAddress,
            rows
          }
        );
        throw error;
      }

      assert(
        restartNonce !== undefined,
        "Expected at least one non-terminal pending tx before MSP restart"
      );

      // Pause MSP node so that it doesn't observe the upcoming block.
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.msp1.containerName);

      // Build a block including the charge tx but do not finalise yet.
      // Note: MSP is paused, so it won't update its watcher/DB state.
      await userApi.block.seal({ finaliseBlock: false });

      // The transaction should still be in the DB with the same state it had before MSP was turned off,
      // as it's not being updated by the paused MSP node.
      // Also, the watched flag should still be true so far.
      await waitFor({
        lambda: async () => {
          assert(restartNonce !== undefined, "Expected restart nonce to be set");
          const row = await userApi.pendingDb.getByNonce({ sql, accountId, nonce: restartNonce });
          assert(row, "Expected pending tx row to still exist after MSP restart");
          strictEqual(
            row.state,
            prePauseState,
            "Pending tx state should not have changed after sealing block"
          );
          strictEqual(row.watched, true, "Expected watched flag to be true");
          return true;
        }
      });

      // Restart MSP 1 container (will unpause due to restart).
      await mspApi.disconnect();
      await restartContainer({ containerName: userApi.shConsts.NODE_INFOS.msp1.containerName });

      // Wait for MSP RPC to be back up and service to be idle again.
      await getContainerPeerId(`http://127.0.0.1:${userApi.shConsts.NODE_INFOS.msp1.port}`, true);
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName,
        searchString: "💤 Idle",
        timeout: 20000,
        tail: 50
      });

      // Recreate MSP API and ensure it catches up to the chain tip.
      {
        const newMspApiMaybe = await createApi(
          `ws://127.0.0.1:${userApi.shConsts.NODE_INFOS.msp1.port}`
        );
        assert(newMspApiMaybe, "Failed to recreate MSP API after restart");
        mspApi = newMspApiMaybe;
        await userApi.wait.nodeCatchUpToChainTip(mspApi);
      }

      // There are two possible scenarios here, based on a race condition within the MSP client:
      // 1. The Blockchain Service attempts to re-watch the transaction BEFORE the Substrate Client
      //    is aware of the new block (i.e. before it knows that the transaction is included in a block).
      //    In this case, the transaction should have been re-watched because calling submitAndWatchExtrinsic
      //    returns successfully, as the transaction is not yet included in a block, and the nonce is not "old".
      //    It should still be in the DB with the state updated to "in_block", and the watched flag should be true.
      // 2. The Blockchain Service attempts to re-watch the transaction AFTER the Substrate Client
      //    is aware of the new block (i.e. after it knows that the transaction is included in a block).
      //    In this case, the transaction should not have been re-watched because calling submitAndWatchExtrinsic
      //    returns an InvalidTransactionOutdated error, as the transaction is now included in a block, and the nonce is "old".
      //    It should still be in the DB with the same state it had before MSP was turned off, and the watched flag should be false.
      await waitFor({
        lambda: async () => {
          assert(restartNonce !== undefined, "Expected restart nonce to be set");
          const row = await userApi.pendingDb.getByNonce({ sql, accountId, nonce: restartNonce });
          assert(row, "Expected pending tx row to still exist after MSP restart");

          if (row.watched) {
            // Scenario 1: The transaction should have been re-watched because calling submitAndWatchExtrinsic
            // returned successfully, and the state should be updated to "in_block".
            strictEqual(row.state, "in_block", "Expected state to be 'in_block'");
          } else {
            // Scenario 2: The transaction should not have been re-watched because calling submitAndWatchExtrinsic
            // returned an InvalidTransactionOutdated error, and the state should be the same as before MSP was turned off.
            strictEqual(
              row.state,
              prePauseState,
              "Expected state to be the same as before MSP was turned off"
            );
          }
          return true;
        }
      });

      // Finalise the block on user and MSP nodes, but the extrinsic should not be updated because it's not watched anymore.
      const latestHeader = await userApi.rpc.chain.getHeader();
      const latestBlockHash = latestHeader.hash.toString();
      await userApi.block.finaliseBlock(latestBlockHash);
      await mspApi.wait.blockImported(latestBlockHash);
      await mspApi.block.finaliseBlock(latestBlockHash);

      // Build and finalise one more block and wait for the row to be cleared by cleanup.
      await userApi.block.seal({ finaliseBlock: true });
      const nextFinalisedHash = await userApi.rpc.chain.getFinalizedHead();
      await mspApi.wait.blockImported(nextFinalisedHash.toString());
      await mspApi.block.finaliseBlock(nextFinalisedHash.toString());

      try {
        await waitFor({
          lambda: async () => {
            assert(restartNonce !== undefined, "Expected restart nonce to be set");
            const row = await userApi.pendingDb.getByNonce({
              sql,
              accountId,
              nonce: restartNonce
            });
            return row === null;
          }
        });
      } catch (error) {
        const rows = await userApi.pendingDb.getAllByAccount({ sql, accountId });
        // eslint-disable-next-line no-console
        console.error(
          "[multi-msp-instances] Pending transactions still present after final cleanup in non-rewatch scenario",
          {
            accountId: mspAddress,
            rows
          }
        );
        throw error;
      }
    });
  }
);
