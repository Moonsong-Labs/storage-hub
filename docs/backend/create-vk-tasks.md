# VK Task Creation Script

Project ID: `711bb4bb-2190-48b9-955d-aeaf2a075d8f`

## Task 1: Phase 1 - Remove Outdated Components

**Title:** Phase 1: Remove outdated database components

**Description:**
```
Remove legacy multi-backend database abstractions to prepare for repository pattern implementation.

**Components to remove:**
- /backend/lib/src/data/any_backend.rs - AnyBackend abstraction
- /backend/lib/src/data/any_connection.rs - AnyConnection abstraction  
- /backend/lib/src/data/sqlite/ - Entire SQLite module
- /backend/lib/src/data/postgres/mock_connection.rs - Old mock system

**Actions:**
1. Delete the files listed above
2. Remove SQLite dependencies from Cargo.toml
3. Clean up imports and references
4. Run cargo clean && cargo build to verify

**Documentation:** See docs/backend/implementation-plan.md Phase 1

**Time:** 2-3 hours
**Dependencies:** None - can start immediately
**Parallel:** Yes - independent cleanup work
```

---

## Task 2: Phase 2 - Implement SmartPool

**Title:** Phase 2: Implement SmartPool with test transactions

**Description:**
```
Create SmartPool pattern for automatic test transaction management.

**Key components:**
- /backend/lib/src/repository/pool.rs - SmartPool implementation
- /backend/lib/src/repository/error.rs - Error types
- /backend/lib/src/repository/mod.rs - Module exports

**Features:**
- Automatic test transactions (single connection in tests)
- Normal pooling in production (32 connections)
- Zero runtime overhead (test code compiled out)
- Uses deadpool-diesel for connection management

**Documentation:** See docs/backend/implementation-plan.md Phase 2

**Time:** 3-4 hours
**Dependencies:** Task 1 must be complete
**Critical path:** Yes - blocks Task 4
```

---

## Task 3A: Phase 3A - Production Repository

**Title:** Phase 3A: Implement Production Repository

**Description:**
```
Implement production repository with PostgreSQL and diesel.

**Key files:**
- /backend/lib/src/repository/traits.rs - StorageOperations trait
- /backend/lib/src/repository/postgres.rs - Production implementation

**Operations to implement:**
- BSP: create, get_by_id, update_capacity, list
- Bucket: create, get_by_id, get_by_user
- File: get_by_key, get_by_user, get_by_bucket

**Documentation:** See docs/backend/implementation-plan.md Phase 3

**Time:** 3-4 hours
**Dependencies:** Task 1 complete
**Parallel:** Can be done alongside Task 3B
**Critical path:** Yes - blocks Task 4
```

---

## Task 3B: Phase 3B - Mock Repository  

**Title:** Phase 3B: Implement Mock Repository (Parallel)

**Description:**
```
Create in-memory mock repository for unit testing.

**Key files:**
- /backend/lib/src/repository/mock.rs - Mock implementation

**Features:**
- HashMap-based in-memory storage
- Thread-safe with RwLock
- Atomic ID generation
- Implements same StorageOperations trait

**Documentation:** See docs/backend/implementation-plan.md Phase 3

**Time:** 2-3 hours
**Dependencies:** Task 1 complete
**Parallel:** Can be done alongside Task 3A
**Note:** Coordinate trait definition with Task 3A
```

---

## Task 4: Phase 4 - Refactor to DBClient

**Title:** Phase 4: Refactor PostgresClient to DBClient

**Description:**
```
Refactor PostgresClient to use repository pattern abstraction.

**Changes:**
- Rename /backend/lib/src/data/postgres/client.rs to db_client.rs
- Update to use repository trait instead of direct connections
- Add constructor for mock repository in tests
- Update Services module to use DBClient
- Update main.rs initialization

**Documentation:** See docs/backend/implementation-plan.md Phase 4

**Time:** 2-3 hours
**Dependencies:** Tasks 2, 3A, and 3B must be complete
**Critical path:** Yes - blocks final testing
```

---

## Task 5A: Phase 5A - Testing Infrastructure

**Title:** Phase 5A: Create Testing Infrastructure

**Description:**
```
Build comprehensive test suite for new architecture.

**Key files:**
- /backend/lib/src/test_helpers.rs - Test utilities
- /backend/tests/repository_tests.rs - Integration tests

**Test coverage:**
- Repository CRUD operations
- Transaction rollback behavior
- Concurrent access patterns
- Error conditions
- Mock repository unit tests

**Documentation:** See docs/backend/implementation-plan.md Phase 5

**Time:** 2-3 hours
**Dependencies:** Task 4 complete
**Parallel:** Can be done alongside Task 5B
```

---

## Task 5B: Phase 5B - Documentation Updates

**Title:** Phase 5B: Update Documentation (Parallel)

**Description:**
```
Update all documentation to reflect new repository architecture.

**Files to update:**
- README.md - Database section
- docs/backend/migration-guide.md - Create migration guide
- API documentation updates

**Reference documents:**
- docs/backend/repository-pattern.md - New architecture
- docs/backend/implementation-plan.md - Implementation details
- docs/backend/tasks-breakdown.md - Task overview

**Time:** 1-2 hours
**Dependencies:** Task 4 complete
**Parallel:** Can be done alongside Task 5A
```

---

## Task Dependencies Summary

```
Sequential chain:
Task 1 → Task 2 → Task 4

Parallel opportunities:
- After Task 1: Tasks 2, 3A, and 3B can start
- After Task 4: Tasks 5A and 5B can run in parallel

Critical path (minimum time):
Task 1 (3hr) → Task 2 (4hr) → Task 3A (4hr) → Task 4 (3hr) = 14 hours
```

## Labels/Tags to Add

- `backend`
- `database`
- `refactoring`
- `repository-pattern`
- Priority: `high` for Tasks 1-4, `medium` for Tasks 5A-5B
- Size: `small` for Task 1 & 5B, `medium` for others