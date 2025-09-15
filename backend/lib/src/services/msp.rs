//! MSP service implementation with mock data
//!
//! TODO(MOCK): the entire set of methods of the MspService returns mocked data

use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use shc_rpc::SaveFileToDisk;

use crate::{
    data::{indexer_db::client::DBClient, rpc::StorageHubRpcClient, storage::BoxedStorage},
    error::Error,
    models::{
        buckets::{Bucket, FileEntry},
        files::{DistributeResponse, FileInfo},
        msp_info::{Capacity, InfoResponse, MspHealthResponse, StatsResponse, ValueProp},
        payment::PaymentStream,
    },
};

/// Placeholder  
#[derive(Debug, Deserialize, Serialize)]
pub struct FileDownloadResult {
    pub file_size: u64,
    pub location: String,
    pub fingerprint: [u8; 32],
    pub temp_path: String,
}

/// Service for handling MSP-related operations
#[derive(Clone)]
pub struct MspService {
    #[allow(dead_code)]
    storage: Arc<dyn BoxedStorage>,
    #[allow(dead_code)]
    postgres: Arc<DBClient>,
    #[allow(dead_code)]
    rpc: Arc<StorageHubRpcClient>,
    msp_callback_url: String,
}

impl MspService {
    /// Create a new MSP service
    pub fn new(
        storage: Arc<dyn BoxedStorage>,
        postgres: Arc<DBClient>,
        rpc: Arc<StorageHubRpcClient>,
        msp_callback_url: String,
    ) -> Self {
        Self {
            storage,
            postgres,
            rpc,
            msp_callback_url,
        }
    }

    /// Get MSP information
    pub async fn get_info(&self) -> Result<InfoResponse, Error> {
        Ok(InfoResponse {
            client: "storagehub-node v1.0.0".to_string(),
            version: "StorageHub MSP v0.1.0".to_string(),
            msp_id: "4c310f61f81475048e8ce5eadf4ee718c42ba285579bb37ac6da55a92c638f42".to_string(),
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
    pub async fn list_user_buckets(&self, _user_address: &str) -> Result<Vec<Bucket>, Error> {
        // Mock implementation returns sample buckets
        Ok(vec![
            Bucket {
                bucket_id: "d8793e4187f5642e96016a96fb33849a7e03eda91358b311bbd426ed38b26692"
                    .to_string(),
                name: "Documents".to_string(),
                root: "3de0c6d1959ece558ec030f37292e383a9c95f497e8235b89701b914be9bd1fb"
                    .to_string(),
                is_public: false,
                size_bytes: 12345678,
                value_prop_id: "f32282ba18056b02cf2feb4cea92aa4552131617cdb7da03acaa554e4e736c32"
                    .to_string(),
                file_count: 12,
            },
            Bucket {
                bucket_id: "a1234e4187f5642e96016a96fb33849a7e03eda91358b311bbd426ed38b26693"
                    .to_string(),
                name: "Photos".to_string(),
                root: "4ef1d7e2070fd659bd1d060b3096f38b5a1d65e608347ca90802c0a1b9bde2fc"
                    .to_string(),
                is_public: true,
                size_bytes: 987654321,
                value_prop_id: "a12345ba18056b02cf2feb4cea92aa4552131617cdb7da03acaa554e4e736c45"
                    .to_string(),
                file_count: 156,
            },
            Bucket {
                bucket_id: "b5678e4187f5642e96016a96fb33849a7e03eda91358b311bbd426ed38b26694"
                    .to_string(),
                name: "Projects".to_string(),
                root: "5af2e8f3181ge770ce2e161c4107g49c6b2e76f719458db01913d1c1c9ce3gd".to_string(),
                is_public: false,
                size_bytes: 45678901,
                value_prop_id: "f32282ba18056b02cf2feb4cea92aa4552131617cdb7da03acaa554e4e736c32"
                    .to_string(),
                file_count: 34,
            },
        ])
    }

    /// Get a specific bucket by ID
    pub async fn get_bucket(&self, bucket_id: &str) -> Result<Bucket, Error> {
        // Mock implementation - in real implementation would query database
        Ok(Bucket {
            bucket_id: bucket_id.to_string(),
            name: "Documents".to_string(),
            root: "3de0c6d1959ece558ec030f37292e383a9c95f497e8235b89701b914be9bd1fb".to_string(),
            is_public: false,
            size_bytes: 12345678,
            value_prop_id: "f32282ba18056b02cf2feb4cea92aa4552131617cdb7da03acaa554e4e736c32"
                .to_string(),
            file_count: 12,
        })
    }

    /// Get files under a path (immediate children only). Path is absolute from bucket root.
    pub async fn get_files(
        &self,
        _bucket_id: &str,
        path: Option<&str>,
    ) -> Result<Vec<FileEntry>, Error> {
        // Normalize path
        let p = path.unwrap_or("");
        let normalized = p.trim_matches('/');

        match normalized {
            "" => Ok(vec![
                FileEntry {
                    name: "Thesis".to_string(),
                    entry_type: "folder".to_string(),
                    size_bytes: None,
                    file_key: None,
                },
                FileEntry {
                    name: "Reports".to_string(),
                    entry_type: "folder".to_string(),
                    size_bytes: None,
                    file_key: None,
                },
                FileEntry {
                    name: "README.md".to_string(),
                    entry_type: "file".to_string(),
                    size_bytes: Some(2048),
                    file_key: Some(
                        "e901c8d212325fe2f18964fd2ea6e7375e2f90835b638ddb3c08692edd7840f7"
                            .to_string(),
                    ),
                },
            ]),
            s if s.eq_ignore_ascii_case("thesis") => Ok(vec![
                FileEntry {
                    name: "Initial_results.png".to_string(),
                    entry_type: "file".to_string(),
                    size_bytes: Some(54321),
                    file_key: Some(
                        "d298c8d212325fe2f18964fd2ea6e7375e2f90835b638ddb3c08692edd7840f2"
                            .to_string(),
                    ),
                },
                FileEntry {
                    name: "chapter1.pdf".to_string(),
                    entry_type: "file".to_string(),
                    size_bytes: Some(234567),
                    file_key: Some(
                        "a123c8d212325fe2f18964fd2ea6e7375e2f90835b638ddb3c08692edd7840f3"
                            .to_string(),
                    ),
                },
                FileEntry {
                    name: "references.docx".to_string(),
                    entry_type: "file".to_string(),
                    size_bytes: Some(45678),
                    file_key: Some(
                        "b456c8d212325fe2f18964fd2ea6e7375e2f90835b638ddb3c08692edd7840f4"
                            .to_string(),
                    ),
                },
            ]),
            s if s.eq_ignore_ascii_case("reports") => Ok(vec![
                FileEntry {
                    name: "Q1-2024.pdf".to_string(),
                    entry_type: "file".to_string(),
                    size_bytes: Some(123456),
                    file_key: Some(
                        "c789c8d212325fe2f18964fd2ea6e7375e2f90835b638ddb3c08692edd7840f5"
                            .to_string(),
                    ),
                },
                FileEntry {
                    name: "Q2-2024.pdf".to_string(),
                    entry_type: "file".to_string(),
                    size_bytes: Some(134567),
                    file_key: Some(
                        "d890c8d212325fe2f18964fd2ea6e7375e2f90835b638ddb3c08692edd7840f6"
                            .to_string(),
                    ),
                },
            ]),
            _ => Err(Error::NotFound("Folder does not exist".to_string())),
        }
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

    /// Download a file by `file_key` via the MSP RPC into `uploads/<file_key>` and
    /// return its size, UTF-8 location, fingerprint, and temp path.
    /// Returns BadRequest on RPC/parse errors.
    ///
    /// We provide an URL as saveFileToDisk RPC requires it to stream the file.
    /// We also implemented the internal_upload_by_key handler to handle this temporary file upload.
    pub async fn get_file_from_key(&self, file_key: &str) -> Result<FileDownloadResult, Error> {
        // Create temp url for download
        let temp_path = format!("uploads/{}", file_key);
        let upload_url = format!("{}/{}", self.msp_callback_url, temp_path);

        // Make the RPC call to download file and get metadata
        let rpc_response: SaveFileToDisk = self
            .rpc
            .call(
                "storagehubclient_saveFileToDisk",
                (file_key, upload_url.as_str()),
            )
            .await
            .map_err(|e| Error::BadRequest(e.to_string()))?;

        match rpc_response {
            SaveFileToDisk::FileNotFound => Err(Error::NotFound("File not found".to_string())),
            SaveFileToDisk::IncompleteFile(_status) => {
                Err(Error::BadRequest("File is incomplete".to_string()))
            }
            SaveFileToDisk::Success(file_metadata) => {
                // Convert location bytes to string
                let location = String::from_utf8_lossy(file_metadata.location()).to_string();
                let fingerprint: [u8; 32] = file_metadata.fingerprint().as_hash();
                let file_size = file_metadata.file_size();

                Ok(FileDownloadResult {
                    file_size,
                    location,
                    fingerprint,
                    temp_path,
                })
            }
        }
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
            "http://localhost:8080".to_string(),
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
    async fn test_get_files_root() {
        let service = create_test_service().await;
        let files = service.get_files("bucket123", None).await.unwrap();
        assert!(files
            .iter()
            .any(|f| f.name == "Thesis" && f.entry_type == "folder"));
        assert!(files
            .iter()
            .any(|f| f.name == "Reports" && f.entry_type == "folder"));
        assert!(files
            .iter()
            .any(|f| f.name == "README.md" && f.entry_type == "file"));
    }

    #[tokio::test]
    async fn test_get_files_thesis() {
        let service = create_test_service().await;
        let files = service
            .get_files("bucket123", Some("thesis"))
            .await
            .unwrap();
        assert!(files
            .iter()
            .any(|f| f.name == "Initial_results.png" && f.entry_type == "file"));
        assert!(files
            .iter()
            .any(|f| f.name == "chapter1.pdf" && f.entry_type == "file"));
        assert!(files
            .iter()
            .any(|f| f.name == "references.docx" && f.entry_type == "file"));
    }
}
