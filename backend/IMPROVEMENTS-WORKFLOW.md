# Backend Improvements Workflow

This document outlines the parallel execution strategy for the improvements listed in IMPROVEMENTS.md.

**STATUS: Phase 1 completed with Stream 6 requiring major architectural work**

## Parallel Work Streams

### Stream 1: Simple Mechanical Changes
**No Cargo.toml changes, can start immediately**
- **7. Endpoint Documentation**: Update handler docs to remove endpoint mentions ⏳
- **9. Documentation Verbosity**: Simplify constructor documentation ⏳
- ~~**15. Mock Module Feature Gating**: Remove redundant feature gates inside mocks module~~ ✅

**Estimated Time**: 1-2 hours
**Dependencies**: None
**Files**: `api/handlers.rs`, various files with constructors
**Status**: Partially complete

### Stream 2: Cargo.toml Changes ✅
**All Cargo.toml modifications - COMPLETED**
- ~~**1. Workspace Dependencies**: Update bin/Cargo.toml to use workspace deps~~ ✅
- ~~**13. Parking_lot Migration**: Add parking_lot dependency to lib/Cargo.toml~~ ✅
- ~~**6. Dependency Grouping**: Reorganize dependency sections by functionality~~ ✅
- ~~**3. CLI Arguments**: Add clap dependency to bin/Cargo.toml~~ ✅

**Status**: Complete

### Stream 3: Code Changes Dependent on Stream 2
**Ready to implement**
- **3. CLI Implementation**: Implement config loading with CLI overrides ⏳
- **2. Environment Filter**: Simplify env filter to pass directly to subscriber ⏳
- **13. Parking_lot Usage**: Replace std::sync with parking_lot in memory.rs ⏳

**Estimated Time**: 3-4 hours
**Dependencies**: Stream 2 complete ✅
**Files**: `bin/src/main.rs`, `data/storage/memory.rs`
**Status**: Not started

### Stream 4: Database Client Improvements
**Ready to implement**
- **4. PostgreSQL Fallback Removal**: Remove automatic mock fallback on connection failure ⏳
- **4b. RPC Fallback Removal**: Remove automatic RPC mock fallback ⏳
- **10. PostgreSQL Query Methods**: Review queries and add todo!() for missing model methods ⏳
- ~~**11. Unit Test Removal**: Remove unit tests that require real database~~ (Updated instead)
- ~~**12. Queries Module Fix**: Fix compilation by addressing get_connection() issue~~ ✅

**Estimated Time**: 2-3 hours
**Dependencies**: None
**Files**: `bin/src/main.rs`, `data/postgres/client.rs`, `data/postgres/queries.rs`
**Status**: Partially complete

### Stream 5: RPC Integration ✅
**COMPLETED**
- ~~**5. StorageHub RPC Client Init**: Add RPC client initialization in binary~~ ✅
- ~~**17. RPC Client Implementation**: Create production RPC client with mock support~~ ✅

**Status**: Complete (basic implementation done, can be enhanced later)

### Stream 6: Architecture Investigation & Refactoring ✅
**COMPLETED (Major architectural work)**
- ~~**14. Storage Trait Investigation**: Understand BoxedStorage vs Storage purpose~~ ✅
- ~~**16. Mock Architecture Redesign**: Design plan to move mocks to data source level~~ ✅
- ~~**8. Test Client Consolidation**: Identify and consolidate TestPostgresClient duplicates~~ ✅

**Status**: Complete
**Note**: This required significant work including:
- Complete redesign to connection-level mocking
- Implementation of trait + enum pattern
- Creation of DbConnection and RpcConnection abstractions
- Removal of old mock client implementations

### Stream 7: Project Maintenance ✅
**COMPLETED**
- ~~**18. CI Branch Pattern**: Check remote for perm-* branches and update workflows~~ ✅

**Status**: Complete (perm-* removed from CI triggers)

## Current Status After Phase 1

### Completed Streams ✅
- **Stream 2**: All Cargo.toml changes
- **Stream 5**: RPC Integration 
- **Stream 6**: Architecture refactoring (major work)
- **Stream 7**: CI maintenance

### Remaining Work

#### High Priority (Can be done in parallel):
1. **Stream 3**: CLI and environment changes (3-4 hours)
   - CLI argument parsing implementation
   - Environment filter simplification
   - Parking_lot usage in memory.rs

2. **Stream 4**: Database/RPC fallback removal (2-3 hours)
   - Remove PostgreSQL connection fallback
   - Remove RPC connection fallback
   - Review query methods

#### Low Priority:
3. **Stream 1**: Documentation updates (1-2 hours)
   - Handler documentation
   - Constructor documentation simplification

## Critical Path

The critical path is:
1. Stream 2 (Cargo.toml) → Stream 3 (CLI/env/parking_lot implementation)
2. Stream 6 investigation → Stream 6 implementation → Potentially impacts Stream 4 & 5

## Merge Order Recommendations

To minimize conflicts:
1. Stream 1 & 7 (no conflicts with anything)
2. Stream 2 (Cargo.toml changes)
3. Stream 3, 4, 5 (can merge in any order after Stream 2)
4. Stream 6 implementation (may touch many files)

## Risk Mitigation

- **Conflict Risk**: Stream 2 should be completed and merged quickly to unblock others
- **Architecture Risk**: Stream 6 findings might require changes to approach in Streams 4 & 5
- **Knowledge Risk**: Stream 5 requires shc-rpc knowledge - assign to someone familiar or budget extra time

## Total Estimated Time

- **Minimum** (with 4 engineers working in parallel): 1-2 days
- **Maximum** (with dependencies and complexity): 3-4 days
- **Single engineer**: 5-7 days sequential work


## Prompt
Given the current branch name, start working on the corresponding stream described in the @backend/IMPROVEMENTS-WORKFLOW.md document.
Please commit your changes often to allow for easy review.
I also suggest you make use of subagents to manage context effectively
