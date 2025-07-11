# Enhanced Indexer-Lite Mode Test Plan

## Overview
This document outlines the test cases needed to verify the enhanced lite mode functionality that supports MSP bucket transfers and BSP tracking.

## Test Structure Analysis

Based on the existing test patterns in the codebase:
- Tests use `describeMspNet` with `indexerMode: "lite"` configuration
- Tests wait for indexer initialization and use SQL queries to verify database state
- Test files follow naming pattern: `indexer-lite-*.test.ts`

## Required Test Files

### 1. `test/suites/integration/msp/indexer-lite-bucket-transfer.test.ts`
**Purpose**: Test bucket transfers between MSPs in lite mode

**Test Cases**:
- **Transfer bucket from MSP2 to MSP1**
  - Create bucket with files on MSP2
  - Transfer bucket to MSP1
  - Verify MSP1's indexer shows:
    - The transferred bucket with MSP1 as owner
    - All files from the transferred bucket
    - Complete file metadata (fingerprint, size, location)
  
- **Transfer bucket from MSP1 to MSP2**
  - Create bucket with files on MSP1
  - Transfer bucket to MSP2
  - Verify MSP1's indexer still shows:
    - The bucket (now owned by MSP2)
    - All files remain indexed
    - MoveBucketAccepted event is indexed

- **Multiple bucket transfers**
  - Transfer bucket MSP2 → MSP1 → MSP3
  - Verify complete transfer history is indexed
  - Verify final ownership is correct

### 2. `test/suites/integration/msp/indexer-lite-bsp-tracking.test.ts`
**Purpose**: Test BSP volunteering and file associations in lite mode

**Test Cases**:
- **BSP volunteers for files in MSP1's bucket**
  - Create files in MSP1's bucket
  - Have BSP1 volunteer for the files
  - Verify bsp_file table contains:
    - BSP ID → File ID associations
    - Correct file fingerprints
    - BspVolunteer and BspConfirmedStoring events indexed

- **Multiple BSPs volunteer for same file**
  - Create file requiring multiple replicas
  - Have BSP1 and BSP2 volunteer
  - Verify both BSP associations are tracked
  
- **BSP volunteers for transferred bucket files**
  - Transfer bucket from MSP2 to MSP1
  - Have BSP volunteer for transferred files
  - Verify BSP associations are correctly indexed

### 3. `test/suites/integration/msp/indexer-lite-cross-msp-integrity.test.ts`
**Purpose**: Test database integrity with cross-MSP references

**Test Cases**:
- **Foreign MSP record creation**
  - Transfer bucket from unindexed MSP3 to MSP1
  - Verify minimal MSP3 record is created to satisfy FK constraints
  - Verify bucket transfer completes successfully

- **Move bucket request events**
  - Test MoveBucketRequested from any MSP is indexed
  - Test MoveBucketRejected events are indexed
  - Test MoveBucketRequestExpired events are indexed
  - Verify all events maintain referential integrity

### 4. `test/suites/integration/msp/indexer-lite-performance.test.ts`
**Purpose**: Verify no significant performance regression

**Test Cases**:
- **Large-scale indexing**
  - Create 100+ buckets across multiple MSPs
  - Create 1000+ files
  - Have multiple BSPs volunteer
  - Measure indexing time and compare to baseline
  - Verify database query performance

- **Event processing throughput**
  - Generate high volume of mixed events
  - Verify indexer keeps up with block production
  - Check for memory leaks or resource exhaustion

### 5. `test/suites/integration/msp/indexer-lite-migration.test.ts`
**Purpose**: Test migration from old to enhanced lite mode

**Test Cases**:
- **Existing data compatibility**
  - Start with old lite mode data (only MSP1's buckets/files)
  - Enable enhanced lite mode
  - Verify existing data remains intact
  - Verify new events are indexed correctly

- **Partial state recovery**
  - Simulate indexer restart with enhanced mode
  - Verify it can catch up on missed BSP events
  - Verify bucket transfers are properly indexed

## Test Utilities Needed

### SQL Query Helpers
```typescript
// Check bsp_file associations
async function getBspFilesForBucket(sql: SqlClient, bucketId: string) {
  return sql`
    SELECT bf.*, f.fingerprint, f.location
    FROM bsp_file bf
    JOIN file f ON bf.file_id = f.id
    WHERE f.bucket_id = ${bucketId}
  `;
}

// Check bucket ownership history
async function getBucketTransferHistory(sql: SqlClient, bucketId: string) {
  return sql`
    SELECT *
    FROM block_event
    WHERE section = 'fileSystem'
    AND method IN ('MoveBucketAccepted', 'MoveBucketRequested')
    AND data::text LIKE '%${bucketId}%'
    ORDER BY block_number
  `;
}
```

### Event Verification Helpers
```typescript
// Verify BSP events are indexed
async function verifyBspEventsIndexed(sql: SqlClient, bspId: string) {
  const events = await sql`
    SELECT method, COUNT(*) as count
    FROM block_event
    WHERE section = 'fileSystem'
    AND method IN ('AcceptedBspVolunteer', 'BspConfirmedStoring')
    AND data::text LIKE '%${bspId}%'
    GROUP BY method
  `;
  
  return events.reduce((acc, e) => {
    acc[e.method] = Number(e.count);
    return acc;
  }, {} as Record<string, number>);
}
```

## Implementation Notes

1. **Test Environment Setup**
   - Use existing `describeMspNet` with `indexerMode: "lite"`
   - Ensure MSP1 runs the indexer in lite mode
   - Create helper functions for common assertions

2. **Database Verification**
   - Always check both domain tables (bucket, file, bsp_file) and event tables
   - Verify foreign key constraints are satisfied
   - Check for orphaned records

3. **Event Sequencing**
   - Use `sleep()` appropriately to allow indexer to process events
   - Verify events are indexed in correct order
   - Check block_number consistency

4. **Error Scenarios**
   - Test handling of missing MSP records
   - Test recovery from indexer restarts
   - Verify no data loss during high-volume events

## Success Criteria

All tests should pass with:
- ✅ Complete bucket and file visibility across MSP transfers
- ✅ BSP associations correctly tracked in bsp_file table
- ✅ No foreign key constraint violations
- ✅ All relevant events indexed regardless of MSP ownership
- ✅ No significant performance degradation
- ✅ Backward compatibility with existing lite mode data