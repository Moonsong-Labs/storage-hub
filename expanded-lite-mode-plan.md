## Implementation Plan: Indexer-Lite for Bucket Transfers and BSP Tracking

### Overview

Modify the indexer-lite implementation to support MSP bucket transfers by indexing all buckets, files, and BSP volunteering events. This will enable MSPs to receive buckets from other MSPs and know which BSPs hold the files.

### Prerequisites

- [ ] Access to client/indexer-service/src/handler.rs
- [ ] Access to client/indexer-service/LITE_MODE_EVENTS.md
- [ ] Understanding of current lite mode filtering logic
- [ ] Database schema remains unchanged (uses existing tables)
- [ ] No breaking changes to existing indexed data

### Steps

1. **Remove Bucket Ownership Filtering**
   - File: `client/indexer-service/src/handler.rs`
   - Operation: Modify `index_file_system_event_lite()` function (lines 559-743)
   - Details: 
     - Remove MSP ownership checks for `NewBucket` event (line 570)
     - Index ALL buckets regardless of MSP ownership
     - Keep bucket metadata (ID, MSP assignment, privacy settings)
   - Success: All buckets in the chain are visible in the database

2. **Index All File Events**
   - File: `client/indexer-service/src/handler.rs`
   - Operation: Remove file ownership filtering in `index_file_system_event_lite()`
   - Details:
     - Remove `check_file_belongs_to_current_msp` calls for:
       - `NewStorageRequest` (lines 590-598)
       - `StorageRequestFulfilled` (lines 602-616)
       - `StorageRequestExpired` (lines 630-644)
       - `StorageRequestRevoked` (lines 647-661)
       - `FileDeletionRequest` (lines 693-701)
     - Index all file events to maintain complete file records
   - Success: All files across all buckets are tracked

3. **Enable BSP Volunteering Event Indexing**
   - File: `client/indexer-service/src/handler.rs`
   - Operation: Remove BSP event filtering (lines 720-726)
   - Details:
     - Index `AcceptedBspVolunteer` events (currently skipped at line 720)
     - Index `BspConfirmedStoring` events (currently skipped at line 723)
     - Process events through existing `index_accepted_bsp_volunteer()` and `index_bsp_confirmed_storing()` methods
   - Success: BSP-to-file relationships are tracked in bsp_file table

4. **Expand Move Bucket Event Coverage**
   - File: `client/indexer-service/src/handler.rs`
   - Operation: Modify move bucket event filtering (lines 573-589, 664-689)
   - Details:
     - Index ALL `MoveBucketRequested` events (remove ownership check at line 666)
     - Index ALL `MoveBucketRejected` events (remove ownership check at line 674)
     - Index ALL `MoveBucketRequestExpired` events (remove ownership check at line 683)
     - Keep existing `MoveBucketAccepted` logic as it already handles both sides
   - Success: Complete visibility of all bucket transfer requests and outcomes

5. **Handle Cross-MSP References**
   - File: `client/indexer-service/src/handler.rs`
   - Operation: Ensure foreign MSP records exist when processing events
   - Details:
     - In `index_move_bucket_accepted()` (line 417), check if new/old MSP exists
     - Create minimal MSP records if needed (just ID, no metadata)
     - Prevents foreign key constraint violations
   - Success: Database integrity maintained for cross-MSP transfers

6. **Update Helper Methods Documentation**
   - File: `client/indexer-service/src/handler.rs`
   - Operation: Update comments for helper methods (lines 131-195)
   - Details:
     - Document that `check_bucket_belongs_to_current_msp` is only used for specific events
     - Note that lite mode indexes all buckets/files for transfer support
   - Success: Code documentation reflects lite mode behavior

7. **Update LITE_MODE_EVENTS.md Documentation**
   - File: `client/indexer-service/LITE_MODE_EVENTS.md`
   - Operation: Update to reflect new indexing behavior
   - Details:
     - Update FileSystem Pallet section to show ALL buckets and files are indexed
     - Move BSP events (`AcceptedBspVolunteer`, `BspConfirmedStoring`) from "Ignored" to "Indexed"
     - Add note about expanded scope for bucket transfer support
     - Update implementation details section
   - Success: Documentation accurately describes the lite mode behavior

### Testing Strategy

- [ ] Test bucket transfer from MSP A to MSP B - verify both see complete file list
- [ ] Test BSP volunteering events are indexed with correct file associations
- [ ] Verify bsp_file table is populated for all files
- [ ] Test that querying BSPs for a transferred bucket returns correct peer IDs
- [ ] Verify no performance regression with increased event indexing
- [ ] Test database migrations work correctly with existing lite mode data

### Rollback Plan

1. Revert changes to `index_file_system_event_lite()` function
2. Re-enable ownership filtering for buckets and files
3. Re-enable BSP event skipping
4. Drop any newly created cross-MSP records if needed
5. Revert LITE_MODE_EVENTS.md to original version
6. Existing indexed data remains valid - no data migration needed