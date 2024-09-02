import type { ApiPromise } from "@polkadot/api";
import { assertEventPresent, assertExtrinsicPresent } from "../asserts";
import { sleep } from "../timer";
import { sealBlock } from "./helpers";

export const waitForBspVolunteer = async (api: ApiPromise) => {
  // To allow node to react
  // TODO poll
  await sleep(500);
  await assertExtrinsicPresent(api, {
    module: "fileSystem",
    method: "bspVolunteer",
    checkTxPool: true
  });
  const { events } = await sealBlock(api);
  assertEventPresent(api, "fileSystem", "AcceptedBspVolunteer", events);
};

export const waitForBspStored = async (api: ApiPromise) => {
  // To allow for local file transfer
  // TODO poll
  await sleep(5000);
  await assertExtrinsicPresent(api, {
    module: "fileSystem",
    method: "bspConfirmStoring",
    checkTxPool: true
  });
  const { events } = await sealBlock(api);
  assertEventPresent(api, "fileSystem", "BspConfirmedStoring", events);
};

export namespace Waits {
  export const bspVolunteer = waitForBspVolunteer;
  export const bspStored = waitForBspStored;
}
