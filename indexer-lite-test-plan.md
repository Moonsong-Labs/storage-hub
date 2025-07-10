# Implementation Plan: Indexer Lite Mode Testing

## Overview

Create comprehensive tests for the indexer lite mode functionality that verify event filtering behavior, compare performance between full and lite modes, and ensure that processed events are handled identically in both modes.

## Prerequisites

- [ ] Storage Hub development environment set up
- [ ] Docker and Docker Compose installed
- [ ] PostgreSQL client tools available
- [ ] Access to modify test framework configuration
- [ ] Understanding of MSP node configuration in tests

## Steps

### 1. Extend Test Framework for Indexer Mode Configuration

- **File**: `test/util/netLaunch/types.ts`
- **Operation**: Add indexer mode to configuration types (around line 20)
- **Details**:
  ```typescript
  export type NetLaunchConfig = {
    indexer?: boolean;
    indexerMode?: 'full' | 'lite';  // Add this line
    // ... existing properties
  }
  ```
- **Success**: TypeScript compilation succeeds

### 2. Update Network Launcher to Support Indexer Mode

- **File**: `test/util/netLaunch/index.ts`
- **Operation**: Modify spawn method to pass indexer mode (around line 180)
- **Details**:
  - Add indexer mode to node command array:
  ```typescript
  if (config.indexer) {
    commands.push("--indexer");
    if (config.indexerMode) {
      commands.push(`--indexer-mode=${config.indexerMode}`);
    }
    commands.push(`--database-url=postgresql://postgres:postgres@sh-postgres:5432/storage_hub`);
  }
  ```
- **Success**: Nodes start with correct indexer mode

### 3. Create Base Test File for Lite Mode

- **File**: `test/suites/integration/msp/indexer-lite-mode-base.test.ts`
- **Operation**: Create new test file with basic lite mode tests
- **Details**:
  ```typescript
  import assert from "node:assert";
  import { describeMspNet, type EnrichedBspApi, type SqlClient } from "../../../util";
  
  describeMspNet(
    "Indexer Lite Mode - Basic Functionality",
    { initialised: false, indexer: true, indexerMode: 'lite' },
    ({ before, it, createMsp1Api, createSqlClient }) => {
      let msp1Api: EnrichedBspApi;
      let sql: SqlClient;

      before(async () => {
        const maybeMsp1Api = await createMsp1Api();
        assert(maybeMsp1Api, "MSP1 API not available");
        msp1Api = maybeMsp1Api;
        sql = createSqlClient();
      });

      it("indexes only MSP-relevant events", async () => {
        // Test implementation
      });
    }
  );
  ```
- **Success**: Test file compiles and runs

### 4. Create Event Filtering Comparison Test

- **File**: `test/suites/integration/msp/indexer-lite-vs-full.test.ts`
- **Operation**: Create test comparing full vs lite mode behavior
- **Details**:
  - Run same operations with both indexer modes
  - Compare database contents after operations
  - Verify lite mode has fewer indexed events
  - Confirm common events are processed identically
- **Success**: Test demonstrates filtering differences

### 5. Create MSP-Specific Event Test

- **File**: `test/suites/integration/msp/indexer-lite-msp-events.test.ts`
- **Operation**: Test MSP-specific event filtering
- **Details**:
  - Test ValueProp events for current MSP vs other MSPs
  - Verify bucket operations for owned vs non-owned buckets
  - Test file operations within MSP buckets
  - Verify provider lifecycle events
- **Success**: Only current MSP events are indexed

### 6. Create Performance Metrics Test

- **File**: `test/suites/integration/msp/indexer-lite-performance.test.ts`
- **Operation**: Measure performance differences
- **Details**:
  ```typescript
  // Count total events indexed
  const eventCount = await sql`SELECT COUNT(*) FROM block_event`;
  
  // Measure database size
  const dbSize = await sql`
    SELECT pg_database_size('storage_hub') as size
  `;
  
  // Track indexing speed
  const startTime = Date.now();
  // ... perform operations ...
  const indexingTime = Date.now() - startTime;
  ```
- **Success**: Metrics show ~80% reduction in indexed events

### 7. Create Event Processing Verification Test

- **File**: `test/suites/integration/msp/indexer-lite-event-processing.test.ts`
- **Operation**: Verify events are processed identically
- **Details**:
  - For each event type that IS indexed in lite mode:
    - Create the event
    - Query database for indexed data
    - Compare with full mode results
    - Verify all fields match
- **Success**: Indexed events have identical data structure

### 8. Add Test Utilities for Mode Switching

- **File**: `test/util/indexer-helpers.ts`
- **Operation**: Create helper functions
- **Details**:
  ```typescript
  export async function runWithIndexerMode(
    mode: 'full' | 'lite',
    testFn: (api: EnrichedBspApi, sql: SqlClient) => Promise<void>
  ) {
    // Implementation to run test with specific mode
  }
  
  export async function compareIndexerModes(
    operation: (api: EnrichedBspApi) => Promise<void>
  ): Promise<{ full: any[], lite: any[] }> {
    // Run operation in both modes and return results
  }
  ```
- **Success**: Utilities simplify mode comparison tests

### 9. Create Comprehensive Test Suite Runner

- **File**: `test/suites/integration/msp/indexer-lite-suite.test.ts`
- **Operation**: Create test suite that runs all lite mode tests
- **Details**:
  - Import and run all individual test files
  - Generate summary report
  - Verify all LITE_MODE_EVENTS.md events are tested
- **Success**: All lite mode tests pass

### 10. Update Test Documentation

- **File**: `test/README-indexer-tests.md`
- **Operation**: Document new test capabilities
- **Details**:
  - How to run lite mode tests
  - Expected behavior and results
  - Performance benchmarks
  - Troubleshooting guide
- **Success**: Documentation is clear and complete

## Testing Strategy

- [ ] Unit tests for event filtering logic
- [ ] Integration tests for database state verification
- [ ] Performance benchmarks comparing modes
- [ ] End-to-end tests with real blockchain events
- [ ] Multi-MSP scenario testing
- [ ] Edge case testing (missing buckets, database errors)

## Test Scenarios

### Scenario 1: Basic Event Filtering
- Create buckets with MSP1 and MSP2
- Verify MSP1's lite indexer only indexes MSP1's bucket events
- Verify MSP2's events are filtered out

### Scenario 2: File Operations
- Create storage requests in MSP1's bucket
- Create storage requests in MSP2's bucket
- Verify only MSP1's file events are indexed

### Scenario 3: Provider Events
- Test MSP sign-up, capacity changes, multiaddress updates
- Test ValueProp events (Added, Unavailable)
- Verify only current MSP's provider events are indexed

### Scenario 4: Performance Comparison
- Run identical operations in full and lite mode
- Measure:
  - Total events indexed
  - Database size
  - Indexing time
  - Query performance

### Scenario 5: Event Processing Consistency
- For each indexed event type:
  - Create event in both modes
  - Compare database records
  - Verify identical data structure and values

## Rollback Plan

1. Remove indexerMode from configuration types
2. Revert network launcher changes
3. Delete new test files
4. Restore original test configuration

## Success Criteria

1. All tests pass in CI/CD pipeline
2. Lite mode shows ~80% reduction in indexed events
3. No false negatives (missing required events)
4. No false positives (indexing filtered events)
5. Performance metrics documented
6. Test coverage for all event types in LITE_MODE_EVENTS.md

## Implementation Notes

### Current Limitations
- Test framework doesn't natively support indexer mode configuration
- Need to extend Docker compose templates or test utilities
- MSP ID detection relies on keystore configuration

### Key Implementation Details
- MSP1 ID: `0x0000000000000000000000000000000000000000000000000000000000000300`
- MSP2 ID: `0x0000000000000000000000000000000000000000000000000000000000000301`
- Database: PostgreSQL at `localhost:5432` (user: postgres, pass: postgres)
- Indexer logs: Check with `docker logs docker-sh-msp-1`

### Event Categories to Test
1. **FileSystem Events** (filtered):
   - NewBucket (only for current MSP)
   - MoveBucketAccepted (to/from current MSP)
   - Storage requests (in current MSP's buckets)
   - Bucket privacy/deletion (for current MSP's buckets)

2. **Providers Events** (filtered):
   - MSP lifecycle (SignUp, SignOff, Delete)
   - Capacity changes
   - ValueProp events (Added, Unavailable)
   - Financial events (Slashed, TopUpFulfilled)

3. **Ignored Pallets**:
   - BucketNfts (all events)
   - PaymentStreams (all events)
   - ProofsDealer (all events)
   - Randomness (all events)

## Future Enhancements

1. Automated performance regression testing
2. Continuous monitoring of event filtering accuracy
3. Integration with CI/CD for automatic lite mode validation
4. Benchmarking suite for different network sizes
5. Dynamic mode switching without restart