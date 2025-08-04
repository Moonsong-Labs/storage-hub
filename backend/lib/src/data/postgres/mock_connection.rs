//! Mock database connection implementation for testing
//!
//! This module provides a mock implementation of the `DbConnection` trait that simulates
//! database behavior for testing purposes. It stores test data in memory and supports
//! error simulation and delay injection for comprehensive testing scenarios.

use std::{
    collections::HashMap,
    fmt::Debug,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

use async_trait::async_trait;
use chrono::NaiveDateTime;
use diesel::{
    connection::{LoadConnection, TransactionManager},
    query_builder::{AsQuery, QueryFragment, QueryId},
    result::{ConnectionResult, Error as DieselError, QueryResult},
    RunQueryDsl,
};
use diesel_async::{AsyncConnection, SimpleAsyncConnection};
use tokio::time::sleep;

use shc_indexer_db::models::{Bucket, File, Msp};

use super::connection::{DbConnection, DbConnectionError};

#[cfg(test)]
use crate::constants::test::{
    accounts, buckets as test_buckets, file_keys, file_metadata, merkle, msp as test_msp, timestamps,
};

/// Test data storage for the mock connection
#[derive(Debug, Default)]
pub struct MockTestData {
    /// Stored files indexed by file_key
    pub files: HashMap<Vec<u8>, File>,
    /// Stored buckets indexed by ID
    pub buckets: HashMap<i64, Bucket>,
    /// Stored MSPs indexed by ID
    pub msps: HashMap<i64, Msp>,
    /// Next available IDs for auto-increment simulation
    pub next_file_id: i64,
    pub next_bucket_id: i64,
    pub next_msp_id: i64,
}

impl MockTestData {
    /// Create new test data with some defaults
    pub fn new() -> Self {
        let mut data = Self {
            files: HashMap::new(),
            buckets: HashMap::new(),
            msps: HashMap::new(),
            next_file_id: 1,
            next_bucket_id: 1,
            next_msp_id: 1,
        };

        // Add default test MSP
        let default_msp = Msp {
            id: data.next_msp_id,
            onchain_msp_id: vec![1, 2, 3, 4],
            account: vec![10, 11, 12, 13],
            value_prop: vec![100, 101, 102],
            created_at: NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
            updated_at: NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
        };
        data.msps.insert(default_msp.id, default_msp);
        data.next_msp_id += 1;

        // Add default test bucket
        let default_bucket = Bucket {
            id: data.next_bucket_id,
            msp_id: Some(1),
            account: hex::encode(&[50, 51, 52, 53]),
            onchain_bucket_id: vec![30, 31, 32, 33],
            name: vec![110, 111, 112, 113],
            collection_id: None,
            private: false,
            created_at: NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
            updated_at: NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
            merkle_root: vec![40, 41, 42, 43],
        };
        data.buckets.insert(default_bucket.id, default_bucket);
        data.next_bucket_id += 1;

        data
    }
}

/// Configuration for error simulation in the mock connection
#[derive(Debug, Clone)]
pub struct MockErrorConfig {
    /// Simulate connection failures
    pub connection_error: Option<String>,
    /// Simulate query errors
    pub query_error: Option<String>,
    /// Simulate timeout errors
    pub timeout_error: bool,
    /// Delay to inject before operations (milliseconds)
    pub delay_ms: Option<u64>,
}

impl Default for MockErrorConfig {
    fn default() -> Self {
        Self {
            connection_error: None,
            query_error: None,
            timeout_error: false,
            delay_ms: None,
        }
    }
}

/// Mock connection that simulates an AsyncPgConnection
///
/// This struct implements the necessary traits to be used as a connection
/// in diesel-async operations, storing test data in memory.
#[derive(Clone)]
pub struct MockAsyncConnection {
    data: Arc<Mutex<MockTestData>>,
    error_config: Arc<Mutex<MockErrorConfig>>,
}

impl MockAsyncConnection {
    /// Create a new mock connection with default test data
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(MockTestData::new())),
            error_config: Arc::new(Mutex::new(MockErrorConfig::default())),
        }
    }

    /// Get access to the test data for manipulation
    pub fn get_test_data(&self) -> MutexGuard<MockTestData> {
        self.data.lock().unwrap()
    }

    /// Configure error simulation
    pub fn set_error_config(&self, config: MockErrorConfig) {
        *self.error_config.lock().unwrap() = config;
    }

    /// Check if we should simulate an error
    async fn check_for_errors(&self) -> Result<(), DieselError> {
        let config = self.error_config.lock().unwrap().clone();

        // Inject delay if configured
        if let Some(delay_ms) = config.delay_ms {
            sleep(Duration::from_millis(delay_ms)).await;
        }

        // Check for connection error
        if let Some(error_msg) = config.connection_error {
            return Err(DieselError::DatabaseError(
                diesel::result::DatabaseErrorKind::ClosedConnection,
                Box::new(error_msg),
            ));
        }

        // Check for timeout
        if config.timeout_error {
            return Err(DieselError::DatabaseError(
                diesel::result::DatabaseErrorKind::Unknown,
                Box::new("Connection timeout".to_string()),
            ));
        }

        // Check for query error
        if let Some(error_msg) = config.query_error {
            return Err(DieselError::DatabaseError(
                diesel::result::DatabaseErrorKind::Unknown,
                Box::new(error_msg),
            ));
        }

        Ok(())
    }
}

impl Debug for MockAsyncConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockAsyncConnection")
            .field("has_data", &true)
            .finish()
    }
}

// Implement SimpleAsyncConnection for basic operations
#[async_trait]
impl SimpleAsyncConnection for MockAsyncConnection {
    async fn batch_execute(&mut self, _query: &str) -> QueryResult<()> {
        self.check_for_errors().await?;
        Ok(())
    }
}

// Implement AsyncConnection to make it work with diesel-async
#[async_trait]
impl AsyncConnection for MockAsyncConnection {
    type Backend = diesel::pg::Pg;
    type TransactionManager = MockTransactionManager;

    async fn establish(_database_url: &str) -> ConnectionResult<Self> {
        Ok(Self::new())
    }

    async fn transaction<'a, R, E, F>(&mut self, f: F) -> Result<R, E>
    where
        F: FnOnce(&mut Self) -> Pin<Box<dyn Future<Output = Result<R, E>> + Send + 'a>> + Send,
        E: From<DieselError> + Send,
        R: Send,
    {
        // For mock, we don't need real transaction semantics
        // Just execute the function
        f(self).await
    }

    async fn begin_test_transaction(&mut self) -> QueryResult<()> {
        Ok(())
    }

    async fn test_transaction<'a, R, E, F>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Self) -> Pin<Box<dyn Future<Output = R> + Send + 'a>> + Send,
        R: Send,
    {
        f(self).await
    }
}

// Mock transaction manager
pub struct MockTransactionManager;

impl<Conn: diesel::Connection> TransactionManager<Conn> for MockTransactionManager {
    type TransactionStateData = ();

    fn begin_transaction(conn: &mut Conn) -> QueryResult<()> {
        Ok(())
    }

    fn rollback_transaction(conn: &mut Conn) -> QueryResult<()> {
        Ok(())
    }

    fn commit_transaction(conn: &mut Conn) -> QueryResult<()> {
        Ok(())
    }

    fn transaction_manager_status_mut(conn: &mut Conn) -> &mut diesel::connection::TransactionManagerStatus {
        // Return a static reference for the mock
        unsafe {
            use std::mem::MaybeUninit;
            static mut STATUS: MaybeUninit<diesel::connection::TransactionManagerStatus> = MaybeUninit::uninit();
            static ONCE: std::sync::Once = std::sync::Once::new();
            
            ONCE.call_once(|| {
                STATUS.write(diesel::connection::TransactionManagerStatus::Valid(
                    diesel::connection::AnsiTransactionManager::default()
                ));
            });
            
            STATUS.assume_init_mut()
        }
    }
}

// Implement LoadConnection to support query execution
impl LoadConnection for MockAsyncConnection {
    type Cursor<'conn, 'query> = MockCursor;
    type Row<'conn, 'query> = MockRow;

    fn load<'conn, 'query, T>(
        &'conn mut self,
        source: T,
    ) -> QueryResult<Self::Cursor<'conn, 'query>>
    where
        T: AsQuery,
        T::Query: QueryFragment<Self::Backend> + QueryId + 'query,
    {
        // For the mock, we return an empty cursor
        // Real query handling would be implemented in the PostgresClient
        Ok(MockCursor::new())
    }
}

// Mock cursor for query results
pub struct MockCursor {
    rows: Vec<MockRow>,
    current: usize,
}

impl MockCursor {
    fn new() -> Self {
        Self {
            rows: vec![],
            current: 0,
        }
    }
}

impl Iterator for MockCursor {
    type Item = QueryResult<MockRow>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.rows.len() {
            let row = self.rows[self.current].clone();
            self.current += 1;
            Some(Ok(row))
        } else {
            None
        }
    }
}

// Mock row for query results
#[derive(Clone)]
pub struct MockRow {
    data: Vec<u8>,
}

impl MockRow {
    fn new() -> Self {
        Self { data: vec![] }
    }
}

// Implement RunQueryDsl to support execute operations
impl<T> RunQueryDsl<MockAsyncConnection> for T {}

/// Mock database connection pool implementation
///
/// This struct implements the `DbConnection` trait using mock connections
/// that store test data in memory and support error simulation.
#[derive(Clone)]
pub struct MockDbConnection {
    /// Shared test data storage
    data: Arc<Mutex<MockTestData>>,
    /// Error configuration
    error_config: Arc<Mutex<MockErrorConfig>>,
    /// Whether the connection pool is "healthy"
    is_healthy: Arc<Mutex<bool>>,
}

impl MockDbConnection {
    /// Create a new mock database connection
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(MockTestData::new())),
            error_config: Arc::new(Mutex::new(MockErrorConfig::default())),
            is_healthy: Arc::new(Mutex::new(true)),
        }
    }

    /// Add a test file to the mock data
    pub fn add_test_file(&self, mut file: File) {
        let mut data = self.data.lock().unwrap();
        if file.id == 0 {
            file.id = data.next_file_id;
            data.next_file_id += 1;
        }
        data.files.insert(file.file_key.clone(), file);
    }

    /// Add a test bucket to the mock data
    pub fn add_test_bucket(&self, mut bucket: Bucket) {
        let mut data = self.data.lock().unwrap();
        if bucket.id == 0 {
            bucket.id = data.next_bucket_id;
            data.next_bucket_id += 1;
        }
        data.buckets.insert(bucket.id, bucket);
    }

    /// Add a test MSP to the mock data
    pub fn add_test_msp(&self, mut msp: Msp) {
        let mut data = self.data.lock().unwrap();
        if msp.id == 0 {
            msp.id = data.next_msp_id;
            data.next_msp_id += 1;
        }
        data.msps.insert(msp.id, msp);
    }

    /// Clear all test data
    pub fn clear_data(&self) {
        let mut data = self.data.lock().unwrap();
        data.files.clear();
        data.buckets.clear();
        data.msps.clear();
        data.next_file_id = 1;
        data.next_bucket_id = 1;
        data.next_msp_id = 1;
    }

    /// Configure error simulation
    pub fn set_error_config(&self, config: MockErrorConfig) {
        *self.error_config.lock().unwrap() = config;
    }

    /// Set the health status of the connection pool
    pub fn set_healthy(&self, healthy: bool) {
        *self.is_healthy.lock().unwrap() = healthy;
    }

    /// Get a reference to the test data (for assertions in tests)
    pub fn get_test_data(&self) -> MutexGuard<MockTestData> {
        self.data.lock().unwrap()
    }
}

impl Default for MockDbConnection {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for MockDbConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockDbConnection")
            .field("is_healthy", &*self.is_healthy.lock().unwrap())
            .field("has_error_config", &self.error_config.lock().is_ok())
            .finish()
    }
}

#[async_trait]
impl DbConnection for MockDbConnection {
    type Connection = MockAsyncConnection;

    async fn get_connection(&self) -> Result<Self::Connection, DbConnectionError> {
        // Check if we should simulate a connection error
        let error_config = self.error_config.lock().unwrap().clone();
        
        if let Some(error_msg) = error_config.connection_error {
            return Err(DbConnectionError::Pool(error_msg));
        }

        if error_config.timeout_error {
            return Err(DbConnectionError::Pool("Connection timeout".to_string()));
        }

        // Create a new mock connection with shared data
        let conn = MockAsyncConnection {
            data: Arc::clone(&self.data),
            error_config: Arc::clone(&self.error_config),
        };

        Ok(conn)
    }

    async fn test_connection(&self) -> Result<(), DbConnectionError> {
        let _conn = self.get_connection().await?;
        Ok(())
    }

    async fn is_healthy(&self) -> bool {
        *self.is_healthy.lock().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shc_indexer_db::models::{File, FileStorageRequestStep};

    #[tokio::test]
    async fn test_successful_connection() {
        let mock_conn = MockDbConnection::new();
        let _client = mock_conn.get_connection().await.unwrap();
        
        // Successfully getting a connection is enough for the mock
        // Real query execution would be tested through the PostgresClient
    }

    #[tokio::test]
    async fn test_connection_failure() {
        let mock_conn = MockDbConnection::new();
        
        // Configure connection failure
        mock_conn.set_error_config(MockErrorConfig {
            connection_error: Some("Simulated connection failure".to_string()),
            ..Default::default()
        });

        let result = mock_conn.get_connection().await;
        assert!(result.is_err());
        match result {
            Err(DbConnectionError::Pool(msg)) => {
                assert_eq!(msg, "Simulated connection failure");
            }
            _ => panic!("Expected Pool error"),
        }
    }

    #[tokio::test]
    async fn test_timeout_simulation() {
        let mock_conn = MockDbConnection::new();
        
        // Configure timeout error
        mock_conn.set_error_config(MockErrorConfig {
            timeout_error: true,
            ..Default::default()
        });

        let result = mock_conn.get_connection().await;
        assert!(result.is_err());
        match result {
            Err(DbConnectionError::Pool(msg)) => {
                assert!(msg.contains("timeout"));
            }
            _ => panic!("Expected timeout error"),
        }
    }

    #[tokio::test]
    async fn test_health_check() {
        let mock_conn = MockDbConnection::new();
        
        // Should be healthy by default
        assert!(mock_conn.is_healthy().await);
        
        // Set unhealthy
        mock_conn.set_healthy(false);
        assert!(!mock_conn.is_healthy().await);
    }

    #[tokio::test]
    async fn test_test_data_management() {
        let mock_conn = MockDbConnection::new();
        
        // Add test file
        let test_file = File {
            id: 0, // Will be auto-assigned
            account: accounts::TEST_USER_ACCOUNT.to_vec(),
            file_key: file_keys::ALTERNATIVE_FILE_KEY.to_vec(),
            bucket_id: 1,
            location: file_metadata::ALTERNATIVE_LOCATION.to_vec(),
            fingerprint: file_metadata::ALTERNATIVE_FINGERPRINT.to_vec(),
            size: file_metadata::TEST_FILE_SIZE,
            step: FileStorageRequestStep::Stored as i32,
            created_at: NaiveDateTime::from_timestamp_opt(timestamps::TEST_TIMESTAMP, 0).unwrap(),
            updated_at: NaiveDateTime::from_timestamp_opt(timestamps::TEST_TIMESTAMP, 0).unwrap(),
        };
        
        mock_conn.add_test_file(test_file);
        
        {
            let data = mock_conn.get_test_data();
            assert_eq!(data.files.len(), 1);
            let stored_file = data.files.get(file_keys::ALTERNATIVE_FILE_KEY).unwrap();
            assert_eq!(stored_file.id, 1); // Auto-assigned ID
        }
        
        // Clear data
        mock_conn.clear_data();
        {
            let data = mock_conn.get_test_data();
            assert_eq!(data.files.len(), 0);
            assert_eq!(data.buckets.len(), 0);
            assert_eq!(data.msps.len(), 0);
        }
    }

    #[tokio::test]
    async fn test_default_test_data() {
        let mock_conn = MockDbConnection::new();
        let data = mock_conn.get_test_data();
        
        // Should have default MSP and bucket
        assert_eq!(data.msps.len(), 1);
        assert_eq!(data.buckets.len(), 1);
        
        // Check default MSP
        let default_msp = data.msps.get(&test_msp::DEFAULT_MSP_ID).unwrap();
        assert_eq!(default_msp.onchain_msp_id, test_msp::TEST_MSP_ONCHAIN_ID);
        
        // Check default bucket
        let default_bucket = data.buckets.get(&1).unwrap();
        assert_eq!(default_bucket.msp_id, Some(1));
    }
}

