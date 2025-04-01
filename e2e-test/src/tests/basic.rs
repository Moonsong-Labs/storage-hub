#[cfg(test)]
pub mod basic_subxt_checks {
    #![allow(missing_docs)]
    use crate::create_subxt_api;
    use anyhow::Result;
    use parity_scale_codec::Decode;
    use rand;
    use reqwest;
    use rstest::rstest;
    use serde::{Deserialize, Serialize};
    use std::str::FromStr;
    use subxt::backend::chain_head::ChainHeadRpcMethods;
    use subxt::backend::legacy::LegacyRpcMethods;
    use subxt::backend::rpc::RpcClient;
    use subxt::config::polkadot::PolkadotExtrinsicParamsBuilder as Params;
    use subxt::dynamic::Value;
    use subxt::utils::AccountId32;
    use subxt::{PolkadotConfig, SubstrateConfig};
    use subxt_signer::sr25519::{dev, Keypair};
    use subxt_signer::SecretUri;
    use tracing::{debug, info};
    use tracing_test::traced_test;

    /// Represents the chain properties returned by system_properties RPC call
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ChainProperties {
        /// Whether the chain is Ethereum-compatible
        #[serde(rename = "isEthereum")]
        pub is_ethereum: Option<bool>,

        /// The SS58 address format (if specified)
        #[serde(rename = "ss58Format")]
        pub ss58_format: Option<u32>,

        /// Token decimal places for each token (if specified)
        /// This is typically a vector, as a chain may have multiple tokens
        #[serde(rename = "tokenDecimals")]
        pub token_decimals: Option<u32>,

        /// Token symbols for each token (if specified)
        /// This is typically a vector, as a chain may have multiple tokens
        #[serde(rename = "tokenSymbol")]
        pub token_symbol: Option<String>,
    }

    // Example of how to use this struct for deserialization
    impl ChainProperties {
        /// Parse ChainProperties from a JSON string
        pub fn from_json(json: &str) -> std::result::Result<Self, serde_json::Error> {
            serde_json::from_str(json)
        }
    }

    #[subxt::subxt(runtime_metadata_path = "./storage-hub-v15.scale")]
    pub mod storage_hub {}
    #[rstest]
    #[tokio::test]
    #[traced_test]
    async fn can_read_constants() -> Result<()> {
        let api = create_subxt_api().await?;

        let constant_query = storage_hub::constants().system().version();
        let version = api.constants().at(&constant_query)?;
        debug!("Full version const value: {:?}", version);

        assert_eq!(version.spec_name, "storage-hub-runtime");
        assert!(version.spec_version > 0);
        info!(
            "Successfully connected to node at {} rt-version {}",
            version.spec_name, version.spec_version
        );
        Ok(())
    }

    #[rstest]
    #[tokio::test]
    #[traced_test]
    async fn can_query_storage() -> Result<()> {
        let api = create_subxt_api().await.unwrap();
        let storage_api = api.storage().at_latest().await?;

        let storage_query = storage_hub::storage().providers().bsp_count();
        let bsps = storage_api.fetch(&storage_query).await?;

        debug!("Storage value returned: {:?}", bsps);
        assert!(bsps.is_none());

        let alice = dev::alice().public_key().into();
        let storage_query = storage_hub::storage().system().account(&alice);
        let alice_bal = storage_api.fetch(&storage_query).await?.unwrap();

        debug!("Storage value returned: {:?}", alice_bal);
        assert!(alice_bal.data.free > 0);
        Ok(())
    }

    #[rstest]
    #[tokio::test]
    #[traced_test]
    async fn can_submit_extrinsic() -> Result<()> {
        let api = create_subxt_api().await.unwrap();
        debug!("Block num: {}", api.blocks().at_latest().await?.number());
        let block_api = api.blocks().at_latest().await?;

        let uri = SecretUri::from_str(&format!("//rand-{}", rand::random::<u32>()))
            .expect("Failed to create random account");
        let dest: AccountId32 = Keypair::from_uri(&uri)
            .expect("valid keypair")
            .public_key()
            .into();

        let storage_query = storage_hub::storage().system().account(dest);
        // let bal_before = api
        //     .storage()
        //     .at_latest()
        //     .await?
        //     .fetch(&storage_query)
        //     .await?
        //     .unwrap();
        // debug!("Bal Before: {:?}", &bal_before);

        let alice = dev::alice();
        let start_block = block_api.header().number;
        debug!("Current block before transaction: {}", start_block);

        let to = Keypair::from_uri(&uri)
            .expect("valid keypair")
            .public_key()
            .into();
        let tx = storage_hub::tx()
            .balances()
            .transfer_allow_death(to, 10000000000000);
        let tx_params = Params::new().build();
        let ext_success = api
            .tx()
            .sign_and_submit_then_watch(&tx, &alice, tx_params)
            .await?
            .wait_for_finalized_success();

        let rpc_client = RpcClient::from_url("ws://127.0.0.1:9944").await?;
        let rpc = LegacyRpcMethods::<SubstrateConfig>::new(rpc_client.clone());
        println!(
            "📛 System Name: {:?}\n🩺 Health: {:?}\n🖫 Properties: {:?}\n🔗 Chain: {:?}\n",
            rpc.system_name().await?,
            rpc.system_health().await?,
            rpc.system_properties().await?,
            rpc.system_chain().await?,
        );

        create_block().await?;
        ext_success.await?;

        debug!("Block num: {}", api.blocks().at_latest().await?.number());
        let bal_after = api
            .storage()
            .at_latest()
            .await?
            .fetch(&storage_query)
            .await?
            .unwrap();
        debug!("Bal After: {:?}", &bal_after);
        assert!(bal_after.data.free > 0);

        Ok(())
    }

    #[rstest]
    #[tokio::test]
    #[traced_test]
    async fn can_decode_v15_metadata() -> Result<()> {
        use subxt_codegen::Metadata;

        let metadata_bytes = std::fs::read("./storage-hub-v15.scale").expect("metadata not found");
        let metadata = Metadata::decode(&mut &*metadata_bytes).expect("the metadata must decode");
        debug!("{:?}", metadata);

        // fn main() {
        //     println!("cargo:rerun-if-changed=phala_metadata.scale");
        //
        //     let output_filename = "src/phala_metadata.rs";
        //
        //     let metadata = include_bytes!("./phala_metadata.scale");
        //     let metadata = Metadata::decode(&mut &metadata[..]).unwrap();
        //     let mut builder = CodegenBuilder::new();
        //     builder.set_target_module(syn::parse_quote! { mod phala {} });
        //     builder.add_derives_for_type(
        //         syn::parse_quote!(phala_pallets::wapod_workers::pallet::TicketInfo),
        //         [syn::parse_quote! { Clone }],
        //         true,
        //     );
        //     builder.add_derives_for_type(
        //         syn::parse_quote!(wapod_types::ticket::Prices),
        //         [syn::parse_quote! { Default }],
        //         true,
        //     );
        //
        //     let code = builder.generate(metadata).unwrap().to_string();
        //     std::fs::write(output_filename, code).unwrap();
        //     std::process::Command::new("rustfmt")
        //         .arg(output_filename)
        //         .status()
        //         .unwrap();
        // }
        Ok(())
    }

    #[ignore] // Need to look at how we expose runtime apis in metadata v15
    #[rstest]
    #[tokio::test]
    #[traced_test]
    async fn can_submit_runtime_api_calls() -> Result<()> {
        let api = create_subxt_api().await.unwrap();
        // let runtime_api_call = storage_hub::apis()
        // let runtime_call = storage_hub::apis().metadata().metadata_versions();
        let runtime_call = subxt::dynamic::runtime_api_call(
            "Metadata",
            "metadata_versions",
            Vec::<Value<()>>::new(),
        );
        let result = api
            .runtime_api()
            .at_latest()
            .await?
            .call(runtime_call)
            .await?;
        println!("result from runtimeapi call is : {:?}", result.to_value());
        Ok(())
    }

    #[rstest]
    #[tokio::test]
    async fn can_submit_rpcs() -> Result<()> {
        let rpc_client = RpcClient::from_url("ws://localhost:9944").await?;
        let rpc = LegacyRpcMethods::<PolkadotConfig>::new(rpc_client.clone());

        println!(
            "🚀 System Name: {:?}\n🩺 Health: {:?}\n📋 Properties: {:?}\n⛓️ Chain: {:?}",
            rpc.system_name().await?,
            rpc.system_health().await?,
            rpc.system_properties().await?,
            rpc.system_chain().await?
        );

        let rpc = ChainHeadRpcMethods::<SubstrateConfig>::new(rpc_client.clone());
        let properties = rpc.chainspec_v1_properties::<ChainProperties>().await?;

        // Log raw JSON to understand its structure
        println!("Raw system properties: {:?}", properties);

        Ok(())
    }

    pub async fn create_block() -> anyhow::Result<()> {
        let response = reqwest::Client::new()
            .post("http://localhost:9944")
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "engine_createBlock",
                "params": [true, true],
                "id": 1
            }))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        println!("Response: {:?}", response);

        //add check that response is ok

        Ok(())
    }
}
