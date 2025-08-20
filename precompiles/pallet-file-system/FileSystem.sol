// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >=0.8.3;

/// @dev The File System contract's address.
address constant FILE_SYSTEM_ADDRESS = 0x0000000000000000000000000000000000000100;

/// @dev The File System contract's instance.
FileSystem constant FILE_SYSTEM_CONTRACT = FileSystem(FILE_SYSTEM_ADDRESS);

/// @author The StorageHub Team
/// @title File System precompile
/// @dev Allows to perform calls to the File System pallet through the EVM.
/// @custom:address 0x0000000000000000000000000000000000000100
interface FileSystem {
    // Replication target enum values
    enum ReplicationTarget {
        Basic,
        Standard,
        HighSecurity,
        SuperHighSecurity,
        UltraHighSecurity,
        Custom
    }

    // File operation enum
    enum FileOperation {
        Delete
    }

    /// @dev File operation intention struct
    struct FileOperationIntention {
        bytes32 fileKey;
        FileOperation operation;
    }

    /// @dev Create a new bucket
    /// @param mspId The MSP (Main Storage Provider) ID
    /// @param name The bucket name
    /// @param _private Whether the bucket is private
    /// @param valuePropId The value proposition ID
    /// @custom:selector d2b3a6d8
    function createBucket(bytes32 mspId, bytes memory name, bool _private, bytes32 valuePropId) external;

    /// @dev Request to move a bucket to a new MSP
    /// @param bucketId The bucket ID to move
    /// @param newMspId The new MSP ID
    /// @param newValuePropId The new value proposition ID
    /// @custom:selector edc9d055
    function requestMoveBucket(bytes32 bucketId, bytes32 newMspId, bytes32 newValuePropId) external;

    /// @dev Update bucket privacy setting
    /// @param bucketId The bucket ID
    /// @param _private The new privacy setting
    /// @custom:selector 9996b391
    function updateBucketPrivacy(bytes32 bucketId, bool _private) external;

    /// @dev Create and associate a collection with a bucket
    /// @param bucketId The bucket ID
    /// @custom:selector 4829b447
    function createAndAssociateCollectionWithBucket(bytes32 bucketId) external;

    /// @dev Delete an empty bucket
    /// @param bucketId The bucket ID to delete
    /// @custom:selector 71f330a9
    function deleteBucket(bytes32 bucketId) external;

    /// @dev Issue a new storage request for a file
    /// @param bucketId The bucket ID where the file will be stored
    /// @param location The file location/path
    /// @param fingerprint The file fingerprint/hash
    /// @param size The file size in storage units
    /// @param mspId The MSP ID to handle the storage
    /// @param peerIds Array of peer IDs
    /// @param replicationTarget The replication target level
    /// @param customReplicationTarget Custom replication target (used if replicationTarget is Custom)
    /// @custom:selector e71dbd43
    function issueStorageRequest(
        bytes32 bucketId,
        bytes memory location,
        bytes32 fingerprint,
        uint64 size,
        bytes32 mspId,
        bytes[] memory peerIds,
        ReplicationTarget replicationTarget,
        uint32 customReplicationTarget
    ) external;

    /// @dev Revoke a storage request
    /// @param fileKey The file key to revoke
    /// @custom:selector 202e7d2d
    function revokeStorageRequest(bytes32 fileKey) external;

    /// @dev Request deletion of a file using a signed delete intention
    /// @param signedIntention The signed file operation intention
    /// @param signature The signature verifying the intention
    /// @param bucketId The bucket ID containing the file
    /// @param location The file location
    /// @param size The file size
    /// @param fingerprint The file fingerprint
    /// @custom:selector 787d538d
    function requestDeleteFile(
        FileOperationIntention memory signedIntention,
        bytes memory signature,
        bytes32 bucketId,
        bytes memory location,
        uint64 size,
        bytes32 fingerprint
    ) external;

    /// @dev Get pending file deletion requests count for a user
    /// @param user The user address
    /// @return count The number of pending deletion requests
    /// @custom:selector 945238db
    function getPendingFileDeletionRequestsCount(address user) external view returns (uint32 count);

    /// @dev Derive a bucket ID from owner and bucket name
    /// @param owner The owner address
    /// @param name The bucket name
    /// @return bucketId The derived bucket ID
    /// @custom:selector a9f1a69d
    function deriveBucketId(address owner, bytes memory name) external view returns (bytes32 bucketId);

    // Events
    event BucketCreated(address indexed who, bytes32 indexed bucketId, bytes32 indexed mspId);
    event BucketMoveRequested(address indexed who, bytes32 indexed bucketId, bytes32 indexed newMspId);
    event BucketPrivacyUpdated(address indexed who, bytes32 indexed bucketId, bool _private);
    event CollectionCreated(address indexed who, bytes32 indexed bucketId, bytes32 indexed collectionId);
    event BucketDeleted(address indexed who, bytes32 indexed bucketId);
    event StorageRequestIssued(address indexed who, bytes32 indexed fileKey, bytes32 indexed bucketId);
    event StorageRequestRevoked(bytes32 indexed fileKey);
    event FileDeletionRequested(bytes32 indexed fileKey, address indexed owner);
}
