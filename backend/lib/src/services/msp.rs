//! MSP service implementation with mock data
//!
//! TODO(MOCK): many of methods of the MspService returns mocked data

use std::{collections::HashSet, sync::Arc};

use chrono::Utc;
use codec::{Decode, Encode};
use sc_network::PeerId;
use serde::{Deserialize, Serialize};
use shc_common::types::{ChunkId, FileKeyProof};
use shc_rpc::{GetValuePropositionsResult, RpcProviderId, SaveFileToDisk};
use sp_core::{Blake2Hasher, H256};
use tracing::{debug, info, warn};

use shc_indexer_db::{models::Bucket as DBBucket, OnchainMspId};
use shp_types::Hash;

use crate::{
    constants::mocks::{PLACEHOLDER_BUCKET_FILE_COUNT, PLACEHOLDER_BUCKET_SIZE_BYTES},
    data::{indexer_db::client::DBClient, rpc::StorageHubRpcClient, storage::BoxedStorage},
    error::Error,
    models::{
        buckets::{Bucket, FileTree},
        files::{DistributeResponse, FileInfo},
        msp_info::{
            Capacity, InfoResponse, MspHealthResponse, StatsResponse, ValuePropositionWithId,
        },
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
    msp_callback_url: String,
}

impl MspService {
    /// Create a new MSP service
    /// Only MSP nodes are supported so it returns an error if the node is not an MSP.
    pub async fn new(
        storage: Arc<dyn BoxedStorage>,
        postgres: Arc<DBClient>,
        rpc: Arc<StorageHubRpcClient>,
        msp_callback_url: String,
    ) -> Result<Self, Error> {
        // Discover provider id from the connected node.
        // If the node is not yet an MSP (which happens in integration tests), retry with
        // a bounded number of attempts.
        // TODO: Think about making it so in integration tests we spin up the backend
        // only after the MSP has been registered on-chain, to avoid having this retry logic.
        let mut retry_attempts = 0;
        let max_retries = 10;
        let delay_between_retries_secs = 5;

        let msp_id = loop {
            let provider_id: RpcProviderId = rpc
                .call_no_params("storagehubclient_getProviderId")
                .await
                .map_err(|e| Error::BadRequest(e.to_string()))?;

            match provider_id {
                RpcProviderId::Msp(id) => break OnchainMspId::new(Hash::from_slice(id.as_ref())),
                RpcProviderId::Bsp(_) => {
                    return Err(Error::BadRequest(
                        "Connected node is a BSP; expected an MSP".to_string(),
                    ))
                }
                RpcProviderId::NotAProvider => {
                    if retry_attempts >= max_retries {
                        return Err(Error::BadRequest(
                            "Connected node not a registered MSP after timeout".to_string(),
                        ));
                    }
                    warn!(
                        "Connected node is not yet a registered MSP; retrying provider discovery... (attempt {})",
                        retry_attempts + 1
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(delay_between_retries_secs))
                        .await;
                    retry_attempts += 1;
                    continue;
                }
            }
        };

        Ok(Self {
            msp_id,
            storage,
            postgres,
            rpc,
            // TODO: dedicated config struct
            // see: https://github.com/Moonsong-Labs/storage-hub/pull/459/files#r2369596519
            msp_callback_url,
        })
    }

    /// Get MSP information
    pub async fn get_info(&self) -> Result<InfoResponse, Error> {
        // Fetch the MSP's local listen multiaddresses via RPC
        let multiaddresses: Vec<String> = self
            .rpc
            .call_no_params("system_localListenAddresses")
            .await
            .map_err(|e| Error::BadRequest(e.to_string()))?;

        Ok(InfoResponse {
            client: "storagehub-node v1.0.0".to_string(),
            version: "StorageHub MSP v0.1.0".to_string(),
            msp_id: self.msp_id.to_string(),
            multiaddresses,
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
    pub async fn get_value_props(&self) -> Result<Vec<ValuePropositionWithId>, Error> {
        // Call RPC to get the value propositions
        let result: GetValuePropositionsResult = self
            .rpc
            .call(
                "storagehubclient_getValuePropositions",
                jsonrpsee::rpc_params![],
            )
            .await
            .map_err(|e| Error::BadRequest(e.to_string()))?;

        // Decode the SCALE-encoded ValuePropositionWithId entries
        match result {
            GetValuePropositionsResult::Success(encoded_props) => {
                let mut props = Vec::with_capacity(encoded_props.len());
                for encoded_value_proposition in encoded_props {
                    let value_prop_with_id =
                        ValuePropositionWithId::decode(&mut encoded_value_proposition.as_slice())
                            .map_err(|e| {
                            Error::BadRequest(format!(
                                "Failed to decode ValuePropositionWithId: {}",
                                e
                            ))
                        })?;

                    props.push(ValuePropositionWithId {
                        id: value_prop_with_id.id,
                        value_prop: value_prop_with_id.value_prop,
                    });
                }
                Ok(props)
            }
            GetValuePropositionsResult::NotAnMsp => Err(Error::BadRequest(
                "The node that we are connected to is not an MSP".to_string(),
            )),
        }
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
        let file_key_hex = file_key.trim_start_matches("0x");

        let file_key = hex::decode(file_key_hex)
            .map_err(|e| Error::BadRequest(format!("Invalid File Key hex encoding: {}", e)))?;

        if file_key.len() != 32 {
            return Err(Error::BadRequest(format!(
                "Invalid File Key length. Expected 32 bytes, got {}",
                file_key.len()
            )));
        }

        // get bucket determine if user can view it
        let bucket = self.get_bucket(bucket_id, user).await?;

        self.postgres
            .get_file_info(&file_key)
            .await
            .map(|file| FileInfo::from_db(&file, bucket.is_public))
    }

    /// Check via MSP RPC if this node is expecting to receive the given file key
    pub async fn is_msp_expecting_file_key(&self, file_key: &str) -> Result<bool, Error> {
        let expected: bool = self
            .rpc
            .call("storagehubclient_isFileKeyExpected", (file_key,))
            .await
            .map_err(|e| Error::BadRequest(e.to_string()))?;
        Ok(expected)
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

    /// Download a file by `file_key` via the MSP RPC into `/tmp/uploads/<file_key>` and
    /// return its size, UTF-8 location, fingerprint, and temp path.
    /// Returns BadRequest on RPC/parse errors.
    ///
    /// We provide an URL as saveFileToDisk RPC requires it to stream the file.
    /// We also implemented the internal_upload_by_key handler to handle this temporary file upload.
    pub async fn get_file_from_key(&self, file_key: &str) -> Result<FileDownloadResult, Error> {
        // TODO: authenticate user

        // Create temp url for download
        let temp_path = format!("/tmp/uploads/{}", file_key);
        let upload_url = format!("{}/internal/uploads/{}", self.msp_callback_url, file_key);

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

    /// Upload a batch of file chunks with their FileKeyProof to the MSP via its RPC.
    ///
    /// This implementation:
    /// 1. Gets the MSP info to get its multiaddresses.
    /// 2. Extracts the peer IDs from the multiaddresses.
    /// 3. Sends the FileKeyProof with the batch of chunks to the MSP through the `receiveBackendFileChunks` RPC method.
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
        let mut peer_ids = HashSet::new();

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
                        peer_ids.insert(peer_id);
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

        Ok(peer_ids.into_iter().collect())
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

        // Get fhe file metadata from the received FileKeyProof.
        let file_metadata = file_key_proof.clone().file_metadata;

        // Get the file key from the file metadata.
        let file_key: H256 = file_metadata.file_key::<Blake2Hasher>();

        // Encode the FileKeyProof as SCALE for transport
        let encoded_proof = file_key_proof.encode();

        // TODO: We should make these configurable.
        let mut retry_attempts = 0;
        let max_retries = 3;
        let delay_between_retries_secs = 1;

        while retry_attempts < max_retries {
            let result: Result<Vec<u8>, _> = self
                .rpc
                .call(
                    "storagehubclient_receiveBackendFileChunks",
                    (file_key, encoded_proof.clone()),
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
                        tokio::time::sleep(std::time::Duration::from_secs(
                            delay_between_retries_secs,
                        ))
                        .await;
                    } else {
                        return Err(Error::Internal);
                    }
                }
            }
        }

        Err(Error::Internal)
    }
}

impl MspService {
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
        let bucket_id_hex = bucket_id.trim_start_matches("0x");

        let bucket_id = hex::decode(bucket_id_hex)
            .map_err(|e| Error::BadRequest(format!("Invalid Bucket ID hex encoding: {}", e)))?;

        if bucket_id.len() != 32 {
            return Err(Error::BadRequest(format!(
                "Invalid Bucket ID length. Expected 32 bytes, got {}",
                bucket_id.len()
            )));
        }

        self.postgres
            .get_bucket(&bucket_id)
            .await
            .and_then(|bucket| self.can_user_view_bucket(bucket, user))
    }
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use std::sync::Arc;

    use serde_json::Value;

    use shc_common::types::{FileKeyProof, FileMetadata};
    use shp_types::Hash;

    use super::*;
    use crate::{
        config::Config,
        constants::{
            mocks::MOCK_ADDRESS,
            rpc::DUMMY_MSP_ID,
            test::{bucket::DEFAULT_BUCKET_NAME, file::DEFAULT_SIZE},
        },
        data::{
            indexer_db::{client::DBClient, mock_repository::MockRepository},
            rpc::{AnyRpcConnection, MockConnection, StorageHubRpcClient},
            storage::{BoxedStorageWrapper, InMemoryStorage},
        },
        models::msp_info::{ValueProposition, ValuePropositionWithId},
        test_utils::random_bytes_32,
    };

    /// Builder for creating MspService instances with mock dependencies for testing
    struct MockMspServiceBuilder {
        storage: Arc<BoxedStorageWrapper<InMemoryStorage>>,
        postgres: Arc<DBClient>,
        rpc: Arc<StorageHubRpcClient>,
    }

    impl MockMspServiceBuilder {
        /// Create a new builder with default empty mocks
        pub fn new() -> Self {
            Self {
                storage: Arc::new(BoxedStorageWrapper::new(InMemoryStorage::new())),
                postgres: Arc::new(DBClient::new(Arc::new(MockRepository::new()))),
                rpc: Arc::new(StorageHubRpcClient::new(Arc::new(AnyRpcConnection::Mock(
                    MockConnection::new(),
                )))),
            }
        }

        /// Initialize repository with custom test data
        pub async fn init_repository_with<F>(self, init: F) -> Self
        where
            F: FnOnce(&DBClient) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + '_>>,
        {
            init(&self.postgres).await;
            self
        }

        /// Set RPC responses for the connection to use
        pub async fn with_rpc_responses(mut self, responses: Vec<(&str, Value)>) -> Self {
            let mock_conn = MockConnection::new();
            for (method, value) in responses {
                mock_conn.set_response(method, value).await;
            }
            self.rpc = Arc::new(StorageHubRpcClient::new(Arc::new(AnyRpcConnection::Mock(
                mock_conn,
            ))));
            self
        }

        /// Build the final MspService
        pub async fn build(self) -> MspService {
            let cfg = Config::default();

            MspService::new(
                self.storage,
                self.postgres,
                self.rpc,
                cfg.storage_hub.msp_callback_url,
            )
            .await
            .expect("Mocked MSP service builder should succeed")
        }
    }

    #[tokio::test]
    async fn test_get_info() {
        let service = MockMspServiceBuilder::new().build().await;
        let info = service.get_info().await.unwrap();

        assert_eq!(info.status, "active");
        assert!(!info.multiaddresses.is_empty());
    }

    #[tokio::test]
    async fn test_get_stats() {
        let service = MockMspServiceBuilder::new().build().await;
        let stats = service.get_stats().await.unwrap();

        assert!(stats.capacity.total_bytes > 0);
        assert!(stats.capacity.available_bytes <= stats.capacity.total_bytes);
    }

    #[tokio::test]
    async fn test_get_value_props() {
        let service = MockMspServiceBuilder::new()
            .with_rpc_responses(vec![(
                "storagehubclient_getValuePropositions",
                serde_json::json!(GetValuePropositionsResult::Success(vec![{
                    let mut value_prop_with_id = ValuePropositionWithId::default();
                    value_prop_with_id.id = H256::from_slice(&random_bytes_32());
                    value_prop_with_id.value_prop = ValueProposition::default();
                    value_prop_with_id
                        .value_prop
                        .price_per_giga_unit_of_data_per_block = 100;
                    value_prop_with_id.value_prop.bucket_data_limit = 100;
                    value_prop_with_id.value_prop.available = true;
                    value_prop_with_id.encode()
                },])),
            )])
            .await
            .build()
            .await;
        let props = service.get_value_props().await.unwrap();

        assert!(!props.is_empty());
        assert!(props.iter().any(|p| p.value_prop.available));
    }

    #[tokio::test]
    async fn test_list_user_buckets() {
        let service = MockMspServiceBuilder::new()
            .init_repository_with(|client| {
                Box::pin(async move {
                    // Create MSP with the ID that matches the default config
                    let msp = client
                        .create_msp(
                            MOCK_ADDRESS,
                            OnchainMspId::new(Hash::from_slice(&DUMMY_MSP_ID)),
                        )
                        .await
                        .expect("should create MSP");

                    // Create a test bucket for the mock user
                    client
                        .create_bucket(
                            MOCK_ADDRESS,
                            Some(msp.id),
                            DEFAULT_BUCKET_NAME.as_bytes(),
                            random_bytes_32().as_slice(),
                            false,
                        )
                        .await
                        .expect("should create bucket");
                })
            })
            .await
            .build()
            .await;

        let buckets = service
            .list_user_buckets(MOCK_ADDRESS)
            .await
            .unwrap()
            .collect::<Vec<_>>();

        assert!(!buckets.is_empty());
    }

    #[tokio::test]
    async fn test_get_bucket() {
        let bucket_name = "my-bucket";
        let bucket_id = random_bytes_32();

        let service = MockMspServiceBuilder::new()
            .init_repository_with(|client| {
                Box::pin(async move {
                    // Create MSP with the ID that matches the default config
                    let msp = client
                        .create_msp(
                            MOCK_ADDRESS,
                            OnchainMspId::new(Hash::from_slice(&DUMMY_MSP_ID)),
                        )
                        .await
                        .expect("should create MSP");

                    // Create a test bucket for the mock user
                    let bucket = client
                        .create_bucket(
                            MOCK_ADDRESS,
                            Some(msp.id),
                            bucket_name.as_bytes(),
                            &bucket_id,
                            false,
                        )
                        .await
                        .expect("should create bucket");

                    client
                        .create_file(
                            MOCK_ADDRESS.as_bytes(),
                            random_bytes_32().as_slice(),
                            bucket.id,
                            &bucket_id,
                            "sample-file.txt".as_bytes(),
                            random_bytes_32().as_slice(),
                            DEFAULT_SIZE,
                        )
                        .await
                        .expect("should create file");
                })
            })
            .await
            .build()
            .await;

        let bucket_id = hex::encode(bucket_id);
        let bucket = service.get_bucket(&bucket_id, MOCK_ADDRESS).await.unwrap();

        assert_eq!(bucket.bucket_id, bucket_id);
        assert_eq!(bucket.name, bucket_name);
    }

    #[tokio::test]
    async fn test_get_files_root() {
        let bucket_id = random_bytes_32();

        let service = MockMspServiceBuilder::new()
            .init_repository_with(|client| {
                Box::pin(async move {
                    // Create MSP with the ID that matches the default config
                    let msp = client
                        .create_msp(
                            MOCK_ADDRESS,
                            OnchainMspId::new(Hash::from_slice(&DUMMY_MSP_ID)),
                        )
                        .await
                        .expect("should create MSP");

                    // Create a test bucket for the mock user
                    let bucket = client
                        .create_bucket(
                            MOCK_ADDRESS,
                            Some(msp.id),
                            DEFAULT_BUCKET_NAME.as_bytes(),
                            &bucket_id,
                            false,
                        )
                        .await
                        .expect("should create bucket");

                    client
                        .create_file(
                            MOCK_ADDRESS.as_bytes(),
                            random_bytes_32().as_slice(),
                            bucket.id,
                            &bucket_id,
                            "sample-file.txt".as_bytes(),
                            random_bytes_32().as_slice(),
                            DEFAULT_SIZE,
                        )
                        .await
                        .expect("should create file");
                })
            })
            .await
            .build()
            .await;

        let tree = service
            .get_file_tree(hex::encode(bucket_id).as_ref(), MOCK_ADDRESS, "/")
            .await
            .unwrap();

        tree.entry.folder().expect("first entry to be a folder");
    }

    #[tokio::test]
    async fn test_get_file_info() {
        let file_key = random_bytes_32();
        let bucket_id = random_bytes_32();

        let service = MockMspServiceBuilder::new()
            .init_repository_with(|client| {
                Box::pin(async move {
                    // Create MSP with the ID that matches the default config
                    let msp = client
                        .create_msp(
                            MOCK_ADDRESS,
                            OnchainMspId::new(Hash::from_slice(&DUMMY_MSP_ID)),
                        )
                        .await
                        .expect("should create MSP");

                    // Create a test bucket for the mock user
                    let bucket = client
                        .create_bucket(
                            MOCK_ADDRESS,
                            Some(msp.id),
                            DEFAULT_BUCKET_NAME.as_bytes(),
                            &bucket_id,
                            false,
                        )
                        .await
                        .expect("should create bucket");

                    client
                        .create_file(
                            MOCK_ADDRESS.as_bytes(),
                            &file_key,
                            bucket.id,
                            &bucket_id,
                            "sample-file.txt".as_bytes(),
                            random_bytes_32().as_slice(),
                            DEFAULT_SIZE,
                        )
                        .await
                        .expect("should create file");
                })
            })
            .await
            .build()
            .await;

        let bucket_id = hex::encode(bucket_id);
        let file_key = hex::encode(file_key);

        let info = service
            .get_file_info(&bucket_id, MOCK_ADDRESS, &file_key)
            .await
            .expect("get_file_info should succeed");

        assert_eq!(info.bucket_id, bucket_id);
        assert_eq!(info.file_key, file_key);
        assert!(!info.location.is_empty());
        assert!(info.size > 0);
    }

    #[tokio::test]
    async fn test_distribute_file() {
        let service = MockMspServiceBuilder::new().build().await;
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
        let service = MockMspServiceBuilder::new().build().await;
        let ps = service
            .get_payment_stream("0x123")
            .await
            .expect("get_payment_stream should succeed");

        assert!(ps.tokens_per_block > 0);
        assert!(ps.user_deposit > 0);
    }

    #[tokio::test]
    async fn test_upload_to_msp() {
        let service = MockMspServiceBuilder::new()
            .with_rpc_responses(vec![(
                "storagehubclient_receiveBackendFileChunks",
                serde_json::json!([]),
            )])
            .await
            .build()
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
