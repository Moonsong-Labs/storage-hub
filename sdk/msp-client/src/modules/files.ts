import { ModuleBase } from "../base.js";
import type {
  DownloadOptions,
  DownloadResult,
  FileInfo,
  UploadOptions,
  UploadReceipt
} from "../types.js";
import { FileMetadata, FileTrie, initWasm } from "@storagehub-sdk/core";

export class FilesModule extends ModuleBase {
  /** Get metadata for a file in a bucket by fileKey */
  async getFileInfo(bucketId: string, fileKey: string, signal?: AbortSignal): Promise<FileInfo> {
    const headers = await this.withAuth();
    const path = `/buckets/${encodeURIComponent(bucketId)}/info/${encodeURIComponent(fileKey)}`;
    type FileInfoWire = Omit<FileInfo, "uploadedAt"> & { uploadedAt: string };
    const wire = await this.ctx.http.get<FileInfoWire>(path, {
      ...(headers ? { headers } : {}),
      ...(signal ? { signal } : {})
    });
    return {
      ...wire,
      uploadedAt: new Date(wire.uploadedAt)
    };
  }

  /** Upload a file to a bucket with a specific key */
  async uploadFile(
    bucketId: string,
    fileKey: string,
    file: Blob | ArrayBuffer | Uint8Array | ReadableStream<Uint8Array> | unknown,
    owner: string,
    location: string,
    _options?: UploadOptions
  ): Promise<UploadReceipt> {
    void _options;

    await initWasm();

    const backendPath = `/buckets/${encodeURIComponent(bucketId)}/upload/${encodeURIComponent(fileKey)}`;
    const authHeaders = await this.withAuth();

    // Convert the file to a blob and get its size
    const fileBlob = await this.coerceToFormPart(file);
    const fileSize = fileBlob.size;

    // Compute the fingerprint first
    const fingerprint = await this.computeFileFingerprint(fileBlob);

    // Create the FileMetadata instance
    const metadata = await this.formFileMetadata(
      owner,
      bucketId,
      location,
      fingerprint,
      BigInt(fileSize)
    );

    // Compute the file key and ensure it matches the provided file key
    const computedFileKey = await this.computeFileKey(metadata);
    const expectedFileKeyBytes = this.hexToBytes(fileKey);
    if (
      computedFileKey.length !== expectedFileKeyBytes.length ||
      !computedFileKey.every((byte, index) => byte === expectedFileKeyBytes[index])
    ) {
      throw new Error(
        `Computed file key ${computedFileKey.toString()} does not match provided file key ${expectedFileKeyBytes.toString()}`
      );
    }

    // Encode the file metadata
    const encodedMetadata = metadata.encode();

    // Create the multipart form with both the file and its metadata
    const form = new FormData();
    const fileMetadataBlob = new Blob([new Uint8Array(encodedMetadata)], {
      type: "application/octet-stream"
    });
    form.append("file_metadata", fileMetadataBlob, "file_metadata");
    form.append("file", fileBlob, "file");

    const res = await this.ctx.http.put<UploadReceipt>(
      backendPath,
      authHeaders
        ? { body: form as unknown as BodyInit, headers: authHeaders }
        : { body: form as unknown as BodyInit }
    );
    return res;
  }

  /** Download a file by key */
  async downloadFile(fileKey: string, options?: DownloadOptions): Promise<DownloadResult> {
    const path = `/download/${encodeURIComponent(fileKey)}`;
    const baseHeaders: Record<string, string> = { Accept: "*/*" };
    if (options?.range) {
      const { start, end } = options.range;
      const rangeValue = `bytes=${start}-${end ?? ""}`;
      baseHeaders.Range = rangeValue;
    }

    const headers = await this.withAuth(baseHeaders);

    try {
      const res = await this.ctx.http.getRaw(path, {
        ...(headers ? { headers } : {}),
        ...(options?.signal ? { signal: options.signal } : {})
      });

      if (!res.body) {
        throw new Error("Response body is null - unable to create stream");
      }

      const contentType = res.headers.get("content-type");
      const contentRange = res.headers.get("content-range");
      const contentLengthHeader = res.headers.get("content-length");
      const parsedLength = contentLengthHeader !== null ? Number(contentLengthHeader) : undefined;
      const contentLength =
        typeof parsedLength === "number" && Number.isFinite(parsedLength) ? parsedLength : null;

      return {
        stream: res.body,
        status: res.status,
        contentType,
        contentRange,
        contentLength
      };
    } catch (error) {
      // Handle HTTP errors by returning them as a DownloadResult with the error status
      if (
        error &&
        typeof error === "object" &&
        "status" in error &&
        typeof error.status === "number"
      ) {
        // Create an empty stream for error responses
        const emptyStream = new ReadableStream<Uint8Array>({
          start(controller) {
            controller.close();
          }
        });

        return {
          stream: emptyStream,
          status: error.status,
          contentType: null,
          contentRange: null,
          contentLength: null
        };
      }
      // Re-throw non-HTTP errors
      throw error;
    }
  }

  // Helpers
  private async coerceToFormPart(
    file: Blob | ArrayBuffer | Uint8Array | ReadableStream<Uint8Array> | unknown
  ): Promise<Blob> {
    if (typeof Blob !== "undefined" && file instanceof Blob) return file;
    if (file instanceof Uint8Array) return new Blob([file.buffer as ArrayBuffer]);
    if (typeof ArrayBuffer !== "undefined" && file instanceof ArrayBuffer) return new Blob([file]);

    // Handle ReadableStream by reading it into memory
    if (file instanceof ReadableStream) {
      const reader = file.getReader();
      const chunks: Uint8Array[] = [];
      let totalLength = 0;

      try {
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;
          if (value) {
            chunks.push(value);
            totalLength += value.length;
          }
        }
      } finally {
        reader.releaseLock();
      }

      // Combine all chunks into a single Uint8Array
      const combined = new Uint8Array(totalLength);
      let offset = 0;
      for (const chunk of chunks) {
        combined.set(chunk, offset);
        offset += chunk.length;
      }

      return new Blob([combined], { type: "application/octet-stream" });
    }

    return new Blob([file as BlobPart], { type: "application/octet-stream" });
  }

  private async computeFileFingerprint(fileBlob: Blob): Promise<Uint8Array> {
    const trie = new FileTrie();
    const fileBytes = new Uint8Array(await fileBlob.arrayBuffer());

    // Process the file in 1KB chunks (matching CHUNK_SIZE from constants)
    const CHUNK_SIZE = 1024;
    let offset = 0;

    while (offset < fileBytes.length) {
      const end = Math.min(offset + CHUNK_SIZE, fileBytes.length);
      const chunk = fileBytes.slice(offset, end);
      trie.push_chunk(chunk);
      offset = end;
    }

    return trie.get_root();
  }

  private async formFileMetadata(
    owner: string,
    bucketId: string,
    location: string,
    fingerprint: Uint8Array,
    size: bigint
  ): Promise<FileMetadata> {
    const ownerBytes = this.hexToBytes(owner);
    const bucketIdBytes = this.hexToBytes(bucketId);
    const locationBytes = new TextEncoder().encode(location);
    await initWasm();
    return new FileMetadata(ownerBytes, bucketIdBytes, locationBytes, size, fingerprint);
  }

  private hexToBytes(hex: string): Uint8Array {
    if (!hex) {
      throw new Error("hex string cannot be empty");
    }

    const cleanHex = hex.startsWith("0x") ? hex.slice(2) : hex;

    if (cleanHex.length % 2 !== 0) {
      throw new Error("hex string must have an even number of characters");
    }

    if (!/^[0-9a-fA-F]*$/.test(cleanHex)) {
      throw new Error("hex string contains invalid characters");
    }

    return new Uint8Array(cleanHex.match(/.{2}/g)?.map((byte) => Number.parseInt(byte, 16)) || []);
  }

  private async computeFileKey(fileMetadata: FileMetadata): Promise<Uint8Array> {
    await initWasm();
    return fileMetadata.getFileKey();
  }
}
