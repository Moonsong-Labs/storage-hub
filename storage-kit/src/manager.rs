use crate::tasks::ResolveBlockchainChallengeRequests;
use crate::traits::SpawnableActor;
use crate::{ActorHandle, EventHandler, Port};
use anyhow::Result;
use libp2p::identity::Keypair;
use tracing::debug;

const DEFAULT_P2P_PORT: Port = 30333;

use crate::blockchain::actor::BlockchainModule;
use crate::p2p::actor::P2PModule;

#[derive(Clone)]
pub struct StorageKitManager {
    pub blockchain_module_handle: ActorHandle<BlockchainModule>,
    pub p2p_module_handle: ActorHandle<P2PModule>,
}

impl StorageKitManager {
    pub fn start_bsp_tasks(&mut self) {
        debug!("Starting BSP tasks.");
        ResolveBlockchainChallengeRequests::new(self.clone())
            .subscribe_to(&self.blockchain_module_handle)
            .start();
    }

    pub fn start_msp_tasks(&mut self) {}
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

    pub fn build(self) -> Result<ConfiguredStorageKitBuilder> {
        let identity = match self.identity {
            Some(identity) => identity,
            None => Keypair::generate_ed25519(),
        };
        let port = self.port;

        let p2p_module = P2PModule::new(identity, port)?;
        let blockchain_module = BlockchainModule::new()?;

        Ok(ConfiguredStorageKitBuilder {
            p2p_module,
            blockchain_module,
        })
    }
}

/// StorageHub client roles.
///
/// The StorageHub client can be configured to run as a user, main storage provider, or backup
/// storage provider.
///
/// Each role has different requirements and responsibilities.
#[derive(Debug, Clone, Copy)]
pub enum Role {
    /// ## User
    ///
    /// ### Requirements:
    ///
    /// None
    ///
    /// ### Responsibilities:
    ///
    /// - Send data to [`Msp`](Roles::Msp) and [`Bsp`](Roles::Bsp) nodes for storage.
    /// - Execute transactions on StorageHub through XCM calls or directly (non exhaustive list)
    ///     - Create buckets.
    ///     - Request file storage for bucket ids.
    ///     - Submitted challenges for bucket ids or file ids.
    ///     - Request bucket id or file deletion.
    User,
    /// ## Main Storage Provider
    ///
    /// ### Requirements:
    ///
    /// - Your public key used to sign transactions submitted to StorageHub must be registered as
    ///   an MSP in order to run the client with this role.
    ///
    /// ### Responsibilities:
    ///
    /// - Store whole data files for users
    /// - Serve data files to [`User`](Roles::User) (via your own service)
    /// > _It is up to you to run your own dedicated service to respond to data requests._
    /// - Submit proofs of storage to StorageHub
    Msp,
    /// ## Backup Storage Provider
    ///
    /// ### Requirements:
    ///
    /// - Your public key used to sign transactions submitted to StorageHub must be registered as
    ///   an BSP in order to run the client with this role.
    ///
    /// ### Responsibilities:
    ///
    /// - Store data chunks
    /// - Serve data chunks to [`Msp`](Roles::Msp)
    /// - Submit proofs of storage to StorageHub
    Bsp,
}

pub struct ConfiguredStorageKitBuilder {
    blockchain_module: BlockchainModule,
    p2p_module: P2PModule,
}

impl ConfiguredStorageKitBuilder {
    pub fn start(self, role: Role) -> Result<StorageKitManager> {
        let blockchain_module_handle = self.blockchain_module.spawn();
        let p2p_module_handle = self.p2p_module.spawn();

        let mut storage_kit_manager = StorageKitManager {
            blockchain_module_handle,
            p2p_module_handle,
        };

        match role {
            Role::User => {}
            Role::Msp => storage_kit_manager.start_msp_tasks(),
            Role::Bsp => storage_kit_manager.start_bsp_tasks(),
        }

        Ok(storage_kit_manager)
    }
}
