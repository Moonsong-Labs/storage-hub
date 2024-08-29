import { assertEventPresent, assertExtrinsicPresent } from "../asserts";
import { sleep } from "../timer";
import type { BspNetApi } from "./types";
import { sealBlock } from "./helpers";

export const waitForBspVolunteer = async (api: BspNetApi) => {
  // To allow node to react
  await sleep(500);
  await assertExtrinsicPresent(api, {
    module: "fileSystem",
    method: "bspVolunteer",
    checkTxPool: true
  });
  const { events } = await sealBlock(api);
  assertEventPresent(api, "fileSystem", "AcceptedBspVolunteer", events);
};

export const waitForBspStored = async (api: BspNetApi) => {
  // To allow for local file transfer
  await sleep(5000);
  await assertExtrinsicPresent(api, {
    module: "fileSystem",
    method: "bspConfirmStoring",
    checkTxPool: true
  });
  const { events } = await api.sealBlock();
  assertEventPresent(api, "fileSystem", "BspConfirmedStoring", events);
};
