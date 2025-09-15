/**
 * EVM client helpers (Core)
 *
 * Normalize how Core talks to an EVM endpoint in both environments (browser EIP‑1193 and Node HTTP)
 * and return viem clients for reads (public) and writes (wallet) with a minimal API.
 */

import { createPublicClient, createWalletClient, http, custom, type Chain, type Account, type EIP1193Provider } from 'viem';
import { privateKeyToAccount } from 'viem/accounts';

// Transport options (exclusive via nested union)
/**
 * HTTP transport configuration.
 * - Requires a JSON‑RPC URL and an explicit viem Chain.
 */
type HttpTransport = { httpUrl: string; chain: Chain };

/**
 * EIP‑1193 transport configuration (e.g., window.ethereum).
 * - Chain can be inferred from the provider; explicit Chain is not required.
 */
type Eip1193Transport = { eip1193: EIP1193Provider };

// Single options struct with nested, exclusive transport
export type EvmClientsOptions = {
  transport: HttpTransport | Eip1193Transport;
  // Account for writes
  account?: Account | `0x${string}`;
  // Optional network tuning
  timeoutMs?: number; // default HTTP timeout
};

/**
 * Returned clients:
 * - readClient: public client for reads/logs (always available)
 * - writeClient: wallet client for transactions (only when account is provided)
 */
export type EvmClients = {
  readClient: ReturnType<typeof createPublicClient>;
  writeClient: ReturnType<typeof createWalletClient> | undefined;
};

/**
 * Normalizes an optional account input into a viem Account.
 * - If a raw private key is provided, converts it via privateKeyToAccount.
 * - If a viem Account is provided, returns it as-is.
 * - If undefined, returns undefined.
 */
function resolveAccount(input?: Account | `0x${string}`): Account | undefined {
  if (!input) return undefined;
  if (typeof input === 'string') {
    // Treat as a raw private key
    return privateKeyToAccount(input);
  }
  return input;
}

/**
 * Factory to create EVM clients for Core.
 *
 * Transport:
 * - HTTP:   { transport: { httpUrl, chain }, account?, timeoutMs? }
 * - EIP1193:{ transport: { eip1193 },          account?, timeoutMs? }
 *
 * Returns:
 * - readClient: always present (public client)
 * - writeClient: present when an account is provided (wallet client)
 */
export function createEvmClients(opts: EvmClientsOptions): EvmClients {
  const { account: accountInput, timeoutMs = 30_000 } = opts;

  let transport: ReturnType<typeof http> | ReturnType<typeof custom>;
  let chain: Chain | undefined;

  if ('httpUrl' in opts.transport) {
    transport = http(opts.transport.httpUrl, { timeout: timeoutMs });
    chain = opts.transport.chain;
  } else if ('eip1193' in opts.transport) {
    transport = custom(opts.transport.eip1193);
    chain = undefined; // optional for EIP-1193; provider supplies chain context
  } else {
    throw new Error('createEvmClients: invalid transport');
  }

  // Public (read) client is always available
  const readClient = createPublicClient({ chain, transport });

  // Wallet (write) client only if an account is provided
  const account = resolveAccount(accountInput);
  const writeClient: ReturnType<typeof createWalletClient> | undefined = account
    ? createWalletClient({ chain, account, transport })
    : undefined;

  return { readClient, writeClient };
}


