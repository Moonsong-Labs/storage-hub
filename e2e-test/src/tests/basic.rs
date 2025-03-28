use reqwest;
#[cfg(test)]
pub mod basic_subxt_checks {
    #![allow(missing_docs)]

    use super::*;
    use crate::create_subxt_api;
    use anyhow::Result;
    use std::str::FromStr;
    use std::time::Duration;
    use subxt::backend::legacy::LegacyRpcMethods;
    use subxt::backend::rpc::RpcClient;
    use subxt::config::polkadot::PolkadotExtrinsicParamsBuilder as Params;
    use subxt::utils::AccountId32;
    use subxt::PolkadotConfig;
    use subxt_signer::sr25519::{dev, Keypair, PublicKey};
    use subxt_signer::{sr25519, SecretUri};
    use tokio::time::sleep;
    use tracing::{debug, info};
    use tracing_test::traced_test;

    #[subxt::subxt(runtime_metadata_path = "./storage-hub.scale")]
    pub mod storage_hub {}
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

    #[tokio::test]
    #[traced_test]
    async fn can_submit_extrinsic() -> Result<()> {
        let api = create_subxt_api().await.unwrap();
        debug!("Block num: {}", api.blocks().at_latest().await?.number());
        let block_api = api.blocks().at_latest().await?;

        let tx_dest = dev::bob().public_key().into();
        let tx = storage_hub::tx()
            .balances()
            .transfer_allow_death(tx_dest, 10000000000000);

        let tx_dest: AccountId32 = dev::bob().public_key().into();
        let storage_query = storage_hub::storage().system().account(&tx_dest);
        let bal_before = api
            .storage()
            .at_latest()
            .await?
            .fetch(&storage_query)
            .await?
            .unwrap();
        debug!("Bal Before: {:?}", &bal_before);

        let alice = dev::alice();
        let start_block = block_api.header().number;
        debug!("Current block before transaction: {}", start_block);

        let tx_params = Params::new().build();

        let ext_success = api
            .tx()
            .sign_and_submit_then_watch(&tx, &alice, tx_params)
            .await?
            .wait_for_finalized_success();

        let rpc_client = RpcClient::from_url("ws://127.0.0.1:9944").await?;
        let rpc = LegacyRpcMethods::<PolkadotConfig>::new(rpc_client.clone());
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
        let new_storage_query = storage_hub::storage().system().account(&tx_dest);
        let bal_after = api
            .storage()
            .at_latest()
            .await?
            .fetch(&new_storage_query)
            .await?
            .unwrap();
        debug!("Bal After: {:?}", &bal_after);
        assert!(bal_after.data.free > bal_before.data.free);

        Ok(())
    }

    #[tokio::test]
    #[traced_test]
    async fn can_submit_rpcs() -> Result<()> {
        Ok(())
    }
}

async fn create_block() -> anyhow::Result<()> {
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
