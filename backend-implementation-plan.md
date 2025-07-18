## Implementation Plan: StorageHub Backend Scaffold

### Overview

Create a production-ready REST API backend for StorageHub that reads from the existing indexer database and provides useful endpoints, with comprehensive mocking capabilities for development and testing.

### Prerequisites

- [ ] Rust toolchain installed (stable)
- [ ] Access to StorageHub workspace at `/Users/karrq/Documents/storagehub/main/`
- [ ] libpq-dev installed for PostgreSQL support
- [ ] Existing shc-indexer-db crate with data models
- [ ] Git repository with write access

### Steps

1. **Create Backend Directory Structure**

   - File: System directories
   - Operation: Create nested directory structure
   - Details:
     ```bash
     mkdir -p backend/lib/src/{api,services,data/{postgres,storage},mocks}
     mkdir -p backend/bin/src
     ```
   - Success: All directories exist with proper nesting

2. **Create Library Crate Cargo.toml**

   - File: `backend/lib/Cargo.toml`
   - Operation: Create new file
   - Details:
     ```toml
     [package]
     name = "sh-backend-lib"
     version = "0.1.0"
     edition = "2021"
     authors.workspace = true
     repository.workspace = true
     license.workspace = true
     homepage.workspace = true

     [dependencies]
     # Web framework
     axum = "0.7"
     tokio = { version = "1", features = ["full"] }
     tower = "0.4"
     tower-http = { version = "0.5", features = ["cors", "trace"] }

     # Serialization
     serde = { workspace = true, features = ["derive"] }
     serde_json = { workspace = true }

     # Database
     shc-indexer-db = { path = "../../client/indexer-db" }
     diesel = { workspace = true, optional = true }
     diesel-async = { workspace = true, optional = true }

     # Configuration
     toml = { workspace = true }

     # Error handling
     thiserror = { workspace = true }
     anyhow = { workspace = true }

     # Logging
     tracing = "0.1"
     tracing-subscriber = "0.3"

     # RPC
     jsonrpsee = { workspace = true, optional = true }

     [features]
     default = []
     dev = ["mocks"]
     test = ["mocks"]
     mocks = ["jsonrpsee", "diesel", "diesel-async"]
     ```
   - Success: File created with correct dependencies

3. **Create Binary Crate Cargo.toml**

   - File: `backend/bin/Cargo.toml`
   - Operation: Create new file
   - Details:
     ```toml
     [package]
     name = "sh-backend-bin"
     version = "0.1.0"
     edition = "2021"
     authors.workspace = true
     repository.workspace = true
     license.workspace = true
     homepage.workspace = true

     [[bin]]
     name = "backend"
     path = "src/main.rs"

     [dependencies]
     sh-backend-lib = { path = "../lib" }
     tokio = { workspace = true, features = ["full"] }
     tracing = "0.1"
     tracing-subscriber = "0.3"
     ```
   - Success: File created with library dependency

4. **Update Root Workspace Configuration**

   - File: `Cargo.toml`
   - Operation: Add backend crates to workspace members (line 17, after existing members)
   - Details:
     ```toml
     members = [
         # ... existing members ...
         "backend/lib",
         "backend/bin",
     ]
     ```
   - Success: `cargo check` recognizes new workspace members

5. **Implement Core Library Structure**

   - File: `backend/lib/src/lib.rs`
   - Operation: Create new file
   - Details:
     ```rust
     pub mod api;
     pub mod config;
     pub mod error;
     pub mod services;
     pub mod data;

     #[cfg(feature = "mocks")]
     pub mod mocks;

     pub use api::create_app;
     pub use config::Config;
     pub use error::{Error, Result};
     ```
   - Success: Module structure defined

6. **Implement Error Handling**

   - File: `backend/lib/src/error.rs`
   - Operation: Create new file
   - Details:
     ```rust
     use axum::{
         http::StatusCode,
         response::{IntoResponse, Response},
         Json,
     };
     use serde_json::json;

     #[derive(Debug, thiserror::Error)]
     pub enum Error {
         #[error("Database error: {0}")]
         Database(String),
         
         #[error("Configuration error: {0}")]
         Config(String),
         
         #[error("Not found")]
         NotFound,
         
         #[error("Internal server error")]
         Internal,
     }

     pub type Result<T> = std::result::Result<T, Error>;

     impl IntoResponse for Error {
         fn into_response(self) -> Response {
             let (status, message) = match self {
                 Error::Database(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
                 Error::Config(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
                 Error::NotFound => (StatusCode::NOT_FOUND, "Not found".to_string()),
                 Error::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "Internal error".to_string()),
             };

             let body = Json(json!({
                 "error": message
             }));

             (status, body).into_response()
         }
     }
     ```
   - Success: Error types compile with proper HTTP mapping

7. **Implement Configuration Management**

   - File: `backend/lib/src/config.rs`
   - Operation: Create new file
   - Details:
     ```rust
     use serde::{Deserialize, Serialize};
     use std::fs;
     use crate::error::{Error, Result};

     #[derive(Debug, Clone, Serialize, Deserialize)]
     pub struct Config {
         pub host: String,
         pub port: u16,
         pub storage_hub: StorageHubConfig,
         pub database: DatabaseConfig,
     }

     #[derive(Debug, Clone, Serialize, Deserialize)]
     pub struct StorageHubConfig {
         pub rpc_url: String,
         #[cfg(feature = "mocks")]
         pub mock_mode: bool,
     }

     #[derive(Debug, Clone, Serialize, Deserialize)]
     pub struct DatabaseConfig {
         pub url: String,
         #[cfg(feature = "mocks")]
         pub mock_mode: bool,
     }

     impl Default for Config {
         fn default() -> Self {
             Self {
                 host: "127.0.0.1".to_string(),
                 port: 8080,
                 storage_hub: StorageHubConfig {
                     rpc_url: "ws://localhost:9944".to_string(),
                     #[cfg(feature = "mocks")]
                     mock_mode: true,
                 },
                 database: DatabaseConfig {
                     url: "postgres://localhost:5432/storage_hub".to_string(),
                     #[cfg(feature = "mocks")]
                     mock_mode: true,
                 },
             }
         }
     }

     impl Config {
         pub fn from_file(path: &str) -> Result<Self> {
             let content = fs::read_to_string(path)
                 .map_err(|e| Error::Config(format!("Failed to read config: {}", e)))?;
             let config: Config = toml::from_str(&content)
                 .map_err(|e| Error::Config(format!("Failed to parse config: {}", e)))?;
             Ok(config)
         }
     }
     ```
   - Success: Configuration loads from TOML files

8. **Create Data Layer Modules**

   - File: `backend/lib/src/data/mod.rs`
   - Operation: Create new file
   - Details:
     ```rust
     pub mod postgres;
     pub mod storage;
     ```
   - Success: Data modules organized

9. **Implement Storage Traits**

   - File: `backend/lib/src/data/storage/traits.rs`
   - Operation: Create new file
   - Details:
     ```rust
     use async_trait::async_trait;
     use crate::error::Result;

     #[async_trait]
     pub trait Storage: Send + Sync {
         async fn get_counter(&self, key: &str) -> Result<i64>;
         async fn set_counter(&self, key: &str, value: i64) -> Result<()>;
         async fn increment_counter(&self, key: &str) -> Result<i64>;
         async fn decrement_counter(&self, key: &str) -> Result<i64>;
     }
     ```
   - Success: Trait defines storage interface

10. **Implement In-Memory Storage**

    - File: `backend/lib/src/data/storage/memory.rs`
    - Operation: Create new file
    - Details:
      ```rust
      use std::collections::HashMap;
      use std::sync::Arc;
      use tokio::sync::RwLock;
      use async_trait::async_trait;
      use crate::data::storage::traits::Storage;
      use crate::error::{Error, Result};

      pub struct MemoryStorage {
          data: Arc<RwLock<HashMap<String, i64>>>,
      }

      impl MemoryStorage {
          pub fn new() -> Self {
              Self {
                  data: Arc::new(RwLock::new(HashMap::new())),
              }
          }
      }

      #[async_trait]
      impl Storage for MemoryStorage {
          async fn get_counter(&self, key: &str) -> Result<i64> {
              let data = self.data.read().await;
              Ok(data.get(key).copied().unwrap_or(0))
          }

          async fn set_counter(&self, key: &str, value: i64) -> Result<()> {
              let mut data = self.data.write().await;
              data.insert(key.to_string(), value);
              Ok(())
          }

          async fn increment_counter(&self, key: &str) -> Result<i64> {
              let mut data = self.data.write().await;
              let value = data.entry(key.to_string()).or_insert(0);
              *value += 1;
              Ok(*value)
          }

          async fn decrement_counter(&self, key: &str) -> Result<i64> {
              let mut data = self.data.write().await;
              let value = data.entry(key.to_string()).or_insert(0);
              *value -= 1;
              Ok(*value)
          }
      }
      ```
    - Success: Storage implementation compiles

11. **Create Storage Module**

    - File: `backend/lib/src/data/storage/mod.rs`
    - Operation: Create new file
    - Details:
      ```rust
      pub mod traits;
      pub mod memory;

      pub use traits::Storage;
      pub use memory::MemoryStorage;
      ```
    - Success: Storage module exports public interface

12. **Create PostgreSQL Client Module**

    - File: `backend/lib/src/data/postgres/mod.rs`
    - Operation: Create new file
    - Details:
      ```rust
      pub mod client;
      pub mod queries;

      pub use client::PostgresClient;
      ```
    - Success: PostgreSQL module structure defined

13. **Implement PostgreSQL Client**

    - File: `backend/lib/src/data/postgres/client.rs`
    - Operation: Create new file
    - Details:
      ```rust
      use diesel_async::{AsyncPgConnection, AsyncConnection};
      use crate::error::{Error, Result};

      pub struct PostgresClient {
          connection_url: String,
      }

      impl PostgresClient {
          pub fn new(connection_url: String) -> Self {
              Self { connection_url }
          }

          pub async fn get_connection(&self) -> Result<AsyncPgConnection> {
              AsyncPgConnection::establish(&self.connection_url)
                  .await
                  .map_err(|e| Error::Database(format!("Connection failed: {}", e)))
          }
      }
      ```
    - Success: Client provides database connections

14. **Create PostgreSQL Queries Module**

    - File: `backend/lib/src/data/postgres/queries.rs`
    - Operation: Create new file
    - Details:
      ```rust
      use diesel::prelude::*;
      use diesel_async::RunQueryDsl;
      use shc_indexer_db::models::{Bsp, File};
      use crate::error::Result;

      pub async fn get_active_bsps(
          conn: &mut diesel_async::AsyncPgConnection,
      ) -> Result<Vec<Bsp>> {
          use shc_indexer_db::schema::bsps::dsl::*;
          
          bsps
              .filter(active.eq(true))
              .load::<Bsp>(conn)
              .await
              .map_err(|e| crate::error::Error::Database(e.to_string()))
      }

      pub async fn get_file_by_id(
          conn: &mut diesel_async::AsyncPgConnection,
          file_id: String,
      ) -> Result<Option<File>> {
          use shc_indexer_db::schema::files::dsl::*;
          
          files
              .filter(id.eq(file_id))
              .first::<File>(conn)
              .await
              .optional()
              .map_err(|e| crate::error::Error::Database(e.to_string()))
      }
      ```
    - Success: Queries use indexer-db models

15. **Create Services Module Structure**

    - File: `backend/lib/src/services/mod.rs`
    - Operation: Create new file
    - Details:
      ```rust
      pub mod counter;
      pub mod health;

      use std::sync::Arc;
      use crate::data::storage::Storage;
      use crate::data::postgres::PostgresClient;

      #[derive(Clone)]
      pub struct Services {
          pub counter: Arc<counter::CounterService>,
          pub storage: Arc<dyn Storage>,
          pub postgres: Arc<PostgresClient>,
      }

      impl Services {
          pub fn new(storage: Arc<dyn Storage>, postgres: Arc<PostgresClient>) -> Self {
              let counter = Arc::new(counter::CounterService::new(storage.clone()));
              Self { counter, storage, postgres }
          }
      }
      ```
    - Success: Services container created

16. **Implement Counter Service**

    - File: `backend/lib/src/services/counter.rs`
    - Operation: Create new file
    - Details:
      ```rust
      use std::sync::Arc;
      use crate::data::storage::Storage;
      use crate::error::Result;

      pub struct CounterService {
          storage: Arc<dyn Storage>,
      }

      impl CounterService {
          pub fn new(storage: Arc<dyn Storage>) -> Self {
              Self { storage }
          }

          pub async fn increment(&self) -> Result<i64> {
              self.storage.increment_counter("default").await
          }

          pub async fn decrement(&self) -> Result<i64> {
              self.storage.decrement_counter("default").await
          }

          pub async fn get(&self) -> Result<i64> {
              self.storage.get_counter("default").await
          }
      }
      ```
    - Success: Counter service implements business logic

17. **Implement Health Service**

    - File: `backend/lib/src/services/health.rs`
    - Operation: Create new file
    - Details:
      ```rust
      use serde::Serialize;

      #[derive(Serialize)]
      pub struct HealthStatus {
          pub status: String,
          pub version: String,
      }

      pub fn get_health() -> HealthStatus {
          HealthStatus {
              status: "healthy".to_string(),
              version: env!("CARGO_PKG_VERSION").to_string(),
          }
      }
      ```
    - Success: Health check provides status

18. **Create API Module Structure**

    - File: `backend/lib/src/api/mod.rs`
    - Operation: Create new file
    - Details:
      ```rust
      pub mod handlers;
      pub mod routes;

      use axum::Router;
      use crate::services::Services;

      pub fn create_app(services: Services) -> Router {
          routes::create_routes(services)
      }
      ```
    - Success: API module exports app creator

19. **Implement API Handlers**

    - File: `backend/lib/src/api/handlers.rs`
    - Operation: Create new file
    - Details:
      ```rust
      use axum::{extract::State, response::Json};
      use serde_json::{json, Value};
      use crate::{services::Services, error::Result};

      pub async fn increment_counter(
          State(services): State<Services>,
      ) -> Result<Json<Value>> {
          let count = services.counter.increment().await?;
          Ok(Json(json!({ "count": count })))
      }

      pub async fn decrement_counter(
          State(services): State<Services>,
      ) -> Result<Json<Value>> {
          let count = services.counter.decrement().await?;
          Ok(Json(json!({ "count": count })))
      }

      pub async fn get_counter(
          State(services): State<Services>,
      ) -> Result<Json<Value>> {
          let count = services.counter.get().await?;
          Ok(Json(json!({ "count": count })))
      }

      pub async fn health_check() -> Json<Value> {
          let health = crate::services::health::get_health();
          Json(json!(health))
      }
      ```
    - Success: Handlers implement endpoints

20. **Create API Routes**

    - File: `backend/lib/src/api/routes.rs`
    - Operation: Create new file
    - Details:
      ```rust
      use axum::{
          routing::{get, post},
          Router,
      };
      use tower_http::cors::CorsLayer;
      use crate::services::Services;
      use super::handlers;

      pub fn create_routes(services: Services) -> Router {
          Router::new()
              .route("/health", get(handlers::health_check))
              .route("/counter", get(handlers::get_counter))
              .route("/counter/inc", post(handlers::increment_counter))
              .route("/counter/dec", post(handlers::decrement_counter))
              .layer(CorsLayer::permissive())
              .with_state(services)
      }
      ```
    - Success: Routes map to handlers

21. **Create Main Binary**

    - File: `backend/bin/src/main.rs`
    - Operation: Create new file
    - Details:
      ```rust
      use sh_backend_lib::{Config, create_app, services::Services};
      use sh_backend_lib::data::{
          storage::MemoryStorage,
          postgres::PostgresClient,
      };
      use std::sync::Arc;
      use tokio::net::TcpListener;
      use tracing_subscriber;

      #[tokio::main]
      async fn main() -> Result<(), Box<dyn std::error::Error>> {
          // Initialize logging
          tracing_subscriber::fmt::init();
          
          // Load configuration
          let config = Config::from_file("backend_config.toml")
              .unwrap_or_else(|_| {
                  tracing::warn!("Config file not found, using defaults");
                  Config::default()
              });
          
          // Setup storage
          let storage = Arc::new(MemoryStorage::new());
          
          // Setup PostgreSQL client
          let postgres = Arc::new(PostgresClient::new(config.database.url.clone()));
          
          // Setup services
          let services = Services::new(storage, postgres);
          
          // Create app with routes
          let app = create_app(services);
          
          // Start server
          let addr = format!("{}:{}", config.host, config.port);
          let listener = TcpListener::bind(&addr).await?;
          tracing::info!("Server starting on {}", addr);
          
          axum::serve(listener, app).await?;
          Ok(())
      }
      ```
    - Success: Server starts and listens

22. **Create Mock Implementations Module**

    - File: `backend/lib/src/mocks/mod.rs`
    - Operation: Create new file
    - Details:
      ```rust
      #[cfg(feature = "mocks")]
      pub mod postgres_mock;
      #[cfg(feature = "mocks")]
      pub mod storage_mock;
      #[cfg(feature = "mocks")]
      pub mod rpc_mock;
      ```
    - Success: Mock modules conditionally compiled

23. **Implement PostgreSQL Mock**

    - File: `backend/lib/src/mocks/postgres_mock.rs`
    - Operation: Create new file
    - Details:
      ```rust
      use shc_indexer_db::models::{Bsp, File};
      use crate::error::Result;

      pub struct MockPostgresClient;

      impl MockPostgresClient {
          pub fn new() -> Self {
              Self
          }

          pub async fn get_active_bsps(&self) -> Result<Vec<Bsp>> {
              Ok(vec![])  // Return mock BSP data
          }

          pub async fn get_file_by_id(&self, _file_id: String) -> Result<Option<File>> {
              Ok(None)  // Return mock file data
          }
      }
      ```
    - Success: Mock provides test data

24. **Create Sample Configuration File**

    - File: `backend_config.toml`
    - Operation: Create new file
    - Details:
      ```toml
      host = "127.0.0.1"
      port = 8080

      [storage_hub]
      rpc_url = "ws://localhost:9944"
      mock_mode = true

      [database]
      url = "postgres://localhost:5432/storage_hub"
      mock_mode = true
      ```
    - Success: Configuration file created

25. **Create GitHub Actions Workflow**

    - File: `.github/workflows/backend.yml`
    - Operation: Create new file
    - Details:
      ```yaml
      name: Backend CI

      on:
        pull_request:
          paths:
            - 'backend/**'
            - '.github/workflows/backend.yml'
        push:
          branches:
            - main
            - perm-*
          paths:
            - 'backend/**'
            - '.github/workflows/backend.yml'
        workflow_dispatch:

      jobs:
        check-backend-fmt:
          name: "Check backend format with rustfmt"
          runs-on: ubuntu-latest
          steps:
            - uses: actions/checkout@v4
            - uses: actions-rust-lang/setup-rust-toolchain@v1.8
            - name: Rustfmt Check Backend
              run: |
                cargo fmt --manifest-path backend/lib/Cargo.toml -- --check
                cargo fmt --manifest-path backend/bin/Cargo.toml -- --check

        check-backend-lint:
          name: "Check backend lint with clippy"
          runs-on: ubuntu-latest
          steps:
            - uses: actions/checkout@v4
            - uses: actions-rust-lang/setup-rust-toolchain@v1.8
            - uses: Swatinem/rust-cache@v2
              with:
                cache-on-failure: true
                workspaces: backend
            - name: Install libpq-dev
              run: sudo apt-get update && sudo apt-get install -y libpq-dev
            - name: Clippy Check Backend
              run: |
                cargo clippy --manifest-path backend/lib/Cargo.toml --all-targets --features dev -- -D warnings
                cargo clippy --manifest-path backend/bin/Cargo.toml --all-targets -- -D warnings

        test-backend:
          name: "Test backend"
          runs-on: ubuntu-latest
          steps:
            - uses: actions/checkout@v4
            - uses: actions-rust-lang/setup-rust-toolchain@v1.8
            - uses: Swatinem/rust-cache@v2
              with:
                cache-on-failure: true
                workspaces: backend
            - name: Install libpq-dev
              run: sudo apt-get update && sudo apt-get install -y libpq-dev
            - name: Run Backend Tests
              run: cargo test --manifest-path backend/lib/Cargo.toml --features test
      ```
    - Success: CI pipeline configured

26. **Add Integration Tests**

    - File: `backend/lib/src/lib.rs` (append to existing file)
    - Operation: Add test module at end of file
    - Details:
      ```rust
      #[cfg(test)]
      mod tests {
          use super::*;
          use axum::{
              body::Body,
              http::{Request, StatusCode},
          };
          use tower::ServiceExt;
          use std::sync::Arc;
          use crate::data::storage::MemoryStorage;
          use crate::data::postgres::PostgresClient;

          #[tokio::test]
          async fn test_health_endpoint() {
              let storage = Arc::new(MemoryStorage::new());
              let postgres = Arc::new(PostgresClient::new("test".to_string()));
              let services = services::Services::new(storage, postgres);
              let app = create_app(services);

              let response = app
                  .oneshot(
                      Request::builder()
                          .uri("/health")
                          .body(Body::empty())
                          .unwrap(),
                  )
                  .await
                  .unwrap();

              assert_eq!(response.status(), StatusCode::OK);
          }

          #[tokio::test]
          async fn test_counter_endpoints() {
              let storage = Arc::new(MemoryStorage::new());
              let postgres = Arc::new(PostgresClient::new("test".to_string()));
              let services = services::Services::new(storage, postgres);
              let app = create_app(services);

              // Test GET /counter
              let response = app.clone()
                  .oneshot(
                      Request::builder()
                          .uri("/counter")
                          .body(Body::empty())
                          .unwrap(),
                  )
                  .await
                  .unwrap();
              assert_eq!(response.status(), StatusCode::OK);

              // Test POST /counter/inc
              let response = app.clone()
                  .oneshot(
                      Request::builder()
                          .method("POST")
                          .uri("/counter/inc")
                          .body(Body::empty())
                          .unwrap(),
                  )
                  .await
                  .unwrap();
              assert_eq!(response.status(), StatusCode::OK);
          }
      }
      ```
    - Success: Tests pass when run with `cargo test --features test`

### Testing Strategy

- [ ] Run `cargo fmt --all` to ensure code formatting
- [ ] Run `cargo clippy --all-targets --features dev -- -D warnings` to check for lints
- [ ] Run `cargo test --features test` to execute unit and integration tests
- [ ] Start server with `cargo run --bin backend --features dev`
- [ ] Test endpoints manually:
  - GET http://localhost:8080/health → 200 OK
  - GET http://localhost:8080/counter → {"count": 0}
  - POST http://localhost:8080/counter/inc → {"count": 1}
  - POST http://localhost:8080/counter/dec → {"count": 0}

### Rollback Plan

1. Remove `backend/` directory entirely
2. Revert changes to root `Cargo.toml` (remove backend workspace members)
3. Delete `.github/workflows/backend.yml`
4. Delete `backend_config.toml`
5. Run `git clean -fd` to remove any untracked files