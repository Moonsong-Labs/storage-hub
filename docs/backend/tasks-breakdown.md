# Database Refactoring Tasks Breakdown

## Tasks Overview

### Sequential Tasks (Must be done in order)

**Task 1: Phase 1 - Remove Outdated Components**
- **Priority:** High
- **Time:** 2-3 hours
- **Dependencies:** None (can start immediately)
- **Description:** Remove AnyBackend, AnyConnection, SQLite module, and old mock system
- **Files to delete:**
  - `/backend/lib/src/data/any_backend.rs`
  - `/backend/lib/src/data/any_connection.rs`
  - `/backend/lib/src/data/sqlite/` (entire directory)
  - `/backend/lib/src/data/postgres/mock_connection.rs`
- **Reference:** `docs/backend/implementation-plan.md` Phase 1

**Task 2: Phase 2 - Implement SmartPool**
- **Priority:** High
- **Time:** 3-4 hours
- **Dependencies:** Task 1
- **Description:** Create SmartPool with automatic test transaction support
- **Key files to create:**
  - `/backend/lib/src/repository/pool.rs`
  - `/backend/lib/src/repository/error.rs`
- **Reference:** `docs/backend/implementation-plan.md` Phase 2

### Parallel Tasks (Can be done simultaneously after Task 1)

**Task 3A: Phase 3 - Production Repository**
- **Priority:** High
- **Time:** 3-4 hours
- **Dependencies:** Task 1
- **Description:** Implement production repository with PostgreSQL
- **Key files:**
  - `/backend/lib/src/repository/traits.rs` (shared with 3B)
  - `/backend/lib/src/repository/postgres.rs`
- **Reference:** `docs/backend/implementation-plan.md` Phase 3

**Task 3B: Phase 3 - Mock Repository**
- **Priority:** High
- **Time:** 2-3 hours
- **Dependencies:** Task 1
- **Description:** Implement in-memory mock repository
- **Key files:**
  - `/backend/lib/src/repository/mock.rs`
- **Reference:** `docs/backend/implementation-plan.md` Phase 3
- **Can be done in parallel with:** Task 3A

### Sequential Tasks (Continue after parallel work)

**Task 4: Phase 4 - Refactor to DBClient**
- **Priority:** High
- **Time:** 2-3 hours
- **Dependencies:** Tasks 2, 3A, 3B
- **Description:** Refactor PostgresClient to DBClient using repository pattern
- **Key changes:**
  - Rename `/backend/lib/src/data/postgres/client.rs` to `/backend/lib/src/data/db_client.rs`
  - Update Services module
  - Update main.rs
- **Reference:** `docs/backend/implementation-plan.md` Phase 4

### Final Parallel Tasks

**Task 5A: Testing Infrastructure**
- **Priority:** Medium
- **Time:** 2-3 hours
- **Dependencies:** Task 4
- **Description:** Create comprehensive test suite
- **Key files:**
  - `/backend/lib/src/test_helpers.rs`
  - `/backend/tests/repository_tests.rs`
- **Reference:** `docs/backend/implementation-plan.md` Phase 5

**Task 5B: Documentation Updates**
- **Priority:** Medium
- **Time:** 1-2 hours
- **Dependencies:** Task 4
- **Description:** Update all documentation to reflect new architecture
- **Files to update:**
  - `README.md` (database section)
  - Create migration guide
- **Reference:** `docs/backend/repository-pattern.md`
- **Can be done in parallel with:** Task 5A

## Parallelization Opportunities

### Phase 1 & Early Phase 3
After Phase 1 cleanup is complete:
- One developer can work on SmartPool (Task 2)
- Another can start on repository trait definition and begin mock repository (Task 3B)

### Phase 3 Split
- Production Repository (Task 3A) and Mock Repository (Task 3B) are independent
- Can be assigned to different developers
- Both need the trait definition, so coordinate on that first

### Phase 5 Split
- Testing (Task 5A) and Documentation (Task 5B) are independent
- Can be done by different people

## Critical Path

The critical path is: Task 1 → Task 2 → Task 3A → Task 4

Optimize by:
1. Starting Task 3B in parallel with Task 2
2. Having trait definition ready early in Phase 3
3. Beginning documentation while testing is in progress

## Task Dependencies Diagram

```
Task 1 (Cleanup)
    ├→ Task 2 (SmartPool)
    │    └→ Task 4 (DBClient)
    │         ├→ Task 5A (Testing)
    │         └→ Task 5B (Documentation)
    ├→ Task 3A (Production Repository)
    │    └→ Task 4 (DBClient)
    └→ Task 3B (Mock Repository)
         └→ Task 4 (DBClient)
```

## Success Metrics

Each task should have these completion criteria:
- ✅ Code compiles without errors
- ✅ No broken imports or references
- ✅ Tests pass (where applicable)
- ✅ Documentation updated
- ✅ Code reviewed

## Risk Areas

**High Risk:**
- SmartPool test transaction behavior (Task 2)
- Repository trait design (Task 3A/3B coordination)

**Medium Risk:**
- DBClient integration (Task 4)
- Test database setup (Task 5A)

**Low Risk:**
- File deletion (Task 1)
- Documentation (Task 5B)