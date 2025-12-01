import type { Bucket, FileListResponse, GetFilesOptions, FileTree, FileStatus } from "../types.js";
import { ModuleBase } from "../base.js";
import { ensure0xPrefix, parseDate } from "@storagehub-sdk/core";

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
  /** List all buckets for the current authenticated user */
  async listBuckets(signal?: AbortSignal): Promise<Bucket[]> {
    const headers = await this.withAuth();
    const wire = await this.ctx.http.get<Bucket[]>("/buckets", {
      ...(headers ? { headers } : {}),
      ...(signal ? { signal } : {})
    });

    return wire.map((bucket: Bucket) => ({
      bucketId: ensure0xPrefix(bucket.bucketId),
      name: bucket.name,
      root: ensure0xPrefix(bucket.root),
      isPublic: bucket.isPublic,
      sizeBytes: bucket.sizeBytes,
      valuePropId: ensure0xPrefix(bucket.valuePropId),
      fileCount: bucket.fileCount
    }));
  }

  /** Get a specific bucket's metadata by its bucket ID */
  async getBucket(bucketId: string, signal?: AbortSignal): Promise<Bucket> {
    const headers = await this.withAuth();
    const path = `/buckets/${encodeURIComponent(bucketId)}`;

    const wire = await this.ctx.http.get<Bucket>(path, {
      ...(headers ? { headers } : {}),
      ...(signal ? { signal } : {})
    });

    return {
      bucketId: ensure0xPrefix(wire.bucketId),
      name: wire.name,
      root: ensure0xPrefix(wire.root),
      isPublic: wire.isPublic,
      sizeBytes: wire.sizeBytes,
      valuePropId: ensure0xPrefix(wire.valuePropId),
      fileCount: wire.fileCount
    };
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
