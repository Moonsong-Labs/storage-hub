//! MSP service implementation with mock data
//!
//! TODO(MOCK): many of methods of the MspService returns mocked data

use std::sync::Arc;

use chrono::Utc;

use shc_indexer_db::models::Bucket as DBBucket;

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
#[derive(Clone)]
pub struct MspService {
    msp_id: String,

    storage: Arc<dyn BoxedStorage>,
    postgres: Arc<DBClient>,
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
        Self {
            msp_id: config.storage_hub.msp_id.clone(),
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
            msp_id: self.msp_id.clone(),
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

    /// Get a specific bucket by ID
    ///
    /// Verifies ownership of bucket is `user`
    pub async fn get_bucket(&self, bucket_id: &str, user: &str) -> Result<Bucket, Error> {
        let bucket_id = hex::decode(bucket_id.trim_start_matches("0x")).map_err(|_| {
            Error::BadRequest(format!("Invalid Bucket ID. Expected a valid hex string"))
        })?;

        self.postgres
            .get_bucket(&bucket_id)
            .await
            .and_then(|bucket| self.can_user_view_bucket(bucket, user))
            .map(|bucket| {
                Bucket::from_db(
                    &bucket,
                    PLACEHOLDER_BUCKET_SIZE_BYTES,
                    PLACEHOLDER_BUCKET_FILE_COUNT,
                )
            })
    }

    /// Get file tree for a bucket
    pub async fn get_file_tree(&self, _bucket_id: &str) -> Result<FileTree, Error> {
        Ok(FileTree {
            name: "/".to_string(),
            node_type: "folder".to_string(),
            children: Some(vec![
                FileTree {
                    name: "Thesis".to_string(),
                    node_type: "folder".to_string(),
                    children: Some(vec![
                        FileTree {
                            name: "Initial_results.png".to_string(),
                            node_type: "file".to_string(),
                            children: None,
                            size_bytes: Some(54321),
                            file_key: Some(
                                "d298c8d212325fe2f18964fd2ea6e7375e2f90835b638ddb3c08692edd7840f2"
                                    .to_string(),
                            ),
                        },
                        FileTree {
                            name: "chapter1.pdf".to_string(),
                            node_type: "file".to_string(),
                            children: None,
                            size_bytes: Some(234567),
                            file_key: Some(
                                "a123c8d212325fe2f18964fd2ea6e7375e2f90835b638ddb3c08692edd7840f3"
                                    .to_string(),
                            ),
                        },
                        FileTree {
                            name: "references.docx".to_string(),
                            node_type: "file".to_string(),
                            children: None,
                            size_bytes: Some(45678),
                            file_key: Some(
                                "b456c8d212325fe2f18964fd2ea6e7375e2f90835b638ddb3c08692edd7840f4"
                                    .to_string(),
                            ),
                        },
                    ]),
                    size_bytes: None,
                    file_key: None,
                },
                FileTree {
                    name: "Reports".to_string(),
                    node_type: "folder".to_string(),
                    children: Some(vec![
                        FileTree {
                            name: "Q1-2024.pdf".to_string(),
                            node_type: "file".to_string(),
                            children: None,
                            size_bytes: Some(123456),
                            file_key: Some(
                                "c789c8d212325fe2f18964fd2ea6e7375e2f90835b638ddb3c08692edd7840f5"
                                    .to_string(),
                            ),
                        },
                        FileTree {
                            name: "Q2-2024.pdf".to_string(),
                            node_type: "file".to_string(),
                            children: None,
                            size_bytes: Some(134567),
                            file_key: Some(
                                "d890c8d212325fe2f18964fd2ea6e7375e2f90835b638ddb3c08692edd7840f6"
                                    .to_string(),
                            ),
                        },
                    ]),
                    size_bytes: None,
                    file_key: None,
                },
                FileTree {
                    name: "README.md".to_string(),
                    node_type: "file".to_string(),
                    children: None,
                    size_bytes: Some(2048),
                    file_key: Some(
                        "e901c8d212325fe2f18964fd2ea6e7375e2f90835b638ddb3c08692edd7840f7"
                            .to_string(),
                    ),
                },
            ]),
            size_bytes: None,
            file_key: None,
        })
    }

    /// Get file information
    pub async fn get_file_info(&self, bucket_id: &str, file_key: &str) -> Result<FileInfo, Error> {
        // Mock implementation
        Ok(FileInfo {
            file_key: file_key.to_string(),
            fingerprint: "5d7a3700e1f7d973c064539f1b18c988dace6b4f1a57650165e9b58305db090f"
                .to_string(),
            bucket_id: bucket_id.to_string(),
            name: "Q1-2024.pdf".to_string(),
            location: "/files/documents/reports".to_string(),
            size: 54321,
            is_public: true,
            uploaded_at: Utc::now() - chrono::Duration::days(30),
        })
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
    use crate::services::Services;

    async fn create_test_service() -> MspService {
        let services = Services::mocks();

        MspService::new(
            services.storage.clone(),
            services.postgres.clone(),
            services.rpc.clone(),
        )
    }

    #[tokio::test]
    async fn test_get_info() {
        let service = create_test_service().await;
        let info = service.get_info().await.unwrap();

        assert_eq!(info.status, "active");
        assert!(!info.multiaddresses.is_empty());
    }

    #[tokio::test]
    async fn test_get_stats() {
        let service = create_test_service().await;
        let stats = service.get_stats().await.unwrap();

        assert!(stats.capacity.total_bytes > 0);
        assert!(stats.capacity.available_bytes <= stats.capacity.total_bytes);
    }

    #[tokio::test]
    async fn test_get_value_props() {
        let service = create_test_service().await;
        let props = service.get_value_props().await.unwrap();

        assert!(!props.is_empty());
        assert!(props.iter().any(|p| p.is_available));
    }

    #[tokio::test]
    async fn test_list_user_buckets() {
        let service = create_test_service().await;
        let buckets = service.list_user_buckets("0x123").await.unwrap();

        assert!(!buckets.is_empty());
        assert!(buckets.iter().all(|b| !b.bucket_id.is_empty()));
    }

    #[tokio::test]
    async fn test_get_file_tree() {
        let service = create_test_service().await;
        let tree = service.get_file_tree("bucket123").await.unwrap();

        assert_eq!(tree.node_type, "folder");
        assert!(tree.children.is_some());
    }
}
