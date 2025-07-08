# MSP ID Detection Logic Analysis

## Overview
The MSP ID detection logic in `sync_provider_id` function (blockchain-service/src/utils.rs:380) is responsible for:
1. Detecting provider IDs linked to BCSV keys in the node's keystore
2. Managing the transition between different provider types (BSP/MSP)
3. Ensuring only one provider is managed per node

## Current Implementation

### Dependencies
The function uses the following key dependencies:

1. **From shc-common (already available):**
   - `BCSV_KEY_TYPE` - KeyTypeId constant
   - `StorageProviderId` - Enum type for provider IDs
   - `MainStorageProviderId` / `BackupStorageProviderId` - Type aliases

2. **From blockchain-service specific types:**
   - `ManagedProvider` - Enum to track managed provider state
   - `BspHandler` / `MspHandler` - Handler structs for provider-specific logic

3. **Runtime API:**
   - `StorageProvidersApi::get_storage_provider_id()` - To query provider ID for a given key

4. **Core dependencies:**
   - `KeystorePtr` - To query BCSV keys
   - `ParachainClient` - To make runtime API calls

### Core Logic Flow
1. Query all BCSV keys from the keystore
2. For each key, call runtime API to get associated provider ID
3. Handle three cases:
   - No provider ID found (expected during startup)
   - Multiple provider IDs found (panic - not supported)
   - Exactly one provider ID found (normal operation)
4. Update managed provider state based on transitions

## Extraction Analysis

### What CAN be extracted to shc-common:
1. The core logic to query provider IDs from BCSV keys
2. The validation that ensures only one provider ID is managed
3. A utility function that returns `Option<StorageProviderId>` 

### What CANNOT be extracted:
1. The `ManagedProvider` state management (blockchain-service specific)
2. The provider transition logic (BSP→MSP, MSP→BSP, etc.)
3. The handler instantiation (`BspHandler::new`, `MspHandler::new`)

### Proposed Solution

Create a new utility function in `shc-common/src/blockchain_utils.rs`:

```rust
/// Get the Provider ID linked to BCSV keys in the keystore.
/// 
/// Returns None if no provider ID is found.
/// Panics if multiple provider IDs are found (not supported).
pub fn get_provider_id_from_keystore<RuntimeApi>(
    client: &Arc<ParachainClient<RuntimeApi>>,
    keystore: &KeystorePtr,
    block_hash: &H256,
) -> Result<Option<StorageProviderId>, GetProviderIdError>
where
    RuntimeApi::RuntimeApi: StorageProvidersApi<...>,
{
    // Core logic extracted from sync_provider_id
}
```

### Benefits of Extraction:
1. **Reusability**: Other services can detect MSP/BSP IDs without blockchain-service dependency
2. **Separation of Concerns**: Core detection logic separated from state management
3. **Testing**: Easier to unit test the detection logic in isolation
4. **Consistency**: Single source of truth for provider ID detection

### Implementation Steps:
1. Create the utility function in shc-common
2. Refactor `sync_provider_id` to use the new utility
3. Keep the state management logic in blockchain-service
4. Update tests to cover both the utility and the state management

### No Blockers Identified:
All required types and traits are already available in shc-common or can be passed as parameters.