import assert, { strictEqual } from "node:assert";
import * as readline from "node:readline";
import { describeMspNet, type EnrichedBspApi, shUser } from "../../../util";

// TODO: Add description of what this test was useful for. It was used to spam the chain with buckets, and expose a bug where the Blockchain Service would start lagging behind in processing blocks, due to querying all of the buckets for each event processed.
// TODO: To see the effects of this test, run it with `only: true, keepAlive: true`, then see the logs in the MSP docker container.
// TODO: Suggestion: To the log at the end of processing a block import, add the time it took to process the block, to measure that in the MSP logs when running the tests.
await describeMspNet(
  "MSP is spammed with A LOT of buckets created",
  { initialised: false, networkConfig: "standard" },
  ({ before, createMsp1Api, it, createUserApi }) => {
    let userApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
      const maybeMspApi = await createMsp1Api();

      assert(maybeMspApi, "MSP API not available");
      mspApi = maybeMspApi;
    });

    const renderProgress = (line: string) => {
      // In CI/non-TTY, fall back to normal logs (can't reliably "overwrite" lines).
      if (!process.stdout.isTTY) {
        console.log(line);
        return;
      }

      readline.clearLine(process.stdout, 0);
      readline.cursorTo(process.stdout, 0);
      process.stdout.write(line);
    };

    const finishProgress = () => {
      if (process.stdout.isTTY) {
        process.stdout.write("\n");
      }
    };

    const setMaxBalanceToUser = async () => {
      const amount = 2n ** 128n - 1n;
      const sudoCall = userApi.tx.sudo.sudo(
        userApi.tx.balances.forceSetBalance(userApi.accounts.shUser.address, amount)
      );

      await userApi.block.seal({
        calls: [sudoCall],
        signer: userApi.accounts.sudo,
        finaliseBlock: true
      });
    };

    it(
      "Create 100k buckets, 500 per block",
      {
        skip: "Test takes way to long to run. This test actually spams the chain with buckets, unskip it if you want to run it."
      },
      async () => {
        const blocksToBuild = 200;
        const bucketsPerBlock = 500;

        const mspNodePeerId = await mspApi.rpc.system.localPeerId();
        strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp1.expectedPeerId);

        const mspId = userApi.shConsts.DUMMY_MSP_ID;
        const valueProps =
          await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
        assert(valueProps.length > 0, "No value propositions found for MSP");
        const valuePropId = valueProps[0].id.toHex();

        const startedAtMs = Date.now();
        let createdBuckets = 0;

        try {
          for (let blockIndex = 0; blockIndex < blocksToBuild; blockIndex++) {
            await setMaxBalanceToUser();

            const calls = new Array(bucketsPerBlock);
            for (let i = 0; i < bucketsPerBlock; i++) {
              const bucketName = `spam-${blockIndex}-${i}`;
              calls[i] = userApi.tx.fileSystem.createBucket(mspId, bucketName, false, valuePropId);
            }

            const sealed = await userApi.block.seal({
              calls,
              signer: shUser,
              finaliseBlock: true
            });

            createdBuckets += bucketsPerBlock;

            const latestHeader = await userApi.rpc.chain.getHeader(sealed.blockReceipt.blockHash);
            const blockNumber = latestHeader.number.toNumber();
            const elapsedSec = Math.max(1, Math.floor((Date.now() - startedAtMs) / 1000));
            const rate = Math.floor(createdBuckets / elapsedSec);
            const freeBalance = (await userApi.query.system.account(shUser.address)).data.free;

            renderProgress(
              `Built block #${blockNumber} (${blockIndex + 1}/${blocksToBuild}) - buckets created: ${createdBuckets} (${rate}/s) - user free balance: ${freeBalance.toString()}`
            );
          }
        } finally {
          finishProgress();
        }

        strictEqual(
          createdBuckets,
          blocksToBuild * bucketsPerBlock,
          "Bucket creation counter must match expected total"
        );
      }
    );
  }
);
