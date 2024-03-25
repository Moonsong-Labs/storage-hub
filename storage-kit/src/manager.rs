use crate::traits::SpawnableActor;
use crate::{ActorHandle, Port};
use anyhow::Result;
use libp2p::identity::Keypair;

const DEFAULT_P2P_PORT: Port = 30333;

use crate::blockchain::actor::BlockchainModule;
use crate::p2p::actor::P2PModule;

pub struct StorageKitManager {
    pub blockchain_module_handle: ActorHandle<BlockchainModule>,
    pub p2p_module_handle: ActorHandle<P2PModule>,
}

impl StorageKitManager {
    pub fn start_as_bsp(&mut self) {
        // self.register_sp_check_storage_proof_requests_task();
        // self.register_file_transfer_task();
    }
}

pub struct StorageKitBuilder {
    port: Port,
    identity: Option<Keypair>,
}

impl StorageKitBuilder {
    pub fn new() -> Self {
        Self {
            port: DEFAULT_P2P_PORT,
            identity: None,
        }
    }

    pub fn with_port(port: Port) -> Self {
        Self {
            port,
            identity: None,
        }
    }

    pub fn with_identity(identity: Keypair) -> Self {
        Self {
            port: DEFAULT_P2P_PORT,
            identity: Some(identity),
        }
    }

    pub fn start(self) -> Result<StorageKitManager> {
        let identity = match self.identity {
            Some(identity) => identity,
            None => Keypair::generate_ed25519(),
        };
        let port = self.port;

        let p2p = P2PModule::new(identity, port)?;
        let blockchain = BlockchainModule::new()?;

        let p2p_module_handle = p2p.spawn();
        let blockchain_module_handle = blockchain.spawn();

        Ok(StorageKitManager {
            p2p_module_handle,
            blockchain_module_handle,
        })
    }
}
