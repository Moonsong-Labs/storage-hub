# Indexer Lite Mode Tests - Fix Required

## Current Issue

The indexer lite mode tests are failing because the Docker image (`storage-hub:local`) does not include the `--indexer-mode` CLI flag. This flag was likely added after the Docker image was last built.

## Error Details

When running the tests, the storage-hub nodes exit with:
```
error: unexpected argument '--indexer-mode' found
```

## Root Cause

1. The `--indexer-mode` flag exists in the current codebase (node/src/cli.rs)
2. The Docker image was built before this flag was added
3. The test framework correctly adds the flag, but the binary in the Docker image doesn't recognize it

## Solution Steps

### Option 1: Rebuild Docker Image (Recommended)

1. **Build the Linux binary** (required on macOS):
   ```bash
   cd test
   pnpm crossbuild:mac
   ```
   Note: This can take 30+ minutes

2. **Build the Docker image**:
   ```bash
   pnpm docker:build
   ```

3. **Re-enable the indexer-mode configuration**:
   - Edit `test/util/netLaunch/index.ts`
   - Uncomment the lines that add `--indexer-mode` flag (lines 157-159 and 176-179)

4. **Run the tests**:
   ```bash
   pnpm test:fullnet:indexer-lite
   ```

### Option 2: Use Pre-built Image

If a newer Docker image is available that includes the indexer-mode support:

1. Update the image reference in your Docker compose files
2. Re-enable the indexer-mode configuration as described above
3. Run the tests

## Test Status

All indexer lite mode tests have been implemented:
- ✅ Test framework configuration support
- ✅ Base functionality tests
- ✅ Full vs lite mode comparison
- ✅ MSP-specific event filtering
- ✅ Performance metrics
- ✅ Event processing verification
- ✅ Test utilities
- ✅ Comprehensive test suite
- ✅ Documentation

The only blocker is the Docker image compatibility issue.

## Temporary Workaround

The indexer-mode flag usage has been temporarily commented out in the test framework. This allows the network to start, but the indexer runs in full mode instead of lite mode, causing the lite-mode-specific tests to fail.

## Verification

Once the Docker image is updated, verify the fix by:

1. Checking that nodes start without the "unexpected argument" error
2. Confirming the indexer runs in lite mode (check logs for mode confirmation)
3. Running all lite mode tests successfully