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
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
#[cfg(test)]
use shc_indexer_db::OnchainBspId;
use shc_indexer_db::{
    models::{bucket::Bucket as DBBucket, payment_stream::PaymentStream, Bsp},
    schema::bsp,
};

#[cfg(test)]
use crate::data::indexer_db::repository::IndexerOpsMut;
use crate::data::indexer_db::repository::{
    error::{RepositoryError, RepositoryResult},
    pool::SmartPool,
    IndexerOps, PaymentStreamData, PaymentStreamKind,
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

    // ============ Payment Stream Operations ============
    async fn get_payment_streams_for_user(
        &self,
        user_account: &str,
    ) -> RepositoryResult<Vec<PaymentStreamData>> {
        let mut conn = self.pool.get().await?;

        // Get all payment streams for the user from the database
        let streams = PaymentStream::get_all_by_user(&mut conn, user_account.to_string()).await?;

        // Convert to our PaymentStreamData format, preserving BigDecimal types
        streams
            .into_iter()
            .map(|stream| {
                let kind = match (stream.rate, stream.amount_provided) {
                    (Some(rate), None) => Ok(PaymentStreamKind::Fixed { rate }),
                    (None, Some(amount_provided)) => {
                        Ok(PaymentStreamKind::Dynamic { amount_provided })
                    }
                    _ => Err(RepositoryError::configuration(
                        "payment stream must be either fixed or dynamic",
                    )),
                }?;

                Ok(PaymentStreamData {
                    provider: stream.provider,
                    total_amount_paid: stream.total_amount_paid,
                    kind,
                })
            })
            .collect::<Result<Vec<_>, _>>()
    }

    async fn calculate_msp_storage_for_user(
        &self,
        msp_id: i64,
        user_account: &str,
    ) -> RepositoryResult<BigDecimal> {
        let mut conn = self.pool.get().await?;

        // Get all buckets for this user with this MSP
        let buckets =
            DBBucket::get_user_buckets_by_msp(&mut conn, user_account.to_string(), msp_id).await?;

        // Calculate total size across all buckets
        let mut total_size = BigDecimal::from(0);
        for bucket in buckets {
            let bucket_size = DBBucket::calculate_size(&mut conn, bucket.id).await?;
            total_size = total_size + bucket_size;
        }

        Ok(total_size)
    }
}

#[cfg(test)]
#[async_trait]
impl IndexerOpsMut for Repository {
    // ============ BSP Write Operations ============
    async fn delete_bsp(&self, account: &OnchainBspId) -> RepositoryResult<()> {
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

        repo.delete_bsp(&OnchainBspId::try_from(bsp.account.clone()).unwrap())
            .await
            .expect("able to delete bsp");

        let changed_bsps = repo.list_bsps(10, 0).await.expect("able to fetch bsps");
        let another_bsp = &changed_bsps[0];

        assert_ne!(bsp.id, another_bsp.id);
    }
}
