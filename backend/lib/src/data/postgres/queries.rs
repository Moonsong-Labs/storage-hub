//! Custom queries for StorageHub indexer database
//!
//! This module provides query functions that use the shc-indexer-db models
//! to retrieve data from the StorageHub indexer database.

use super::{PostgresClient, PostgresError};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use shc_indexer_db::models::{BackupStorageProvider, File, MainStorageProvider, PaymentStream};
use shc_indexer_db::schema::{
    backup_storage_providers, files, main_storage_providers, payment_streams,
};

impl PostgresClient {
    /// Get all active backup storage providers (BSPs)
    ///
    /// # Returns
    /// A vector of active BSPs from the indexer database
    pub async fn get_active_bsps(&self) -> Result<Vec<BackupStorageProvider>, PostgresError> {
        let mut conn = self.get_connection().await?;

        let bsps = backup_storage_providers::table
            .filter(backup_storage_providers::status.eq("Active"))
            .load::<BackupStorageProvider>(&mut conn)
            .await?;

        Ok(bsps)
    }

    /// Get all active main storage providers (MSPs)
    ///
    /// # Returns
    /// A vector of active MSPs from the indexer database
    pub async fn get_active_msps(&self) -> Result<Vec<MainStorageProvider>, PostgresError> {
        let mut conn = self.get_connection().await?;

        let msps = main_storage_providers::table
            .filter(main_storage_providers::status.eq("Active"))
            .load::<MainStorageProvider>(&mut conn)
            .await?;

        Ok(msps)
    }

    /// Get a file by its ID
    ///
    /// # Arguments
    /// * `file_id` - The unique identifier of the file
    ///
    /// # Returns
    /// The file metadata if found
    pub async fn get_file_by_id(&self, file_id: &str) -> Result<Option<File>, PostgresError> {
        let mut conn = self.get_connection().await?;

        let file = files::table
            .filter(files::file_id.eq(file_id))
            .first::<File>(&mut conn)
            .await
            .optional()?;

        Ok(file)
    }

    /// Get all files for a specific user
    ///
    /// # Arguments
    /// * `user_id` - The user's account ID
    ///
    /// # Returns
    /// A vector of files owned by the user
    pub async fn get_files_by_user(&self, user_id: &str) -> Result<Vec<File>, PostgresError> {
        let mut conn = self.get_connection().await?;

        let files = files::table
            .filter(files::owner.eq(user_id))
            .order(files::created_at.desc())
            .load::<File>(&mut conn)
            .await?;

        Ok(files)
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
        user_id: &str,
    ) -> Result<Vec<PaymentStream>, PostgresError> {
        let mut conn = self.get_connection().await?;

        let streams = payment_streams::table
            .filter(payment_streams::user_account.eq(user_id))
            .order(payment_streams::created_at.desc())
            .load::<PaymentStream>(&mut conn)
            .await?;

        Ok(streams)
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
        provider_id: &str,
    ) -> Result<Vec<PaymentStream>, PostgresError> {
        let mut conn = self.get_connection().await?;

        let streams = payment_streams::table
            .filter(payment_streams::provider_account.eq(provider_id))
            .filter(payment_streams::status.eq("Active"))
            .order(payment_streams::created_at.desc())
            .load::<PaymentStream>(&mut conn)
            .await?;

        Ok(streams)
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
        user_id: &str,
    ) -> Result<i64, PostgresError> {
        let mut conn = self.get_connection().await?;

        let total: Option<i64> = files::table
            .filter(files::owner.eq(user_id))
            .select(diesel::dsl::sum(files::size))
            .first(&mut conn)
            .await?;

        Ok(total.unwrap_or(0))
    }

    /// Count active BSPs
    ///
    /// # Returns
    /// The number of active backup storage providers
    pub async fn count_active_bsps(&self) -> Result<i64, PostgresError> {
        let mut conn = self.get_connection().await?;

        let count = backup_storage_providers::table
            .filter(backup_storage_providers::status.eq("Active"))
            .count()
            .get_result(&mut conn)
            .await?;

        Ok(count)
    }

    /// Count active MSPs
    ///
    /// # Returns
    /// The number of active main storage providers
    pub async fn count_active_msps(&self) -> Result<i64, PostgresError> {
        let mut conn = self.get_connection().await?;

        let count = main_storage_providers::table
            .filter(main_storage_providers::status.eq("Active"))
            .count()
            .get_result(&mut conn)
            .await?;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires actual database
    async fn test_get_active_bsps() {
        let client = PostgresClient::new("postgres://localhost/storagehub")
            .await
            .unwrap();

        let result = client.get_active_bsps().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires actual database
    async fn test_get_file_by_id() {
        let client = PostgresClient::new("postgres://localhost/storagehub")
            .await
            .unwrap();

        let result = client.get_file_by_id("test-file-id").await;
        assert!(result.is_ok());
    }
}