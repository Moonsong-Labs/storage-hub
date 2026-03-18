import { describe, it, expect } from "vitest";
import { StorageHubClient } from "../src/evm/storageHubClient.js";
import { FileOperation } from "../src/evm/types.js";

const TEST_FILE_KEY =
  "0x93c7637a94182998665e26786728f4c52eaf612df3d7b3d54022549a08995d61" as const;
const TEST_BUCKET_ID =
  "0xd99fa5fe0c6bceee920aa457feb3e67702c24ecb6f6da10935501dc04b4b6cd8" as const;

describe("StorageHubClient.buildIntentionMessage", () => {
  it("produces the correct human-readable message for a delete operation", () => {
    const msg = StorageHubClient.buildIntentionMessage(
      TEST_FILE_KEY,
      FileOperation.Delete,
      "documents/report.pdf",
      TEST_BUCKET_ID,
      1048576n
    );

    const expected = [
      "StorageHub File Deletion Request",
      "",
      "File: documents/report.pdf",
      "Size: 1048576 bytes",
      `Bucket: ${TEST_BUCKET_ID}`,
      `File Key: ${TEST_FILE_KEY}`,
      "Action: Delete"
    ].join("\n");

    expect(msg).toBe(expected);
  });

  it("lowercases file key and bucket id", () => {
    const mixedCaseFileKey =
      "0x93C7637A94182998665E26786728F4C52EAF612DF3D7B3D54022549A08995D61" as `0x${string}`;
    const mixedCaseBucketId =
      "0xD99FA5FE0C6BCEEE920AA457FEB3E67702C24ECB6F6DA10935501DC04B4B6CD8" as `0x${string}`;

    const msg = StorageHubClient.buildIntentionMessage(
      mixedCaseFileKey,
      FileOperation.Delete,
      "file.txt",
      mixedCaseBucketId,
      100n
    );

    expect(msg).toContain(`Bucket: ${mixedCaseBucketId.toLowerCase()}`);
    expect(msg).toContain(`File Key: ${mixedCaseFileKey.toLowerCase()}`);
  });

  it("throws on unknown operation", () => {
    expect(() =>
      StorageHubClient.buildIntentionMessage(
        TEST_FILE_KEY,
        999 as FileOperation,
        "file.txt",
        TEST_BUCKET_ID,
        100n
      )
    ).toThrow("Unknown file operation");
  });

  it("handles zero size and empty location", () => {
    const msg = StorageHubClient.buildIntentionMessage(
      TEST_FILE_KEY,
      FileOperation.Delete,
      "",
      TEST_BUCKET_ID,
      0n
    );

    expect(msg).toContain("Size: 0 bytes");
    expect(msg).toContain("File: \n");
  });
});
