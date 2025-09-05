//! MSP service implementation with mock data
//!
//! TODO(MOCK): many of methods of the MspService returns mocked data

use std::sync::Arc;

use chrono::Utc;

use shc_indexer_db::{models::Bucket as DBBucket, OnchainMspId};
use shp_types::Hash;

use crate::{
    config::Config,
    constants::mocks::{PLACEHOLDER_BUCKET_FILE_COUNT, PLACEHOLDER_BUCKET_SIZE_BYTES},
    data::{indexer_db::client::DBClient, rpc::StorageHubRpcClient, storage::BoxedStorage},
    error::Error,
    models::{
        buckets::{Bucket, FileTree},
        files::{DistributeResponse, FileInfo},
        msp_info::{Capacity, InfoResponse, MspHealthResponse, StatsResponse, ValueProp},
        payment::PaymentStream,
    },
};

/// Service for handling MSP-related operations
//TODO: remove dead_code annotations when we actually use these items
// storage: anything that the backend will need to store temporarily
// rpc: anything that the backend needs to request to the underlying MSP node
#[derive(Clone)]
pub struct MspService {
    msp_id: OnchainMspId,

    #[allow(dead_code)]
    storage: Arc<dyn BoxedStorage>,
    postgres: Arc<DBClient>,
    #[allow(dead_code)]
    rpc: Arc<StorageHubRpcClient>,
}

impl MspService {
    /// Create a new MSP service
    pub fn new(
        config: &Config,
        storage: Arc<dyn BoxedStorage>,
        postgres: Arc<DBClient>,
        rpc: Arc<StorageHubRpcClient>,
    ) -> Self {
        let msp_id = hex::decode(config.storage_hub.msp_id.trim_start_matches("0x"))
            .map(|decoded| Hash::from_slice(&decoded))
            .map(OnchainMspId::new)
            .expect("valid MSP ID");

        Self {
            msp_id,
            storage,
            postgres,
            rpc,
        }
    }

    /// Get MSP information
    pub async fn get_info(&self) -> Result<InfoResponse, Error> {
        Ok(InfoResponse {
            client: "storagehub-node v1.0.0".to_string(),
            version: "StorageHub MSP v0.1.0".to_string(),
            msp_id: self.msp_id.to_string(),
            multiaddresses: vec![
                "/ip4/192.168.0.10/tcp/30333/p2p/12D3KooWJAgnKUrQkGsKxRxojxcFRhtH6ovWfJTPJjAkhmAz2yC8".to_string()
            ],
            owner_account: "0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac".to_string(),
            payment_account: "0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac".to_string(),
            status: "active".to_string(),
            active_since: 123,
            uptime: "2 days, 1 hour".to_string(),
        })
    }

    /// Get MSP statistics
    pub async fn get_stats(&self) -> Result<StatsResponse, Error> {
        Ok(StatsResponse {
            capacity: Capacity {
                total_bytes: 1099511627776,
                available_bytes: 879609302220,
                used_bytes: 219902325556,
            },
            active_users: 152,
            last_capacity_change: 123,
            value_props_amount: 42,
            buckets_amount: 1024,
        })
    }

    /// Get MSP value propositions
    pub async fn get_value_props(&self) -> Result<Vec<ValueProp>, Error> {
        Ok(vec![
            ValueProp {
                id: "f32282ba18056b02cf2feb4cea92aa4552131617cdb7da03acaa554e4e736c32".to_string(),
                price_per_gb_block: 0.5,
                data_limit_per_bucket_bytes: 10737418240,
                is_available: true,
            },
            ValueProp {
                id: "a12345ba18056b02cf2feb4cea92aa4552131617cdb7da03acaa554e4e736c45".to_string(),
                price_per_gb_block: 0.3,
                data_limit_per_bucket_bytes: 5368709120,
                is_available: true,
            },
            ValueProp {
                id: "b67890ba18056b02cf2feb4cea92aa4552131617cdb7da03acaa554e4e736c67".to_string(),
                price_per_gb_block: 0.8,
                data_limit_per_bucket_bytes: 21474836480,
                is_available: false,
            },
        ])
    }

    /// Get MSP health status
    pub async fn get_health(&self) -> Result<MspHealthResponse, Error> {
        Ok(MspHealthResponse {
            status: "healthy".to_string(),
            components: serde_json::json!({
                "database": {
                    "status": "healthy",
                    "details": "PostgreSQL connection active"
                },
                "mspClient": {
                    "status": "healthy",
                    "details": "Connected to StorageHub MSP client"
                },
                "storageHubNetwork": {
                    "status": "healthy",
                    "details": "Node synced with network"
                },
                "diskSpace": {
                    "status": "healthy",
                    "details": "80% capacity available"
                }
            }),
            last_checked: Utc::now(),
        })
    }

    /// List buckets for a user
    pub async fn list_user_buckets(
        &self,
        user_address: &str,
    ) -> Result<impl Iterator<Item = Bucket>, Error> {
        // TODO: request by page
        self.postgres
            .get_user_buckets(&self.msp_id, user_address, None, None)
            .await
            .map(|buckets| {
                buckets.into_iter().map(|entry| {
                    Bucket::from_db(
                        &entry,
                        PLACEHOLDER_BUCKET_SIZE_BYTES,
                        PLACEHOLDER_BUCKET_FILE_COUNT,
                    )
                })
            })
    }

    /// Verifies user can access the given bucket
    fn can_user_view_bucket(&self, bucket: DBBucket, user: &str) -> Result<DBBucket, Error> {
        // TODO: NFT ownership
        if bucket.private {
            if bucket.account.as_str() == user {
                Ok(bucket)
            } else {
                Err(Error::Unauthorized(format!(
                    "Specified user is not authorized to view this bucket"
                )))
            }
        } else {
            Ok(bucket)
        }
    }

    /// Retrieve a bucket from the DB and verify read permission
    async fn get_db_bucket(
        &self,
        bucket_id: &str,
        user: &str,
    ) -> Result<shc_indexer_db::models::Bucket, Error> {
        let bucket_id = hex::decode(bucket_id.trim_start_matches("0x")).map_err(|_| {
            Error::BadRequest(format!("Invalid Bucket ID. Expected a valid hex string"))
        })?;

        self.postgres
            .get_bucket(&bucket_id)
            .await
            .and_then(|bucket| self.can_user_view_bucket(bucket, user))
    }

    /// Get a specific bucket by ID
    ///
    /// Verifies ownership of bucket is `user`
    pub async fn get_bucket(&self, bucket_id: &str, user: &str) -> Result<Bucket, Error> {
        self.get_db_bucket(bucket_id, user).await.map(|bucket| {
            Bucket::from_db(
                &bucket,
                PLACEHOLDER_BUCKET_SIZE_BYTES,
                PLACEHOLDER_BUCKET_FILE_COUNT,
            )
        })
    }

    /// Get file tree for a bucket
    ///
    /// Verifies ownership of bucket is `user`
    /// Returns only direct children of the given path
    ///
    /// ## Business Rules for File Location Handling
    ///
    /// The given path is normalized using the following rules:
    /// * root is implicit
    /// * duplicated slashes are collapsed
    /// * trailing slashes are trimmed
    pub async fn get_file_tree(
        &self,
        bucket_id: &str,
        user: &str,
        path: &str,
    ) -> Result<FileTree, Error> {
        // first, get the bucket from the db and determine if user can view the bucket
        let bucket = self.get_db_bucket(bucket_id, user).await?;

        // TODO: request by page
        // TODO: optimize query by requesting only matching paths
        let files = self
            .postgres
            .get_bucket_files(bucket.id, None, None)
            .await?;

        // Create hierarchy based on location segments
        Ok(FileTree::from_files_filtered(files, path))
    }

    /// Get file information
    pub async fn get_file_info(
        &self,
        bucket_id: &str,
        user: &str,
        file_key: &str,
    ) -> Result<FileInfo, Error> {
        let file_key = hex::decode(file_key.trim_start_matches("0x")).map_err(|_| {
            Error::BadRequest(format!("Invalid File Key. Expected a valid hex string"))
        })?;

        // get bucket determine if user can view it
        let bucket = self.get_bucket(bucket_id, user).await?;

        self.postgres
            .get_file_info(&file_key)
            .await
            .map(|file| FileInfo::from_db(&file, bucket.is_public))
    }

    /// Distribute a file to BSPs
    pub async fn distribute_file(
        &self,
        _bucket_id: &str,
        file_key: &str,
    ) -> Result<DistributeResponse, Error> {
        // Mock implementation
        Ok(DistributeResponse {
            status: "distribution_initiated".to_string(),
            file_key: file_key.to_string(),
            message: "File distribution to volunteering BSPs has been initiated".to_string(),
        })
    }

    /// Get payment stream for a user
    pub async fn get_payment_stream(&self, _user_address: &str) -> Result<PaymentStream, Error> {
        // Mock implementation
        Ok(PaymentStream {
            tokens_per_block: 100,
            last_charged_tick: 1234567,
            user_deposit: 100000,
            out_of_funds_tick: None,
        })
    }
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use super::*;
    use crate::{
        constants::mocks::MOCK_ADDRESS,
        data::{
            indexer_db::{
                client::DBClient,
                mock_repository::{
                    tests::{inject_bucket_with_account, inject_sample_msp},
                    MockRepository,
                },
            },
            rpc::{AnyRpcConnection, MockConnection, StorageHubRpcClient},
            storage::{BoxedStorageWrapper, InMemoryStorage},
        },
    };
    use std::sync::Arc;

    /// Builder for creating MspService instances with mock dependencies for testing
    struct MockMspServiceBuilder {
        storage: Arc<BoxedStorageWrapper<InMemoryStorage>>,
        repo: Arc<MockRepository>,
        rpc: Arc<StorageHubRpcClient>,
        config: Config,
    }

    impl MockMspServiceBuilder {
        /// Create a new builder with default empty mocks
        fn new() -> Self {
            let memory_storage = InMemoryStorage::new();
            let storage = Arc::new(BoxedStorageWrapper::new(memory_storage));

            let repo = Arc::new(MockRepository::new());

            let mock_conn = MockConnection::new();
            let rpc_conn = Arc::new(AnyRpcConnection::Mock(mock_conn));
            let rpc = Arc::new(StorageHubRpcClient::new(rpc_conn));

            Self {
                storage,
                repo,
                rpc,
                config: Config::default(),
            }
        }

        /// Initialize repository with custom test data
        async fn init_repository_with<F>(self, init: F) -> Self
        where
            F: FnOnce(
                &MockRepository,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + '_>>,
        {
            init(&self.repo).await;
            self
        }

        /// Build the final MspService
        fn build(self) -> MspService {
            let postgres = Arc::new(DBClient::new(self.repo));
            MspService::new(&self.config, self.storage, postgres, self.rpc)
        }
    }

    #[tokio::test]
    async fn test_get_info() {
        let service = MockMspServiceBuilder::new().build();
        let info = service.get_info().await.unwrap();

        assert_eq!(info.status, "active");
        assert!(!info.multiaddresses.is_empty());
    }

    #[tokio::test]
    async fn test_get_stats() {
        let service = MockMspServiceBuilder::new().build();
        let stats = service.get_stats().await.unwrap();

        assert!(stats.capacity.total_bytes > 0);
        assert!(stats.capacity.available_bytes <= stats.capacity.total_bytes);
    }

    #[tokio::test]
    async fn test_get_value_props() {
        let service = MockMspServiceBuilder::new().build();
        let props = service.get_value_props().await.unwrap();

        assert!(!props.is_empty());
        assert!(props.iter().any(|p| p.is_available));
    }

    #[tokio::test]
    async fn test_list_user_buckets() {
        let service = MockMspServiceBuilder::new()
            .init_repository_with(|repo| {
                Box::pin(async move {
                    // Inject MSP with the ID that matches the default config
                    let msp_id = inject_sample_msp(repo).await;
                    // Inject a test bucket for the mock user
                    inject_bucket_with_account(
                        repo,
                        Some(msp_id),
                        MOCK_ADDRESS,
                        Some("test-bucket"),
                    )
                    .await;
                })
            })
            .await
            .build();

        let buckets = service
            .list_user_buckets(MOCK_ADDRESS)
            .await
            .unwrap()
            .collect::<Vec<_>>();

        assert!(!buckets.is_empty());
    }

    #[tokio::test]
    async fn test_get_files_root() {
        use crate::constants::test::bucket::DEFAULT_BUCKET_ID;

        let service = MockMspServiceBuilder::new()
            .init_repository_with(|repo| {
                Box::pin(async move {
                    // Inject MSP with the ID that matches the default config
                    let msp_id = inject_sample_msp(repo).await;
                    // Inject a test bucket for the mock user
                    inject_bucket_with_account(
                        repo,
                        Some(msp_id),
                        MOCK_ADDRESS,
                        Some("test-bucket"),
                    )
                    .await;
                })
            })
            .await
            .build();

        let tree = service
            .get_file_tree(hex::encode(DEFAULT_BUCKET_ID).as_ref(), MOCK_ADDRESS, "/")
            .await
            .unwrap();

        tree.entry.folder().expect("first entry to be a folder");
    }
}
