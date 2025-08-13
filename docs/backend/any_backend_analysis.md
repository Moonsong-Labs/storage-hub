# AnyBackend AsyncConnection Implementation Analysis

## Current State

The `AnyAsyncConnection` implementation currently hardcodes `diesel::pg::Pg` as its Backend type:

```rust
impl diesel_async::AsyncConnection for AnyAsyncConnection {
    type Backend = diesel::pg::Pg;  // HARDCODED!
    // ...
}
```

## Why It's Hardcoded

The fundamental issue is that Diesel's type system requires a **single, concrete Backend type** at compile time. Each backend (PostgreSQL, SQLite) has:
- Different SQL dialects
- Different type mappings  
- Different query builders
- Incompatible type systems

## Complete List of Blockers

### 1. **Type System Incompatibility**
- `diesel::pg::Pg` and `diesel::sqlite::Sqlite` are completely different types
- Cannot be unified under a single trait without losing type safety
- Diesel queries are statically typed against specific backends

### 2. **Associated Types in AsyncConnection Trait**
```rust
trait AsyncConnection {
    type Backend: Backend;  // Must be ONE concrete type
    type TransactionManager;
    type ExecuteFuture<'conn, 'query>;
    type LoadFuture<'conn, 'query>;
    type Stream<'conn, 'query>;
    type Row<'conn, 'query>;
}
```
All these associated types depend on the Backend type and cannot be dynamically dispatched.

### 3. **Query Fragment Requirements**
- Queries must implement `QueryFragment<Backend>` for a specific backend
- A PostgreSQL query literally cannot compile against SQLite backend
- Example: `RETURNING` clause exists in PostgreSQL but not SQLite

### 4. **Schema Type Differences**
- PostgreSQL: `SERIAL`, `TEXT`, `JSONB`, arrays, custom types
- SQLite: `INTEGER PRIMARY KEY`, `TEXT`, no arrays, limited types
- Table definitions are backend-specific

### 5. **Transaction Manager Differences**
- PostgreSQL uses `AnsiTransactionManager` with savepoints
- SQLite has different transaction semantics
- Cannot unify under single type

### 6. **Connection Methods**
Methods like `load()` and `execute_returning_count()` require:
- `T::Query: QueryFragment<Self::Backend>`
- The query must be built for the specific backend

## What Would Be Needed for True AnyBackend Support

### Option 1: Dynamic Dispatch (Not Viable)
Would require:
- Making AsyncConnection object-safe (it's not)
- Boxing all futures (performance penalty)
- Losing compile-time query validation
- Complete rewrite of Diesel's architecture

### Option 2: Separate Code Paths (Current Approach)
What we're doing now:
- Keep separate PostgreSQL and SQLite paths
- Panic when wrong backend is used
- Provide helper methods for backend detection
- User must ensure queries match backend

### Option 3: Query Abstraction Layer (Complex)
Would need:
- Abstract query representation
- Runtime query translation
- Backend-specific query builders
- Essentially reimplementing an ORM on top of Diesel

### Option 4: Compile-Time Backend Selection (Recommended)
Use generics and feature flags:
```rust
#[cfg(feature = "postgres")]
type DefaultBackend = diesel::pg::Pg;

#[cfg(feature = "sqlite")]
type DefaultBackend = diesel::sqlite::Sqlite;
```

## Current Limitations with Our Implementation

1. **PostgreSQL-Only Queries**: Current implementation panics if you try to use PostgreSQL queries with SQLite connection
2. **No Query Translation**: Queries must be written for the specific backend
3. **Runtime Errors**: Backend mismatches are caught at runtime, not compile time
4. **Limited Portability**: Cannot write truly backend-agnostic queries

## Practical Workarounds

### 1. Backend-Specific Clients
Create separate client implementations:
```rust
pub enum AnyClient {
    Postgres(PostgresClient),
    Sqlite(SqliteClient),
}
```

### 2. Backend Detection Before Queries
Always check backend before executing:
```rust
if connection.is_postgres() {
    // PostgreSQL-specific query
} else {
    // SQLite-specific query
}
```

### 3. Common Query Subset
Stick to SQL features supported by both:
- Basic SELECT, INSERT, UPDATE, DELETE
- Standard data types
- Avoid backend-specific features

## Recommendation

The current approach of hardcoding PostgreSQL as the backend type and panicking for SQLite queries is actually the most practical solution given Diesel's architecture. The `AnyBackend` enum we created is useful for:

1. **Backend detection and routing**
2. **Connection management**
3. **Raw SQL execution**
4. **Future abstraction layers**

But it **cannot** be used as the `AsyncConnection::Backend` type due to fundamental type system constraints in Diesel.

## Alternative Solutions

If true backend abstraction is needed:
1. **Use SQLx instead of Diesel** - Has better runtime backend selection
2. **Use separate binaries** - One for PostgreSQL, one for SQLite  
3. **Use feature flags** - Compile-time backend selection
4. **Write raw SQL** - Use `execute_raw_sql()` method we added