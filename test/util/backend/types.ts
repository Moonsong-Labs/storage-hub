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
  isPublic: boolean;
  sizeBytes: number;
  valuePropId: string;
  fileCount: number;
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
    }
);

export interface FileListResponse {
  bucketId: string;
  tree: {
    name: string;
    children: FileTree[];
  };
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
  providerType: "msp" | "bsp";
  totalAmountPaid: string;
  costPerTick: string;
}

export interface PaymentStreamsResponse {
  streams: PaymentStream[];
}
