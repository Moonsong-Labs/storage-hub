//! StorageHub RPC client implementation

use std::{future::Future, sync::Arc};

use bigdecimal::BigDecimal;
use jsonrpsee::core::traits::ToRpcParams;
use serde::de::DeserializeOwned;
use subxt::{
    dynamic::{DecodedValueThunk, Value},
    storage::Address,
    utils::Yes,
    OnlineClient, PolkadotConfig,
};
use tracing::debug;

// use pallet_storage_providers::types::MainStorageProvider;
// use sh_solochain_evm_runtime::Runtime;
use shc_indexer_db::OnchainMspId;
use shc_rpc::{
    GetFileFromFileStorageResult, GetValuePropositionsResult, RpcProviderId, SaveFileToDisk,
};
// use sp_core::{blake2_256, storage::StorageKey, twox_128};

use crate::data::rpc::{
    connection::error::{RpcConnectionError, RpcResult},
    methods, runtime_apis, state_queries, AnyRpcConnection, RpcConnection,
};

/// StorageHub RPC client that uses an RpcConnection
pub struct StorageHubRpcClient {
    connection: Arc<AnyRpcConnection>,
}

impl StorageHubRpcClient {
    /// Create a new StorageHubRpcClient with the given connection
    pub fn new(connection: Arc<AnyRpcConnection>) -> Self {
        Self { connection }
    }

    pub async fn is_connected(&self) -> bool {
        self.connection.is_connected().await
    }

    /// Attempts to reconnect if the connection isn't connected
    async fn ensure_connected(&self) {
        if !self.is_connected().await {
            // TODO: More robust reconnection mechanism, like we do for the original connection
            _ = self.connection.reconnect().await;
        }
    }

    /// Wrapper over [`call`] for runtime APIs
    ///
    /// # Arguments:
    /// - `api` is the api method to invoke
    /// - `params` is the set of parameters for the api call
    ///
    /// The `api` method should look like "<TraitName>_<trait_method_name>",
    /// for example "Core_version" to invoke the `version` method of the `core` runtime api
    pub async fn call_runtime_api<P: codec::Encode, R: codec::Decode>(
        &self,
        api: &str,
        params: P,
    ) -> RpcResult<R> {
        // the RPC method expectes the parameters to be scale encoded and as a hex string
        let encoded = format!("0x{}", hex::encode(params.encode()));
        debug!(method = %api, ?encoded, "calling runtime api");

        let response = self
            .call::<_, String>(methods::API_CALL, jsonrpsee::rpc_params![api, encoded])
            .await?;

        // the RPC also replies with scale-encoded response as a hex string
        let response = hex::decode(response.trim_start_matches("0x")).map_err(|e| {
            RpcConnectionError::Serialization(format!(
                "RPC runtime API did not respond with a valid hex string: {}",
                e.to_string()
            ))
        })?;

        R::decode(&mut response.as_slice())
            .map_err(|e| RpcConnectionError::Serialization(e.to_string()))
    }

    /// Wrapper over [`call`] for reading storage
    ///
    /// # Arguments:
    /// - `key` is the storage key to attempt reading
    /// - `transform` is a closure to transform the raw storage value returned by the API into the target type
    pub async fn query_storage<K, F, FT, R>(&self, key: K, transform: F) -> RpcResult<Option<R>>
    where
        K: Address<Target = DecodedValueThunk, IsFetchable = Yes>,
        FT: Future<Output = RpcResult<Option<R>>>,
        F: Fn(Result<Option<K::Target>, subxt::Error>) -> FT,
    {
        //FIXME: replace Config for DH/SH one
        let api =
            OnlineClient::<PolkadotConfig>::from_rpc_client(Arc::clone(&self.connection)).await?;

        let result = api.storage().at_latest().await?.fetch(&key).await;

        transform(result).await
    }

    /// Call a JSON-RPC method on the connected node
    pub async fn call<P, R>(&self, method: &str, params: P) -> RpcResult<R>
    where
        P: ToRpcParams + Send,
        R: DeserializeOwned,
    {
        self.ensure_connected().await;

        self.connection.call(method, params).await
    }

    // Call a JSON-RPC method on the connected node without parameters
    pub async fn call_no_params<R>(&self, method: &str) -> RpcResult<R>
    where
        R: DeserializeOwned,
    {
        self.ensure_connected().await;

        self.connection.call_no_params(method).await
    }

    // TODO: Explore the possibility of directly using StorageHubClientApi trait
    // from the client's RPC module to avoid having to manually implement new RPC calls

    /// Get the current price per giga unit per tick
    ///
    /// Returns the price value that represents the cost per giga unit per tick
    /// in the StorageHub network.
    pub async fn get_current_price_per_giga_unit_per_tick(&self) -> RpcResult<BigDecimal> {
        debug!(target: "rpc::client::get_current_price_per_giga_unit_per_tick", "RPC call: get_current_price_per_giga_unit_per_tick");

        self.call_runtime_api::<_, runtime_apis::CurrentPrice>(runtime_apis::CURRENT_PRICE, ())
            .await
            .map(|price| price.into())
    }

    /// Retrieve the current available capacity for given provider, in storage units
    pub async fn get_available_capacity(&self, provider: OnchainMspId) -> RpcResult<BigDecimal> {
        debug!(target: "rpc::client::get_available_capacity", "Runtime API: get_available_capacity");

        self.call_runtime_api::<_, runtime_apis::AvailableCapacity>(
            runtime_apis::AVAILABLE_CAPACITY,
            provider.as_h256(),
        )
        .await
        .map(|capacity| capacity.into())
    }

    /// Retrieve the MSP information for the given provider
    ///
    /// This function will read into the chain state from the Provider pallet's MainStorageProviders map
    // TODO: replace return value with proper typing from runtime
    pub async fn get_msp_info(
        &self,
        provider: OnchainMspId,
    ) -> RpcResult<Option<state_queries::MspInfo>> {
        debug!(target: "rpc::client::get_msp", provider = %provider, "State Query: get_msp_info");
        let key = subxt::dynamic::storage(
            state_queries::MSP_INFO_MODULE,
            state_queries::MSP_INFO_METHOD,
            vec![Value::from_bytes(provider.as_bytes())],
        );

        self.query_storage(key, |storage| async move {
            let Some(_value) = storage?
                .map(|storage| storage.to_value())
                .transpose()
                .map_err(subxt::Error::from)?
            else {
                return Ok(None);
            };

            //FIXME: convert to MspInfo

            Ok(None)
        })
        .await
    }

    /// Returns whether the given `file_key` is expected to be received by the MSP node
    pub async fn is_file_key_expected(&self, file_key: &str) -> RpcResult<bool> {
        debug!(target: "rpc::client::is_file_key_expected", file_key = %file_key, "RPC call: is_file_key_expected");

        self.call(methods::FILE_KEY_EXPECTED, jsonrpsee::rpc_params![file_key])
            .await
    }

    /// Checks the status of a file in MSP storage by its file key.
    ///
    /// # Returns
    /// - `FileNotFound`: File does not exist in storage
    /// - `FileFound`: File exists and is complete
    /// - `IncompleteFile`: File exists but is missing chunks
    /// - `FileFoundWithInconsistency`: File exists but has data integrity issues
    pub async fn is_file_in_file_storage(
        &self,
        file_key: &str,
    ) -> RpcResult<GetFileFromFileStorageResult> {
        self.call(
            methods::IS_FILE_IN_FILE_STORAGE,
            jsonrpsee::rpc_params![file_key],
        )
        .await
    }

    /// Request the MSP node to export the given `file_key` to the given URL
    pub async fn save_file_to_disk(&self, file_key: &str, url: &str) -> RpcResult<SaveFileToDisk> {
        debug!(
            target: "rpc::client::save_file_to_disk",
            file_key = %file_key,
            url = %url,
            "RPC call: save_file_to_disk"
        );

        self.call(
            methods::SAVE_FILE_TO_DISK,
            jsonrpsee::rpc_params![file_key, url],
        )
        .await
    }

    /// Request the MSP to accept a FileKeyProof (`proof`) for the given `file_key`
    pub async fn receive_file_chunks(&self, file_key: &str, proof: Vec<u8>) -> RpcResult<Vec<u8>> {
        debug!(
            target: "rpc::client::receive_file_chunks",
            file_key = %file_key,
            proof_size = proof.len(),
            "RPC call: receive_file_chunks"
        );

        self.call(
            methods::RECEIVE_FILE_CHUNKS,
            jsonrpsee::rpc_params![file_key, proof],
        )
        .await
    }

    /// Retrieve the Onchain Provider ID of the MSP Node (therefore the MSP ID)
    pub async fn get_provider_id(&self) -> RpcResult<RpcProviderId> {
        debug!(target: "rpc::client::get_provider_id", "RPC call: get_provider_id");

        self.call_no_params(methods::PROVIDER_ID).await
    }

    /// Retrieve the list of value propositions of the MSP Node
    pub async fn get_value_props(&self) -> RpcResult<GetValuePropositionsResult> {
        debug!(target: "rpc::client::get_value_props", "RPC call: get_value_props");

        self.call_no_params(methods::VALUE_PROPS).await
    }

    /// Retrieve the list of multiaddresses associated with the MSP Node
    pub async fn get_multiaddresses(&self) -> RpcResult<Vec<String>> {
        debug!(target: "rpc::client::get_multiaddresses", "RPC call: get_multiaddresses");

        self.call_no_params(methods::PEER_IDS).await
    }
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use bigdecimal::Signed;
    use codec::Decode;

    use shp_types::Hash;

    use super::*;
    use crate::{
        constants::rpc::DUMMY_MSP_ID,
        data::rpc::{AnyRpcConnection, MockConnection},
        models::msp_info::ValuePropositionWithId,
        test_utils::random_bytes_32,
    };

    // TODO(SCAFFOLDING): this will contain proper tests when we have defined
    // what RPC methods to make use of
    #[tokio::test]
    async fn use_mock_connection() {
        let mock_conn = MockConnection::new();
        mock_conn.disconnect().await;

        let connection = Arc::new(AnyRpcConnection::Mock(mock_conn));
        let client = StorageHubRpcClient::new(connection);

        let connected = client.is_connected().await;
        assert!(!connected);
    }

    fn mock_rpc() -> StorageHubRpcClient {
        let mock_conn = MockConnection::new();
        let connection = Arc::new(AnyRpcConnection::Mock(mock_conn));
        StorageHubRpcClient::new(connection)
    }

    #[tokio::test]
    async fn reconnect_automatically() {
        let conn = MockConnection::new();
        conn.disconnect().await;
        let conn = Arc::new(AnyRpcConnection::Mock(conn));
        let client = StorageHubRpcClient::new(conn);

        assert!(
            !client.is_connected().await,
            "Should not be connected initially"
        );

        let result = client.get_provider_id().await;
        assert!(
            result.is_ok(),
            "Should reconnect and be able to retrieve provider id"
        );

        assert!(client.is_connected().await, "Should be connected now");
    }

    #[tokio::test]
    async fn get_current_price_per_unit_per_tick() {
        let client = mock_rpc();

        // Test that the mock returns the expected price
        let price = client
            .get_current_price_per_giga_unit_per_tick()
            .await
            .expect("able to retrieve current price per giga unit");
        assert!(
            price.is_positive(),
            "Price per giga unit should always be > 0"
        );
    }

    #[tokio::test]
    async fn is_file_key_expected() {
        let client = mock_rpc();

        let result = client
            .is_file_key_expected(&hex::encode(random_bytes_32()))
            .await
            .expect("able to retrieve if the given file key was expected");

        assert!(result);
    }

    #[tokio::test]
    async fn save_file_to_disk() {
        let client = mock_rpc();

        let file_name = "my_file.jpg";

        let result = client
            .save_file_to_disk(
                &hex::encode(random_bytes_32()),
                &format!("http://localhost/upload/{file_name}"),
            )
            .await
            .expect("able to call save file to disk");

        assert!(
            matches!(result, SaveFileToDisk::Success(_)),
            "should be successfull"
        );

        let SaveFileToDisk::Success(metadata) = result else {
            unreachable!();
        };

        assert_eq!(
            metadata.location(),
            file_name.as_bytes(),
            "resulting file name should match input in url"
        );
        assert!(metadata.file_size() > 0, "should have some data");
    }

    #[tokio::test]
    async fn receive_file_chunks() {
        let client = mock_rpc();

        let response = client
            .receive_file_chunks(&hex::encode(random_bytes_32()), random_bytes_32().to_vec())
            .await
            .expect("able to call receive file chunks");

        // the mock response is an empty vec, but that's most likely different
        // from the real RPC
        assert!(response.is_empty())
    }

    #[tokio::test]
    async fn get_provider_id() {
        let client = mock_rpc();

        let response = client
            .get_provider_id()
            .await
            .expect("able to get provider id");

        assert!(
            matches!(response, RpcProviderId::Msp(_)),
            "should be an MSP with an assigned ID"
        );

        let RpcProviderId::Msp(msp_id) = response else {
            unreachable!()
        };

        assert_eq!(
            msp_id,
            Hash::from_slice(DUMMY_MSP_ID.as_slice()),
            "should be set to DUMMY_MSP_ID"
        );
    }

    #[tokio::test]
    async fn get_value_props() {
        let client = mock_rpc();

        let response = client
            .get_value_props()
            .await
            .expect("able to get value props");

        assert!(
            matches!(response, GetValuePropositionsResult::Success(_)),
            "should be successfull"
        );

        let GetValuePropositionsResult::Success(props) = response else {
            unreachable!()
        };

        assert!(props.len() > 0, "should have at least 1 value prop");

        let decoded = props
            .into_iter()
            .map(|encoded| ValuePropositionWithId::decode(&mut encoded.as_slice()))
            .collect::<Result<Vec<_>, _>>()
            .expect("able to decode ValuePropositionWithId");

        assert!(
            decoded
                .iter()
                .filter(|prop| prop.value_prop.available)
                .count()
                > 0,
            "should have at least 1 available value prop"
        );
    }

    #[tokio::test]
    async fn get_multiaddresses() {
        let client = mock_rpc();

        let response = client
            .get_multiaddresses()
            .await
            .expect("should be able to retrieve multiaddresses");

        assert!(response.len() > 0, "should have at least 1 multiaddress");
    }

    #[tokio::test]
    async fn is_file_in_file_storage() {
        let client = mock_rpc();

        let response = client
            .is_file_in_file_storage(&hex::encode(random_bytes_32()))
            .await
            .expect("should be able to upload file");

        assert!(
            matches!(response, GetFileFromFileStorageResult::FileFound(_)),
            "should be successfull"
        );
    }
}
