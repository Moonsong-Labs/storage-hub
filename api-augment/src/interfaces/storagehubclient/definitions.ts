export default {
  types: {
    FileMetadata: {
      owner: "Vec<u8>",
      bucket_id: "Vec<u8>",
      location: "Vec<u8>",
      size: "number",
      fingerprint: "[u8; 32]"
    }
  },
  rpc: {
    loadFileInStorage: {
      description:
        "Load a file in the local storage. This is the first step when uploading a file.",
      params: [
        {
          name: "file_path",
          type: "String"
        },
        {
          name: "location",
          type: "String"
        },
        {
          name: "owner",
          type: "AccountId32"
        },
        {
          name: "bucket_id",
          type: "H256"
        }
      ],
      type: "FileMetadata"
    },
    getForestRoot: {
      description: "Get the root of the forest trie.",
      params: [],
      type: "H256"
    }
  }
};
