/**
 * Base class for network contexts.
 *
 * Provides shared functionality that both NetworkLauncher and DynamicNetworkContext
 * inherit. Methods here are the actual implementations, not wrappers.
 *
 * Usage pattern:
 * ```ts
 * const api = await network.getBlockProducerApi();
 * await network.setupRuntimeParams(api);
 * await network.preFundAccounts(api);
 * ```
 */

import type { EnrichedBspApi } from "../bspNet/test-api";
import { MILLIUNIT, UNIT } from "../constants";

/**
 * Runtime parameter configuration for test networks.
 *
 * These values are optimized for testing scenarios with fast feedback loops.
 */
export interface RuntimeParamsConfig {
  /** Slash amount per max file size (default: 20 MILLIUNIT) */
  slashAmountPerMaxFileSize?: bigint;
  /** Stake to challenge period ratio (default: 1000 UNIT) */
  stakeToChallengePeriod?: bigint;
  /** Checkpoint challenge period in blocks (default: 10) */
  checkpointChallengePeriod?: number;
  /** Minimum challenge period in blocks (default: 5) */
  minChallengePeriod?: number;
  /** Basic replication target (default: 3) */
  basicReplicationTarget?: number;
  /** Maximum replication target (default: 9) */
  maxReplicationTarget?: number;
  /** Tick range to maximum threshold (default: 10) */
  tickRangeToMaximumThreshold?: number;
  /** Minimum wait for stop storing in blocks (default: 15) */
  minWaitForStopStoring?: number;
  /** Storage request TTL in blocks (default: 20) */
  storageRequestTtl?: number;
}

/**
 * Default runtime parameters optimized for testing.
 */
export const DEFAULT_RUNTIME_PARAMS: Required<RuntimeParamsConfig> = {
  slashAmountPerMaxFileSize: 20n * MILLIUNIT,
  stakeToChallengePeriod: 1000n * UNIT,
  checkpointChallengePeriod: 10,
  minChallengePeriod: 5,
  basicReplicationTarget: 3,
  maxReplicationTarget: 9,
  tickRangeToMaximumThreshold: 10,
  minWaitForStopStoring: 15,
  storageRequestTtl: 20
};

/**
 * Default funding amount: 10,000 UNITS
 */
export const DEFAULT_FUND_AMOUNT = 10000n * 10n ** 12n;

/**
 * Abstract base class for network contexts.
 *
 * Both NetworkLauncher and DynamicNetworkContext extend this class
 * to inherit shared functionality with identical implementations.
 */
export abstract class BaseNetworkContext {
  /**
   * Runtime type for this network.
   *
   * Determines which chain specification to use:
   * - "parachain": Polkadot parachain runtime (default)
   * - "solochain": Solochain EVM runtime
   */
  abstract readonly runtimeType: "parachain" | "solochain";

  /**
   * Returns all account addresses that should be pre-funded.
   *
   * Override in subclasses to provide the appropriate addresses:
   * - NetworkLauncher: standard test accounts (bspKey, shUser, etc.)
   * - DynamicNetworkContext: dynamically generated node accounts
   */
  protected abstract getAccountsToFund(api: EnrichedBspApi): string[];

  /**
   * Cleans up all network resources.
   *
   * Must disconnect API connections and stop containers.
   * Should be called in test cleanup (e.g., after() hooks).
   */
  abstract cleanup(): Promise<void>;

  /**
   * Pre-funds all network accounts using sudo.
   *
   * Uses `balances.forceSetBalance` via sudo to set account balances directly.
   * All funding transactions are batched into a single block for efficiency.
   *
   * @param api - API with sudo access (typically block producer)
   * @param amount - Amount to set for each account (default: 10,000 UNITS)
   */
  async preFundAccounts(api: EnrichedBspApi, amount: bigint = DEFAULT_FUND_AMOUNT): Promise<void> {
    const addresses = this.getAccountsToFund(api);
    if (addresses.length === 0) return;

    const sudo = api.accounts.sudo;
    const nonce = await api.rpc.system.accountNextIndex(sudo.address);
    const startNonce = nonce.toNumber();

    const signedCalls = await Promise.all(
      addresses.map((addr, i) =>
        api.tx.sudo
          .sudo(api.tx.balances.forceSetBalance(addr, amount))
          .signAsync(sudo, { nonce: startNonce + i })
      )
    );

    await api.block.seal({ calls: signedCalls });
  }

  /**
   * Sets up runtime parameters for optimal test conditions.
   *
   * Configures the chain's runtime parameters to values suitable for testing.
   * Each parameter is set in a separate block to ensure changes take effect.
   *
   * @param api - API with sudo access (typically block producer)
   * @param config - Optional custom configuration (uses defaults if not specified)
   */
  async setupRuntimeParams(api: EnrichedBspApi, config: RuntimeParamsConfig = {}): Promise<void> {
    const params = { ...DEFAULT_RUNTIME_PARAMS, ...config };

    // SlashAmountPerMaxFileSize
    await api.block.seal({
      calls: [
        api.tx.sudo.sudo(
          api.tx.parameters.setParameter({
            RuntimeConfig: {
              SlashAmountPerMaxFileSize: [null, params.slashAmountPerMaxFileSize]
            }
          })
        )
      ]
    });

    // StakeToChallengePeriod
    await api.block.seal({
      calls: [
        api.tx.sudo.sudo(
          api.tx.parameters.setParameter({
            RuntimeConfig: {
              StakeToChallengePeriod: [null, params.stakeToChallengePeriod]
            }
          })
        )
      ]
    });

    // CheckpointChallengePeriod
    await api.block.seal({
      calls: [
        api.tx.sudo.sudo(
          api.tx.parameters.setParameter({
            RuntimeConfig: {
              CheckpointChallengePeriod: [null, params.checkpointChallengePeriod]
            }
          })
        )
      ]
    });

    // MinChallengePeriod
    await api.block.seal({
      calls: [
        api.tx.sudo.sudo(
          api.tx.parameters.setParameter({
            RuntimeConfig: {
              MinChallengePeriod: [null, params.minChallengePeriod]
            }
          })
        )
      ]
    });

    // BasicReplicationTarget
    await api.block.seal({
      calls: [
        api.tx.sudo.sudo(
          api.tx.parameters.setParameter({
            RuntimeConfig: {
              BasicReplicationTarget: [null, params.basicReplicationTarget]
            }
          })
        )
      ]
    });

    // MaxReplicationTarget
    await api.block.seal({
      calls: [
        api.tx.sudo.sudo(
          api.tx.parameters.setParameter({
            RuntimeConfig: {
              MaxReplicationTarget: [null, params.maxReplicationTarget]
            }
          })
        )
      ]
    });

    // TickRangeToMaximumThreshold
    await api.block.seal({
      calls: [
        api.tx.sudo.sudo(
          api.tx.parameters.setParameter({
            RuntimeConfig: {
              TickRangeToMaximumThreshold: [null, params.tickRangeToMaximumThreshold]
            }
          })
        )
      ]
    });

    // MinWaitForStopStoring
    await api.block.seal({
      calls: [
        api.tx.sudo.sudo(
          api.tx.parameters.setParameter({
            RuntimeConfig: {
              MinWaitForStopStoring: [null, params.minWaitForStopStoring]
            }
          })
        )
      ]
    });

    // StorageRequestTtl
    await api.block.seal({
      calls: [
        api.tx.sudo.sudo(
          api.tx.parameters.setParameter({
            RuntimeConfig: {
              StorageRequestTtl: [null, params.storageRequestTtl]
            }
          })
        )
      ]
    });
  }
}
