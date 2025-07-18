# Backend Improvements Workflow

This document outlines the parallel execution strategy for the improvements listed in IMPROVEMENTS.md.

## Parallel Work Streams

### Stream 1: Simple Mechanical Changes
**No Cargo.toml changes, can start immediately**
- **7. Endpoint Documentation**: Update handler docs to remove endpoint mentions
- **9. Documentation Verbosity**: Simplify constructor documentation
- **15. Mock Module Feature Gating**: Remove redundant feature gates inside mocks module

**Estimated Time**: 1-2 hours
**Dependencies**: None
**Files**: `api/handlers.rs`, various files with constructors, `mocks/mod.rs`

### Stream 2: Cargo.toml Changes
**All Cargo.toml modifications - must be done as a single unit to avoid conflicts**
- **1. Workspace Dependencies**: Update bin/Cargo.toml to use workspace deps
- **13. Parking_lot Migration**: Add parking_lot dependency to lib/Cargo.toml
- **6. Dependency Grouping**: Reorganize dependency sections by functionality
- **3. CLI Arguments**: Add clap dependency to bin/Cargo.toml

**Estimated Time**: 1 hour
**Dependencies**: None
**Files**: `bin/Cargo.toml`, `lib/Cargo.toml`

### Stream 3: Code Changes Dependent on Stream 2
**Must wait for Stream 2 completion**
- **3. CLI Implementation**: Implement config loading with CLI overrides (needs clap)
- **2. Environment Filter**: Simplify env filter to pass directly to subscriber
- **13. Parking_lot Usage**: Replace std::sync with parking_lot (needs dependency)

**Estimated Time**: 3-4 hours
**Dependencies**: Stream 2 must be complete
**Files**: `bin/src/main.rs`, `mocks/postgres_mock.rs`, any file using Mutex

### Stream 4: Database Client Improvements
**Can start after Stream 2 (needs parking_lot for mocks)**
- **4. PostgreSQL Fallback Removal**: Remove automatic mock fallback on connection failure
- **10. PostgreSQL Query Methods**: Review queries and add todo!() for missing model methods
- **11. Unit Test Removal**: Remove unit tests that require real database
- **12. Queries Module Fix**: Fix compilation by addressing get_connection() issue

**Estimated Time**: 2-3 hours
**Dependencies**: Stream 2 (for parking_lot in mocks)
**Files**: `bin/src/main.rs`, `data/postgres/client.rs`, `data/postgres/queries.rs`

### Stream 5: RPC Integration
**Independent work stream**
- **5. StorageHub RPC Client Init**: Add RPC client initialization in binary
- **17. RPC Client Implementation**: Create production RPC client with mock support

**Estimated Time**: 4-6 hours
**Dependencies**: None (but benefits from Stream 6 architecture decisions)
**Files**: `bin/src/main.rs`, new RPC module files, `services/mod.rs`
**Note**: Requires familiarity with `shc-rpc` crate

### Stream 6: Architecture Investigation & Refactoring
**Should start early as findings may impact other streams**
- **14. Storage Trait Investigation**: Understand BoxedStorage vs Storage purpose
- **16. Mock Architecture Redesign**: Design plan to move mocks to data source level
- **8. Test Client Consolidation**: Identify and consolidate TestPostgresClient duplicates

**Estimated Time**: 2-3 hours investigation + 4-6 hours implementation
**Dependencies**: None for investigation, implementation may depend on findings
**Files**: `data/storage/`, mock implementations, test files

### Stream 7: Project Maintenance
**Quick independent task**
- **18. CI Branch Pattern**: Check remote for perm-* branches and update workflows

**Estimated Time**: 30 minutes
**Dependencies**: None
**Files**: `.github/workflows/lint.yml`, `.github/workflows/backend.yml`

## Execution Phases

### Phase 1: Parallel Start (Day 1)
All of these can begin simultaneously with different engineers:
- **Stream 1**: Simple mechanical changes (1 engineer, 2 hours)
- **Stream 2**: Cargo.toml changes (1 engineer, 1 hour)
- **Stream 6**: Architecture investigation (1 senior engineer, 2-3 hours)
- **Stream 7**: CI maintenance (anyone, 30 min)

### Phase 2: Follow-up Work (Day 1-2)
After Phase 1 completions:
- **Stream 3**: Implement features from Stream 2 (1-2 engineers, 3-4 hours)
- **Stream 4**: Database improvements (1 engineer, 2-3 hours)
- **Stream 5**: RPC integration (1 engineer familiar with shc-rpc, 4-6 hours)
- **Stream 6**: Continue with implementation based on investigation

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