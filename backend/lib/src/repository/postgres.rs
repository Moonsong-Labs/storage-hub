//! PostgreSQL repository implementation.
//!
//! This module provides the production repository implementation using
//! PostgreSQL as the backing database through diesel-async.
//!
//! ## Key Components
//! - [`Repository`] - PostgreSQL implementation of StorageOperations
//!
//! ## Features
//! - Connection pooling through SmartPool
//! - Automatic test transactions in test mode
//! - Type-safe queries through diesel
//! - Comprehensive error handling

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use super::{
    error::RepositoryResult,
    pool::SmartPool,
    IndexerOps,
};

#[cfg(test)]
use super::{IndexerOpsMut, NewBsp, NewBucket, NewFile};
#[cfg(test)]
use bigdecimal::BigDecimal;
#[cfg(test)]
use chrono::Utc;

// Import models and schema from shc_indexer_db
use shc_indexer_db::{
    models::{Bsp, Bucket, File},
    schema::{bsp, bucket, file},
};

/// PostgreSQL repository implementation.
///
/// Provides all database operations using a connection pool
/// with automatic test transaction management.
pub struct Repository {
    pool: SmartPool,
}

impl Repository {
    /// Create a new Repository with the given database URL.
    ///
    /// # Arguments
    /// * `database_url` - PostgreSQL connection string
    ///
    /// # Returns
    /// * `Result<Self, RepositoryError>` - Repository instance or error
    pub async fn new(database_url: &str) -> RepositoryResult<Self> {
        Ok(Self {
            pool: SmartPool::new(database_url).await?,
        })
    }
}

#[async_trait]
impl IndexerOps for Repository {
    // ============ BSP Read Operations ============

    async fn get_bsp_by_id(&self, id: i64) -> RepositoryResult<Option<Bsp>> {
        let mut conn = self.pool.get().await?;

        let result: Option<Bsp> = bsp::table
            .filter(bsp::id.eq(id))
            .first(&mut *conn)
            .await
            .optional()?;

        Ok(result)
    }

    async fn list_bsps(&self, limit: i64, offset: i64) -> RepositoryResult<Vec<Bsp>> {
        let mut conn = self.pool.get().await?;

        let results: Vec<Bsp> = bsp::table
            .order(bsp::id.asc())
            .limit(limit)
            .offset(offset)
            .load(&mut *conn)
            .await?;

        Ok(results)
    }

    // ============ Bucket Read Operations ============

    async fn get_bucket_by_id(&self, id: i64) -> RepositoryResult<Option<Bucket>> {
        let mut conn = self.pool.get().await?;

        let result: Option<Bucket> = bucket::table
            .filter(bucket::id.eq(id))
            .first(&mut *conn)
            .await
            .optional()?;

        Ok(result)
    }

    async fn get_buckets_by_user(&self, user_account: &str) -> RepositoryResult<Vec<Bucket>> {
        let mut conn = self.pool.get().await?;

        let results: Vec<Bucket> = bucket::table
            .filter(bucket::account.eq(user_account))
            .order(bucket::id.asc())
            .load(&mut *conn)
            .await?;

        Ok(results)
    }

    // ============ File Read Operations ============

    async fn get_file_by_key(&self, key: &[u8]) -> RepositoryResult<Option<File>> {
        let mut conn = self.pool.get().await?;

        let result: Option<File> = file::table
            .filter(file::file_key.eq(key))
            .first(&mut *conn)
            .await
            .optional()?;

        Ok(result)
    }

    async fn get_files_by_user(&self, user_account: &[u8]) -> RepositoryResult<Vec<File>> {
        let mut conn = self.pool.get().await?;

        let results: Vec<File> = file::table
            .filter(file::account.eq(user_account))
            .order(file::id.asc())
            .load(&mut *conn)
            .await?;

        Ok(results)
    }

    async fn get_files_by_bucket(&self, bucket_id: i64) -> RepositoryResult<Vec<File>> {
        let mut conn = self.pool.get().await?;

        let results: Vec<File> = file::table
            .filter(file::bucket_id.eq(bucket_id))
            .order(file::id.asc())
            .load(&mut *conn)
            .await?;

        Ok(results)
    }
}

// Test-only implementation of IndexerOpsMut
#[cfg(test)]
#[async_trait]
impl IndexerOpsMut for Repository {
    // ============ BSP Write Operations ============

    async fn create_bsp(&self, new_bsp: NewBsp) -> RepositoryResult<Bsp> {
        let mut conn = self.pool.get().await?;

        let now = Utc::now().naive_utc();

        // Insert the BSP
        let result: Bsp = diesel::insert_into(bsp::table)
            .values((
                bsp::account.eq(&new_bsp.account),
                bsp::capacity.eq(&new_bsp.capacity),
                bsp::stake.eq(&new_bsp.stake),
                bsp::onchain_bsp_id.eq(&new_bsp.onchain_bsp_id),
                bsp::merkle_root.eq(&new_bsp.merkle_root),
                bsp::last_tick_proven.eq(0i64),
                bsp::created_at.eq(&now),
                bsp::updated_at.eq(&now),
            ))
            .get_result(&mut *conn)
            .await?;

        // TODO: Handle multiaddresses insertion if needed
        // This would require creating multiaddress records and associations

        Ok(result)
    }

    async fn update_bsp_capacity(&self, id: i64, capacity: BigDecimal) -> RepositoryResult<Bsp> {
        let mut conn = self.pool.get().await?;

        let now = Utc::now().naive_utc();

        let result: Bsp = diesel::update(bsp::table)
            .filter(bsp::id.eq(id))
            .set((bsp::capacity.eq(&capacity), bsp::updated_at.eq(&now)))
            .get_result(&mut *conn)
            .await?;

        Ok(result)
    }

    async fn delete_bsp(&self, account: &str) -> RepositoryResult<()> {
        let mut conn = self.pool.get().await?;
        
        diesel::delete(bsp::table.filter(bsp::account.eq(account)))
            .execute(&mut *conn)
            .await?;
        
        Ok(())
    }

    // ============ Bucket Write Operations ============

    async fn create_bucket(&self, new_bucket: NewBucket) -> RepositoryResult<Bucket> {
        let mut conn = self.pool.get().await?;

        let now = Utc::now().naive_utc();

        let result: Bucket = diesel::insert_into(bucket::table)
            .values((
                bucket::msp_id.eq(&new_bucket.msp_id),
                bucket::account.eq(&new_bucket.account),
                bucket::onchain_bucket_id.eq(&new_bucket.onchain_bucket_id),
                bucket::name.eq(&new_bucket.name),
                bucket::collection_id.eq(&new_bucket.collection_id),
                bucket::private.eq(&new_bucket.private),
                bucket::merkle_root.eq(&new_bucket.merkle_root),
                bucket::created_at.eq(&now),
                bucket::updated_at.eq(&now),
            ))
            .get_result(&mut *conn)
            .await?;

        Ok(result)
    }

    // ============ File Write Operations ============

    async fn create_file(&self, new_file: NewFile) -> RepositoryResult<File> {
        let mut conn = self.pool.get().await?;
        let now = Utc::now().naive_utc();
        
        let result = diesel::insert_into(file::table)
            .values((
                file::account.eq(&new_file.account),
                file::file_key.eq(&new_file.file_key),
                file::bucket_id.eq(&new_file.bucket_id),
                file::location.eq(&new_file.location),
                file::fingerprint.eq(&new_file.fingerprint),
                file::size.eq(&new_file.size),
                file::step.eq(&new_file.step),
                file::created_at.eq(&now),
                file::updated_at.eq(&now),
            ))
            .get_result(&mut *conn)
            .await?;
        
        Ok(result)
    }

    async fn update_file_step(&self, file_key: &[u8], step: i32) -> RepositoryResult<()> {
        let mut conn = self.pool.get().await?;
        
        diesel::update(file::table.filter(file::file_key.eq(file_key)))
            .set((
                file::step.eq(step),
                file::updated_at.eq(Utc::now().naive_utc()),
            ))
            .execute(&mut *conn)
            .await?;
        
        Ok(())
    }

    async fn delete_file(&self, file_key: &[u8]) -> RepositoryResult<()> {
        let mut conn = self.pool.get().await?;
        
        diesel::delete(file::table.filter(file::file_key.eq(file_key)))
            .execute(&mut *conn)
            .await?;
        
        Ok(())
    }

    async fn clear_all(&self) {
        // For PostgreSQL in tests, rely on transaction rollback instead
        // This method becomes a no-op as test transactions handle cleanup
        // The SmartPool automatically rolls back test transactions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Note: These tests require a test database to be available
    // They will automatically use test transactions that rollback
    use crate::constants::test::DEFAULT_TEST_DATABASE_URL;

    #[tokio::test]
    #[ignore] // Ignore by default since it requires database
    async fn test_repository_creation() {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| DEFAULT_TEST_DATABASE_URL.to_string());

        let repo = Repository::new(&database_url).await;
        assert!(repo.is_ok());
    }
}