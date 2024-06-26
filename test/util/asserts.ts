import type { EventRecord } from "@polkadot/types/interfaces";
import { strictEqual } from "node:assert";
import type { ApiPromise } from "@polkadot/api";
import type { AugmentedEvent } from "@polkadot/api/types";

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
