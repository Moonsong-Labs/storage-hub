import type { Abi } from "viem";

export const filesystemAbi = [
  {
    anonymous: false,
    inputs: [
      {
        indexed: true,
        internalType: "address",
        name: "who",
        type: "address"
      },
      {
        indexed: true,
        internalType: "bytes32",
        name: "bucketId",
        type: "bytes32"
      },
      {
        indexed: true,
        internalType: "bytes32",
        name: "mspId",
        type: "bytes32"
      }
    ],
    name: "BucketCreated",
    type: "event"
  },
  {
    anonymous: false,
    inputs: [
      {
        indexed: true,
        internalType: "address",
        name: "who",
        type: "address"
      },
      {
        indexed: true,
        internalType: "bytes32",
        name: "bucketId",
        type: "bytes32"
      }
    ],
    name: "BucketDeleted",
    type: "event"
  },
  {
    anonymous: false,
    inputs: [
      {
        indexed: true,
        internalType: "address",
        name: "who",
        type: "address"
      },
      {
        indexed: true,
        internalType: "bytes32",
        name: "bucketId",
        type: "bytes32"
      },
      {
        indexed: true,
        internalType: "bytes32",
        name: "newMspId",
        type: "bytes32"
      }
    ],
    name: "BucketMoveRequested",
    type: "event"
  },
  {
    anonymous: false,
    inputs: [
      {
        indexed: true,
        internalType: "address",
        name: "who",
        type: "address"
      },
      {
        indexed: true,
        internalType: "bytes32",
        name: "bucketId",
        type: "bytes32"
      },
      {
        indexed: false,
        internalType: "bool",
        name: "_private",
        type: "bool"
      }
    ],
    name: "BucketPrivacyUpdated",
    type: "event"
  },
  {
    anonymous: false,
    inputs: [
      {
        indexed: true,
        internalType: "address",
        name: "who",
        type: "address"
      },
      {
        indexed: true,
        internalType: "bytes32",
        name: "bucketId",
        type: "bytes32"
      },
      {
        indexed: true,
        internalType: "bytes32",
        name: "collectionId",
        type: "bytes32"
      }
    ],
    name: "CollectionCreated",
    type: "event"
  },
  {
    anonymous: false,
    inputs: [
      {
        indexed: true,
        internalType: "bytes32",
        name: "fileKey",
        type: "bytes32"
      },
      {
        indexed: true,
        internalType: "address",
        name: "owner",
        type: "address"
      }
    ],
    name: "FileDeletionRequested",
    type: "event"
  },
  {
    anonymous: false,
    inputs: [
      {
        indexed: true,
        internalType: "address",
        name: "who",
        type: "address"
      },
      {
        indexed: true,
        internalType: "bytes32",
        name: "fileKey",
        type: "bytes32"
      },
      {
        indexed: true,
        internalType: "bytes32",
        name: "bucketId",
        type: "bytes32"
      }
    ],
    name: "StorageRequestIssued",
    type: "event"
  },
  {
    anonymous: false,
    inputs: [
      {
        indexed: true,
        internalType: "bytes32",
        name: "fileKey",
        type: "bytes32"
      }
    ],
    name: "StorageRequestRevoked",
    type: "event"
  },
  {
    inputs: [
      {
        internalType: "bytes32",
        name: "bucketId",
        type: "bytes32"
      }
    ],
    name: "createAndAssociateCollectionWithBucket",
    outputs: [],
    stateMutability: "nonpayable",
    type: "function"
  },
  {
    inputs: [
      {
        internalType: "bytes32",
        name: "mspId",
        type: "bytes32"
      },
      {
        internalType: "bytes",
        name: "name",
        type: "bytes"
      },
      {
        internalType: "bool",
        name: "_private",
        type: "bool"
      },
      {
        internalType: "bytes32",
        name: "valuePropId",
        type: "bytes32"
      }
    ],
    name: "createBucket",
    outputs: [],
    stateMutability: "nonpayable",
    type: "function"
  },
  {
    inputs: [
      {
        internalType: "bytes32",
        name: "bucketId",
        type: "bytes32"
      }
    ],
    name: "deleteBucket",
    outputs: [],
    stateMutability: "nonpayable",
    type: "function"
  },
  {
    inputs: [
      {
        internalType: "address",
        name: "owner",
        type: "address"
      },
      {
        internalType: "bytes",
        name: "name",
        type: "bytes"
      }
    ],
    name: "deriveBucketId",
    outputs: [
      {
        internalType: "bytes32",
        name: "bucketId",
        type: "bytes32"
      }
    ],
    stateMutability: "view",
    type: "function"
  },
  {
    inputs: [
      {
        internalType: "address",
        name: "user",
        type: "address"
      }
    ],
    name: "getPendingFileDeletionRequestsCount",
    outputs: [
      {
        internalType: "uint32",
        name: "count",
        type: "uint32"
      }
    ],
    stateMutability: "view",
    type: "function"
  },
  {
    inputs: [
      {
        internalType: "bytes32",
        name: "bucketId",
        type: "bytes32"
      },
      {
        internalType: "bytes",
        name: "location",
        type: "bytes"
      },
      {
        internalType: "bytes32",
        name: "fingerprint",
        type: "bytes32"
      },
      {
        internalType: "uint64",
        name: "size",
        type: "uint64"
      },
      {
        internalType: "bytes32",
        name: "mspId",
        type: "bytes32"
      },
      {
        internalType: "bytes[]",
        name: "peerIds",
        type: "bytes[]"
      },
      {
        internalType: "enum FileSystem.ReplicationTarget",
        name: "replicationTarget",
        type: "uint8"
      },
      {
        internalType: "uint32",
        name: "customReplicationTarget",
        type: "uint32"
      }
    ],
    name: "issueStorageRequest",
    outputs: [],
    stateMutability: "nonpayable",
    type: "function"
  },
  {
    inputs: [
      {
        components: [
          {
            internalType: "bytes32",
            name: "fileKey",
            type: "bytes32"
          },
          {
            internalType: "enum FileSystem.FileOperation",
            name: "operation",
            type: "uint8"
          }
        ],
        internalType: "struct FileSystem.FileOperationIntention",
        name: "signedIntention",
        type: "tuple"
      },
      {
        internalType: "bytes",
        name: "signature",
        type: "bytes"
      },
      {
        internalType: "bytes32",
        name: "bucketId",
        type: "bytes32"
      },
      {
        internalType: "bytes",
        name: "location",
        type: "bytes"
      },
      {
        internalType: "uint64",
        name: "size",
        type: "uint64"
      },
      {
        internalType: "bytes32",
        name: "fingerprint",
        type: "bytes32"
      }
    ],
    name: "requestDeleteFile",
    outputs: [],
    stateMutability: "nonpayable",
    type: "function"
  },
  {
    inputs: [
      {
        internalType: "bytes32",
        name: "bucketId",
        type: "bytes32"
      },
      {
        internalType: "bytes32",
        name: "newMspId",
        type: "bytes32"
      },
      {
        internalType: "bytes32",
        name: "newValuePropId",
        type: "bytes32"
      }
    ],
    name: "requestMoveBucket",
    outputs: [],
    stateMutability: "nonpayable",
    type: "function"
  },
  {
    inputs: [
      {
        internalType: "bytes32",
        name: "fileKey",
        type: "bytes32"
      }
    ],
    name: "revokeStorageRequest",
    outputs: [],
    stateMutability: "nonpayable",
    type: "function"
  },
  {
    inputs: [
      {
        internalType: "bytes32",
        name: "bucketId",
        type: "bytes32"
      },
      {
        internalType: "bool",
        name: "_private",
        type: "bool"
      }
    ],
    name: "updateBucketPrivacy",
    outputs: [],
    stateMutability: "nonpayable",
    type: "function"
  }
] as const satisfies Abi;
