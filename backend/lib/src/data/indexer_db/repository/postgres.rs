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
use shc_indexer_db::{models::Bsp, schema::bsp};

use crate::data::indexer_db::repository::{error::RepositoryResult, pool::SmartPool, IndexerOps};

#[cfg(test)]
use crate::data::indexer_db::repository::IndexerOpsMut;

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
}

#[cfg(test)]
#[async_trait]
impl IndexerOpsMut for Repository {
    // ============ BSP Write Operations ============
    async fn delete_bsp(&self, account: &str) -> RepositoryResult<()> {
        let mut conn = self.pool.get().await?;

        diesel::delete(bsp::table.filter(bsp::account.eq(account)))
            .execute(&mut *conn)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::test::DEFAULT_TEST_DATABASE_URL;

    #[tokio::test]
    // TODO: should NOT panic when we add testcontainers
    #[should_panic]
    async fn test_repo_read() {
        let repo = Repository::new(DEFAULT_TEST_DATABASE_URL)
            .await
            .expect("db available");

        repo.list_bsps(10, 0).await.expect("able to fetch bsps");
    }

    #[tokio::test]
    // TODO: should NOT panic when we add testcontainers
    #[should_panic]
    async fn test_repo_write() {
        let repo = Repository::new(DEFAULT_TEST_DATABASE_URL)
            .await
            .expect("db available");

        let original_bsps = repo.list_bsps(10, 0).await.expect("able to fetch bsps");
        let bsp = &original_bsps[0];

        repo.delete_bsp(&bsp.account)
            .await
            .expect("able to delete bsp");

        let changed_bsps = repo.list_bsps(10, 0).await.expect("able to fetch bsps");
        let another_bsp = &changed_bsps[0];

        assert_ne!(bsp.id, another_bsp.id);
    }
}
