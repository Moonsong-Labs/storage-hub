import type { EventRecord } from "@polkadot/types/interfaces";
import { strictEqual } from "node:assert";
import type { ApiPromise } from "@polkadot/api";
import type { AugmentedEvent } from "@polkadot/api/types";
import type { BspNetApi } from "./bspNet";

//TODO: add ability to search nested extrinsics e.g. sudo.sudo(balance.forceTransfer(...))
export const assertExtrinsicPresent = async (
  api: BspNetApi,
  options: {
    blockHeight?: string;
    enforceSuccess?: boolean;
    checkTxPool?: boolean;
    module: string;
    method: string;
    ignoreParamCheck?: boolean;
  }
): Promise<
  {
    module: string;
    method: string;
    extIndex: number;
  }[]
> => {
  if (options.ignoreParamCheck !== true) {
    strictEqual(
      options.module in api.tx,
      true,
      `Module ${options.module} not found in API metadata. Turn off this check with "ignoreParamCheck: true" if you are sure this exists`
    );
    strictEqual(
      options.method in api.tx[options.module],
      true,
      `Method ${options.module}.${options.method} not found in metadata. Turn off this check with "ignoreParamCheck: true" if you are sure this exists`
    );
  }

  const blockHash = await api.rpc.chain.getBlockHash(options?.blockHeight);
  const extrinsics = !options.checkTxPool
    ? await (async () => {
        const response = await api.rpc.chain.getBlock(blockHash);
        return response.block.extrinsics;
      })()
    : await api.rpc.author.pendingExtrinsics();
  const transformed = extrinsics.map(({ method: { method, section } }, index) => {
    return { module: section, method, extIndex: index };
  });

  const matches = transformed.filter(
    ({ method, module }) => method === options?.method && module === options?.module
  );

  strictEqual(
    matches.length > 0,
    true,
    `No extrinsics matching ${options?.module}.${options?.method} found. \n Extrinsics in block ${options.blockHeight || blockHash}: ${extrinsics.map(({ method: { method, section } }) => `${section}.${method}`).join(" | ")}`
  );

  if (options?.enforceSuccess) {
    const events = await (await api.at(blockHash)).query.system.events();
    assertEventPresent(api, "system", "ExtrinsicSuccess", events);
  }

  return matches;
};

export const assertEventPresent = (
  api: ApiPromise,
  module: string,
  method: string,
  events?: EventRecord[]
) => {
  strictEqual(events && events.length > 0, true, "No events emitted in block");
  if (!events) {
    throw new Error("No events found, should be caught by assert");
  }

  const event = events.find((e) => e.event.section === module && e.event.method === method);
  strictEqual(event !== undefined, true, `No events matching ${module}.${method}`);
  if (!event) {
    throw new Error("No event found, should be caught by assert");
  }
  strictEqual(api.events[module][method].is(event.event), true);
  if (!api.events[module][method].is(event.event)) {
    throw new Error("Event doesn't match, should be caught by assert");
  }

  return { event: event.event, data: event.event.data };
};

export const assertEventMany = (
  api: ApiPromise,
  module: string,
  method: string,
  events?: EventRecord[]
) => {
  strictEqual(events && events.length > 0, true, "No events emitted in block");
  if (!events) {
    throw new Error("No events found, should be caught by assert");
  }

  const matchingEvents = events.filter((event) => api.events[module][method].is(event.event));

  if (matchingEvents.length === 0) {
    throw new Error(`No events matching ${module}.${method} found`);
  }

  return matchingEvents;
};

type EventData<T extends AugmentedEvent<"promise">> = T extends AugmentedEvent<"promise", infer D>
  ? D
  : never;

export const fetchEventData = <T extends AugmentedEvent<"promise">>(
  matcher: T,
  events?: EventRecord[]
): EventData<T> => {
  strictEqual(events && events.length > 0, true, "No events emitted in block");
  if (!events) {
    throw new Error("No events found, should be caught by assert");
  }

  const eventRecord = events.find((e) => matcher.is(e.event));

  if (!eventRecord) {
    throw new Error(`No event found for matcher, ${matcher.meta.name}`);
  }

  const event = eventRecord.event;

  if (matcher.is(event)) {
    return event.data as unknown as EventData<T>;
  }

  throw new Error("Event doesn't match, should be caught earlier");
};
