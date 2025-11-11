import type { Bucket, FileListResponse, GetFilesOptions, FileTree, FileStatus } from "../types.js";
import { ModuleBase } from "../base.js";
import { ensure0xPrefix } from "@storagehub-sdk/core";

export class BucketsModule extends ModuleBase {
  /** Recursively fix hex prefixes in FileTree structures */
  private fixFileTree(item: FileTree): FileTree {
    if (item.type === "file") {
      return {
        name: item.name,
        type: item.type,
        sizeBytes: item.sizeBytes,
        fileKey: ensure0xPrefix(item.fileKey),
        status: item.status
      };
    }
    return {
      name: item.name,
      type: item.type,
      children: item.children?.map((child) => this.fixFileTree(child)) || []
    };
  }
  /** List all buckets for the current authenticated user */
  async listBuckets(signal?: AbortSignal): Promise<Bucket[]> {
    const headers = await this.withAuth();
    const wire = await this.ctx.http.get<Bucket[]>("/buckets", {
      ...(headers ? { headers } : {}),
      ...(signal ? { signal } : {})
    });

    return wire.map((bucket) => ({
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

    const wire = await this.ctx.http.get<FileListResponse>(path, {
      ...(headers ? { headers } : {}),
      ...(options?.signal ? { signal: options.signal } : {}),
      ...(options?.path ? { query: { path: this.normalizePath(options.path) } } : {})
    });

    return {
      bucketId: ensure0xPrefix(wire.bucketId),
      files: wire.files.map((file) => this.fixFileTree(file))
    };
  }
}
