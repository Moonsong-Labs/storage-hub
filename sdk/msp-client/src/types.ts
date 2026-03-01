import type { FileInfo } from "@storagehub-sdk/core";

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

export enum AuthState {
  NotAuthenticated = "NotAuthenticated",
  TokenExpired = "TokenExpired",
  Authenticated = "Authenticated"
}

export interface AuthStatus {
  status: AuthState;
  [k: string]: unknown;
}

export interface UserInfo {
  address: string;
  ens: string;
}

export interface Session {
  token: string;
  user: {
    address: string;
    [k: string]: unknown;
  };
  [k: string]: unknown;
}

// Session provider function
export type SessionProvider = () => Promise<Readonly<Session> | undefined>;

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

export type ListBucketsInput = {
  limit?: number;
  page?: number;
  signal?: AbortSignal;
};

// Buckets and files
export interface Bucket {
  bucketId: `0x${string}`;
  name: string;
  root: `0x${string}`;
  isPublic: boolean;
  sizeBytes: number;
  valuePropId: string;
  fileCount: number;
}

export type FileStatus =
  | "inProgress"
  | "ready"
  | "expired"
  | "revoked"
  | "rejected"
  | "deletionInProgress";

export type FileTree = {
  name: string;
} & (
  | {
      type: "file";
      sizeBytes: number;
      fileKey: `0x${string}`;
      status: FileStatus;
      uploadedAt: Date;
    }
  | {
      type: "folder";
      children: FileTree[];
    }
);

export interface FileListResponse {
  bucketId: `0x${string}`;
  files: FileTree[];
}

export interface GetFilesOptions {
  path?: string;
  signal?: AbortSignal;
}

export interface ListBucketsByPage {
  buckets: Bucket[];
  /** Zero-based page index used for the request */
  page: number;
  /** Limit used for the request */
  limit: number;
  /** Total amount of buckets for the current authenticated user */
  totalBuckets: number;
}

// MSP info
export interface InfoResponse {
  client: string;
  version: string;
  mspId: `0x${string}`;
  multiaddresses: string[];
  ownerAccount: `0x${string}`;
  paymentAccount: `0x${string}`;
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

export interface StorageFileInfo extends FileInfo {
  isPublic: boolean;
  uploadedAt: Date;
  status: FileStatus;
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
