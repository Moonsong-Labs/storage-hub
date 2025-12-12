/**
 * Progress reporting for network startup operations.
 *
 * Provides user feedback during long-running network launches (100+ nodes can
 * take minutes). Supports phase-based reporting with node-level granularity.
 */

/**
 * Interface for progress reporting callbacks.
 *
 * Implement this interface to customize progress output (e.g., JSON logs,
 * UI updates, metrics collection).
 */
export interface ProgressReporter {
  /**
   * Called when a startup phase begins.
   *
   * @param phase - Human-readable phase name (e.g., "Collators", "BSPs")
   * @param total - Total number of nodes in this phase
   */
  onPhaseStart(phase: string, total: number): void;

  /**
   * Called when a single node becomes ready.
   *
   * @param nodeType - Type of node (e.g., "bsp", "msp", "fisherman")
   * @param index - Index of the node within its type
   * @param total - Total nodes in this phase
   */
  onNodeReady(nodeType: string, index: number, total: number): void;

  /**
   * Called when a startup phase completes successfully.
   *
   * @param phase - Human-readable phase name
   * @param durationMs - Time taken for this phase in milliseconds
   */
  onPhaseComplete(phase: string, durationMs: number): void;

  /**
   * Called when a node fails to start.
   *
   * @param nodeType - Type of node that failed
   * @param index - Index of the failed node
   * @param error - Error that occurred
   */
  onError(nodeType: string, index: number, error: Error): void;
}

/**
 * Console-based progress reporter with formatted output.
 *
 * Outputs progress to stdout with:
 * - Phase banners
 * - Progress percentages
 * - Duration tracking
 * - Error highlighting
 *
 * @example
 * ```
 * [Phase: BSPs] Starting 100 nodes...
 *   [1/100] 1% - bsp-0 ready
 *   [2/100] 2% - bsp-1 ready
 *   ...
 *   [100/100] 100% - bsp-99 ready
 * [Phase: BSPs] Complete in 45.32s
 * ```
 */
export class ConsoleProgressReporter implements ProgressReporter {
  private phaseStartTimes = new Map<string, number>();

  onPhaseStart(phase: string, total: number): void {
    this.phaseStartTimes.set(phase, Date.now());
    console.log(`\n[Phase: ${phase}] Starting ${total} node(s)...`);
  }

  onNodeReady(nodeType: string, index: number, total: number): void {
    const current = index + 1;
    const percent = Math.round((current / total) * 100);
    console.log(`  [${current}/${total}] ${percent}% - ${nodeType}-${index} ready`);
  }

  onPhaseComplete(phase: string, durationMs: number): void {
    const durationSec = (durationMs / 1000).toFixed(2);
    console.log(`[Phase: ${phase}] Complete in ${durationSec}s`);
    this.phaseStartTimes.delete(phase);
  }

  onError(nodeType: string, index: number, error: Error): void {
    console.error(`  [ERROR] ${nodeType}-${index}: ${error.message}`);
  }
}

/**
 * Silent progress reporter that does nothing.
 *
 * Useful for automated tests where output should be suppressed.
 */
export class SilentProgressReporter implements ProgressReporter {
  onPhaseStart(_phase: string, _total: number): void {
    // Silent
  }

  onNodeReady(_nodeType: string, _index: number, _total: number): void {
    // Silent
  }

  onPhaseComplete(_phase: string, _durationMs: number): void {
    // Silent
  }

  onError(_nodeType: string, _index: number, _error: Error): void {
    // Silent - errors will still be thrown
  }
}

/**
 * Tracks execution time for a phase.
 */
export class PhaseTimer {
  private startTime: number;

  constructor() {
    this.startTime = Date.now();
  }

  /**
   * Gets elapsed time in milliseconds since timer creation.
   */
  elapsed(): number {
    return Date.now() - this.startTime;
  }
}
