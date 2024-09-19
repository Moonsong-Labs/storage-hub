export const ALL_TYPES = {
  FileMetadata: {
    owner: "Vec<u8>",
    bucket_id: "Vec<u8>",
    location: "Vec<u8>",
    file_size: "u64",
    fingerprint: "[u8; 32]"
  },
  IncompleteFileStatus: {
    file_metadata: "FileMetadata",
    stored_chunks: "u64",
    total_chunks: "u64"
  },
  SaveFileToDisk: {
    _enum: {
      FileNotFound: null,
      Success: "FileMetadata",
      IncompleteFile: "IncompleteFileStatus"
    }
  },
  GetFileFromFileStorageResult: {
    _enum: {
      FileNotFound: null,
      FileFound: "FileMetadata",
      IncompleteFile: "IncompleteFileStatus",
      FileFoundWithInconsistency: "FileMetadata"
    }
  },
  ProviderId: "H256",
  Key: "H256",
  RandomnessOutput: "H256",
  TrieRemoveMutation: {},
  BackupStorageProviderId: "H256",
  MainStorageProviderId: "H256",
  StorageData: "u32",
  MerklePatriciaRoot: "H256",
  ChunkId: "u64",
  StorageDataUnit: "u32",
  BackupStorageProvider: {
    capacity: "StorageData",
    data_used: "StorageData",
    multiaddresses: "BoundedVec<u8, 5>",
    root: "MerklePatriciaRoot",
    last_capacity_change: "BlockNumber",
    owner_account: "AccountId",
    payment_account: "AccountId"
  },
  StorageProviderId: {
    _enum: {
      BackupStorageProvider: "BackupStorageProviderId",
      MainStorageProvider: "MainStorageProviderId"
    }
  },
  GetLastTickProviderSubmittedProofError: {
    _enum: {
      ProviderNotRegistered: null,
      ProviderNeverSubmittedProof: null,
      InternalApiError: null
    }
  },
  GetCheckpointChallengesError: {
    _enum: {
      TickGreaterThanLastCheckpointTick: null,
      NoCheckpointChallengesInTick: null,
      InternalApiError: null
    }
  },
  GetChallengeSeedError: {
    _enum: {
      TickBeyondLastSeedStored: null,
      TickIsInTheFuture: null,
      InternalApiError: null
    }
  },
  GetChallengePeriodError: {
    _enum: {
      ProviderNotRegistered: null,
      InternalApiError: null
    }
  },
  GetBspInfoError: {
    _enum: {
      BspNotRegistered: null,
      InternalApiError: null
    }
  },
  GetNextDeadlineTickError: {
    _enum: {
      ProviderNotRegistered: null,
      ProviderNotInitialised: null,
      ArithmeticOverflow: null,
      InternalApiError: null
    }
  },
  QueryFileEarliestVolunteerBlockError: {
    _enum: {
      FailedToEncodeFingerprint: null,
      FailedToEncodeBsp: null,
      ThresholdArithmeticError: null,
      StorageRequestNotFound: null,
      InternalError: null
    }
  },
  QueryBspConfirmChunksToProveForFileError: {
    _enum: {
      StorageRequestNotFound: null,
      InternalError: null
    }
  },
  GetUsersWithDebtOverThresholdError: {
    _enum: {
      ProviderNotRegistered: null,
      ProviderWithoutPaymentStreams: null,
      AmountToChargeOverflow: null,
      DebtOverflow: null,
      InternalApiError: null
    }
  },
  QueryStorageProviderCapacityError: {
    _enum: {
      ProviderNotRegistered: null,
      InternalApiError: null
    }
  },
  QueryAvailableStorageCapacityError: {
    _enum: {
      ProviderNotRegistered: null,
      InternalApiError: null
    }
  },
  QueryEarliestChangeCapacityBlockError: {
    _enum: {
      ProviderNotRegistered: null,
      InternalApiError: null
    }
  }
};
//# sourceMappingURL=types.js.map
