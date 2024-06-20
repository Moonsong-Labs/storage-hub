import type { EventRecord } from "@polkadot/types/interfaces";
import { strictEqual } from "node:assert";
import type { BspNetApi } from "./bspNet";

export const assertEventPresent = (
  api: BspNetApi,
  module: string,
  method: string,
  events?: EventRecord[],
) => {
  strictEqual(events && events.length > 0, true, "No events emitted in block");
  if (!events) {
    throw new Error("No events found, should be caught by assert");
  }

  const event = events.find(
    (e) =>
      e.event.section === module &&
      e.event.method === method,
  );
  strictEqual(event !== undefined, true, `No events matching ${module}.${method}`);
  if (!event) {
    throw new Error("No event found, should be caught by assert");
  }

  strictEqual(api.events[module][method].is(event.event), true);
  return event.event;
};
