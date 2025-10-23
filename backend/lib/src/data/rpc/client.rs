//! StorageHub RPC client implementation

use std::sync::Arc;

use jsonrpsee::core::traits::ToRpcParams;
use serde::de::DeserializeOwned;
use tracing::debug;

use shc_rpc::{
    GetFileFromFileStorageResult, GetValuePropositionsResult, RpcProviderId, SaveFileToDisk,
};

use crate::data::rpc::{connection::error::RpcResult, methods, AnyRpcConnection, RpcConnection};

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

    /// Call a JSON-RPC method on the connected node
    pub async fn call<P, R>(&self, method: &str, params: P) -> RpcResult<R>
    where
        P: ToRpcParams + Send,
        R: DeserializeOwned,
    {
        self.connection.call(method, params).await
    }

    // Call a JSON-RPC method on the connected node without parameters
    pub async fn call_no_params<R>(&self, method: &str) -> RpcResult<R>
    where
        R: DeserializeOwned,
    {
        self.connection.call_no_params(method).await
    }

    // TODO: Explore the possibility of directly using StorageHubClientApi trait
    // from the client's RPC module to avoid having to manually implement new RPC calls

    /// Get the current price per giga unit per tick
    ///
    /// Returns the price value (u128) that represents the cost per giga unit per tick
    /// in the StorageHub network.
    pub async fn get_current_price_per_giga_unit_per_tick(&self) -> RpcResult<u128> {
        debug!(target: "rpc::client::get_current_price_per_giga_unit_per_tick", "RPC call: get_current_price_per_giga_unit_per_tick");

        self.connection.call_no_params(methods::CURRENT_PRICE).await
    }

    /// Returns whether the given `file_key` is expected to be received by the MSP node
    pub async fn is_file_key_expected(&self, file_key: &str) -> RpcResult<bool> {
        debug!(target: "rpc::client::is_file_key_expected", file_key = %file_key, "RPC call: is_file_key_expected");

        self.connection
            .call(methods::FILE_KEY_EXPECTED, jsonrpsee::rpc_params![file_key])
            .await
    }

    // Returns the status of a give file_key in the MSP storage.
    // The possible results are:
    //  FileNotFound
    //  FileFound
    //  IncompleteFile
    //  FileFoundWithInconsistency
    pub async fn is_file_in_file_storage(
        &self,
        file_key: &str,
    ) -> RpcResult<GetFileFromFileStorageResult> {
        self.connection
            .call(
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

        self.connection
            .call(
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

        self.connection
            .call(
                methods::RECEIVE_FILE_CHUNKS,
                jsonrpsee::rpc_params![file_key, proof],
            )
            .await
    }

    /// Retrieve the Onchain Provider ID of the MSP Node (therefore the MSP ID)
    pub async fn get_provider_id(&self) -> RpcResult<RpcProviderId> {
        debug!(target: "rpc::client::get_provider_id", "RPC call: get_provider_id");

        self.connection.call_no_params(methods::PROVIDER_ID).await
    }

    /// Retrieve the list of value propositions of the MSP Node
    pub async fn get_value_props(&self) -> RpcResult<GetValuePropositionsResult> {
        debug!(target: "rpc::client::get_value_props", "RPC call: get_value_props");

        self.connection.call_no_params(methods::VALUE_PROPS).await
    }

    /// Retrieve the list of multiaddresses associated with the MSP Node
    pub async fn get_multiaddresses(&self) -> RpcResult<Vec<String>> {
        debug!(target: "rpc::client::get_multiaddresses", "RPC call: get_multiaddresses");

        self.connection.call_no_params(methods::PEER_IDS).await
    }
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
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
    async fn get_current_price_per_unit_per_tick() {
        let client = mock_rpc();

        // Test that the mock returns the expected price
        let price = client
            .get_current_price_per_giga_unit_per_tick()
            .await
            .expect("able to retrieve current price per giga unit");
        assert!(price > 0);
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
}
