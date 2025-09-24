import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { FileTrie } from "@storagehub/wasm";

const CHUNK_SIZE = 1024;

const resource = (name: string) => readFileSync(join(__dirname, "../../../docker/resource", name));

const TEST_FILES = [
  {
    filename: "adolphus.jpg",
    rootHash: "34eb5f637e05fc18f857ccb013250076534192189894d174ee3aa6d3525f6970"
  },
  {
    filename: "smile.jpg",
    rootHash: "535dd863026735ffe0919cc0fc3d8e5da45b9203f01fbf014dbe98005bd8d2fe"
  }
];

describe("FileTrie root hashes (WASM)", () => {
  TEST_FILES.forEach(({ filename, rootHash }) => {
    it(`${filename} root matches`, () => {
      const bytes = resource(filename);
      const trie = new FileTrie();
      for (let offset = 0; offset < bytes.length; offset += CHUNK_SIZE) {
        const chunk = bytes.subarray(offset, Math.min(offset + CHUNK_SIZE, bytes.length));
        trie.push_chunk(chunk);
      }
      const fingerprint = Buffer.from(trie.get_root()).toString("hex");
      expect(fingerprint).toBe(rootHash);
    });
  });
});
