//! Custom queries for StorageHub indexer database
//!
//! This module provides query functions that use the shc-indexer-db models
//! to retrieve data from the StorageHub indexer database.

use shc_indexer_db::models::{Bsp, File, Msp, PaymentStream};

use super::DBClient;

impl DBClient {
    /// Get all active backup storage providers (BSPs)
    ///
    /// # Returns
    /// A vector of active BSPs from the indexer database
    pub async fn get_active_bsps(&self) -> Result<Vec<Bsp>, crate::error::Error> {
        todo!("Add to shc-indexer-db: SELECT * FROM bsp WHERE status = 'Active'")
    }

    /// Get all active main storage providers (MSPs)
    ///
    /// # Returns
    /// A vector of active MSPs from the indexer database
    pub async fn get_active_msps(&self) -> Result<Vec<Msp>, crate::error::Error> {
        todo!("Add to shc-indexer-db: SELECT * FROM msp WHERE status = 'Active'")
    }

    /// Get a file by its ID
    ///
    /// # Arguments
    /// * `file_id` - The unique identifier of the file
    ///
    /// # Returns
    /// The file metadata if found
    pub async fn get_file_by_id(
        &self,
        _file_id: &str,
    ) -> Result<Option<File>, crate::error::Error> {
        todo!("Add to shc-indexer-db: SELECT * FROM files WHERE file_id = $1")
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
    ) -> Result<Vec<PaymentStream>, crate::error::Error> {
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
    ) -> Result<Vec<PaymentStream>, crate::error::Error> {
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
    ) -> Result<i64, crate::error::Error> {
        todo!("Add to shc-indexer-db: SELECT COALESCE(SUM(size), 0) FROM files WHERE owner = $1")
    }

    /// Count active BSPs
    ///
    /// # Returns
    /// The number of active backup storage providers
    pub async fn count_active_bsps(&self) -> Result<i64, crate::error::Error> {
        todo!("Add to shc-indexer-db: SELECT COUNT(*) FROM bsp WHERE status = 'Active'")
    }

    /// Count active MSPs
    ///
    /// # Returns
    /// The number of active main storage providers
    pub async fn count_active_msps(&self) -> Result<i64, crate::error::Error> {
        todo!("Add to shc-indexer-db: SELECT COUNT(*) FROM msp WHERE status = 'Active'")
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        constants::test::{
            accounts::{TEST_BSP_ACCOUNT_STR, TEST_USER_ACCOUNT},
            bsp::{DEFAULT_CAPACITY, DEFAULT_STAKE, TEST_BSP_ONCHAIN_ID_STR},
            buckets,
            file_keys::ALTERNATIVE_FILE_KEY,
            file_metadata::{
                ALTERNATIVE_FINGERPRINT, ALTERNATIVE_LOCATION, TEST_FILE_SIZE, UPDATED_STEP,
            },
            merkle::BSP_MERKLE_ROOT,
            network::TEST_MULTIADDRESSES,
        },
        data::postgres::DBClient,
    };

    #[cfg(feature = "mocks")]
    #[tokio::test]
    async fn test_db_client_with_mock_repository() {
        use std::sync::Arc;

        use bigdecimal::BigDecimal;

        use crate::repository::{IndexerOpsMut, MockRepository, NewBsp};

        // Create mock repository and add test data
        let mock_repo = MockRepository::new();

        // Add a test BSP
        let new_bsp = NewBsp {
            account: TEST_BSP_ACCOUNT_STR.to_string(),
            capacity: BigDecimal::from(DEFAULT_CAPACITY),
            stake: BigDecimal::from(DEFAULT_STAKE * 5),
            onchain_bsp_id: TEST_BSP_ONCHAIN_ID_STR.to_string(),
            merkle_root: BSP_MERKLE_ROOT.to_vec(),
            multiaddresses: vec![TEST_MULTIADDRESSES.to_vec()],
        };

        let created_bsp = mock_repo.create_bsp(new_bsp).await.unwrap();

        // Create DBClient with mock repository
        let client = DBClient::new(Arc::new(mock_repo));

        // Test that we can retrieve the BSP
        let retrieved_bsp = client.get_bsp_by_id(created_bsp.id).await.unwrap();
        assert!(retrieved_bsp.is_some());

        let bsp = retrieved_bsp.unwrap();
        assert_eq!(bsp.account, TEST_BSP_ACCOUNT_STR);
        assert_eq!(bsp.onchain_bsp_id, TEST_BSP_ONCHAIN_ID_STR);
    }

    #[cfg(feature = "mocks")]
    #[tokio::test]
    async fn test_db_client_file_operations() {
        use std::sync::Arc;

        use crate::repository::{IndexerOpsMut, MockRepository, NewFile};

        let mock_repo = MockRepository::new();

        // Add a test file using the mock repository directly
        let new_file = NewFile {
            account: TEST_USER_ACCOUNT.to_vec(),
            file_key: ALTERNATIVE_FILE_KEY.to_vec(),
            bucket_id: buckets::TEST_BUCKET_ID_INT,
            location: ALTERNATIVE_LOCATION.to_vec(),
            fingerprint: ALTERNATIVE_FINGERPRINT.to_vec(),
            size: TEST_FILE_SIZE as i64,
            step: UPDATED_STEP as i32,
        };

        // Create the file directly in the mock repository
        let created_file = mock_repo.create_file(new_file).await.unwrap();

        // Now create the DBClient with the mock that contains data
        let client = DBClient::new(Arc::new(mock_repo));

        // Test file retrieval - should now find the file
        let result = client.get_file_by_key(ALTERNATIVE_FILE_KEY).await.unwrap();
        assert_eq!(result.file_key, ALTERNATIVE_FILE_KEY);
        assert_eq!(result.account, TEST_USER_ACCOUNT);
        assert_eq!(result.size, TEST_FILE_SIZE as i64);

        // Test getting files by user
        let files = client
            .get_files_by_user(TEST_USER_ACCOUNT, None, None)
            .await
            .unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].id, created_file.id);
    }
}
