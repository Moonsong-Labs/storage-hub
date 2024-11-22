import { notStrictEqual, strictEqual } from "assert";
import { describeBspNet, type EnrichedBspApi } from "../../../util";

describeBspNet(
  "BSP proofs resubmitted on chain re-org ♻️",
  { initialised: true, networkConfig: "standard", keepAlive: true },
  ({ before, createUserApi, it }) => {
    let userApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
    });

    // This is skipped because it currently fails with timeout for ext inclusion
    it("resubmits a dropped proof Ext", async () => {
      await userApi.block.seal(); // To make sure we have a finalized head
      const nextChallengeTick = await getNextChallengeHeight(userApi);
      await userApi.block.skipTo(nextChallengeTick, { waitBetweenBlocks: true });

      await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true
      });

      await userApi.node.dropTxn({ module: "proofsDealer", method: "submitProof" });

      await userApi.block.seal();
      await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true
      });
    });

    // This is skipped because: 1) the underlying functionality is not yet implemented
    it("proof re-submitted when new fork has longer chain", { skip: "Not Impl" }, async () => {
      await userApi.block.seal(); // To make sure we have a finalized head
      const nextChallengeTick = await getNextChallengeHeight(userApi);
      await userApi.block.skipTo(nextChallengeTick, { waitBetweenBlocks: true, finalised: false });
      const { events } = await userApi.block.seal({ finaliseBlock: false });

      await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof"
      });

      const { data: fork1 } = userApi.assert.fetchEvent(
        userApi.events.proofsDealer.ProofAccepted,
        events
      );
      console.dir(fork1.toHuman());

      // TODO: Do this better using BTreeMap methods
      const challengeCount: number = (Object.values(fork1.proof.keyProofs.toJSON())[0] as any)[
        "challengeCount"
      ];

      await userApi.block.reOrg();
      await userApi.block.skipTo(nextChallengeTick, { waitBetweenBlocks: true });

      const { data: fork2 } = userApi.assert.fetchEvent(
        userApi.events.proofsDealer.ProofAccepted,
        await userApi.query.system.events()
      );
      console.dir(fork2.toHuman(), { depth: null });
      strictEqual(
        fork2.lastTickProven.toNumber(),
        nextChallengeTick,
        "Submitted proof should be for relevant next challenge tick"
      );
      const newChallengeCount: number = (Object.values(fork1.proof.keyProofs.toJSON())[0] as any)[
        "challengeCount"
      ];
      strictEqual(challengeCount, newChallengeCount, "Challenge count should be the same");
      notStrictEqual(
        fork1.proof.forestProof,
        fork2.proof.forestProof,
        "Forest proof should be different since seeds have changed"
      );
    });

    // This is skipped because: 1) the underlying functionality is not yet implemented, and 2) our node panics when we try to extend the chain
    // 024-11-22 15:15:35        RPC-CORE: createBlock(createEmpty: bool, finalize: bool, parentHash?: BlockHash): CreatedBlock:: 20000: Error at calling runtime api: Execution failed: Execution aborted due to trap: wasm trap: wasm `unreachable` instruction executed
    // WASM backtrace:
    // error while executing at wasm backtrace:
    //     0: 0x79af9 - storage_hub_runtime.wasm!rust_begin_unwind
    //     1: 0x13c33 - storage_hub_runtime.wasm!core::panicking::panic_fmt::hc05ac5641cbd6ea9
    //     2: 0x15604 - storage_hub_runtime.wasm!core::option::expect_failed::hf60f25ad85c5da12
    //     3: 0x1ccc20 - storage_hub_runtime.wasm!<cumulus_pallet_parachain_system::pallet::Pallet<T> as frame_support::traits::hooks::OnFinalize<<<<T as frame_system::pallet::Config>::Block as sp_runtime::traits::HeaderProvider>::HeaderT as sp_runtime::traits::Header>::Number>>::on_finalize::h2524ab923df2489d
    //     4: 0x548237 - storage_hub_runtime.wasm!<(TupleElement0,TupleElement1,TupleElement2,TupleElement3,TupleElement4,TupleElement5,TupleElement6,TupleElement7,TupleElement8,TupleElement9,TupleElement10,TupleElement11,TupleElement12,TupleElement13,TupleElement14,TupleElement15,TupleElement16,TupleElement17,TupleElement18,TupleElement19,TupleElement20,TupleElement21,TupleElement22,TupleElement23) as frame_support::traits::hooks::OnFinalize<BlockNumber>>::on_finalize::h7bae4232a268b405
    //     5: 0x3f0132 - storage_hub_runtime.wasm!frame_executive::Executive<System,Block,Context,UnsignedValidator,AllPalletsWithSystem,COnRuntimeUpgrade>::finalize_block::h29d519d55162f43f
    //     6: 0x30a417 - storage_hub_runtime.wasm!BlockBuilder_finalize_block
    it("proof re-submitted when new fork has longer chain", { skip: "Not Impl" }, async () => {
      await userApi.block.seal(); // To make sure we have a finalized head
      const nextChallengeTick = await getNextChallengeHeight(userApi);
      const finalisedHead = await userApi.rpc.chain.getFinalizedHead();
      await userApi.block.skipTo(nextChallengeTick, { waitBetweenBlocks: true, finalised: false });
      const { events: fork1Events } = await userApi.block.seal({ finaliseBlock: false });

      await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof"
      });

      const { data: fork1 } = userApi.assert.fetchEvent(
        userApi.events.proofsDealer.ProofAccepted,
        fork1Events
      );
      // console.dir(fork1.toHuman(), {depth: null});

      // TODO: Do this better using BTreeMap methods
      const challengeCount: number = (Object.values(fork1.proof.keyProofs.toJSON())[0] as any)[
        "challengeCount"
      ];

      strictEqual(
        fork1.lastTickProven.toNumber(),
        nextChallengeTick,
        "Submitted proof should be for relevant next challenge tick"
      );
      await userApi.block.extendFork({
        parentBlockHash: finalisedHead.toHex(),
        amountToExtend: nextChallengeTick,
        verbose: true
      });

      const { events: fork2Events } = await userApi.block.seal({ finaliseBlock: false });
      await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof"
      });

      const { data: fork2 } = userApi.assert.fetchEvent(
        userApi.events.proofsDealer.ProofAccepted,
        fork2Events
      );
      // console.dir(fork2.toHuman(), {depth: null});
      strictEqual(
        fork2.lastTickProven.toNumber(),
        nextChallengeTick,
        "Submitted proof should be for relevant next challenge tick"
      );
      const newChallengeCount: number = (Object.values(fork1.proof.keyProofs.toJSON())[0] as any)[
        "challengeCount"
      ];
      strictEqual(challengeCount, newChallengeCount, "Challenge count should be the same");
      notStrictEqual(
        fork1.proof.forestProof,
        fork2.proof.forestProof,
        "Forest proof should be different since seeds have changed"
      );
    });
  }
);

async function getNextChallengeHeight(api: EnrichedBspApi): Promise<number> {
  const lastTickResult = await api.call.proofsDealerApi.getLastTickProviderSubmittedProof(
    api.shConsts.DUMMY_BSP_ID
  );
  const lastTickBspSubmittedProof = lastTickResult.asOk.toNumber();
  console.log(
    `The last block that has a proof submitted by the BSP is ${lastTickBspSubmittedProof}`
  );
  const challengePeriodResult = await api.call.proofsDealerApi.getChallengePeriod(
    api.shConsts.DUMMY_BSP_ID
  );
  const challengePeriod = challengePeriodResult.asOk.toNumber();
  console.log(`The challenge period is ${challengePeriod}`);

  console.log(`therefore we skip to block ${lastTickBspSubmittedProof + challengePeriod}`);
  return lastTickBspSubmittedProof + challengePeriod;
}
