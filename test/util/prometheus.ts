/**
 * Prometheus utilities for integration tests.
 *
 * This module provides shared functions and types for querying and asserting
 * Prometheus metrics in StorageHub integration tests.
 */
import assert from "node:assert";
import { NODE_INFOS } from "./bspNet/consts";

/** Default Prometheus server URL for tests. */
export const PROMETHEUS_URL = `http://localhost:${NODE_INFOS.prometheus.port}`;

/** Default Grafana server URL for tests. */
export const GRAFANA_URL = `http://localhost:${NODE_INFOS.grafana.port}`;

/**
 * Result from a Prometheus instant query.
 *
 * @see https://prometheus.io/docs/prometheus/latest/querying/api/#instant-queries
 */
export interface PrometheusQueryResult {
  status: string;
  data: {
    resultType: string;
    result: Array<{
      metric: Record<string, string>;
      value?: [number, string];
      values?: Array<[number, string]>;
    }>;
  };
}

/**
 * Result from the Prometheus targets API.
 *
 * @see https://prometheus.io/docs/prometheus/latest/querying/api/#targets
 */
export interface PrometheusTargetsResult {
  status: string;
  data: {
    activeTargets: Array<{
      labels: Record<string, string>;
      scrapeUrl: string;
      health: string;
      lastScrape: string;
    }>;
  };
}

/**
 * Definition for a StorageHub metric.
 */
export interface MetricDefinition {
  /** Full Prometheus metric name (e.g., "storagehub_bsp_storage_requests_total"). */
  name: string;
  /** Metric type: "counter", "gauge", or "histogram". */
  type: "counter" | "gauge" | "histogram";
  /** Labels associated with this metric. */
  labels: string[];
  /** Human-readable description. */
  description: string;
}

/**
 * All StorageHub custom metrics as defined in client/src/metrics.rs.
 *
 * This catalog serves as the canonical TypeScript definition for all metrics
 * exposed by StorageHub nodes.
 */
export const ALL_STORAGEHUB_METRICS: Record<string, MetricDefinition> = {
  // BSP Counters
  bsp_storage_requests_total: {
    name: "storagehub_bsp_storage_requests_total",
    type: "counter",
    labels: ["status"],
    description: "BSP storage request processing"
  },
  bsp_proofs_submitted_total: {
    name: "storagehub_bsp_proofs_submitted_total",
    type: "counter",
    labels: ["status"],
    description: "BSP proof submissions"
  },
  bsp_fees_charged_total: {
    name: "storagehub_bsp_fees_charged_total",
    type: "counter",
    labels: ["status"],
    description: "BSP fee charging events"
  },
  bsp_files_deleted_total: {
    name: "storagehub_bsp_files_deleted_total",
    type: "counter",
    labels: ["status"],
    description: "BSP file deletion events"
  },
  bsp_bucket_moves_total: {
    name: "storagehub_bsp_bucket_moves_total",
    type: "counter",
    labels: ["status"],
    description: "BSP bucket move events"
  },
  bsp_download_requests_total: {
    name: "storagehub_bsp_download_requests_total",
    type: "counter",
    labels: ["status"],
    description: "BSP download request handling"
  },

  // MSP Counters
  msp_storage_requests_total: {
    name: "storagehub_msp_storage_requests_total",
    type: "counter",
    labels: ["status"],
    description: "MSP storage request processing"
  },
  msp_files_distributed_total: {
    name: "storagehub_msp_files_distributed_total",
    type: "counter",
    labels: ["status"],
    description: "MSP file distribution events"
  },
  msp_files_deleted_total: {
    name: "storagehub_msp_files_deleted_total",
    type: "counter",
    labels: ["status"],
    description: "MSP file deletion events"
  },
  msp_buckets_deleted_total: {
    name: "storagehub_msp_buckets_deleted_total",
    type: "counter",
    labels: ["status"],
    description: "MSP bucket deletion events"
  },
  msp_fees_charged_total: {
    name: "storagehub_msp_fees_charged_total",
    type: "counter",
    labels: ["status"],
    description: "MSP fee charging events"
  },
  msp_bucket_moves_total: {
    name: "storagehub_msp_bucket_moves_total",
    type: "counter",
    labels: ["status"],
    description: "MSP bucket move events"
  },

  // Other Counters
  insolvent_users_processed_total: {
    name: "storagehub_insolvent_users_processed_total",
    type: "counter",
    labels: ["status"],
    description: "Insolvent user processing events"
  },
  fisherman_batch_deletions_total: {
    name: "storagehub_fisherman_batch_deletions_total",
    type: "counter",
    labels: ["status"],
    description: "Fisherman batch deletion events"
  },

  // Histograms
  bsp_proof_generation_seconds: {
    name: "storagehub_bsp_proof_generation_seconds",
    type: "histogram",
    labels: ["status"],
    description: "BSP proof generation duration"
  },
  storage_request_seconds: {
    name: "storagehub_storage_request_seconds",
    type: "histogram",
    labels: ["status"],
    description: "Storage request processing duration"
  },
  file_transfer_seconds: {
    name: "storagehub_file_transfer_seconds",
    type: "histogram",
    labels: ["status"],
    description: "File transfer duration"
  },

  // Download Metrics
  bytes_downloaded_total: {
    name: "storagehub_bytes_downloaded_total",
    type: "counter",
    labels: ["status"],
    description: "Total bytes downloaded from peers"
  },
  chunks_downloaded_total: {
    name: "storagehub_chunks_downloaded_total",
    type: "counter",
    labels: ["status"],
    description: "Total chunks downloaded from peers"
  },
  file_download_seconds: {
    name: "storagehub_file_download_seconds",
    type: "histogram",
    labels: ["status"],
    description: "File download duration"
  }
};

/**
 * Query the Prometheus API for a specific metric.
 *
 * @param query - PromQL query string
 * @returns Prometheus query result
 * @throws Error if the query fails
 *
 * @example
 * ```typescript
 * const result = await queryPrometheus('storagehub_bsp_storage_requests_total{job="storagehub-bsp"}');
 * ```
 */
export async function queryPrometheus(query: string): Promise<PrometheusQueryResult> {
  const response = await fetch(`${PROMETHEUS_URL}/api/v1/query?query=${encodeURIComponent(query)}`);
  if (!response.ok) {
    throw new Error(`Prometheus query failed: ${response.statusText}`);
  }
  return (await response.json()) as PrometheusQueryResult;
}

/**
 * Get the current value of a metric from Prometheus.
 *
 * @param query - PromQL query string
 * @returns Numeric value of the metric, or 0 if not found
 *
 * @example
 * ```typescript
 * const value = await getMetricValue('storagehub_bsp_storage_requests_total{status="success"}');
 * ```
 */
export async function getMetricValue(query: string): Promise<number> {
  const result = await queryPrometheus(query);
  if (result.status !== "success" || result.data.result.length === 0) {
    return 0;
  }
  return Number.parseFloat(result.data.result[0].value?.[1] ?? "0");
}

/**
 * Get the targets that Prometheus is currently scraping.
 *
 * @returns Prometheus targets result with active scrape targets
 * @throws Error if the request fails
 */
export async function getPrometheusTargets(): Promise<PrometheusTargetsResult> {
  const response = await fetch(`${PROMETHEUS_URL}/api/v1/targets`);
  if (!response.ok) {
    throw new Error(`Failed to get Prometheus targets: ${response.statusText}`);
  }
  return (await response.json()) as PrometheusTargetsResult;
}

/**
 * Wait for Prometheus to scrape and reflect updated metrics.
 *
 * Prometheus scrapes every 5 seconds by default, so this waits 7 seconds
 * to ensure metrics are updated.
 */
export async function waitForMetricsScrape(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 7000));
}

/**
 * Wait for the Prometheus server to become ready.
 *
 * Polls the `/-/ready` endpoint up to 30 times with 2 second intervals (60 seconds total).
 *
 * @throws Error if Prometheus does not become ready within the timeout
 */
export async function waitForPrometheusReady(): Promise<void> {
  for (let i = 0; i < 30; i++) {
    try {
      const response = await fetch(`${PROMETHEUS_URL}/-/ready`);
      if (response.ok) return;
    } catch {
      // Prometheus not ready yet
    }
    await new Promise((resolve) => setTimeout(resolve, 2000));
  }
  throw new Error("Prometheus did not become ready in time");
}

// ============================================================================
// Assertion Helpers
// ============================================================================

/**
 * Options for metric increment assertion.
 */
export interface AssertMetricIncrementedOptions {
  /** PromQL query string. */
  query: string;
  /** Initial value before the operation. */
  initialValue: number;
  /** Optional custom assertion message. */
  message?: string;
}

/**
 * Assert that a metric has incremented from an initial value.
 *
 * Waits for Prometheus to scrape metrics before checking.
 *
 * @example
 * ```typescript
 * const initial = await api.prometheus.getMetricValue('storagehub_bsp_storage_requests_total');
 * // ... perform operation ...
 * await api.prometheus.assertMetricIncremented({
 *   query: 'storagehub_bsp_storage_requests_total',
 *   initialValue: initial
 * });
 * ```
 */
export async function assertMetricIncremented(
  options: AssertMetricIncrementedOptions
): Promise<void> {
  await waitForMetricsScrape();
  const currentValue = await getMetricValue(options.query);
  const message =
    options.message ?? `Expected metric ${options.query} to increment from ${options.initialValue}`;
  assert(currentValue > options.initialValue, `${message} (got ${currentValue})`);
}

/**
 * Options for metric threshold assertion.
 */
export interface AssertMetricAboveOptions {
  /** PromQL query string. */
  query: string;
  /** Threshold value that the metric must exceed. */
  threshold: number;
  /** Optional custom assertion message. */
  message?: string;
}

/**
 * Assert that a metric is above a threshold.
 *
 * Waits for Prometheus to scrape metrics before checking.
 *
 * @example
 * ```typescript
 * await api.prometheus.assertMetricAbove({
 *   query: 'storagehub_bsp_storage_requests_total{status="success"}',
 *   threshold: 5
 * });
 * ```
 */
export async function assertMetricAbove(options: AssertMetricAboveOptions): Promise<void> {
  await waitForMetricsScrape();
  const currentValue = await getMetricValue(options.query);
  const message =
    options.message ?? `Expected metric ${options.query} to be above ${options.threshold}`;
  assert(currentValue > options.threshold, `${message} (got ${currentValue})`);
}

/**
 * Options for metric equality assertion.
 */
export interface AssertMetricEqualsOptions {
  /** PromQL query string. */
  query: string;
  /** Expected value. */
  expected: number;
  /** Optional custom assertion message. */
  message?: string;
}

/**
 * Assert that a metric equals an expected value.
 *
 * Waits for Prometheus to scrape metrics before checking.
 *
 * @example
 * ```typescript
 * await api.prometheus.assertMetricEquals({
 *   query: 'storagehub_files_stored',
 *   expected: 10
 * });
 * ```
 */
export async function assertMetricEquals(options: AssertMetricEqualsOptions): Promise<void> {
  await waitForMetricsScrape();
  const currentValue = await getMetricValue(options.query);
  const message =
    options.message ?? `Expected metric ${options.query} to equal ${options.expected}`;
  assert.strictEqual(currentValue, options.expected, message);
}
