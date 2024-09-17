import type { EventRecord } from "@polkadot/types/interfaces";
import invariant from "tiny-invariant";
import type { ApiPromise } from "@polkadot/api";
import type { AugmentedEvent } from "@polkadot/api/types";
import { sleep } from "./timer";

export type AssertExtrinsicOptions = {
  /** The block height to check. If not provided, the latest block will be used. */
  blockHeight?: string;
  /** The block hash to check. Takes precedence over blockHeight if provided. */
  blockHash?: string;
  /** If true, skips the check for an associated `ExtrinsicSuccess` event. */
  skipSuccessCheck?: boolean;
  /** If true, checks the pending transaction pool instead of a finalized block. */
  checkTxPool?: boolean;
  /** The module name of the extrinsic to check (e.g., "balances"). */
  module: string;
  /** The method name of the extrinsic to check (e.g., "transfer"). */
  method: string;
  /** If true, skips the validation check for the module.method existence in the API metadata. */
  ignoreParamCheck?: boolean;
  /** If provided, asserts that the number of extrinsics found matches this value. */
  assertLength?: number;
  /**If provided, will not throw until this timeout is reached. */
  timeout?: number;
};
/**
 * Asserts that a specific extrinsic (module.method) is present in a blockchain block or transaction pool.
 *
 * @param api The API instance connected to the blockchain network.
 * @param options Configuration options for the extrinsic check.
 * @returns A list of objects representing the extrinsics that match the module.method criteria.
 * @throws Error if no matching extrinsic is found, or if the success check fails (unless skipped).
 *
 * TODO: add ability to search nested extrinsics e.g. sudo.sudo(balance.forceTransfer(...))
 */
export const assertExtrinsicPresent = async (
  api: ApiPromise,
  options: AssertExtrinsicOptions
): Promise<
  {
    module: string;
    method: string;
    extIndex: number;
  }[]
> => {
  const timeoutMs = options.timeout || 5000; // Default timeout of 5 seconds
  const iterations = Math.floor(timeoutMs / 100);

  // Perform invariant checks outside the loop to fail fast on critical errors
  if (options.ignoreParamCheck !== true) {
    invariant(
      options.module in api.tx,
      `Module ${options.module} not found in API metadata. Turn off this check with "ignoreParamCheck: true" if you are sure this exists`
    );
    invariant(
      options.method in api.tx[options.module],
      `Method ${options.module}.${options.method} not found in metadata. Turn off this check with "ignoreParamCheck: true" if you are sure this exists`
    );
  }

  let lastError: Error | null = null;

  for (let i = 0; i < iterations + 1; i++) {
    try {
      const blockHash = options?.blockHash
        ? options.blockHash
        : options?.blockHeight
          ? await api.rpc.chain.getBlockHash(options?.blockHeight)
          : await api.rpc.chain.getBlockHash();

      const extrinsics = !options.checkTxPool
        ? await (async () => {
            const response = await api.rpc.chain.getBlock(blockHash);

            if (!options.blockHeight && !options.blockHash) {
              console.log(
                `No block height provided, using latest at ${response.block.header.number.toNumber()}`
              );
            }
            return response.block.extrinsics;
          })()
        : await api.rpc.author.pendingExtrinsics();

      const transformed = extrinsics.map(({ method: { method, section } }, index) => {
        return { module: section, method, extIndex: index };
      });

      const matches = transformed.filter(
        ({ method, module }) => method === options?.method && module === options?.module
      );

      if (matches.length > 0) {
        if (options?.assertLength !== undefined) {
          invariant(
            matches.length === options.assertLength,
            `Expected ${options.assertLength} extrinsics matching ${options?.module}.${options?.method}, but found ${matches.length}`
          );
        }

        if (options?.skipSuccessCheck !== true && options.checkTxPool !== true) {
          const events = await (await api.at(blockHash)).query.system.events();
          assertEventPresent(api, "system", "ExtrinsicSuccess", events);
        }

        return matches;
      }

      throw new Error(`No matching extrinsic found for ${options.module}.${options.method}`);
    } catch (error) {
      lastError = error as Error;
      if (i === iterations) {
        break;
      }
      await sleep(100);
    }
  }

  throw new Error(`Failed to find matching extrinsic after ${timeoutMs}ms: ${lastError?.message}`);
};

/**
 * Asserts that a specific event (module.method) is present in the provided list of events.
 *
 * @param api The API instance connected to the blockchain network.
 * @param module The module name of the event to check (e.g., "system").
 * @param method The method name of the event to check (e.g., "ExtrinsicSuccess").
 * @param events The list of events to search through. If not provided or empty, an error is thrown.
 * @returns An object containing the matching event and its data.
 * @throws Error if no matching event is found, or if the event does not match the expected structure.
 */
export const assertEventPresent = (
  api: ApiPromise,
  module: string,
  method: string,
  events?: EventRecord[]
) => {
  invariant(events && events.length > 0, "No events emitted in block");

  const event = events.find((e) => e.event.section === module && e.event.method === method);
  invariant(event !== undefined, `No events matching ${module}.${method}`);

  invariant(
    api.events[module][method].is(event.event),
    "Event doesn't match, should be caught by assert"
  );

  return { event: event.event, data: event.event.data };
};

/**
 * Asserts that multiple instances of a specific event (module.method) are present in the provided list of events.
 *
 * @param api The API instance connected to the blockchain network.
 * @param module The module name of the event to check (e.g., "system").
 * @param method The method name of the event to check (e.g., "ExtrinsicSuccess").
 * @param events The list of events to search through. If not provided or empty, an error is thrown.
 * @returns An array of matching events.
 * @throws Error if no matching events are found.
 */
export const assertEventMany = (
  api: ApiPromise,
  module: string,
  method: string,
  events?: EventRecord[]
) => {
  invariant(events && events.length > 0, "No events emitted in block");
  const matchingEvents = events.filter((event) => api.events[module][method].is(event.event));

  invariant(matchingEvents.length !== 0, `No events matching ${module}.${method} found`);

  return matchingEvents;
};

type EventData<T extends AugmentedEvent<"promise">> = T extends AugmentedEvent<"promise", infer D>
  ? D
  : never;

export const fetchEventData = <T extends AugmentedEvent<"promise">>(
  matcher: T,
  events?: EventRecord[]
): EventData<T> => {
  invariant(events && events.length > 0, "No events emitted in block");

  const eventRecord = events.find((e) => matcher.is(e.event));

  invariant(eventRecord !== undefined, `No event found for matcher, ${matcher.meta.name}`);

  const event = eventRecord.event;

  return event.data as unknown as EventData<T>;
};

/**
 * Wait some time before sealing a block and checking if the provider was slashed.
 * @param api
 * @param providerId
 */
export async function checkProviderWasSlashed(api: ApiPromise, providerId: string) {
  // Wait for provider to be slashed.
  // TODO Replace with poll
  await sleep(500);
  // await sealBlock(api);

  const [provider, _amountSlashed] = fetchEventData(
    api.events.providers.Slashed,
    await api.query.system.events()
  );

  invariant(provider.toString() === providerId, `Provider ${providerId} was not slashed`);
}
