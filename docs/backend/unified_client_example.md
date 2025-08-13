# Unified Client with Compile-Time Backend Selection

## Yes! The Same Client Interface Works for All Backends

The beauty of this approach is that your application code doesn't need to change at all. The client interface remains exactly the same, and the correct backend gets compiled in based on the build configuration.

## How It Works

### 1. Single Client Interface

```rust
// backend/lib/src/data/client.rs

/// This is the ONLY client your application code needs to use
/// The backend is selected at compile time via feature flags
pub struct DatabaseClient {
    #[cfg(feature = "postgres")]
    connection: Arc<PostgresConnection>,
    
    #[cfg(feature = "sqlite")]
    connection: Arc<SqliteConnection>,
    
    #[cfg(feature = "mock")]
    connection: Arc<MockConnection>,
}

impl DatabaseClient {
    /// Same constructor regardless of backend
    pub async fn new(database_url: &str) -> Result<Self, Error> {
        #[cfg(feature = "postgres")]
        {
            let conn = PostgresConnection::establish(database_url).await?;
            Ok(Self {
                connection: Arc::new(conn),
            })
        }
        
        #[cfg(feature = "sqlite")]
        {
            let conn = SqliteConnection::establish(database_url).await?;
            Ok(Self {
                connection: Arc::new(conn),
            })
        }
        
        #[cfg(feature = "mock")]
        {
            let conn = MockConnection::new();
            Ok(Self {
                connection: Arc::new(conn),
            })
        }
    }
    
    /// All methods work the same regardless of backend
    pub async fn get_bsp_by_id(&self, id: i64) -> Result<Bsp, Error> {
        #[cfg(feature = "postgres")]
        return self.connection.get_bsp_by_id_pg(id).await;
        
        #[cfg(feature = "sqlite")]
        return self.connection.get_bsp_by_id_sqlite(id).await;
        
        #[cfg(feature = "mock")]
        return self.connection.get_bsp_by_id_mock(id).await;
    }
    
    pub async fn create_bucket(&self, bucket: NewBucket) -> Result<Bucket, Error> {
        // Same pattern - the implementation switches at compile time
        #[cfg(feature = "postgres")]
        return self.connection.create_bucket_pg(bucket).await;
        
        #[cfg(feature = "sqlite")]
        return self.connection.create_bucket_sqlite(bucket).await;
        
        #[cfg(feature = "mock")]
        return self.connection.create_bucket_mock(bucket).await;
    }
}
```

### 2. Application Code Remains Unchanged

```rust
// backend/bin/src/main.rs

use sh_msp_backend_lib::DatabaseClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // This code doesn't change at all!
    // The backend is determined by how you compile it
    let database_url = env::var("DATABASE_URL")?;
    let client = DatabaseClient::new(&database_url).await?;
    
    // Use the client normally
    let bsp = client.get_bsp_by_id(123).await?;
    println!("Found BSP: {:?}", bsp);
    
    // Create a new bucket
    let new_bucket = NewBucket {
        name: "my-bucket".to_string(),
        // ...
    };
    let bucket = client.create_bucket(new_bucket).await?;
    
    // Everything just works!
    Ok(())
}
```

### 3. Even Cleaner with Type Aliases

```rust
// backend/lib/src/data/client.rs

// The connection type is determined at compile time
#[cfg(feature = "postgres")]
type ConnectionImpl = PostgresConnection;

#[cfg(feature = "sqlite")]
type ConnectionImpl = SqliteConnection;

#[cfg(feature = "mock")]
type ConnectionImpl = MockConnection;

/// Single client that works with any backend
pub struct DatabaseClient {
    connection: Arc<ConnectionImpl>,
}

impl DatabaseClient {
    pub async fn new(database_url: &str) -> Result<Self, Error> {
        let conn = ConnectionImpl::establish(database_url).await?;
        Ok(Self {
            connection: Arc::new(conn),
        })
    }
    
    // Methods can be implemented once if the backends share traits
    pub async fn get_bsp_by_id(&self, id: i64) -> Result<Bsp, Error> {
        self.connection.get_bsp_by_id(id).await
    }
}
```

### 4. Using Traits for Shared Behavior

```rust
// backend/lib/src/data/traits.rs

#[async_trait]
pub trait DatabaseOperations {
    async fn get_bsp_by_id(&self, id: i64) -> Result<Bsp, Error>;
    async fn create_bucket(&self, bucket: NewBucket) -> Result<Bucket, Error>;
    async fn update_file(&self, file_id: i64, update: FileUpdate) -> Result<File, Error>;
    // ... all other operations
}

// Each backend implements this trait
#[async_trait]
impl DatabaseOperations for PostgresConnection {
    async fn get_bsp_by_id(&self, id: i64) -> Result<Bsp, Error> {
        // PostgreSQL-specific implementation
        use crate::schema::postgres::bsp;
        bsp::table
            .filter(bsp::id.eq(id))
            .first(&mut self.get_conn().await?)
            .await
    }
}

#[async_trait]
impl DatabaseOperations for SqliteConnection {
    async fn get_bsp_by_id(&self, id: i64) -> Result<Bsp, Error> {
        // SQLite-specific implementation
        use crate::schema::sqlite::bsp;
        bsp::table
            .filter(bsp::id.eq(id))
            .first(&mut self.get_conn().await?)
            .await
    }
}

// Now the client is super clean
pub struct DatabaseClient {
    connection: Arc<dyn DatabaseOperations + Send + Sync>,
}
```

## Building for Different Backends

```bash
# Build for PostgreSQL (production)
cargo build --release --no-default-features --features postgres

# Build for SQLite (edge deployments or local dev)
cargo build --release --no-default-features --features sqlite

# Build for Mock (testing)
cargo build --no-default-features --features mock
```

## Configuration Examples

### Docker Multi-Stage Build

```dockerfile
# Build stage for PostgreSQL
FROM rust:1.75 as builder-postgres
WORKDIR /app
COPY . .
RUN cargo build --release --no-default-features --features postgres

# Build stage for SQLite
FROM rust:1.75 as builder-sqlite
WORKDIR /app
COPY . .
RUN cargo build --release --no-default-features --features sqlite

# Runtime stage (choose which binary to use)
FROM debian:bookworm-slim
ARG BACKEND=postgres
COPY --from=builder-${BACKEND} /app/target/release/storage-hub /usr/local/bin/
```

### Environment-Based Configuration

```rust
// The client can even auto-detect based on the DATABASE_URL
impl DatabaseClient {
    pub async fn from_env() -> Result<Self, Error> {
        let database_url = env::var("DATABASE_URL")?;
        
        // But remember: only one backend is actually compiled in!
        // This is just for nice error messages
        #[cfg(feature = "postgres")]
        {
            if !database_url.starts_with("postgres") {
                return Err(Error::WrongBackend(
                    "This binary was compiled for PostgreSQL".into()
                ));
            }
        }
        
        #[cfg(feature = "sqlite")]
        {
            if !database_url.starts_with("sqlite") && !database_url.ends_with(".db") {
                return Err(Error::WrongBackend(
                    "This binary was compiled for SQLite".into()
                ));
            }
        }
        
        Self::new(&database_url).await
    }
}
```

## Benefits for Your Use Case

1. **Same Client API**: Your application code doesn't change at all
2. **Type Safety**: Full compile-time checking for the selected backend
3. **No Runtime Overhead**: No dynamic dispatch or backend checks
4. **Clear Deployment**: Each binary is built for a specific backend
5. **Easy Testing**: Mock backend can be swapped in for tests

## Example Integration with Existing Code

Your existing handlers don't need to change:

```rust
// backend/lib/src/api/handlers.rs

pub async fn get_bsp_handler(
    State(client): State<Arc<DatabaseClient>>,
    Path(id): Path<i64>,
) -> Result<Json<Bsp>, Error> {
    // This code stays exactly the same!
    let bsp = client.get_bsp_by_id(id).await?;
    Ok(Json(bsp))
}

pub async fn create_bucket_handler(
    State(client): State<Arc<DatabaseClient>>,
    Json(new_bucket): Json<NewBucket>,
) -> Result<Json<Bucket>, Error> {
    // No changes needed here either!
    let bucket = client.create_bucket(new_bucket).await?;
    Ok(Json(bucket))
}
```

## Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_client_operations() {
        // In tests, use whatever backend is compiled
        #[cfg(feature = "postgres")]
        let url = "postgres://test:test@localhost/test";
        
        #[cfg(feature = "sqlite")]
        let url = ":memory:";
        
        #[cfg(feature = "mock")]
        let url = "mock://";
        
        let client = DatabaseClient::new(url).await.unwrap();
        
        // Tests work the same regardless of backend
        let bsp = client.get_bsp_by_id(1).await;
        assert!(bsp.is_ok());
    }
}
```

## Migration Path from Current Code

1. **Step 1**: Keep your current `AnyAsyncConnection` during transition
2. **Step 2**: Gradually introduce the compile-time features
3. **Step 3**: Update CI/CD to build multiple variants
4. **Step 4**: Deploy backend-specific binaries

The best part: your application logic, API handlers, and business logic don't need to change at all. The same `DatabaseClient` interface works regardless of which backend is compiled in!