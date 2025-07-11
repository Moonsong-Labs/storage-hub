# Indexer Lite Test Fixes Required

## Summary
Multiple indexer-lite test files have similar issues with incorrect API method usage and database column names.

## Issues Found

### 1. Files using `accountId()` instead of correct MSP address pattern

These files need to replace `msp1Api.accountId()` and `msp2Api.accountId()` with the correct pattern:
- **Correct pattern**: `userApi.shConsts.NODE_INFOS.msp1.AddressId` or `userApi.shConsts.NODE_INFOS.msp2.AddressId`

**Affected files:**
- `test/suites/integration/msp/indexer-lite-basic.test.ts` (1 occurrence)
- `test/suites/integration/msp/indexer-lite-core.test.ts` (18 occurrences)
- `test/suites/integration/msp/indexer-lite-mode-base.test.ts` (6 occurrences)
- `test/suites/integration/msp/indexer-lite-msp-events.test.ts` (18 occurrences)

### 2. Files using wrong database column names

**Affected file:**
- `test/suites/integration/msp/indexer-lite-core.test.ts`
  - Line 306: Uses `file_name` instead of `location`
  - Should be: `WHERE location = ${msp2FileName}`

### 3. Files that already use correct patterns (no changes needed)

These files can be used as reference for the correct patterns:
- `test/suites/integration/msp/indexer-lite-suite.test.ts` - Uses `userApi.shConsts.NODE_INFOS.msp1.AddressId`
- `test/suites/integration/msp/indexer-lite-mode-filtering.test.ts` - Uses `msp1Api.ss58.storageHub(msp1Api.keyringPair.address)`
- `test/suites/integration/msp/indexer-lite-mode.test.ts` - Uses correct `location` column
- `test/suites/integration/msp/indexer-lite-mode-env.test.ts` - Uses correct `location` column

## Database Schema Reference

Based on the working tests, the correct file table columns are:
- `location` (not `file_name` or `name`)
- `fingerprint`
- `bucket_id`
- `size`

## Next Steps

1. Update all occurrences of `accountId()` to use `userApi.shConsts.NODE_INFOS.mspX.AddressId`
2. Fix the `file_name` column reference in `indexer-lite-core.test.ts`
3. Run all indexer-lite tests to ensure they pass
4. Consider adding a comment in the test files about the correct patterns to use