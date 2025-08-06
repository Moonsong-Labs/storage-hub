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

    // Bucket move request response enum
    enum BucketMoveRequestResponse {
        Accepted,
        Rejected
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
    function createBucket(bytes32 mspId, bytes memory name, bool _private, bytes32 valuePropId) external;

    /// @dev Request to move a bucket to a new MSP
    /// @param bucketId The bucket ID to move
    /// @param newMspId The new MSP ID
    /// @param newValuePropId The new value proposition ID
    function requestMoveBucket(bytes32 bucketId, bytes32 newMspId, bytes32 newValuePropId) external;

    /// @dev MSP responds to a move bucket request
    /// @param bucketId The bucket ID
    /// @param response The response (Accepted or Rejected)
    function mspRespondMoveBucketRequest(bytes32 bucketId, BucketMoveRequestResponse response) external;

    /// @dev Update bucket privacy setting
    /// @param bucketId The bucket ID
    /// @param _private The new privacy setting
    function updateBucketPrivacy(bytes32 bucketId, bool _private) external;

    /// @dev Create and associate a collection with a bucket
    /// @param bucketId The bucket ID
    function createAndAssociateCollectionWithBucket(bytes32 bucketId) external;

    /// @dev Delete an empty bucket
    /// @param bucketId The bucket ID to delete
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
    function revokeStorageRequest(bytes32 fileKey) external;

    /// @dev MSP responds to storage requests (simplified interface)
    /// @param fileKeys Array of file keys to process
    /// @param responses Array of booleans indicating accept (true) or reject (false)
    function mspRespondStorageRequests(bytes32[] memory fileKeys, bool[] memory responses) external;

    /// @dev MSP stops storing a bucket
    /// @param bucketId The bucket ID to stop storing
    function mspStopStoringBucket(bytes32 bucketId) external;

    /// @dev BSP volunteers to store a file
    /// @param fileKey The file key to volunteer for
    function bspVolunteer(bytes32 fileKey) external;

    /// @dev BSP confirms storing files (simplified interface)
    /// @param fileKeys Array of file keys being confirmed
    /// @param newRoot The new Merkle root after storing files
    function bspConfirmStoring(bytes32[] memory fileKeys, bytes32 newRoot) external;

    /// @dev BSP requests to stop storing a file
    /// @param fileKey The file key
    /// @param bucketId The bucket ID
    /// @param location The file location
    /// @param owner The file owner
    /// @param fingerprint The file fingerprint
    /// @param size The file size
    /// @param canServe Whether the BSP can still serve the file
    function bspRequestStopStoring(
        bytes32 fileKey,
        bytes32 bucketId,
        bytes memory location,
        address owner,
        bytes32 fingerprint,
        uint64 size,
        bool canServe
    ) external;

    /// @dev BSP confirms to stop storing a file
    /// @param fileKey The file key to stop storing
    function bspConfirmStopStoring(bytes32 fileKey) external;

    /// @dev Storage provider stops storing a file from an insolvent user
    /// @param fileKey The file key
    /// @param bucketId The bucket ID
    /// @param location The file location
    /// @param owner The file owner
    /// @param fingerprint The file fingerprint
    /// @param size The file size
    function stopStoringForInsolventUser(
        bytes32 fileKey,
        bytes32 bucketId,
        bytes memory location,
        address owner,
        bytes32 fingerprint,
        uint64 size
    ) external;

    /// @dev MSP stops storing a bucket for an insolvent user
    /// @param bucketId The bucket ID to stop storing
    function mspStopStoringBucketForInsolventUser(bytes32 bucketId) external;

    /// @dev Request deletion of a file using a signed delete intention
    /// @param signedIntention The signed file operation intention
    /// @param signature The signature verifying the intention
    /// @param bucketId The bucket ID containing the file
    /// @param location The file location
    /// @param size The file size
    /// @param fingerprint The file fingerprint
    function requestDeleteFile(
        FileOperationIntention memory signedIntention,
        bytes memory signature,
        bytes32 bucketId,
        bytes memory location,
        uint64 size,
        bytes32 fingerprint
    ) external;

    // Events
    event BucketCreated(address indexed who, bytes32 indexed bucketId, bytes32 indexed mspId);
    event BucketMoveRequested(address indexed who, bytes32 indexed bucketId, bytes32 indexed newMspId);
    event BucketPrivacyUpdated(address indexed who, bytes32 indexed bucketId, bool _private);
    event CollectionCreated(address indexed who, bytes32 indexed bucketId, bytes32 indexed collectionId);
    event BucketDeleted(address indexed who, bytes32 indexed bucketId);
    event StorageRequestIssued(address indexed who, bytes32 indexed fileKey, bytes32 indexed bucketId);
    event StorageRequestRevoked(bytes32 indexed fileKey);
    event BspVolunteered(bytes32 indexed bspId, bytes32 indexed fileKey);
    event BspConfirmedStoring(bytes32 indexed bspId, bytes32[] fileKeys, bytes32 newRoot);
    event BspStopStoringRequested(bytes32 indexed bspId, bytes32 indexed fileKey);
    event BspStopStoringConfirmed(bytes32 indexed bspId, bytes32 indexed fileKey);
    event FileDeletionRequested(bytes32 indexed fileKey, address indexed owner);
}
