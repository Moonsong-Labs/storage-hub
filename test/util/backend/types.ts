// Backend API Types
// TODO: Add a script in the backend to generate these types instead.

export interface ComponentHealth {
  status: string;
  message?: string;
}

export interface HealthComponents {
  storage: ComponentHealth;
  postgres: ComponentHealth;
  rpc: ComponentHealth;
}

export interface HealthResponse {
  status: string;
  version: string;
  service: string;
  components: HealthComponents;
}

export interface Bucket {
  bucketId: string;
  name: string;
  root: string;
  is_public: boolean;
  size_bytes: number;
  value_prop_id: string;
  file_count: number;
}

export type FileTree = {
  name: string;
} & (
  | {
      type: "file";
      sizeBytes: number;
      fileKey: string;
    }
  | {
      type: "folder";
      children: FileTree[];
    }
);

export interface FileListResponse {
  bucketId: string;
  files: FileTree[];
}

export interface FileInfo {
  fileKey: string;
  fingerprint: string;
  bucketId: string;
  location: string;
  size: number;
  isPublic: boolean;
  uploadedAt: string;
}

export interface PaymentStream {
  provider: string;
  provider_type: "msp" | "bsp";
  total_amount_paid: string;
  cost_per_tick: string;
}

export interface PaymentStreamsResponse {
  streams: PaymentStream[];
}
