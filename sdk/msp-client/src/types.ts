export enum HealthState {
  Healthy = "healthy",
  Unhealthy = "unhealthy",
  Degraded = "degraded",
  Unknown = "unknown"
}

export interface ComponentHealth {
  status: HealthState;
  details?: string;
  [k: string]: unknown;
}

export type HealthComponents = Record<string, ComponentHealth>;

export interface HealthStatus {
  status: HealthState;
  version?: string;
  service?: string;
  lastChecked?: string;
  components: HealthComponents;
  // Allow future changes in response without breaking the type
  [k: string]: unknown;
}

// Upload (doc-aligned) primitives
export type Hash = string; // 0x-prefixed hex string
export type CustomMetadata = Record<string, string>;
export enum Priority {
  Low = "low",
  Normal = "normal",
  High = "high"
}

export interface UploadProgress {
  uploadedChunks: number;
  totalChunks: number;
  uploadedBytes: number;
  totalBytes: number;
  speed: number; // bytes per second
  eta: number; // seconds remaining
}

export type UploadState = "staged" | "committed";

export interface UploadOptions {
  // Documented fields
  bucketId?: Hash;
  replicationFactor?: number;
  priority?: Priority;
  onProgress?: (progress: UploadProgress) => void;
  metadata?: CustomMetadata;
  mspDistribution: boolean;

  // Transport/HTTP-level fields (optional helpers)
  path?: string;
  checksumSha256?: string;
  owner?: string;
  idempotencyKey?: string;
  contentLength?: number;
  signal?: AbortSignal;
}

export interface UploadReceipt {
  status: string;
  fileKey: string;
  bucketId: string;
  fingerprint: string;
  location: string;
}

// Auth
export interface NonceResponse {
  message: string;
}

export interface VerifyResponse {
  token: string;
  user: {
    address: string;
    [k: string]: unknown;
  };
  [k: string]: unknown;
}

// Download
export interface DownloadOptions {
  range?: { start: number; end?: number };
  signal?: AbortSignal;
}

export interface DownloadResult {
  stream: ReadableStream<Uint8Array>;
  status: number;
  contentType?: string | null;
  contentLength?: number | null;
  contentRange?: string | null;
}

// Buckets and files
export interface Bucket {
  bucketId: string;
  name: string;
  root: string;
  isPublic: boolean;
  sizeBytes: number;
  valuePropId: string;
  fileCount: number;
}

export type FileTree = {
  name: string;
} & (
  | { type: "file"; sizeBytes: number; fileKey: string }
  | { type: "folder"; children: FileTree[] }
);

export interface FileListResponse {
  bucketId: string;
  files: FileTree[];
}

export interface GetFilesOptions {
  path?: string;
  signal?: AbortSignal;
}

// MSP info
export interface InfoResponse {
  client: string;
  version: string;
  mspId: string;
  multiaddresses: string[];
  ownerAccount: string;
  paymentAccount: string;
  status: string;
  activeSince: number;
  uptime: string;
}

export interface Capacity {
  totalBytes: number;
  availableBytes: number;
  usedBytes: number;
}

export interface StatsResponse {
  capacity: Capacity;
  activeUsers: number;
  lastCapacityChange: number;
  valuePropsAmount: number;
  bucketsAmount: number;
}

export interface ValueProp {
  id: string;
  pricePerGbBlock: number;
  dataLimitPerBucketBytes: number;
  isAvailable: boolean;
}

export interface FileInfo {
  fileKey: string;
  fingerprint: string;
  bucketId: string;
  name: string;
  location: string;
  size: number;
  isPublic: boolean;
  uploadedAt: Date;
}

// Payments
export type PaymentProviderType = "msp" | "bsp";

export interface PaymentStreamInfo {
  provider: string;
  providerType: PaymentProviderType;
  totalAmountPaid: string;
  costPerTick: string;
}

export interface PaymentStreamsResponse {
  streams: PaymentStreamInfo[];
}
