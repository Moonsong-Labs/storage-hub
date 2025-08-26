//! Database client wrapper using repository pattern abstraction
//!
//! This module provides a database client that delegates all operations
//! to an underlying repository implementation, allowing for both production
//! PostgreSQL and mock implementations for testing.

use std::sync::Arc;

use shc_indexer_db::models::Bsp;

use crate::{
    constants::database::DEFAULT_PAGE_LIMIT, data::indexer_db::repository::StorageOperations,
};

/// Database client that delegates to a repository implementation
///
/// This client provides a clean abstraction over database operations,
/// delegating all actual work to an underlying repository that implements
/// the `StorageOperations` trait. This allows for easy swapping between
/// production PostgreSQL and mock implementations for testing.
///
/// ## Usage Example
/// ```ignore
/// use repository::{Repository, StorageOperations};
/// use data::postgres::DBClient;
///
/// // Production usage with PostgreSQL
/// let repo = Repository::new(&database_url).await?;
/// let client = DBClient::new(Arc::new(repo));
///
/// // Test usage with mock (when available)
/// let mock_repo = MockRepository::new();
/// let client = DBClient::new(Arc::new(mock_repo));
/// ```
#[derive(Clone)]
pub struct DBClient {
    repository: Arc<dyn StorageOperations>,
}

impl DBClient {
    /// Create a new database client with the given repository
    ///
    /// # Arguments
    /// * `repository` - Repository implementation to use for database operations
    pub fn new(repository: Arc<dyn StorageOperations>) -> Self {
        Self { repository }
    }

    /// Test the database connection
    pub async fn test_connection(&self) -> crate::error::Result<()> {
        // Try to list BSPs with a limit of 1 to test the connection
        self.repository
            .list_bsps(1, 0)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))?;
        Ok(())
    }

    /// Get all BSPs with optional pagination
    pub async fn get_all_bsps(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> crate::error::Result<Vec<Bsp>> {
        let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);
        let offset = offset.unwrap_or(0);

        self.repository
            .list_bsps(limit, offset)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }
}

// Test-only mutable operations
#[cfg(test)]
impl DBClient {
    /// Delete a BSP
    pub async fn delete_bsp(&self, account: &str) -> crate::error::Result<()> {
        self.repository
            .delete_bsp(account)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{
        constants::{database::DEFAULT_DATABASE_URL, test::bsp::DEFAULT_BSP_ID},
        data::indexer_db::{mock_repository::tests::inject_sample_bsp, MockRepository, Repository},
    };

    async fn delete_bsp(client: DBClient, id: i64) {
        let bsps = client
            // ensure we get as many as possible
            .get_all_bsps(Some(i64::MAX), Some(0))
            .await
            .expect("able to retrieve all bsps");

        let amount_of_bsps = bsps.len();
        assert!(amount_of_bsps > 0);

        let target_bsp = bsps
            .iter()
            .find(|bsp| bsp.id == id)
            .expect("bsp id in list of bsps");

        client
            .delete_bsp(&target_bsp.account)
            .await
            .expect("able to delete bsp");

        let bsps = client
            .get_all_bsps(Some(i64::MAX), Some(0))
            .await
            .expect("able to retrieve all bsps");

        assert_eq!(bsps.len(), amount_of_bsps - 1);
    }

    #[tokio::test]
    async fn delete_bsp_with_mock_repo() {
        // Create mock repository and add test data
        let repo = MockRepository::new();
        let id = inject_sample_bsp(&repo).await;

        // initialize client
        let client = DBClient::new(Arc::new(repo));
        delete_bsp(client, id).await;
    }

    #[tokio::test]
    // TODO: should NOT panic when we add testcontainers
    #[should_panic]
    async fn delete_bsp_with_repo() {
        // TODO: seed db with bsp

        let repo = Repository::new(DEFAULT_DATABASE_URL)
            .await
            .expect("able to connect to db");

        let client = DBClient::new(Arc::new(repo));
        delete_bsp(client, DEFAULT_BSP_ID).await;
    }
}
