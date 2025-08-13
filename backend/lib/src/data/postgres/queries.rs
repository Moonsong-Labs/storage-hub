//! Custom queries for StorageHub indexer database
//!
//! This module provides query functions that use the shc-indexer-db models
//! to retrieve data from the StorageHub indexer database.

use shc_indexer_db::models::{Bsp, File, Msp, PaymentStream};

use super::{PostgresClient, PostgresError};

impl PostgresClient {
    /// Get all active backup storage providers (BSPs)
    ///
    /// # Returns
    /// A vector of active BSPs from the indexer database
    pub async fn get_active_bsps(&self) -> Result<Vec<Bsp>, PostgresError> {
        todo!("Add to shc-indexer-db: SELECT * FROM bsp WHERE status = 'Active'")
    }

    /// Get all active main storage providers (MSPs)
    ///
    /// # Returns
    /// A vector of active MSPs from the indexer database
    pub async fn get_active_msps(&self) -> Result<Vec<Msp>, PostgresError> {
        todo!("Add to shc-indexer-db: SELECT * FROM msp WHERE status = 'Active'")
    }

    /// Get a file by its ID
    ///
    /// # Arguments
    /// * `file_id` - The unique identifier of the file
    ///
    /// # Returns
    /// The file metadata if found
    pub async fn get_file_by_id(&self, _file_id: &str) -> Result<Option<File>, PostgresError> {
        todo!("Add to shc-indexer-db: SELECT * FROM files WHERE file_id = $1")
    }

    /// Get all files for a specific user
    ///
    /// # Arguments
    /// * `user_id` - The user's account ID
    ///
    /// # Returns
    /// A vector of files owned by the user
    pub async fn get_files_by_user(&self, _user_id: &str) -> Result<Vec<File>, PostgresError> {
        todo!(
            "Add to shc-indexer-db: SELECT * FROM files WHERE owner = $1 ORDER BY created_at DESC"
        )
    }

    /// Get payment streams for a specific user
    ///
    /// # Arguments
    /// * `user_id` - The user's account ID
    ///
    /// # Returns
    /// A vector of payment streams associated with the user
    pub async fn get_payment_streams_for_user(
        &self,
        _user_id: &str,
    ) -> Result<Vec<PaymentStream>, PostgresError> {
        todo!("Add to shc-indexer-db: SELECT * FROM payment_streams WHERE user_account = $1 ORDER BY created_at DESC")
    }

    /// Get active payment streams for a provider
    ///
    /// # Arguments
    /// * `provider_id` - The provider's account ID
    ///
    /// # Returns
    /// A vector of active payment streams for the provider
    pub async fn get_active_payment_streams_for_provider(
        &self,
        _provider_id: &str,
    ) -> Result<Vec<PaymentStream>, PostgresError> {
        todo!("Add to shc-indexer-db: SELECT * FROM payment_streams WHERE provider_account = $1 AND status = 'Active' ORDER BY created_at DESC")
    }

    /// Get total storage used by a user
    ///
    /// # Arguments
    /// * `user_id` - The user's account ID
    ///
    /// # Returns
    /// The total storage in bytes used by the user
    pub async fn get_total_storage_used_by_user(
        &self,
        _user_id: &str,
    ) -> Result<i64, PostgresError> {
        todo!("Add to shc-indexer-db: SELECT COALESCE(SUM(size), 0) FROM files WHERE owner = $1")
    }

    /// Count active BSPs
    ///
    /// # Returns
    /// The number of active backup storage providers
    pub async fn count_active_bsps(&self) -> Result<i64, PostgresError> {
        todo!("Add to shc-indexer-db: SELECT COUNT(*) FROM bsp WHERE status = 'Active'")
    }

    /// Count active MSPs
    ///
    /// # Returns
    /// The number of active main storage providers
    pub async fn count_active_msps(&self) -> Result<i64, PostgresError> {
        todo!("Add to shc-indexer-db: SELECT COUNT(*) FROM msp WHERE status = 'Active'")
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::data::postgres::{AnyDbConnection, DbConfig, PgConnection};

    #[tokio::test]
    #[ignore = "Requires actual database"]
    // TODO
    async fn test_get_active_bsps() {
        let config = DbConfig::new("postgres://localhost/storagehub");
        let pg_conn = PgConnection::new(config)
            .await
            .expect("Failed to create connection");
        let client = PostgresClient::new(Arc::new(AnyDbConnection::Postgres(pg_conn))).await;

        let result = client.get_active_bsps().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "Requires actual database"]
    // TODO
    async fn test_get_file_by_id() {
        let config = DbConfig::new("postgres://localhost/storagehub");
        let pg_conn = PgConnection::new(config)
            .await
            .expect("Failed to create connection");
        let client = PostgresClient::new(Arc::new(AnyDbConnection::Postgres(pg_conn))).await;

        let result = client.get_file_by_id("test-file-id").await;
        assert!(result.is_ok());
    }
}
