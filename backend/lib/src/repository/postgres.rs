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
use bigdecimal::BigDecimal;
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use super::{
    error::RepositoryResult,
    pool::SmartPool,
    Bsp, Bucket, File, NewBsp, NewBucket, StorageOperations,
};

// Import the schema from shc_indexer_db
use shc_indexer_db::schema::{bsp, bucket, file};

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
impl StorageOperations for Repository {
    // ============ BSP Operations ============
    
    async fn create_bsp(&self, new_bsp: NewBsp) -> RepositoryResult<Bsp> {
        let mut conn = self.pool.get().await?;
        
        let now = Utc::now().naive_utc();
        
        // Insert the BSP
        let result: shc_indexer_db::models::Bsp = diesel::insert_into(bsp::table)
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
        
        Ok(Bsp {
            id: result.id,
            account: result.account,
            capacity: result.capacity,
            stake: result.stake,
            last_tick_proven: result.last_tick_proven,
            onchain_bsp_id: result.onchain_bsp_id,
            merkle_root: result.merkle_root,
            created_at: result.created_at,
            updated_at: result.updated_at,
        })
    }
    
    async fn get_bsp_by_id(&self, id: i64) -> RepositoryResult<Option<Bsp>> {
        let mut conn = self.pool.get().await?;
        
        let result: Option<shc_indexer_db::models::Bsp> = bsp::table
            .filter(bsp::id.eq(id))
            .first(&mut *conn)
            .await
            .optional()?;
        
        Ok(result.map(|r| Bsp {
            id: r.id,
            account: r.account,
            capacity: r.capacity,
            stake: r.stake,
            last_tick_proven: r.last_tick_proven,
            onchain_bsp_id: r.onchain_bsp_id,
            merkle_root: r.merkle_root,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    }
    
    async fn update_bsp_capacity(&self, id: i64, capacity: BigDecimal) -> RepositoryResult<Bsp> {
        let mut conn = self.pool.get().await?;
        
        let now = Utc::now().naive_utc();
        
        let result: shc_indexer_db::models::Bsp = diesel::update(bsp::table)
            .filter(bsp::id.eq(id))
            .set((
                bsp::capacity.eq(&capacity),
                bsp::updated_at.eq(&now),
            ))
            .get_result(&mut *conn)
            .await?;
        
        Ok(Bsp {
            id: result.id,
            account: result.account,
            capacity: result.capacity,
            stake: result.stake,
            last_tick_proven: result.last_tick_proven,
            onchain_bsp_id: result.onchain_bsp_id,
            merkle_root: result.merkle_root,
            created_at: result.created_at,
            updated_at: result.updated_at,
        })
    }
    
    async fn list_bsps(&self, limit: i64, offset: i64) -> RepositoryResult<Vec<Bsp>> {
        let mut conn = self.pool.get().await?;
        
        let results: Vec<shc_indexer_db::models::Bsp> = bsp::table
            .order(bsp::id.asc())
            .limit(limit)
            .offset(offset)
            .load(&mut *conn)
            .await?;
        
        Ok(results.into_iter().map(|r| Bsp {
            id: r.id,
            account: r.account,
            capacity: r.capacity,
            stake: r.stake,
            last_tick_proven: r.last_tick_proven,
            onchain_bsp_id: r.onchain_bsp_id,
            merkle_root: r.merkle_root,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }).collect())
    }
    
    // ============ Bucket Operations ============
    
    async fn create_bucket(&self, new_bucket: NewBucket) -> RepositoryResult<Bucket> {
        let mut conn = self.pool.get().await?;
        
        let now = Utc::now().naive_utc();
        
        let result: shc_indexer_db::models::Bucket = diesel::insert_into(bucket::table)
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
        
        Ok(Bucket {
            id: result.id,
            msp_id: result.msp_id,
            account: result.account,
            onchain_bucket_id: result.onchain_bucket_id,
            name: result.name,
            collection_id: result.collection_id,
            private: result.private,
            merkle_root: result.merkle_root,
            created_at: result.created_at,
            updated_at: result.updated_at,
        })
    }
    
    async fn get_bucket_by_id(&self, id: i64) -> RepositoryResult<Option<Bucket>> {
        let mut conn = self.pool.get().await?;
        
        let result: Option<shc_indexer_db::models::Bucket> = bucket::table
            .filter(bucket::id.eq(id))
            .first(&mut *conn)
            .await
            .optional()?;
        
        Ok(result.map(|r| Bucket {
            id: r.id,
            msp_id: r.msp_id,
            account: r.account,
            onchain_bucket_id: r.onchain_bucket_id,
            name: r.name,
            collection_id: r.collection_id,
            private: r.private,
            merkle_root: r.merkle_root,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    }
    
    async fn get_buckets_by_user(&self, user_account: &str) -> RepositoryResult<Vec<Bucket>> {
        let mut conn = self.pool.get().await?;
        
        let results: Vec<shc_indexer_db::models::Bucket> = bucket::table
            .filter(bucket::account.eq(user_account))
            .order(bucket::id.asc())
            .load(&mut *conn)
            .await?;
        
        Ok(results.into_iter().map(|r| Bucket {
            id: r.id,
            msp_id: r.msp_id,
            account: r.account,
            onchain_bucket_id: r.onchain_bucket_id,
            name: r.name,
            collection_id: r.collection_id,
            private: r.private,
            merkle_root: r.merkle_root,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }).collect())
    }
    
    // ============ File Operations ============
    
    async fn get_file_by_key(&self, key: &[u8]) -> RepositoryResult<Option<File>> {
        let mut conn = self.pool.get().await?;
        
        let result: Option<shc_indexer_db::models::File> = file::table
            .filter(file::file_key.eq(key))
            .first(&mut *conn)
            .await
            .optional()?;
        
        Ok(result.map(|r| File {
            id: r.id,
            account: r.account,
            file_key: r.file_key,
            bucket_id: r.bucket_id,
            location: r.location,
            fingerprint: r.fingerprint,
            size: r.size,
            step: r.step,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    }
    
    async fn get_files_by_user(&self, user_account: &[u8]) -> RepositoryResult<Vec<File>> {
        let mut conn = self.pool.get().await?;
        
        let results: Vec<shc_indexer_db::models::File> = file::table
            .filter(file::account.eq(user_account))
            .order(file::id.asc())
            .load(&mut *conn)
            .await?;
        
        Ok(results.into_iter().map(|r| File {
            id: r.id,
            account: r.account,
            file_key: r.file_key,
            bucket_id: r.bucket_id,
            location: r.location,
            fingerprint: r.fingerprint,
            size: r.size,
            step: r.step,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }).collect())
    }
    
    async fn get_files_by_bucket(&self, bucket_id: i64) -> RepositoryResult<Vec<File>> {
        let mut conn = self.pool.get().await?;
        
        let results: Vec<shc_indexer_db::models::File> = file::table
            .filter(file::bucket_id.eq(bucket_id))
            .order(file::id.asc())
            .load(&mut *conn)
            .await?;
        
        Ok(results.into_iter().map(|r| File {
            id: r.id,
            account: r.account,
            file_key: r.file_key,
            bucket_id: r.bucket_id,
            location: r.location,
            fingerprint: r.fingerprint,
            size: r.size,
            step: r.step,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Note: These tests require a test database to be available
    // They will automatically use test transactions that rollback
    
    #[tokio::test]
    #[ignore] // Ignore by default since it requires database
    async fn test_repository_creation() {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://test:test@localhost/test_db".to_string());
        
        let repo = Repository::new(&database_url).await;
        assert!(repo.is_ok());
    }
}