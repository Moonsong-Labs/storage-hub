# Implementation Plan: Fix Indexer Lite Mode Tests

## Overview

Update the indexer lite mode tests to match the actual indexer implementation by having only MSP1 run the indexer in lite mode and verifying domain-specific table filtering rather than expecting raw event storage.

## Prerequisites

- [ ] Indexer initialization deadlock fix is applied (already done)
- [ ] Understanding that indexer uses domain tables (msp, bucket, file) not block_event table
- [ ] Access to test framework configuration files
- [ ] PostgreSQL database accessible for tests

## Steps

### 1. Update Test Framework to Run Indexer Only on MSP1

- **File**: `test/util/netLaunch/index.ts`
- **Operation**: Modify `remapComposeYaml` method (around line 250-280)
- **Details**:
  ```typescript
  // In the MSP node configuration section
  if (nodeName.includes("msp")) {
    const mspNumber = nodeName.match(/msp-(\d+)/)?.[1];
    
    // Only run indexer on MSP1 in lite mode
    if (config.indexer && config.indexerMode === "lite" && mspNumber === "1") {
      commands.push("--indexer");
      commands.push(`--indexer-mode=${config.indexerMode}`);
      commands.push(`--database-url=postgresql://postgres:postgres@sh-postgres:5432/storage_hub`);
    } else if (config.indexer && config.indexerMode !== "lite") {
      // In full mode, all MSPs can run indexer
      commands.push("--indexer");
      commands.push(`--database-url=postgresql://postgres:postgres@sh-postgres:5432/storage_hub`);
    }
  }
  ```
- **Success**: Only sh-msp-1 container has --indexer flags when indexerMode is "lite"

### 2. Create Core Lite Mode Test File

- **File**: `test/suites/integration/msp/indexer-lite-core.test.ts` (create new)
- **Operation**: Create comprehensive test for lite mode filtering
- **Details**:
  ```typescript
  import assert from "node:assert";
  import { describeMspNet, type EnrichedBspApi, type SqlClient, shUser, sleep } from "../../../util";

  describeMspNet(
    "Indexer Lite Mode - Core Functionality",
    { initialised: true, indexer: true, indexerMode: "lite" },
    ({ before, it, createMsp1Api, createMsp2Api, createUserApi, createSqlClient }) => {
      // Test implementation focusing on:
      // 1. Verify only MSP1 data is indexed
      // 2. Create buckets for both MSPs, verify only MSP1's are indexed
      // 3. Create files in both MSPs' buckets, verify filtering
    }
  );
  ```
- **Success**: Test file created with proper structure

### 3. Update Existing Test Files to Remove block_event References

- **File**: `test/suites/integration/msp/indexer-lite-mode.test.ts`
- **Operation**: Replace all SQL queries referencing block_event
- **Details**:
  - Line 46: Remove "block_event" from expectedTables array
  - Lines 56-70: Replace NewBucket event check with bucket table query
  - Lines 84-117: Update to check bucket deletion in bucket table
  - Remove any SQL queries with `FROM block_event`
- **Success**: No references to block_event remain

- **File**: `test/suites/integration/msp/indexer-lite-mode-filtering.test.ts`
- **Operation**: Rewrite event filtering verification
- **Details**:
  - Replace event counting with entity counting
  - Check bucket table instead of block_event for NewBucket
  - Check file table instead of block_event for NewStorageRequest
- **Success**: Tests query domain tables only

### 4. Simplify Test Suite Runner

- **File**: `test/suites/integration/msp/indexer-lite-suite.test.ts`
- **Operation**: Update to focus on domain table verification
- **Details**:
  - Remove all block_event queries
  - Focus on three key verifications:
    1. MSP table contains only MSP1
    2. Bucket table contains only MSP1's buckets
    3. File table contains only files in MSP1's buckets
- **Success**: Test provides clear pass/fail for lite mode filtering

### 5. Remove Complex Test Files That Don't Match Architecture

- **Files to remove**:
  - `test/suites/integration/msp/indexer-lite-vs-full.test.ts` (compares events, not applicable)
  - `test/suites/integration/msp/indexer-lite-event-processing.test.ts` (expects block_event table)
  - `test/suites/integration/msp/indexer-lite-performance.test.ts` (measures event counts)
- **Operation**: Delete files or move to archive folder
- **Success**: Only applicable tests remain

### 6. Create Focused MSP Filtering Test

- **File**: `test/suites/integration/msp/indexer-lite-msp-filtering.test.ts` (update existing)
- **Operation**: Rewrite to verify MSP-specific filtering
- **Details**:
  ```typescript
  it("verifies only MSP1 data is indexed", async () => {
    // Create value props for both MSPs
    await msp1Api.sealBlock(
      msp1Api.tx.providers.addValueProp(100n, "msp1-service")
    );
    await msp2Api.sealBlock(
      msp2Api.tx.providers.addValueProp(200n, "msp2-service")
    );
    
    await sleep(5000);
    
    // Check MSP table
    const msps = await sql`
      SELECT onchain_msp_id, value_prop
      FROM msp
    `;
    
    assert(msps.length === 1, "Should only have MSP1");
    assert(msps[0].onchain_msp_id === msp1Api.address, "Should be MSP1");
  });
  ```
- **Success**: Test clearly verifies MSP filtering

### 7. Update Test Documentation

- **File**: `test/suites/integration/msp/README-indexer-lite-tests.md`
- **Operation**: Update to reflect new test architecture
- **Details**:
  - Remove references to block_event table
  - Document that only MSP1 runs indexer in lite mode
  - Update expected behavior section
  - Simplify troubleshooting guide
- **Success**: Documentation matches implementation

## Testing Strategy

- [ ] Run `pnpm test:fullnet -- indexer-lite-core.test.ts` - should pass
- [ ] Verify in logs that only sh-msp-1 has indexer flags
- [ ] Check database has only MSP1's data after test run
- [ ] No errors about missing block_event table

## Rollback Plan

1. Restore original netLaunch/index.ts configuration
2. Restore deleted test files from git history
3. Revert changes to existing test files
4. Keep the indexer deadlock fix (it's a bug fix, not test-specific)

## Success Criteria

1. **Single Indexer**: Only MSP1 runs indexer in lite mode tests
2. **No Event Table Refs**: All tests use domain tables (msp, bucket, file)
3. **Clear Filtering**: Tests clearly show only MSP1's data is indexed
4. **All Tests Pass**: The simplified test suite passes consistently
5. **Performance**: Tests complete in under 2 minutes

## Notes

- The initialization deadlock fix in the indexer must remain in place
- Focus on testing observable behavior (what's in the database) not implementation details
- Keep tests simple and focused on the core lite mode promise: "only index current MSP's data"