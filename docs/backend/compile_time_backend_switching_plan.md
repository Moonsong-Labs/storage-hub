# Compile-Time Backend Switching Implementation Plan

## Executive Summary

This document outlines a concrete plan to implement compile-time backend switching for the StorageHub project, allowing the final application to be compiled with either PostgreSQL or SQLite backend (but not both), based on Option 4 from the analysis document.

## Current State Analysis

### Dependencies Already in Place
- **Diesel**: Already configured with both `postgres` and `sqlite` features enabled
- **Diesel-async**: Configured with both backend features
- **Backend abstraction**: `AnyBackend` enum and `AnyAsyncConnection` already implemented
- **Connection routing**: Runtime backend detection via URL parsing

### Key Limitations
- `AnyAsyncConnection` currently hardcodes `diesel::pg::Pg` as its Backend type
- Queries are backend-specific (PostgreSQL queries won't work with SQLite)
- Schema definitions use PostgreSQL-specific types

## Implementation Strategy

### Phase 1: Feature Flag Configuration

#### 1.1 Update Cargo.toml Structure

```toml
# backend/lib/Cargo.toml
[features]
default = ["postgres"]  # Default to PostgreSQL for backwards compatibility
postgres = ["diesel/postgres", "diesel-async/postgres"]
sqlite = ["diesel/sqlite", "diesel-async/sqlite"]
mock = []  # For mock backend support

# Ensure only one backend is selected at compile time
# This will be enforced via build script
```

#### 1.2 Create Build Script for Mutual Exclusivity

```rust
// backend/lib/build.rs
fn main() {
    let postgres = cfg!(feature = "postgres");
    let sqlite = cfg!(feature = "sqlite");
    
    if postgres && sqlite {
        panic!("Cannot enable both 'postgres' and 'sqlite' features simultaneously");
    }
    
    if !postgres && !sqlite {
        panic!("Must enable either 'postgres' or 'sqlite' feature");
    }
}
```

### Phase 2: Type Aliasing Strategy

#### 2.1 Create Backend Type Module

```rust
// backend/lib/src/data/backend_type.rs

#[cfg(feature = "postgres")]
pub type SelectedBackend = diesel::pg::Pg;

#[cfg(feature = "sqlite")]
pub type SelectedBackend = diesel::sqlite::Sqlite;

#[cfg(feature = "postgres")]
pub type SelectedAsyncConnection = diesel_async::AsyncPgConnection;

#[cfg(feature = "sqlite")]
pub type SelectedAsyncConnection = diesel_async::AsyncSqliteConnection;

#[cfg(feature = "postgres")]
pub type SelectedQueryBuilder = diesel::pg::PgQueryBuilder;

#[cfg(feature = "sqlite")]
pub type SelectedQueryBuilder = diesel::sqlite::SqliteQueryBuilder;
```

#### 2.2 Update AnyAsyncConnection Implementation

```rust
// backend/lib/src/data/postgres/connection.rs

use crate::data::backend_type::SelectedBackend;

impl diesel_async::AsyncConnection for AnyAsyncConnection {
    type Backend = SelectedBackend;  // Now uses compile-time selected backend
    // ...
}
```

### Phase 3: Schema Abstraction

#### 3.1 Create Backend-Specific Schema Modules

```rust
// backend/lib/src/data/schema/mod.rs
#[cfg(feature = "postgres")]
pub mod schema {
    pub use super::postgres_schema::*;
}

#[cfg(feature = "sqlite")]
pub mod schema {
    pub use super::sqlite_schema::*;
}

// backend/lib/src/data/schema/postgres_schema.rs
diesel::table! {
    bsp (id) {
        id -> Int8,  // PostgreSQL type
        // ...
    }
}

// backend/lib/src/data/schema/sqlite_schema.rs
diesel::table! {
    bsp (id) {
        id -> Integer,  // SQLite type
        // ...
    }
}
```

#### 3.2 Create Migration Scripts

```sql
-- migrations/postgres/001_initial_schema.sql
CREATE TABLE bsp (
    id BIGSERIAL PRIMARY KEY,
    account VARCHAR NOT NULL,
    capacity NUMERIC NOT NULL,
    -- PostgreSQL specific syntax
);

-- migrations/sqlite/001_initial_schema.sql
CREATE TABLE bsp (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account TEXT NOT NULL,
    capacity REAL NOT NULL,
    -- SQLite specific syntax
);
```

### Phase 4: Query Abstraction Layer

#### 4.1 Create Query Trait

```rust
// backend/lib/src/data/queries/mod.rs
#[async_trait]
pub trait QueryExecutor {
    async fn get_bsp_by_id(&self, id: i64) -> Result<Bsp, Error>;
    async fn create_bsp(&self, bsp: NewBsp) -> Result<Bsp, Error>;
    // ... other queries
}

#[cfg(feature = "postgres")]
pub type DefaultQueryExecutor = PostgresQueryExecutor;

#[cfg(feature = "sqlite")]
pub type DefaultQueryExecutor = SqliteQueryExecutor;
```

#### 4.2 Implement Backend-Specific Queries

```rust
// backend/lib/src/data/queries/postgres.rs
pub struct PostgresQueryExecutor<'a> {
    conn: &'a mut AsyncPgConnection,
}

#[async_trait]
impl<'a> QueryExecutor for PostgresQueryExecutor<'a> {
    async fn get_bsp_by_id(&self, id: i64) -> Result<Bsp, Error> {
        // PostgreSQL-specific query using RETURNING clause
        bsp::table
            .filter(bsp::id.eq(id))
            .first(self.conn)
            .await
    }
}

// backend/lib/src/data/queries/sqlite.rs
pub struct SqliteQueryExecutor<'a> {
    conn: &'a mut AsyncSqliteConnection,
}

#[async_trait]
impl<'a> QueryExecutor for SqliteQueryExecutor<'a> {
    async fn get_bsp_by_id(&self, id: i64) -> Result<Bsp, Error> {
        // SQLite-specific query without RETURNING
        bsp::table
            .filter(bsp::id.eq(id))
            .first(self.conn)
            .await
    }
}
```

### Phase 5: Client Abstraction

#### 5.1 Update Client Implementation

```rust
// backend/lib/src/data/postgres/client.rs

#[cfg(feature = "postgres")]
pub type DbClient = PostgresClient;

#[cfg(feature = "sqlite")]
pub type DbClient = SqliteClient;

pub struct UnifiedClient {
    #[cfg(feature = "postgres")]
    inner: PostgresClient,
    
    #[cfg(feature = "sqlite")]
    inner: SqliteClient,
}

impl UnifiedClient {
    pub async fn new(config: DbConfig) -> Result<Self, Error> {
        #[cfg(feature = "postgres")]
        {
            Ok(Self {
                inner: PostgresClient::new(config).await?,
            })
        }
        
        #[cfg(feature = "sqlite")]
        {
            Ok(Self {
                inner: SqliteClient::new(config).await?,
            })
        }
    }
}
```

### Phase 6: Build Configuration

#### 6.1 Create Build Profiles

```toml
# .cargo/config.toml
[build]
# PostgreSQL build (default)
[profile.postgres]
inherits = "release"
features = ["postgres"]

# SQLite build
[profile.sqlite]
inherits = "release"
features = ["sqlite"]

# Mock build for testing
[profile.mock]
inherits = "dev"
features = ["mock"]
```

#### 6.2 Create Build Scripts

```bash
#!/bin/bash
# scripts/build-postgres.sh
cargo build --release --no-default-features --features postgres

#!/bin/bash
# scripts/build-sqlite.sh
cargo build --release --no-default-features --features sqlite

#!/bin/bash
# scripts/build-mock.sh
cargo build --no-default-features --features mock
```

## Implementation Timeline

### Week 1: Foundation
- [ ] Set up feature flags in Cargo.toml
- [ ] Create build script for mutual exclusivity
- [ ] Implement backend type aliases

### Week 2: Schema Abstraction
- [ ] Create backend-specific schema modules
- [ ] Write migration scripts for both backends
- [ ] Update existing models to use type aliases

### Week 3: Query Layer
- [ ] Design and implement QueryExecutor trait
- [ ] Create PostgreSQL query implementations
- [ ] Create SQLite query implementations

### Week 4: Client Integration
- [ ] Update client to use compile-time selection
- [ ] Update connection management
- [ ] Create unified API surface

### Week 5: Testing & Documentation
- [ ] Create backend-specific test suites
- [ ] Write integration tests
- [ ] Update documentation

## Benefits of This Approach

1. **Zero Runtime Overhead**: Backend selection happens at compile time
2. **Type Safety**: Full Diesel type checking for the selected backend
3. **Smaller Binary Size**: Only includes code for one backend
4. **Clear Separation**: Backend-specific code is clearly isolated
5. **Maintainability**: Easy to add new backends or modify existing ones

## Potential Challenges & Solutions

### Challenge 1: Schema Differences
**Solution**: Use conditional compilation for schema definitions and maintain separate migration files.

### Challenge 2: Query Compatibility
**Solution**: Abstract common queries behind traits, implement backend-specific versions where needed.

### Challenge 3: Type Mappings
**Solution**: Create type alias module that maps common types to backend-specific ones.

### Challenge 4: Testing
**Solution**: Use feature flags in tests to run backend-specific test suites.

## Alternative Considerations

### Option A: Separate Crates
Create separate crates for each backend:
- `sh-backend-postgres`
- `sh-backend-sqlite`
- `sh-backend-common` (shared interfaces)

**Pros**: Complete isolation, cleaner dependency tree
**Cons**: More maintenance overhead, potential code duplication

### Option B: Dynamic Library Loading
Use dynamic libraries for backend implementations:
- Load backend at runtime based on configuration
- Use FFI for communication

**Pros**: True runtime switching without recompilation
**Cons**: Complex FFI boundary, potential performance overhead

## Recommendation

Proceed with the **compile-time feature flag approach** as it provides:
1. Best performance (zero runtime overhead)
2. Full type safety from Diesel
3. Smallest binary size
4. Clearest separation of concerns
5. Easiest migration path from current code

The mock backend can be implemented as a third feature flag that doesn't use Diesel at all, providing a lightweight option for testing.

## Next Steps

1. Review and approve this plan
2. Create feature branch for implementation
3. Start with Phase 1 (Feature Flag Configuration)
4. Implement incrementally with regular testing
5. Update CI/CD to build multiple variants

## Appendix: File Structure

```
backend/
├── lib/
│   ├── Cargo.toml (with feature flags)
│   ├── build.rs (mutual exclusivity check)
│   └── src/
│       └── data/
│           ├── backend_type.rs (type aliases)
│           ├── schema/
│           │   ├── mod.rs
│           │   ├── postgres_schema.rs
│           │   └── sqlite_schema.rs
│           ├── queries/
│           │   ├── mod.rs (trait definitions)
│           │   ├── postgres.rs
│           │   └── sqlite.rs
│           └── client/
│               ├── mod.rs
│               ├── postgres.rs
│               └── sqlite.rs
├── migrations/
│   ├── postgres/
│   │   └── *.sql
│   └── sqlite/
│       └── *.sql
└── scripts/
    ├── build-postgres.sh
    ├── build-sqlite.sh
    └── build-mock.sh
```