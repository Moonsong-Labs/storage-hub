# Indexer-Lite Test Specification

## Overview

This document describes the test scenarios and invariants that must be verified for the indexer-lite mode implementation. The lite mode indexes ALL buckets, files, and BSP events to support cross-MSP bucket transfers.

## Core Invariants

### 1. Universal Bucket Visibility
- **Invariant**: All buckets in the network are indexed, regardless of MSP ownership
- **Verification**: Query `buckets` table and verify it contains buckets from ALL MSPs, not just the current MSP

### 2. Complete File Tracking
- **Invariant**: All files across all buckets are indexed with complete metadata
- **Verification**: Query `files` table and verify it contains files from buckets owned by other MSPs

### 3. BSP-File Associations
- **Invariant**: All BSP volunteering and storage confirmations are tracked in `bsp_file` table
- **Verification**: Query `bsp_file` table for any file and verify BSP associations exist

### 4. Provider Registration Events
- **Invariant**: Provider registration events (MSPRegistered, BSPRegistered) are now indexed in lite mode
- **Verification**: All providers are tracked through their registration events, eliminating the need for minimal records

## Test Scenarios

### Test Suite 1: Bucket Transfer Visibility

**Purpose**: Verify that bucket transfers between MSPs maintain complete visibility of all data

#### Scenario 1.1: Basic Bucket Transfer
1. **Setup**:
   - MSP1 creates bucket with 5 files
   - Each file has 2 BSPs volunteering
   - Wait for indexing to complete
2. **Action**: Transfer bucket from MSP1 to MSP2
3. **Verify**:
   - MSP1's indexer still shows the bucket (now owned by MSP2)
   - MSP2's indexer shows the bucket with all 5 files
   - Both indexers show the same BSP associations for all files
   - Query: `SELECT * FROM buckets WHERE bucket_id = ?` returns same data on both MSPs
   - Query: `SELECT * FROM files WHERE bucket_id = ?` returns 5 files on both MSPs
   - Query: `SELECT * FROM bsp_file WHERE file_id IN (?)` returns 10 BSP associations

#### Scenario 1.2: Chain Transfer
1. **Setup**: MSP1 → MSP2 → MSP3 bucket transfer chain
2. **Verify**: All three MSPs see complete bucket/file/BSP data
3. **Check**: Move bucket events are indexed by all MSPs:
   - `MoveBucketRequested` events visible to all
   - `MoveBucketAccepted` events visible to all
   - `MoveBucketRejected` events visible to all (if any)

#### Scenario 1.3: Concurrent Transfers
1. **Setup**: Multiple buckets being transferred simultaneously
2. **Verify**: No race conditions or missing data
3. **Check**: Database integrity maintained during high transfer volume

### Test Suite 2: BSP Event Tracking

**Purpose**: Verify BSP volunteering and storage confirmations are properly indexed

#### Scenario 2.1: BSP Volunteering
1. **Setup**: Create file and have BSP1 volunteer
2. **Verify**:
   - `AcceptedBspVolunteer` event is indexed (Note: Implementation pending)
   - `bsp_file` table contains BSP1-file association
   - BSP record exists from BSPRegistered event
   - Query: `SELECT * FROM bsp_file WHERE bsp_id = ? AND file_id = ?`

#### Scenario 2.2: BSP Storage Confirmation
1. **Setup**: BSP confirms storing a file
2. **Verify**:
   - `BspConfirmedStoring` event is indexed
   - `bsp_file` record updated with confirmation
   - File's replica count is accurate
   - Query: `SELECT confirmed FROM bsp_file WHERE bsp_id = ? AND file_id = ?`

#### Scenario 2.3: Cross-MSP BSP Tracking
1. **Setup**: BSP volunteers for file in bucket owned by different MSP
2. **Verify**:
   - Original MSP's indexer tracks the BSP association
   - Transferred bucket's new MSP sees all BSP associations
   - Query BSPs for any file returns complete peer information

### Test Suite 3: Provider Registration Tracking

**Purpose**: Verify that provider registration events are properly indexed in lite mode

#### Scenario 3.1: MSP Registration
1. **Setup**: New MSP registers on-chain
2. **Verify**:
   - `MSPRegistered` event is indexed
   - MSP record created with full details from event:
     - `msp_id`: The on-chain ID
     - `account`: From event data
     - `value_prop`: From event data
     - `capacity`: From event data
   - Query: `SELECT * FROM msp WHERE msp_id = ?` returns complete record

#### Scenario 3.2: BSP Registration
1. **Setup**: New BSP registers on-chain
2. **Verify**:
   - `BSPRegistered` event is indexed
   - BSP record created with full details from event:
     - `bsp_id`: The on-chain ID
     - `account`: From event data
     - `capacity`: From event data
   - Query: `SELECT * FROM bsp WHERE bsp_id = ?` returns complete record

#### Scenario 3.3: Provider Reference Before Registration
1. **Setup**: Process event referencing provider before registration event is processed
2. **Verify**:
   - System gracefully handles the temporary missing reference
   - Once registration event is processed, all references are valid
   - No foreign key constraint violations occur

### Test Suite 4: Event Coverage Verification

**Purpose**: Ensure all required events are indexed in lite mode

#### Scenario 4.1: FileSystem Events
1. **Verify these events are indexed for ALL MSPs**:
   - `NewBucket` - regardless of MSP assignment
   - `NewStorageRequest` - for all files
   - `StorageRequestFulfilled` - for all files
   - `StorageRequestExpired` - for all files
   - `StorageRequestRevoked` - for all files
   - `FileDeletionRequest` - for all files
   - `MoveBucketRequested` - all requests
   - `MoveBucketAccepted` - all acceptances
   - `MoveBucketRejected` - all rejections
   - `MoveBucketRequestExpired` - all expirations

#### Scenario 4.2: Provider Events
1. **Verify these provider registration events are indexed**:
   - `MSPRegistered` - all MSP registrations
   - `BSPRegistered` - all BSP registrations

#### Scenario 4.3: BSP File Events
1. **Verify these events are indexed**:
   - `AcceptedBspVolunteer` (Note: Implementation pending)
   - `BspConfirmedStoring`

### Test Suite 5: Performance Validation

**Purpose**: Ensure lite mode indexing doesn't cause performance degradation

#### Scenario 5.1: High Volume Test
1. **Setup**:
   - Create 10 MSPs
   - Each MSP creates 10 buckets
   - Each bucket has 10 files
   - 3 BSPs volunteer per file
2. **Measure**:
   - Indexing lag (time between event and database update)
   - Query performance for bucket/file listings
   - Memory usage of indexer service
3. **Verify**:
   - Indexing lag < 2 seconds
   - Query response time < 100ms
   - No memory leaks or excessive growth

#### Scenario 5.2: Concurrent Operations
1. **Setup**: Simulate multiple bucket transfers happening simultaneously
2. **Verify**: No deadlocks or transaction conflicts
3. **Check**: All events are processed in correct order

## Database State Verification Queries

### Essential Queries for Test Validation

```sql
-- Verify all buckets are indexed
SELECT msp_id, COUNT(*) as bucket_count 
FROM buckets 
GROUP BY msp_id;

-- Verify files from other MSPs' buckets exist
SELECT b.msp_id, COUNT(f.file_id) as file_count
FROM files f
JOIN buckets b ON f.bucket_id = b.bucket_id
WHERE b.msp_id != ? -- current MSP ID
GROUP BY b.msp_id;

-- Verify BSP associations exist for all files
SELECT f.file_id, COUNT(bf.bsp_id) as bsp_count
FROM files f
LEFT JOIN bsp_file bf ON f.file_id = bf.file_id
GROUP BY f.file_id
HAVING COUNT(bf.bsp_id) > 0;

-- Check provider registrations are indexed
SELECT msp_id, account, value_prop, capacity
FROM msp
ORDER BY msp_id;

-- Check BSP registrations are indexed
SELECT bsp_id, account, capacity
FROM bsp
ORDER BY bsp_id;

-- Verify move bucket events are indexed
SELECT event_name, COUNT(*) as event_count
FROM file_system_events
WHERE event_name IN ('MoveBucketRequested', 'MoveBucketAccepted', 'MoveBucketRejected')
GROUP BY event_name;

-- Verify provider registration events are indexed
SELECT event_name, COUNT(*) as event_count
FROM file_system_events  
WHERE event_name IN ('MSPRegistered', 'BSPRegistered')
GROUP BY event_name;

-- Verify BSP file events are indexed
SELECT event_name, COUNT(*) as event_count
FROM file_system_events  
WHERE event_name IN ('AcceptedBspVolunteer', 'BspConfirmedStoring')
GROUP BY event_name;
```

## Test Execution Guidelines

1. **Test Environment**:
   - Use Docker-based test setup with multiple MSP nodes
   - Each MSP runs its own indexer in lite mode
   - Share the same blockchain but separate databases

2. **Timing Considerations**:
   - Wait for indexer initialization: "Starting indexer in lite mode"
   - Allow 1-2 seconds after each blockchain operation for indexing
   - Use polling for database state verification

3. **Error Scenarios**:
   - Test recovery from database connection failures
   - Verify behavior when events arrive out of order
   - Check handling of duplicate events

## Success Criteria

The lite mode implementation is considered successful when:

1. All buckets and files are visible to all MSPs
2. BSP associations are tracked for all files
3. Bucket transfers maintain complete data visibility
4. Provider registration events (MSPRegistered, BSPRegistered) are indexed
5. Performance remains acceptable under load
6. All specified events are indexed without filtering
7. AcceptedBspVolunteer event indexing is implemented (currently pending)