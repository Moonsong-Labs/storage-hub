# Indexer Lite Mode Tests

This directory contains comprehensive tests for verifying the indexer's lite mode functionality.

## Overview

The indexer lite mode is designed to reduce database size and improve performance by only indexing essential events. When running in lite mode, the indexer filters events to only include:

### Events Indexed in Lite Mode:
- **Providers Module**:
  - `MspSignUpSuccess`
  - `MspSignOffSuccess` 
  - `BspSignUpSuccess`
  - `BspSignOffSuccess`
  - `CapacityChanged`
  - `MultiAddressesChanged`
  - `ValuePropUpserted` (filtered for current MSP only)
  - `Slashed`
  - `TopUpFulfilled`

- **FileSystem Module**:
  - `NewBucket` (filtered for current MSP only)
  - `BucketPrivacyUpdateAccepted`
  - `MoveBucketAccepted` (when involving current MSP)
  - `BucketDeleted`
  - `NewStorageRequest` (for buckets owned by current MSP)
  - Other file events filtered by bucket ownership

All events from `BucketNfts`, `PaymentStreams`, `ProofsDealer`, and `Randomness` pallets are filtered out.

## Test Files

### Core Test Files

1. **indexer-lite-mode-base.test.ts** - Comprehensive base test for lite mode functionality
   - Tests event filtering for MSP-relevant events only
   - Verifies provider events are filtered correctly
   - Tests file events filtering by bucket ownership
   - Validates ignored pallets have no indexed events

2. **indexer-lite-vs-full.test.ts** - Comparison test between full and lite modes
   - Runs identical scenarios in both modes
   - Collects and compares statistics
   - Demonstrates ~80% event reduction
   - Verifies filtering rules are applied correctly

3. **indexer-lite-msp-events.test.ts** - MSP-specific event filtering tests
   - ValueProp event filtering for current MSP only
   - Bucket operations filtered by MSP ownership
   - MoveBucket events involving current MSP
   - Provider lifecycle events

4. **indexer-lite-performance.test.ts** - Performance metrics and benchmarking
   - Measures total events indexed and database size
   - Tracks indexing speed and query performance
   - Calculates event reduction percentage
   - Validates performance improvements

5. **indexer-lite-event-processing.test.ts** - Event processing verification
   - Ensures events are processed identically to full mode
   - Verifies data structure consistency
   - Validates database relationships

6. **indexer-lite-suite.test.ts** - Comprehensive test suite runner
   - Runs all lite mode tests
   - Generates coverage report
   - Verifies all documented events are tested

### Supporting Test Files

7. **indexer-lite-mode.test.ts** - Basic lite mode functionality tests
8. **indexer-lite-mode-filtering.test.ts** - Event filtering verification
9. **indexer-lite-mode-env.test.ts** - Environment and configuration tests

## Test Utilities

The `test/util/indexer-helpers.ts` file provides utility functions for:
- Running tests in specific indexer modes
- Comparing results between full and lite modes
- Collecting event statistics
- Verifying filtering rules
- Measuring performance metrics

## Running the Tests

### Running Individual Tests
```bash
# Run all indexer lite mode tests
pnpm test:node -- indexer-lite

# Run a specific test file
pnpm test:node -- indexer-lite-mode-base.test.ts

# Run with filter for specific test
FILTER="filters ValueProp" pnpm test:node -- indexer-lite-msp-events.test.ts
```

### Running the Complete Test Suite
```bash
# Run comprehensive test suite
pnpm test:node -- indexer-lite-suite.test.ts
```

### Test Framework Configuration

The test framework now supports indexer mode configuration natively. Tests can specify the indexer mode in their configuration:

```typescript
describeMspNet(
  "Test Name",
  { initialised: false, indexer: true, indexerMode: "lite" },
  // ... test implementation
);
```

## Verifying Lite Mode

To verify the indexer is running in lite mode:

1. Check the indexer logs:
```bash
docker logs docker-sh-msp-1 | grep -i "indexer.*mode"
```

2. Query the database to see which events are being indexed:
```sql
SELECT section, method, COUNT(*) as count 
FROM block_event 
GROUP BY section, method 
ORDER BY count DESC;
```

3. Check for filtered events:
```sql
-- Should return 0 in lite mode
SELECT COUNT(*) FROM block_event 
WHERE section IN ('bucketNfts', 'paymentStreams', 'randomness');
```

## Test Expectations

### Event Reduction
- ~80% reduction in total indexed events
- No events from ignored pallets
- Only current MSP's events indexed

### Database Size
- Significantly smaller `block_event` table
- Reduced storage requirements
- Faster query performance

### Performance Metrics
- Indexing speed: >100 events/second
- Query response time: <100ms for common queries
- Database size: <50MB for typical test scenarios

### MSP Filtering
- Only events for the current MSP (MSP1 in tests)
- No events from other MSPs (MSP2, etc.)
- ValueProp events strictly filtered

## Troubleshooting

### Common Issues

1. **All events are being indexed**: 
   - Verify `indexerMode: "lite"` is set in test configuration
   - Check docker container logs for `--indexer-mode=lite` flag

2. **No events are being indexed**: 
   - Ensure postgres is running: `docker ps | grep postgres`
   - Check for database connection errors in logs

3. **Test failures due to timing**:
   - Increase sleep times after operations
   - Wait for specific log messages before assertions

4. **MSP filtering not working**:
   - Verify MSP ID in test matches expected value
   - Check keystore configuration for MSP nodes

### Debug Commands

```bash
# View indexer configuration
docker inspect docker-sh-msp-1 | jq '.[0].Args'

# Check database event counts
docker exec -it docker-sh-postgres-1 psql -U postgres -d storage_hub -c "SELECT section, COUNT(*) FROM block_event GROUP BY section;"

# Monitor indexer logs in real-time
docker logs -f docker-sh-msp-1 | grep -i indexer
```

## Performance Benchmarks

Expected performance metrics in lite mode:

| Metric | Full Mode | Lite Mode | Improvement |
|--------|-----------|-----------|-------------|
| Events Indexed | ~500 | ~100 | 80% reduction |
| Database Size | ~10MB | ~2MB | 80% reduction |
| Indexing Time | ~5s | ~1s | 80% faster |
| Query Time | ~50ms | ~10ms | 80% faster |

## Future Improvements

1. **Enhanced Testing**:
   - Add stress tests with high event volumes
   - Test mode switching without restart
   - Add multi-MSP network scenarios

2. **Performance Monitoring**:
   - Continuous benchmarking in CI/CD
   - Memory usage tracking
   - Long-running stability tests

3. **Configuration**:
   - Dynamic mode switching via RPC
   - Per-pallet filtering configuration
   - Custom event filtering rules

## Contributing

When adding new tests:
1. Follow existing test patterns
2. Update this documentation
3. Ensure tests work in both full and lite modes
4. Add performance assertions where applicable
5. Use the test utilities for consistency