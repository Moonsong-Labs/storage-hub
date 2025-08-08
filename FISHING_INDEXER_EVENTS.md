# Fishing Mode Indexer Events Documentation

This document details the events processed by the indexer service when running in **fishing mode**. Fishing mode is designed for fisherman-only nodes that require minimal database load while maintaining essential file availability tracking.

## Overview

Fishing mode processes a focused subset of runtime events that are critical for:
- File creation and deletion tracking
- BSP (Backup Storage Provider) and MSP (Main Storage Provider) associations with files
- Provider lifecycle management
- Bucket operations

## Database Tables

The following database tables are updated by the fishing mode indexer:

| Table | Description |
|-------|-------------|
| `bsp` | Backup Storage Providers |
| `bsp_file` | BSP-to-file associations |
| `bsp_multiaddress` | BSP multiaddress mappings |
| `bucket` | Storage buckets |
| `file` | File metadata and storage requests |
| `file_peer_id` | File-to-peer-id associations |
| `msp` | Main Storage Providers |
| `msp_file` | MSP-to-file associations |
| `msp_multiaddress` | MSP multiaddress mappings |
| `multiaddress` | Network multiaddresses |
| `peer_id` | Peer identifiers |

## FileSystem Events Processed

### File Creation & Lifecycle

#### `NewStorageRequest`
**Purpose:** Tracks new file storage requests  
**Tables Updated:**
- `peer_id` - Creates peer ID records
- `file` - Creates file record with request status

#### `StorageRequestFulfilled`
**Purpose:** Marks storage request as fulfilled  
**Tables Updated:**
- `file` - Updates file step to "Stored"

#### `StorageRequestExpired`
**Purpose:** Marks storage request as expired  
**Tables Updated:**
- `file` - Updates file step to "Expired"

#### `StorageRequestRevoked`
**Purpose:** Removes revoked storage requests  
**Tables Updated:**
- `file` - Deletes file record

### File Deletion Events

#### `MspFileDeletionCompleted`
**Purpose:** Handles completed MSP file deletions  
**Tables Updated:**
- `msp_file` - Removes MSP-file association
- `file` - Deletes file record
- `bucket` - Updates bucket merkle root

#### `BspFileDeletionCompleted`
**Purpose:** Handles completed BSP file deletions  
**Tables Updated:**
- `bsp_file` - Removes BSP-file association
- `file` - Deletes file record
- `bsp` - Updates BSP merkle root

#### `SpStopStoringInsolventUser`
**Purpose:** Removes files for insolvent users  
**Tables Updated:**
- `bsp_file` - Removes BSP-file association

### BSP Association Events

#### `BspConfirmedStoring`
**Purpose:** Confirms BSP is storing files  
**Tables Updated:**
- `bsp` - Updates BSP merkle root
- `bsp_file` - Creates BSP-file associations

#### `BspConfirmStoppedStoring`
**Purpose:** Confirms BSP stopped storing file  
**Tables Updated:**
- `bsp` - Updates BSP merkle root
- `bsp_file` - Removes BSP-file association

### MSP Association Events

#### `MspAcceptedStorageRequest`
**Purpose:** Records MSP acceptance of storage request  
**Tables Updated:**
- `msp_file` - Creates MSP-file association

#### `MspStopStoringBucketInsolventUser`
**Purpose:** Removes MSP associations for insolvent user buckets  
**Tables Updated:**
- `msp_file` - Removes MSP-file associations for bucket
- `bucket` - Unsets MSP reference

#### `MspStoppedStoringBucket`
**Purpose:** Removes MSP associations when stopping bucket storage  
**Tables Updated:**
- `msp_file` - Removes MSP-file associations for bucket
- `bucket` - Unsets MSP reference

### Bucket Operations

#### `NewBucket`
**Purpose:** Creates new storage bucket  
**Tables Updated:**
- `bucket` - Creates bucket record

#### `BucketDeleted`
**Purpose:** Removes deleted bucket  
**Tables Updated:**
- `bucket` - Deletes bucket record

#### `MoveBucketAccepted`
**Purpose:** Handles bucket moves between MSPs  
**Tables Updated:**
- `msp_file` - Updates or creates MSP-file associations
- `bucket` - Updates bucket's MSP reference

## Provider Events Processed

### BSP (Backup Storage Provider) Events

#### `BspSignUpSuccess`
**Purpose:** Records successful BSP registration  
**Tables Updated:**
- `multiaddress` - Creates multiaddress records
- `bsp_multiaddress` - Creates BSP-multiaddress associations
- `bsp` - Creates BSP record

#### `BspSignOffSuccess`
**Purpose:** Records BSP sign-off  
**Tables Updated:**
- `bsp` - Deletes BSP record

#### `BspDeleted`
**Purpose:** Records BSP deletion  
**Tables Updated:**
- `bsp` - Deletes BSP record

### MSP (Main Storage Provider) Events

#### `MspSignUpSuccess`
**Purpose:** Records successful MSP registration  
**Tables Updated:**
- `multiaddress` - Creates multiaddress records
- `msp_multiaddress` - Creates MSP-multiaddress associations
- `msp` - Creates MSP record

#### `MspSignOffSuccess`
**Purpose:** Records MSP sign-off  
**Tables Updated:**
- `msp` - Deletes MSP record

#### `MspDeleted`
**Purpose:** Records MSP deletion  
**Tables Updated:**
- `msp` - Deletes MSP record

## Implementation Details

The fishing mode indexer is implemented in `/client/indexer-service/src/handler/fishing.rs` and delegates to the main handler implementations in `/client/indexer-service/src/handler.rs`.

All database operations are performed within transactions to ensure consistency, and the indexer processes events sequentially to maintain data integrity.

## Use Case

Fishing mode is specifically designed for nodes that only run fisherman services and need to track file availability without the overhead of processing all runtime events. This enables efficient monitoring of the storage network's health and file availability guarantees by focusing only on the essential events required for file tracking and provider management.