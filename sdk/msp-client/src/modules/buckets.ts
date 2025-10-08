import type { Bucket, FileListResponse, GetFilesOptions } from "../types.js";
import { ModuleBase } from "../base.js";

export class BucketsModule extends ModuleBase {
  /** List all buckets for the current authenticated user */
  list(signal?: AbortSignal): Promise<Bucket[]> {
    const headers = this.withAuth();
    return this.ctx.http.get<Bucket[]>("/buckets", {
      ...(headers ? { headers } : {}),
      ...(signal ? { signal } : {})
    });
  }

  /** Get a specific bucket's metadata by its bucket ID */
  get(bucketId: string, signal?: AbortSignal): Promise<Bucket> {
    const headers = this.withAuth();
    const path = `/buckets/${encodeURIComponent(bucketId)}`;
    return this.ctx.http.get<Bucket>(path, {
      ...(headers ? { headers } : {}),
      ...(signal ? { signal } : {})
    });
  }

  /** List files/folders under a path for a bucket (root if no path) */
  getFiles(bucketId: string, options?: GetFilesOptions): Promise<FileListResponse> {
    const headers = this.withAuth();
    const path = `/buckets/${encodeURIComponent(bucketId)}/files`;
    return this.ctx.http.get<FileListResponse>(path, {
      ...(headers ? { headers } : {}),
      ...(options?.signal ? { signal: options.signal } : {}),
      ...(options?.path ? { query: { path: this.normalizePath(options.path) } } : {})
    });
  }
}
