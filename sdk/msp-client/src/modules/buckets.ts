import type {
  Bucket,
  FileListResponse,
  GetFilesOptions,
  FileTree,
  FileStatus,
  ListBucketsByPage,
  ListBucketsInput
} from "../types.js";
import { ModuleBase } from "../base.js";
import { ensure0xPrefix, parseDate } from "@storagehub-sdk/core";

const BACKEND_MAX_BUCKETS_PER_PAGE = 500;

type BucketWire = Omit<Bucket, "bucketId" | "root" | "valuePropId"> & {
  bucketId: `0x${string}`;
  root: `0x${string}`;
  valuePropId: `0x${string}`;
};

type ListBucketsByPageWire = {
  buckets: BucketWire[];
  totalBuckets: string;
};

// Wire types received from backend JSON responses
type FileTreeWireFile = {
  name: string;
  type: "file";
  sizeBytes: number;
  fileKey: string; // may lack 0x
  status: FileStatus;
  uploadedAt: string; // ISO timestamp
};

type FileTreeWireFolder = {
  name: string;
  type: "folder";
  children?: readonly FileTreeWire[];
};

type FileTreeWire = FileTreeWireFile | FileTreeWireFolder;

type FileListResponseWire =
  | { bucketId: string; files: readonly FileTreeWire[] }
  | { bucketId: string; tree: FileTreeWireFolder };

function fixBucket(bucket: BucketWire): Bucket {
  return {
    bucketId: ensure0xPrefix(bucket.bucketId),
    name: bucket.name,
    root: ensure0xPrefix(bucket.root),
    isPublic: bucket.isPublic,
    sizeBytes: bucket.sizeBytes,
    valuePropId: ensure0xPrefix(bucket.valuePropId),
    fileCount: bucket.fileCount
  };
}

/** Recursively fix hex prefixes in FileTree structures */
function fixFileTree(item: FileTreeWire): FileTree {
  if (item.type === "file") {
    return {
      name: item.name,
      type: item.type,
      sizeBytes: item.sizeBytes,
      fileKey: ensure0xPrefix(item.fileKey),
      status: item.status,
      uploadedAt: parseDate(item.uploadedAt)
    };
  }
  return {
    name: item.name,
    type: item.type,
    children: (item.children ?? []).map(fixFileTree)
  };
}

export class BucketsModule extends ModuleBase {
  /**
   * List first page of buckets for the current authenticated user
   *
   * - `limit` defaults to 100
   * - `limit` is capped at 500 (backend max)
   */
  async listBuckets(options: ListBucketsInput): Promise<Bucket[]> {
    options.page = 0;
    const res = await this.listBucketsByPage(options);
    return res.buckets;
  }

  /** Fetch a single page of buckets using backend pagination (`page` + `limit`). */
  async listBucketsByPage(options: ListBucketsInput): Promise<ListBucketsByPage> {
    const limit = options.limit ?? 100;
    const page = options.page ?? 0;
    const headers = await this.withAuth();
    const cappedLimit = Math.min(Math.max(1, limit), BACKEND_MAX_BUCKETS_PER_PAGE);
    const safePage = Math.max(0, Math.floor(page));

    const wire = await this.ctx.http.get<ListBucketsByPageWire>("/buckets", {
      ...(headers ? { headers } : {}),
      ...(options.signal ? { signal: options.signal } : {}),
      query: { page: safePage, limit: cappedLimit }
    });

    const buckets = wire.buckets.map(fixBucket);
    return {
      buckets,
      page: safePage,
      limit: cappedLimit,
      totalBuckets: BigInt(wire.totalBuckets)
    };
  }

  /** Get a specific bucket's metadata by its bucket ID */
  async getBucket(bucketId: string, signal?: AbortSignal): Promise<Bucket> {
    const headers = await this.withAuth();
    const path = `/buckets/${encodeURIComponent(bucketId)}`;

    const wire = await this.ctx.http.get<BucketWire>(path, {
      ...(headers ? { headers } : {}),
      ...(signal ? { signal } : {})
    });

    return fixBucket(wire);
  }

  /** List files/folders under a path for a bucket (root if no path) */
  async getFiles(bucketId: string, options?: GetFilesOptions): Promise<FileListResponse> {
    const headers = await this.withAuth();
    const path = `/buckets/${encodeURIComponent(bucketId)}/files`;

    const wire = await this.ctx.http.get<FileListResponseWire>(path, {
      ...(headers ? { headers } : {}),
      ...(options?.signal ? { signal: options.signal } : {}),
      ...(options?.path ? { query: { path: this.normalizePath(options.path) } } : {})
    });

    const filesWire: readonly FileTreeWire[] = "files" in wire ? wire.files : [wire.tree];
    const files: FileTree[] = filesWire.map(fixFileTree);
    const tree = files[0];
    return {
      bucketId: ensure0xPrefix(wire.bucketId),
      files,
      ...(tree ? { tree } : {})
    } as unknown as FileListResponse;
  }
}
