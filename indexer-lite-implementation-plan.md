# Implementation Plan: Indexer-Lite Mode

## Overview

Implement an `indexer-lite` mode that only indexes file system events relevant to the MSP running the indexer, while retaining all current functionality in an `indexer-full` mode controlled by a runtime configuration flag.

## Prerequisites

- [ ] Existing indexer service at `/client/indexer-service`
- [ ] Existing indexer database at `/client/indexer-db`
- [ ] CLI flag `--indexer` already exists to enable/disable indexer
- [ ] Database URL configuration via `--database-url` or `DATABASE_URL` env var

## Steps

### 1. Add Indexer Mode Configuration

- **File**: `/node/src/cli.rs`
- **Operation**: Add new enum and field to `IndexerConfigurations` struct after line 469
- **Details**:
  ```rust
  #[derive(Debug, Clone, Copy, Parser, Deserialize)]
  pub enum IndexerMode {
      #[serde(rename = "full")]
      Full,
      #[serde(rename = "lite")]
      Lite,
  }
  
  impl Default for IndexerMode {
      fn default() -> Self {
          Self::Full
      }
  }
  
  // In IndexerConfigurations struct, add after line 469:
  /// Indexer mode: 'full' indexes all events, 'lite' only indexes events for the current MSP
  #[arg(long, value_enum, default_value = "full")]
  pub indexer_mode: IndexerMode,
  ```
- **Success**: Code compiles without errors

### 2. Pass Indexer Mode to Service Spawning

- **File**: `/node/src/service.rs`
- **Operation**: Update `spawn_indexer_service` call at lines 1305-1312
- **Details**:
  - Import `IndexerMode` at top of file
  - Pass `indexer_config.indexer_mode` to spawn function:
  ```rust
  spawn_indexer_service(
      &task_spawner,
      client.clone(),
      maybe_db_pool.clone().expect(
          "Indexer is enabled but no database URL is provided (via CLI using --database-url or setting DATABASE_URL environment variable)",
      ),
      indexer_config.indexer_mode,
  )
  .await;
  ```
- **Success**: Function call updated with new parameter

### 3. Update Indexer Service Spawn Function

- **File**: `/client/indexer-service/src/lib.rs`
- **Operation**: Add indexer_mode parameter to spawn function at lines 12-26
- **Details**:
  ```rust
  pub async fn spawn_indexer_service<
      RuntimeApi: StorageEnableRuntimeApi<RuntimeApi: StorageEnableApiCollection>,
  >(
      task_spawner: &TaskSpawner,
      client: Arc<ParachainClient<RuntimeApi>>,
      db_pool: DbPool,
      indexer_mode: IndexerMode,
  ) -> ActorHandle<IndexerService<RuntimeApi>> {
      let task_spawner = task_spawner
          .with_name("indexer-service")
          .with_group("network");
  
      let indexer_service = IndexerService::new(client, db_pool, indexer_mode);
  
      task_spawner.spawn_actor(indexer_service)
  }
  ```
- **Success**: Function signature updated

### 4. Extract Provider ID Detection to Shared Utility

- **File**: `/client/common/src/blockchain_utils.rs`
- **Operation**: Add new utility function for provider ID detection
- **Details**:
  ```rust
  use shc_common::types::{StorageProviderId, BCSV_KEY_TYPE};
  
  /// Get the Provider ID linked to BCSV keys in the keystore.
  /// Returns None if no provider ID is found.
  /// Returns error if multiple provider IDs are found (not supported).
  pub fn get_provider_id_from_keystore<RuntimeApi>(
      client: &Arc<ParachainClient<RuntimeApi>>,
      keystore: &KeystorePtr,
      block_hash: &H256,
  ) -> Result<Option<StorageProviderId>, GetProviderIdError>
  where
      RuntimeApi: StorageEnableRuntimeApi,
      RuntimeApi::RuntimeApi: StorageProvidersApi<Block, AccountId, StorageProviderId>,
  {
      let mut provider_ids_found = Vec::new();
      
      for key in keystore.sr25519_public_keys(BCSV_KEY_TYPE) {
          let maybe_provider_id = client
              .runtime_api()
              .get_storage_provider_id(*block_hash, &key.into())
              .map_err(|e| GetProviderIdError::RuntimeApiError(e.into()))?;
              
          if let Some(provider_id) = maybe_provider_id {
              provider_ids_found.push(provider_id);
          }
      }
      
      match provider_ids_found.len() {
          0 => Ok(None),
          1 => Ok(Some(provider_ids_found[0])),
          _ => Err(GetProviderIdError::MultipleProvidersNotSupported),
      }
  }
  
  #[derive(Debug, thiserror::Error)]
  pub enum GetProviderIdError {
      #[error("Runtime API error: {0}")]
      RuntimeApiError(anyhow::Error),
      #[error("Multiple providers in one node is not supported")]
      MultipleProvidersNotSupported,
  }
  ```
- **Success**: Shared utility function available for use

### 5. Add MSP ID Detection to Indexer Service

- **File**: `/client/indexer-service/src/handler.rs`
- **Operation**: Add fields and initialization logic to IndexerService struct
- **Details**:
  - Add imports after line 8:
  ```rust
  use sp_keystore::KeystorePtr;
  use node_cli::IndexerMode;
  use shc_common::blockchain_utils::get_provider_id_from_keystore;
  ```
  - Update struct definition after line 31:
  ```rust
  pub struct IndexerService<RuntimeApi> {
      client: Arc<ParachainClient<RuntimeApi>>,
      db_pool: DbPool,
      indexer_mode: IndexerMode,
      keystore: KeystorePtr,
      maybe_msp_id: Option<MainStorageProviderId>,
  }
  ```
  - Update new() method at lines 69-71:
  ```rust
  pub fn new(
      client: Arc<ParachainClient<RuntimeApi>>, 
      db_pool: DbPool, 
      indexer_mode: IndexerMode,
      keystore: KeystorePtr,
  ) -> Self {
      Self { 
          client, 
          db_pool, 
          indexer_mode, 
          keystore,
          maybe_msp_id: None 
      }
  }
  ```
- **Success**: Service struct updated with mode configuration

### 6. Add Keystore Support for Spawn Function

- **File**: `/client/indexer-service/src/lib.rs`
- **Operation**: Add keystore parameter to spawn function
- **Details**:
  ```rust
  pub async fn spawn_indexer_service<
      RuntimeApi: StorageEnableRuntimeApi<RuntimeApi: StorageEnableApiCollection>,
  >(
      task_spawner: &TaskSpawner,
      client: Arc<ParachainClient<RuntimeApi>>,
      db_pool: DbPool,
      indexer_mode: IndexerMode,
      keystore: KeystorePtr,
  ) -> ActorHandle<IndexerService<RuntimeApi>> {
      let task_spawner = task_spawner
          .with_name("indexer-service")
          .with_group("network");
  
      let indexer_service = IndexerService::new(client, db_pool, indexer_mode, keystore);
  
      task_spawner.spawn_actor(indexer_service)
  }
  ```
- **Success**: Keystore parameter added to spawn function

### 7. Implement MSP ID Synchronization Using Shared Utility

- **File**: `/client/indexer-service/src/handler.rs`
- **Operation**: Add method to detect current MSP ID using shared utility
- **Details**:
  ```rust
  async fn sync_msp_id(&mut self, block_hash: H256) -> Result<(), HandleFinalityNotificationError> {
      // Only sync if in lite mode and not already synced
      if matches!(self.indexer_mode, IndexerMode::Full) || self.maybe_msp_id.is_some() {
          return Ok(());
      }
      
      // Use shared utility to get provider ID
      match get_provider_id_from_keystore(&self.client, &self.keystore, &block_hash) {
          Ok(Some(StorageProviderId::MainStorageProvider(msp_id))) => {
              info!(target: LOG_TARGET, "Detected MSP ID for lite mode: {:?}", msp_id);
              self.maybe_msp_id = Some(msp_id);
              Ok(())
          }
          Ok(Some(StorageProviderId::BackupStorageProvider(_))) => {
              warn!(target: LOG_TARGET, "BSP detected in lite mode, but lite mode only supports MSPs");
              Ok(())
          }
          Ok(None) => {
              warn!(target: LOG_TARGET, "No MSP ID found for current node in lite mode");
              Ok(())
          }
          Err(e) => {
              error!(target: LOG_TARGET, "Error detecting provider ID: {:?}", e);
              Err(HandleFinalityNotificationError::ClientError(e.into()))
          }
      }
  }
  ```
- **Success**: MSP detection implemented using shared utility

### 8. Update Blockchain Service to Use Shared Utility

- **File**: `/client/blockchain-service/src/utils.rs`
- **Operation**: Refactor `sync_provider_id` to use shared utility (lines 380-464)
- **Details**:
  - Import the shared utility: `use shc_common::blockchain_utils::get_provider_id_from_keystore;`
  - Replace detection logic with call to shared utility
  - Keep state management and provider transition logic intact
  ```rust
  pub(crate) fn sync_provider_id(&mut self, block_hash: &H256) {
      match get_provider_id_from_keystore(&self.client, &self.keystore, block_hash) {
          Ok(Some(provider_id)) => {
              // Keep existing state management logic for provider transitions
              match (&self.maybe_managed_provider, provider_id) {
                  // ... existing transition logic ...
              }
          }
          Ok(None) => {
              warn!(target: LOG_TARGET, "🔑 No Provider ID linked to BCSV keys");
          }
          Err(e) => {
              error!(target: LOG_TARGET, "Error detecting provider ID: {:?}", e);
          }
      }
  }
  ```
- **Success**: Blockchain service refactored to use shared utility

### 9. Update handle_finality_notification to Sync MSP ID

- **File**: `/client/indexer-service/src/handler.rs`
- **Operation**: Add sync_msp_id call in handle_finality_notification at line 88
- **Details**:
  ```rust
  async fn handle_finality_notification<Block>(
      &mut self,
      notification: sc_client_api::FinalityNotification<Block>,
  ) -> Result<(), HandleFinalityNotificationError>
  where
      Block: sp_runtime::traits::Block,
      Block::Header: Header<Number = BlockNumber>,
  {
      let finalized_block_hash = notification.hash;
      let finalized_block_number = *notification.header.number();

      info!(target: LOG_TARGET, "Finality notification (#{}): {}", finalized_block_number, finalized_block_hash);

      // Sync MSP ID if in lite mode
      self.sync_msp_id(finalized_block_hash).await?;

      let mut db_conn = self.db_pool.get().await?;
      // ... rest of existing code
  }
  ```
- **Success**: MSP ID sync integrated

### 10. Filter File System Events in Lite Mode

- **File**: `/client/indexer-service/src/handler.rs`
- **Operation**: Modify index_file_system_event method starting at line 187
- **Details**:
  - Add filtering logic at the beginning of the method:
  ```rust
  async fn index_file_system_event<'a, 'b: 'a>(
      &'b self,
      conn: &mut DbConnection<'a>,
      event: &pallet_file_system::Event<storage_hub_runtime::Runtime>,
  ) -> Result<(), diesel::result::Error> {
      // In lite mode, only process events for our MSP
      if matches!(self.indexer_mode, IndexerMode::Lite) {
          if let Some(our_msp_id) = &self.maybe_msp_id {
              // Check if event involves our MSP
              let involves_our_msp = match event {
                  pallet_file_system::Event::NewBucket { msp_id, .. } => msp_id == our_msp_id,
                  pallet_file_system::Event::MoveBucketAccepted { new_msp_id, .. } => new_msp_id == our_msp_id,
                  pallet_file_system::Event::BucketPrivacyUpdated { bucket_id, .. } => {
                      // Check if bucket belongs to our MSP
                      if let Ok(bucket) = Bucket::get_by_onchain_bucket_id(conn, bucket_id.as_ref().to_vec()).await {
                          if let Some(msp) = bucket.msp_id {
                              let msp_record = Msp::get(conn, msp).await?;
                              &msp_record.onchain_msp_id == our_msp_id.to_string()
                          } else {
                              false
                          }
                      } else {
                          false
                      }
                  }
                  // Add similar checks for other relevant events
                  _ => false,
              };
              
              if !involves_our_msp {
                  return Ok(());
              }
          }
      }
      
      // Continue with existing event processing...
      match event {
          // existing match arms...
      }
  }
  ```
- **Success**: Event filtering implemented

### 11. Filter Provider Events in Lite Mode

- **File**: `/client/indexer-service/src/handler.rs`
- **Operation**: Add filtering to index_providers_event method at line 463
- **Details**:
  ```rust
  async fn index_providers_event<'a, 'b: 'a>(
      &'b self,
      conn: &mut DbConnection<'a>,
      event: &pallet_storage_providers::Event<storage_hub_runtime::Runtime>,
      block_hash: H256,
  ) -> Result<(), diesel::result::Error> {
      // In lite mode, only index MSP events for our MSP
      if matches!(self.indexer_mode, IndexerMode::Lite) {
          if let Some(our_msp_id) = &self.maybe_msp_id {
              let involves_our_msp = match event {
                  pallet_storage_providers::Event::MspSignUpSuccess { msp_id, .. } => msp_id == our_msp_id,
                  pallet_storage_providers::Event::MspSignOffSuccess { msp_id, .. } => msp_id == our_msp_id,
                  pallet_storage_providers::Event::CapacityChanged { provider_id, .. } => {
                      if let StorageProviderId::MainStorageProvider(msp_id) = provider_id {
                          msp_id == our_msp_id
                      } else {
                          false
                      }
                  }
                  // Skip BSP events entirely in lite mode
                  pallet_storage_providers::Event::BspSignUpSuccess { .. } |
                  pallet_storage_providers::Event::BspSignOffSuccess { .. } |
                  pallet_storage_providers::Event::BspDeleted { .. } => return Ok(()),
                  _ => false,
              };
              
              if !involves_our_msp {
                  return Ok(());
              }
          }
      }
      
      // Continue with existing processing...
      match event {
          // existing match arms...
      }
  }
  ```
- **Success**: Provider event filtering added

### 12. Update Node Service to Pass Keystore

- **File**: `/node/src/service.rs`
- **Operation**: Update spawn_indexer_service call to include keystore
- **Details**:
  ```rust
  spawn_indexer_service(
      &task_spawner,
      client.clone(),
      maybe_db_pool.clone().expect(
          "Indexer is enabled but no database URL is provided (via CLI using --database-url or setting DATABASE_URL environment variable)",
      ),
      indexer_config.indexer_mode,
      keystore.clone(),
  )
  .await;
  ```
- **Success**: Keystore passed to indexer service

### 13. Add Database Query Methods for User Files/Buckets

- **File**: `/client/indexer-db/src/models/file.rs`
- **Operation**: Add method to query files by user and MSP after line 193
- **Details**:
  ```rust
  impl File {
      pub async fn get_by_user_and_msp<'a>(
          conn: &mut DbConnection<'a>,
          user_account: Vec<u8>,
          msp_id: i64,
      ) -> Result<Vec<Self>, diesel::result::Error> {
          let files = file::table
              .inner_join(bucket::table.on(file::bucket_id.eq(bucket::id)))
              .filter(file::account.eq(user_account))
              .filter(bucket::msp_id.eq(msp_id))
              .select(File::as_select())
              .load(conn)
              .await?;
          Ok(files)
      }
  }
  ```
- **Success**: Query method added

### 14. Add Bucket Query Method by User and MSP

- **File**: `/client/indexer-db/src/models/bucket.rs`
- **Operation**: Add query method for user buckets
- **Details**:
  ```rust
  impl Bucket {
      pub async fn get_by_user_and_msp<'a>(
          conn: &mut DbConnection<'a>,
          user_account: String,
          msp_id: i64,
      ) -> Result<Vec<Self>, diesel::result::Error> {
          let buckets = bucket::table
              .filter(bucket::user.eq(user_account))
              .filter(bucket::msp_id.eq(msp_id))
              .load(conn)
              .await?;
          Ok(buckets)
      }
  }
  ```
- **Success**: Bucket query method added

## Testing Strategy

- [ ] Unit test IndexerMode parsing in CLI
- [ ] Integration test with indexer in full mode (default behavior unchanged)
- [ ] Integration test with indexer in lite mode filtering events correctly
- [ ] Test MSP ID detection from keystore
- [ ] Test database queries return only current MSP's data in lite mode

## Rollback Plan

Remove IndexerMode enum and all conditional logic, reverting to original behavior. The changes are additive and backward-compatible, so existing deployments will continue working in full mode by default.

## Usage Examples

### Running Indexer in Full Mode (Default)
```bash
storage-hub --indexer --database-url postgresql://user:pass@localhost/db
# OR explicitly:
storage-hub --indexer --indexer-mode full --database-url postgresql://user:pass@localhost/db
```

### Running Indexer in Lite Mode
```bash
storage-hub --indexer --indexer-mode lite --database-url postgresql://user:pass@localhost/db
```

## Benefits of Lite Mode

1. **Reduced Database Size**: Only stores data relevant to the current MSP
2. **Better Performance**: Fewer events to process and index
3. **Privacy**: MSP only has data about its own users and files
4. **Focused Queries**: Database optimized for single MSP queries

## Implementation Notes

- The lite mode requires the node to be running as an MSP (not a BSP)
- The MSP must be registered on-chain before lite mode can function
- If no MSP is detected, lite mode will log a warning and effectively index nothing
- Full mode remains the default to ensure backward compatibility