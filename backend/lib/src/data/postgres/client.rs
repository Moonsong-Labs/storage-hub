//! PostgreSQL client for accessing StorageHub indexer database
//!
//! This module provides a client wrapper around diesel-async connections
//! for querying the existing StorageHub indexer database in a read-only manner.

use std::sync::Arc;

use thiserror::Error;

use super::connection::{AnyDbConnection, DbConnection, DbConnectionError};

/// Errors that can occur during PostgreSQL operations
#[derive(Debug, Error)]
pub enum PostgresError {
    /// Connection error
    #[error("Connection error: {0}")]
    Connection(#[from] DbConnectionError),

    /// Database query error
    #[error("Database error: {0}")]
    Query(#[from] diesel::result::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
}

/// PostgreSQL client for read-only access to StorageHub indexer database
///
/// This client provides methods to query BSP/MSP information, file metadata,
/// payment streams, and other blockchain-indexed data.
#[derive(Clone)]
pub struct PostgresClient {
    /// Database connection abstraction
    conn: Arc<AnyDbConnection>,
}

impl PostgresClient {
    /// Create a new PostgreSQL client with the given connection
    pub async fn new(conn: Arc<AnyDbConnection>) -> Self {
        Self { conn }
    }

    /// Test the database connection
    pub async fn test_connection(&self) -> Result<(), PostgresError> {
        self.conn.test_connection().await?;
        Ok(())
    }
}

// Implement PostgresClientTrait for PostgresClient
#[async_trait::async_trait]
impl super::PostgresClientTrait for PostgresClient {
    async fn test_connection(&self) -> crate::error::Result<()> {
        self.test_connection()
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }

    async fn get_file_by_key(
        &self,
        file_key: &[u8],
    ) -> crate::error::Result<shc_indexer_db::models::File> {
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;
        use shc_indexer_db::schema::file;

        let mut conn = self
            .conn
            .get_connection()
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))?;

        let file_key = file_key.to_vec();
        file::table
            .filter(file::file_key.eq(file_key))
            .first::<shc_indexer_db::models::File>(&mut conn)
            .await
            .map_err(|e| match e {
                diesel::result::Error::NotFound => {
                    crate::error::Error::NotFound("File not found".to_string())
                }
                _ => crate::error::Error::Database(e.to_string()),
            })
    }

    async fn get_files_by_user(
        &self,
        user_account: &[u8],
        pagination: Option<super::PaginationParams>,
    ) -> crate::error::Result<Vec<shc_indexer_db::models::File>> {
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;
        use shc_indexer_db::schema::file;

        let mut conn = self
            .conn
            .get_connection()
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))?;

        let account = user_account.to_vec();
        let mut query = file::table.filter(file::account.eq(account)).into_boxed();

        if let Some(params) = pagination {
            if let Some(limit) = params.limit {
                query = query.limit(limit);
            }
            if let Some(offset) = params.offset {
                query = query.offset(offset);
            }
        }

        query
            .load(&mut conn)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }

    async fn get_files_by_user_and_msp(
        &self,
        user_account: &[u8],
        msp_id: i64,
        pagination: Option<super::PaginationParams>,
    ) -> crate::error::Result<Vec<shc_indexer_db::models::File>> {
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;
        use shc_indexer_db::schema::{bucket, file};

        let mut conn = self
            .conn
            .get_connection()
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))?;

        let account = user_account.to_vec();
        let mut query = file::table
            .inner_join(bucket::table.on(file::bucket_id.eq(bucket::id)))
            .filter(file::account.eq(account))
            .filter(bucket::msp_id.eq(msp_id))
            .select(file::all_columns)
            .into_boxed();

        if let Some(params) = pagination {
            if let Some(limit) = params.limit {
                query = query.limit(limit);
            }
            if let Some(offset) = params.offset {
                query = query.offset(offset);
            }
        }

        query
            .load(&mut conn)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }

    async fn get_files_by_bucket_id(
        &self,
        bucket_id: i64,
        pagination: Option<super::PaginationParams>,
    ) -> crate::error::Result<Vec<shc_indexer_db::models::File>> {
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;
        use shc_indexer_db::schema::file;

        let mut conn = self
            .conn
            .get_connection()
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))?;

        let mut query = file::table
            .filter(file::bucket_id.eq(bucket_id))
            .into_boxed();

        if let Some(params) = pagination {
            if let Some(limit) = params.limit {
                query = query.limit(limit);
            }
            if let Some(offset) = params.offset {
                query = query.offset(offset);
            }
        }

        query
            .load(&mut conn)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }

    async fn create_file(
        &self,
        _file: shc_indexer_db::models::File,
    ) -> crate::error::Result<shc_indexer_db::models::File> {
        // Note: The indexer database should be read-only from the backend perspective
        // This method is primarily for testing with mocks
        Err(crate::error::Error::Database(
            "Cannot create files in read-only database".to_string(),
        ))
    }

    async fn update_file_step(
        &self,
        _file_key: &[u8],
        _step: shc_indexer_db::models::FileStorageRequestStep,
    ) -> crate::error::Result<()> {
        // Note: The indexer database should be read-only from the backend perspective
        // This method is primarily for testing with mocks
        Err(crate::error::Error::Database(
            "Cannot update files in read-only database".to_string(),
        ))
    }

    async fn delete_file(&self, _file_key: &[u8]) -> crate::error::Result<()> {
        // Note: The indexer database should be read-only from the backend perspective
        // This method is primarily for testing with mocks
        Err(crate::error::Error::Database(
            "Cannot delete files in read-only database".to_string(),
        ))
    }

    async fn get_bucket_by_id(
        &self,
        bucket_id: i64,
    ) -> crate::error::Result<shc_indexer_db::models::Bucket> {
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;
        use shc_indexer_db::schema::bucket;

        let mut conn = self
            .conn
            .get_connection()
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))?;

        bucket::table
            .filter(bucket::id.eq(bucket_id))
            .first::<shc_indexer_db::models::Bucket>(&mut conn)
            .await
            .map_err(|e| match e {
                diesel::result::Error::NotFound => {
                    crate::error::Error::NotFound("Bucket not found".to_string())
                }
                _ => crate::error::Error::Database(e.to_string()),
            })
    }

    async fn get_buckets_by_user(
        &self,
        user_account: &[u8],
        pagination: Option<super::PaginationParams>,
    ) -> crate::error::Result<Vec<shc_indexer_db::models::Bucket>> {
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;
        use shc_indexer_db::schema::bucket;

        let mut conn = self
            .conn
            .get_connection()
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))?;

        // Convert user_account bytes to hex string for comparison with bucket.account
        let account = hex::encode(user_account);
        let mut query = bucket::table
            .filter(bucket::account.eq(account))
            .into_boxed();

        if let Some(params) = pagination {
            if let Some(limit) = params.limit {
                query = query.limit(limit);
            }
            if let Some(offset) = params.offset {
                query = query.offset(offset);
            }
        }

        query
            .load(&mut conn)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }

    async fn get_msp_by_id(
        &self,
        msp_id: i64,
    ) -> crate::error::Result<shc_indexer_db::models::Msp> {
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;
        use shc_indexer_db::schema::msp;

        let mut conn = self
            .conn
            .get_connection()
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))?;

        msp::table
            .filter(msp::id.eq(msp_id))
            .first::<shc_indexer_db::models::Msp>(&mut conn)
            .await
            .map_err(|e| match e {
                diesel::result::Error::NotFound => {
                    crate::error::Error::NotFound("MSP not found".to_string())
                }
                _ => crate::error::Error::Database(e.to_string()),
            })
    }

    async fn get_all_msps(
        &self,
        pagination: Option<super::PaginationParams>,
    ) -> crate::error::Result<Vec<shc_indexer_db::models::Msp>> {
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;
        use shc_indexer_db::schema::msp;

        let mut conn = self
            .conn
            .get_connection()
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))?;

        let mut query = msp::table.into_boxed();

        if let Some(params) = pagination {
            if let Some(limit) = params.limit {
                query = query.limit(limit);
            }
            if let Some(offset) = params.offset {
                query = query.offset(offset);
            }
        }

        query
            .load(&mut conn)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }

    async fn execute_raw_query(
        &self,
        _query: &str,
    ) -> crate::error::Result<Vec<serde_json::Value>> {
        // For security reasons, raw queries might be disabled in production
        Err(crate::error::Error::Database(
            "Raw queries are not supported".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::postgres::{AnyDbConnection, DbConfig, PgConnection};

    #[tokio::test]
    #[ignore = "Requires actual database"]
    async fn test_client_creation() {
        let config = DbConfig::new("postgres://localhost/test");
        let pg_conn = PgConnection::new(config)
            .await
            .expect("Failed to create connection");
        let client = PostgresClient::new(Arc::new(AnyDbConnection::Real(pg_conn))).await;
        let result = client.test_connection().await;
        assert!(result.is_ok());
    }
}
