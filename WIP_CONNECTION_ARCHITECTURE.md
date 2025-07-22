# WIP: Connection Architecture Status

## Current State
The connection architecture is partially implemented. Real PostgreSQL connections work, but mock support is incomplete due to diesel trait complexity.

## Architecture Overview

### Working Components
1. **AnyDbConnection enum** - Wraps Real and Mock connections (Mock variant commented out)
2. **DbConnection trait** - Abstraction for database connections
3. **Real PostgreSQL connections** - Fully functional via PgConnection

### Incomplete Components
1. **AnyAsyncConnection enum** - Defined but not fully functional
   - Would need to implement AsyncConnection trait with all associated types
   - Currently we use `diesel_async::AsyncPgConnection` directly as the associated type
2. **Mock connections** - Cannot return mock connections that implement AsyncConnection
3. **Transaction managers** - Partially implemented but not functional for the enum wrapper

## Technical Challenges
1. **AsyncConnection trait complexity**:
   - Requires many associated types (ExecuteFuture, LoadFuture, Stream, Row)
   - Requires implementing Connection trait as well
   - Transaction manager integration is complex

2. **Delegation pattern limitations**:
   - Can't easily delegate associated types from enum variants
   - Lifetime parameters make it even more complex

## Current Workaround
- `AnyDbConnection::Connection` type is set to `diesel_async::AsyncPgConnection` directly
- This means mock connections can't be used through this interface
- Real connections work fine

## Future Solutions
1. **Use a different mocking strategy** - Mock at service level instead of connection level
2. **Use test containers** - Real PostgreSQL in Docker for tests
3. **Create a simpler trait** - Abstract at a higher level than diesel's AsyncConnection
4. **Wait for diesel improvements** - Future versions might make this easier

## Code Structure
```rust
// Works
enum AnyDbConnection {
    Real(PgConnection),
    // Mock(MockDbConnection), // Commented out
}

// Exists but doesn't implement AsyncConnection properly
enum AnyAsyncConnection {
    Real(diesel_async::AsyncPgConnection),
    // Mock(MockAsyncConnection), // Would need diesel traits
}

// Currently using concrete type
impl DbConnection for AnyDbConnection {
    type Connection = diesel_async::AsyncPgConnection; // Not AnyAsyncConnection
}
```