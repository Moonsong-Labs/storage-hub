//! MSP service implementation with mock data
//!
//! TODO(MOCK): the entire set of methods of the MspService returns mocked data

use std::{collections::HashSet, sync::Arc};

use chrono::Utc;
use codec::Encode;
use sc_network::PeerId;
use shc_common::types::{ChunkId, FileKeyProof};
use sp_core::{Blake2Hasher, H256};
use tracing::{debug, info, warn};

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

/// Service for handling MSP-related operations
#[derive(Clone)]
pub struct MspService {
    #[allow(dead_code)]
    storage: Arc<dyn BoxedStorage>,
    #[allow(dead_code)]
    postgres: Arc<DBClient>,
    #[allow(dead_code)]
    rpc: Arc<StorageHubRpcClient>,
}

impl MspService {
    /// Create a new MSP service
    pub fn new(
        storage: Arc<dyn BoxedStorage>,
        postgres: Arc<DBClient>,
        rpc: Arc<StorageHubRpcClient>,
    ) -> Self {
        Self {
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
            msp_id: "4c310f61f81475048e8ce5eadf4ee718c42ba285579bb37ac6da55a92c638f42".to_string(),
						// TODO: Until we have actual MSP info, we should at least get the multiaddress from an RPC.
						// This way the backend can actually upload files to the MSP without having to change this code.
            multiaddresses: vec![
                "/ip4/192.168.0.10/tcp/30333/p2p/12D3KooWSUvz8QM5X4tfAaSLErAZjR2puojo16pULBHyqTMGKtNV".to_string()
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

    /// Upload a batch of file chunks with their FileKeyProof to the MSP via its RPC.
    ///
    /// This implementation:
    /// 1. Gets the MSP info to get its multiaddresses.
    /// 2. Extracts the peer IDs from the multiaddresses.
    /// 3. Sends the FileKeyProof with the batch of chunks to the MSP through the `uploadToPeer` RPC method.
    ///
    /// Note: obtaining the peer ID previous to sending the request is needed as this is the peer ID that the MSP
    /// will send the file to. If it's different than its local one, it will probably fail.
    pub async fn upload_to_msp(
        &self,
        chunk_ids: &HashSet<ChunkId>,
        file_key_proof: &FileKeyProof,
    ) -> Result<(), Error> {
        // Ensure we are not incorrectly trying to upload an empty file.
        if chunk_ids.is_empty() {
            return Err(Error::BadRequest(
                "Cannot upload file with no chunks".to_string(),
            ));
        }

        // Get the MSP's info including its multiaddresses.
        let msp_info = self.get_info().await?;

        // Extract the peer IDs from the multiaddresses.
        let peer_ids = self.extract_peer_ids_from_multiaddresses(&msp_info.multiaddresses)?;

        // Try to send the chunks batch to each peer until one succeeds.
        let mut last_err = None;
        for peer_id in peer_ids {
            match self
                .send_upload_request_to_msp_peer(peer_id, file_key_proof.clone())
                .await
            {
                Ok(()) => {
                    info!(
                        "Successfully uploaded {} chunks to MSP {} for file {} in bucket {}",
                        chunk_ids.len(),
                        msp_info.msp_id,
                        hex::encode(file_key_proof.file_metadata.file_key::<Blake2Hasher>()),
                        hex::encode(file_key_proof.file_metadata.bucket_id())
                    );
                    return Ok(());
                }
                Err(e) => {
                    warn!("Failed to send chunks to peer {:?}: {:?}", peer_id, e);
                    last_err = Some(e);
                    continue;
                }
            }
        }

        Err(last_err.expect("At least one peer_id was tried, so last_err must be Some"))
    }

    /// Extract peer IDs from multiaddresses
    fn extract_peer_ids_from_multiaddresses(
        &self,
        multiaddresses: &[String],
    ) -> Result<Vec<PeerId>, Error> {
        let mut peer_ids = Vec::new();

        for multiaddr_str in multiaddresses {
            // Parse multiaddress string to extract peer ID
            // Format example: "/ip4/192.168.0.10/tcp/30333/p2p/12D3KooWJAgnKUrQkGsKxRxojxcFRhtH6ovWfJTPJjAkhmAz2yC8"
            if let Some(p2p_part) = multiaddr_str.split("/p2p/").nth(1) {
                // Extract the peer ID part (everything after /p2p/)
                let peer_id_str = p2p_part.split('/').next().unwrap_or(p2p_part);

                match peer_id_str.parse::<PeerId>() {
                    Ok(peer_id) => {
                        debug!(
                            "Extracted peer ID {:?} from multiaddress {}",
                            peer_id, multiaddr_str
                        );
                        peer_ids.push(peer_id);
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse peer ID from multiaddress {}: {:?}",
                            multiaddr_str, e
                        );
                    }
                }
            } else {
                warn!("No /p2p/ section found in multiaddress: {}", multiaddr_str);
            }
        }

        if peer_ids.is_empty() {
            return Err(Error::BadRequest(
                "No valid peer IDs found in multiaddresses".to_string(),
            ));
        }

        Ok(peer_ids)
    }

    /// Send an upload request to a specific peer ID of the MSP with retry logic.
    /// TODO: Make the number of retries configurable.
    async fn send_upload_request_to_msp_peer(
        &self,
        peer_id: PeerId,
        file_key_proof: FileKeyProof,
    ) -> Result<(), Error> {
        debug!(
            "Attempting to send upload request to MSP peer {:?} with file key proof",
            peer_id
        );

        // Encode the peer ID to a base-58 string to make it serializable, the RPC method then decodes it back to a PeerId.
        let peer_id_str = peer_id.to_base58();

        // Get fhe file metadata from the received FileKeyProof.
        let file_metadata = file_key_proof.clone().file_metadata;

        // Get the bucket ID and file key from the file metadata.
        let bucket_id = file_metadata.bucket_id();
        let bucket_id_hash = H256::from_slice(bucket_id.as_slice());
        let file_key: H256 = file_metadata.file_key::<Blake2Hasher>();

        // Encode the FileKeyProof as SCALE for transport
        let encoded_proof = file_key_proof.encode();

        let mut retry_attempts = 0;
        let max_retries = 3;

        while retry_attempts < max_retries {
            let result: Result<Vec<u8>, _> = self
                .rpc
                .call(
                    "storagehubclient_uploadToPeer",
                    (
                        peer_id_str.clone(),
                        file_key,
                        encoded_proof.clone(),
                        Some(bucket_id_hash),
                    ),
                )
                .await;

            match result {
                Ok(_raw) => {
                    info!("Successfully sent upload request to MSP peer {:?}", peer_id);
                    return Ok(());
                }
                Err(e) => {
                    retry_attempts += 1;
                    if retry_attempts < max_retries {
                        warn!(
                            "Upload request to MSP peer {:?} failed via RPC, retrying... (attempt {}): {:?}",
                            peer_id,
                            retry_attempts,
                            e
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    } else {
                        return Err(Error::Internal);
                    }
                }
            }
        }

        Err(Error::Internal)
    }
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use super::*;

    use crate::{
        data::rpc::{AnyRpcConnection, MockConnection, StorageHubRpcClient},
        services::Services,
    };

    use serde_json::Value;
    use shc_common::types::{FileKeyProof, FileMetadata};

    async fn create_test_service() -> MspService {
        let services = Services::mocks();

        MspService::new(
            services.storage.clone(),
            services.postgres.clone(),
            services.rpc.clone(),
        )
    }

    async fn create_test_service_with_rpc_responses(responses: Vec<(&str, Value)>) -> MspService {
        let services = Services::mocks();
        let mock_conn = MockConnection::new();
        for (method, value) in responses {
            mock_conn.set_response(method, value).await;
        }
        let rpc = Arc::new(StorageHubRpcClient::new(Arc::new(AnyRpcConnection::Mock(
            mock_conn,
        ))));
        MspService::new(services.storage.clone(), services.postgres.clone(), rpc)
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
    async fn test_get_bucket() {
        let service = create_test_service().await;
        let bucket = service.get_bucket("test_bucket").await.unwrap();

        assert_eq!(bucket.bucket_id, "test_bucket");
        assert!(!bucket.name.is_empty());
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

    #[tokio::test]
    async fn test_get_file_info() {
        let service = create_test_service().await;
        let bucket_id = "bucket123";
        let file_key = "abc123";
        let info = service
            .get_file_info(bucket_id, file_key)
            .await
            .expect("get_file_info should succeed");

        assert_eq!(info.bucket_id, bucket_id);
        assert_eq!(info.file_key, file_key);
        assert!(!info.name.is_empty());
        assert!(info.size > 0);
    }

    #[tokio::test]
    async fn test_distribute_file() {
        let service = create_test_service().await;
        let file_key = "abc123";
        let resp = service
            .distribute_file("bucket123", file_key)
            .await
            .expect("distribute_file should succeed");

        assert_eq!(resp.status, "distribution_initiated");
        assert_eq!(resp.file_key, file_key);
        assert!(!resp.message.is_empty());
    }

    #[tokio::test]
    async fn test_get_payment_stream() {
        let service = create_test_service().await;
        let ps = service
            .get_payment_stream("0x123")
            .await
            .expect("get_payment_stream should succeed");

        assert!(ps.tokens_per_block > 0);
        assert!(ps.user_deposit > 0);
    }

    #[tokio::test]
    async fn test_upload_to_msp() {
        let service = create_test_service_with_rpc_responses(vec![(
            "storagehubclient_uploadToPeer",
            serde_json::json!([]),
        )])
        .await;

        // Provide at least one chunk id (upload_to_msp rejects empty sets)
        let mut chunk_ids = HashSet::new();
        chunk_ids.insert(ChunkId::new(0));

        // Create test file metadata
        let file_metadata = FileMetadata::new(
            vec![0u8; 32],
            vec![0u8; 32],
            b"test_location".to_vec(),
            1000,
            [0u8; 32].into(),
        )
        .unwrap();

        // Create test FileKeyProof
        let file_key_proof = FileKeyProof::new(
            file_metadata.owner().clone(),
            file_metadata.bucket_id().clone(),
            file_metadata.location().clone(),
            file_metadata.file_size(),
            *file_metadata.fingerprint(),
            sp_trie::CompactProof {
                encoded_nodes: vec![],
            },
        )
        .unwrap();

        let result = service.upload_to_msp(&chunk_ids, &file_key_proof).await;

        assert!(result.is_ok());
    }
}
