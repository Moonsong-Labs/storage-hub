# Indexer Lite Mode Tests

This directory contains tests for verifying the indexer's lite mode functionality.

## Overview

The indexer lite mode is designed to reduce database size and improve performance by only indexing essential events. When running in lite mode, the indexer filters events to only include:

### Events Indexed in Lite Mode:
- **Providers Module**:
  - `MspSignUpSuccess`
  - `MspSignOffSuccess` 
  - `BspSignUpSuccess`
  - `BspSignOffSuccess`
  - `ValuePropUpserted` (filtered for current MSP only)
  - `ValuePropDeleted` (filtered for current MSP only)

- **FileSystem Module**:
  - `NewBucket`
  - `BucketPrivacyUpdateAccepted`
  - `MoveBucketAccepted`
  - `BucketDeleted`

- **ProofsDealer Module**:
  - `ProofAccepted`

All other events are filtered out and not stored in the database.

## Test Files

1. **indexer-lite-mode.test.ts** - Basic lite mode functionality tests
2. **indexer-lite-mode-filtering.test.ts** - Comprehensive event filtering verification
3. **indexer-lite-mode-env.test.ts** - Environment configuration and database size tests

## Running the Tests

### Standard Test Run
```bash
# Run all indexer lite mode tests
pnpm test:node -- indexer-lite-mode

# Run a specific test file
pnpm test:node -- indexer-lite-mode-filtering.test.ts
```

### Running with Lite Mode Enabled

To properly test lite mode, the indexer needs to be started with the `--indexer-mode lite` flag. Currently, the test framework starts the indexer automatically, so there are two approaches:

#### Option 1: Modify Docker Compose (Temporary)
Edit `docker/fullnet-base-template.yml` and add the indexer mode flag to the user node:
```yaml
sh-user:
  command: [
    # ... existing flags ...
    "--indexer",
    "--indexer-mode", "lite"
  ]
```

#### Option 2: Environment Variable (If Supported)
Set the environment variable before running tests:
```bash
INDEXER_MODE=lite pnpm test:node -- indexer-lite-mode
```

## Verifying Lite Mode

To verify the indexer is running in lite mode:

1. Check the indexer logs:
```bash
docker logs docker-sh-user-1 | grep -i "indexer.*mode"
```

2. Query the database to see which events are being indexed:
```sql
SELECT section, method, COUNT(*) as count 
FROM block_event 
GROUP BY section, method 
ORDER BY count DESC;
```

In lite mode, you should only see the events listed above.

## Test Expectations

### Database Size
- Significantly fewer rows in the `block_event` table
- No detailed file tracking events (e.g., `NewStorageRequest`, `BspConfirmedStoring`)
- Reduced overall database size

### Performance
- Faster indexing due to fewer events to process
- Lower memory usage
- Reduced disk I/O

### MSP Filtering
- ValueProp events should only be indexed for the MSP running the indexer
- Other MSPs' ValueProp events should be filtered out

## Troubleshooting

1. **All events are being indexed**: Verify the indexer is actually running in lite mode by checking the startup logs.

2. **No events are being indexed**: Ensure the postgres database is running and migrations have been applied.

3. **Test failures**: Some tests may need adjustment based on the exact implementation of lite mode filtering.

## Future Improvements

1. Add configuration option to test framework for setting indexer mode
2. Create performance benchmarks comparing full vs lite mode
3. Add tests for mode switching scenarios
4. Implement proper MSP-specific ValueProp filtering tests