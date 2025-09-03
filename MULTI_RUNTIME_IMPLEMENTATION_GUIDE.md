# Multi-Runtime Support Implementation Guide for Substrate Projects

This guide provides step-by-step instructions for implementing multi-runtime support in a Substrate project, allowing a single node binary to operate with different runtime configurations (e.g., mainnet, testnet, stagenet).

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Project Structure Setup](#project-structure-setup)
3. [Runtime Dependencies Configuration](#runtime-dependencies-configuration)
4. [Chain Specification Implementation](#chain-specification-implementation)
5. [Service Layer Modifications](#service-layer-modifications)
6. [Command Layer Runtime Selection](#command-layer-runtime-selection)
7. [Feature Flag Management](#feature-flag-management)
8. [Testing and Validation](#testing-and-validation)

## Prerequisites

- Existing Substrate project with a single runtime
- Understanding of Substrate's service architecture
- Familiarity with Rust generics and traits

## 1. Project Structure Setup

### Create Runtime Variants

First, organize your runtime crates to support multiple variants:

```
your-project/
├── runtime/
│   ├── common/           # Shared runtime components
│   ├── mainnet/         # Production runtime
│   ├── testnet/         # Test network runtime
│   └── stagenet/        # Development/staging runtime
└── node/                # Node implementation
```

### Update Workspace Cargo.toml

Add all runtime variants to your workspace:

```toml
[workspace]
members = [
    "runtime/common",
    "runtime/mainnet",
    "runtime/testnet",
    "runtime/stagenet",
    "node",
    # ... other members
]
```

## 2. Runtime Dependencies Configuration

### Update Node Cargo.toml

Modify your node's `Cargo.toml` to include all runtime dependencies:

```toml
[package]
name = "your-project-node"
# ... other package info

[dependencies]
# Local runtime dependencies
your-project-mainnet-runtime = { workspace = true }
your-project-testnet-runtime = { workspace = true }
your-project-stagenet-runtime = { workspace = true }
your-project-runtime-common = { workspace = true }

# ... other dependencies

[features]
default = ["std"]
std = [
    "your-project-runtime-common/std",
    "your-project-mainnet-runtime/std",
    "your-project-testnet-runtime/std",
    "your-project-stagenet-runtime/std",
]

runtime-benchmarks = [
    "frame-benchmarking-cli/runtime-benchmarks",
    "frame-system/runtime-benchmarks",
    "sc-service/runtime-benchmarks",
    "your-project-runtime-common/runtime-benchmarks",
    "your-project-mainnet-runtime/runtime-benchmarks",
    "your-project-testnet-runtime/runtime-benchmarks",
    "your-project-stagenet-runtime/runtime-benchmarks",
    "sp-runtime/runtime-benchmarks",
]

try-runtime = [
    "frame-system/try-runtime",
    "pallet-transaction-payment/try-runtime",
    "your-project-mainnet-runtime/try-runtime",
    "your-project-testnet-runtime/try-runtime",
    "your-project-stagenet-runtime/try-runtime",
    "sp-runtime/try-runtime",
]
```

## 3. Chain Specification Implementation

### Create NetworkType Trait

Create `src/chain_spec/mod.rs`:

```rust
pub mod mainnet;
pub mod testnet;
pub mod stagenet;

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec;

/// Can be called for a chain spec `Configuration` to determine the network type.
pub trait NetworkType {
    /// Returns `true` if this is a configuration for the `Mainnet` network.
    fn is_mainnet(&self) -> bool;

    /// Returns `true` if this is a configuration for the `Testnet` network.
    fn is_testnet(&self) -> bool;

    /// Returns `true` if this is a configuration for the `Stagenet` network.
    fn is_stagenet(&self) -> bool;

    /// Returns `true` if this is a configuration for a dev network.
    fn is_dev(&self) -> bool;
}

impl NetworkType for Box<dyn sc_service::ChainSpec> {
    fn is_dev(&self) -> bool {
        self.chain_type() == sc_service::ChainType::Development
    }

    fn is_mainnet(&self) -> bool {
        self.id().starts_with("your_project_mainnet")
    }

    fn is_testnet(&self) -> bool {
        self.id().starts_with("your_project_testnet")
    }

    fn is_stagenet(&self) -> bool {
        self.id().starts_with("your_project_stagenet")
    }
}
```

### Create Runtime-Specific Chain Specs

Create `src/chain_spec/mainnet.rs`:

```rust
use your_project_mainnet_runtime::WASM_BINARY;
use sc_service::ChainType;
use super::ChainSpec;

const CHAIN_ID: u64 = 1000; // Choose appropriate chain ID
const SS58_FORMAT: u16 = CHAIN_ID as u16;
const TOKEN_DECIMALS: u8 = 18;
const TOKEN_SYMBOL: &str = "TOKEN";

pub fn development_chain_spec() -> Result<ChainSpec, String> {
    let mut properties = sc_service::Properties::new();
    properties.insert("tokenSymbol".into(), TOKEN_SYMBOL.into());
    properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
    properties.insert("ss58Format".into(), SS58_FORMAT.into());

    Ok(ChainSpec::builder(
        WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?,
        None,
    )
    .with_name("Your Project Mainnet Dev")
    .with_id("your_project_mainnet_dev")
    .with_chain_type(ChainType::Development)
    .with_genesis_config_preset_name(sp_genesis_builder::DEV_RUNTIME_PRESET)
    .with_properties(properties)
    .build())
}

pub fn local_chain_spec() -> Result<ChainSpec, String> {
    let mut properties = sc_service::Properties::new();
    properties.insert("tokenSymbol".into(), TOKEN_SYMBOL.into());
    properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
    properties.insert("ss58Format".into(), SS58_FORMAT.into());

    Ok(ChainSpec::builder(
        WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?,
        None,
    )
    .with_name("Your Project Mainnet Local")
    .with_id("your_project_mainnet_local")
    .with_chain_type(ChainType::Local)
    .with_genesis_config_preset_name(sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET)
    .with_properties(properties)
    .build())
}
```

Create similar files for `testnet.rs` and `stagenet.rs` with appropriate chain IDs and configurations.

### Update CLI Chain Spec Loading

In your `command.rs` or CLI implementation, update the `load_spec` method:

```rust
impl SubstrateCli for Cli {
    // ... other methods

    fn load_spec(&self, id: &str) -> Result<Box<dyn sc_service::ChainSpec>, String> {
        Ok(match id {
            // Stagenet variants (default fallback)
            "dev" | "stagenet-dev" => Box::new(chain_spec::stagenet::development_chain_spec()?),
            "" | "local" | "stagenet-local" => Box::new(chain_spec::stagenet::local_chain_spec()?),

            // Testnet variants
            "testnet-dev" => Box::new(chain_spec::testnet::development_chain_spec()?),
            "testnet-local" => Box::new(chain_spec::testnet::local_chain_spec()?),

            // Mainnet variants
            "mainnet-dev" => Box::new(chain_spec::mainnet::development_chain_spec()?),
            "mainnet-local" => Box::new(chain_spec::mainnet::local_chain_spec()?),

            // Custom chain spec from file
            path => Box::new(chain_spec::ChainSpec::from_json_file(
                std::path::PathBuf::from(path),
            )?),
        })
    }
}
```

## 4. Service Layer Modifications

### Create Generic Service Types

Update your `service.rs` to support generic runtime types:

```rust
use crate::chain_spec::NetworkType;

// Generic client type
pub(crate) type FullClient<RuntimeApi> = sc_service::TFullClient<
    Block,
    RuntimeApi,
    sc_executor::WasmExecutor<sp_io::SubstrateHostFunctions>,
>;

// Other generic types
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;

// Runtime API trait that all runtimes must implement
pub(crate) trait FullRuntimeApi:
    sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
    + sp_api::Metadata<Block>
    + frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>
    + sp_session::SessionKeys<Block>
    + sp_api::ApiExt<Block>
    + pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
    + sp_offchain::OffchainWorkerApi<Block>
    + sp_block_builder::BlockBuilder<Block>
    + sp_consensus_babe::BabeApi<Block>
    + sp_consensus_grandpa::GrandpaApi<Block>
    // Add other required runtime APIs here
{
}

impl<T> FullRuntimeApi for T where
    T: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
        + sp_api::Metadata<Block>
        + frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>
        + sp_session::SessionKeys<Block>
        + sp_api::ApiExt<Block>
        + pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
        + sp_offchain::OffchainWorkerApi<Block>
        + sp_block_builder::BlockBuilder<Block>
        + sp_consensus_babe::BabeApi<Block>
        + sp_consensus_grandpa::GrandpaApi<Block>
        // Add other required runtime APIs here
{
}

// Generic service partial components type
pub type Service<RuntimeApi> = sc_service::PartialComponents<
    FullClient<RuntimeApi>,
    FullBackend,
    FullSelectChain,
    sc_consensus::DefaultImportQueue<Block>,
    sc_transaction_pool::BasicPool<
        sc_transaction_pool::FullChainApi<FullClient<RuntimeApi>, Block>,
        Block,
    >,
    (
        // Add your consensus components here
        // For example: GrandpaBlockImport, BabeLink, etc.
    ),
>;
```

### Create Generic Service Functions

```rust
pub fn new_partial<RuntimeApi>(
    config: &Configuration,
) -> Result<Service<RuntimeApi>, ServiceError>
where
    RuntimeApi: sp_api::ConstructRuntimeApi<Block, FullClient<RuntimeApi>> + Send + Sync + 'static,
    RuntimeApi::RuntimeApi: FullRuntimeApi,
{
    // Implementation similar to your existing new_partial but generic over RuntimeApi
    // ... consensus setup, client creation, etc.
}

pub async fn new_full<RuntimeApi>(
    config: Configuration,
) -> Result<TaskManager, ServiceError>
where
    RuntimeApi: sp_api::ConstructRuntimeApi<Block, FullClient<RuntimeApi>> + Send + Sync + 'static,
    RuntimeApi::RuntimeApi: FullRuntimeApi,
{
    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        other: (/* your consensus components */),
    } = new_partial::<RuntimeApi>(&config)?;

    // ... rest of service setup
}
```

## 5. Command Layer Runtime Selection

### Create Runtime Selection Macros

Add these macros to your `command.rs`:

```rust
macro_rules! construct_async_run {
    (|$components:ident, $cli:ident, $cmd:ident, $config:ident| $( $code:tt )* ) => {{
        let runner = $cli.create_runner($cmd)?;
        match runner.config().chain_spec {
            ref spec if spec.is_mainnet() => {
                runner.async_run(|$config| {
                    let $components = service::new_partial::<your_project_mainnet_runtime::RuntimeApi>(
                        &$config,
                    )?;
                    let task_manager = $components.task_manager;
                    { $( $code )* }.map(|v| (v, task_manager))
                })
            }
            ref spec if spec.is_testnet() => {
                runner.async_run(|$config| {
                    let $components = service::new_partial::<your_project_testnet_runtime::RuntimeApi>(
                        &$config,
                    )?;
                    let task_manager = $components.task_manager;
                    { $( $code )* }.map(|v| (v, task_manager))
                })
            }
            _ => {
                runner.async_run(|$config| {
                    let $components = service::new_partial::<your_project_stagenet_runtime::RuntimeApi>(
                        &$config,
                    )?;
                    let task_manager = $components.task_manager;
                    { $( $code )* }.map(|v| (v, task_manager))
                })
            }
        }
    }}
}

macro_rules! construct_benchmark_partials {
    ($cli:expr, $config:expr, |$partials:ident| $code:expr) => {
        match $config.chain_spec {
            ref spec if spec.is_mainnet() => {
                let $partials = service::new_partial::<your_project_mainnet_runtime::RuntimeApi>(
                    &$config,
                )?;
                $code
            }
            ref spec if spec.is_testnet() => {
                let $partials = service::new_partial::<your_project_testnet_runtime::RuntimeApi>(
                    &$config,
                )?;
                $code
            }
            _ => {
                let $partials = service::new_partial::<your_project_stagenet_runtime::RuntimeApi>(
                    &$config,
                )?;
                $code
            }
        }
    };
}
```

### Update Command Handling

Replace your existing command handling with runtime-aware versions:

```rust
pub fn run() -> sc_cli::Result<()> {
    let cli = Cli::from_args();

    match &cli.subcommand {
        Some(Subcommand::Key(cmd)) => cmd.run(&cli),
        Some(Subcommand::BuildSpec(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
        }
        Some(Subcommand::CheckBlock(cmd)) => {
            construct_async_run!(|components, cli, cmd, config| {
                Ok(cmd.run(components.client, components.import_queue))
            })
        }
        // ... other subcommands using construct_async_run!

        None => {
            let runner = cli.create_runner(&cli.run)?;
            runner.run_node_until_exit(|config| async move {
                match config.chain_spec {
                    ref spec if spec.is_mainnet() => {
                        service::new_full::<your_project_mainnet_runtime::RuntimeApi>(config).await
                    }
                    ref spec if spec.is_testnet() => {
                        service::new_full::<your_project_testnet_runtime::RuntimeApi>(config).await
                    }
                    _ => {
                        service::new_full::<your_project_stagenet_runtime::RuntimeApi>(config).await
                    }
                }
                .map_err(sc_cli::Error::Service)
            })
        }
    }
}
```

## 6. Feature Flag Management

### Ensure Consistent Feature Propagation

Make sure all runtime-specific features are consistently propagated:

```toml
[features]
default = ["std"]

std = [
    "your-project-runtime-common/std",
    "your-project-mainnet-runtime/std",
    "your-project-testnet-runtime/std",
    "your-project-stagenet-runtime/std",
    # ... other std features
]

runtime-benchmarks = [
    "your-project-runtime-common/runtime-benchmarks",
    "your-project-mainnet-runtime/runtime-benchmarks",
    "your-project-testnet-runtime/runtime-benchmarks",
    "your-project-stagenet-runtime/runtime-benchmarks",
    # ... other benchmark features
]

try-runtime = [
    "your-project-mainnet-runtime/try-runtime",
    "your-project-testnet-runtime/try-runtime",
    "your-project-stagenet-runtime/try-runtime",
    # ... other try-runtime features
]

# Add any runtime-specific features
mainnet-features = ["your-project-mainnet-runtime/specific-feature"]
testnet-features = ["your-project-testnet-runtime/test-feature"]
```

## 7. Testing and Validation

### Test Runtime Selection

Create integration tests to verify runtime selection works correctly:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mainnet_chain_spec_detection() {
        let spec = chain_spec::mainnet::development_chain_spec().unwrap();
        assert!(spec.is_mainnet());
        assert!(!spec.is_testnet());
        assert!(!spec.is_stagenet());
    }

    #[test]
    fn test_chain_spec_loading() {
        let cli = Cli::from_args();

        // Test mainnet loading
        let mainnet_spec = cli.load_spec("mainnet-dev").unwrap();
        assert!(mainnet_spec.is_mainnet());

        // Test testnet loading
        let testnet_spec = cli.load_spec("testnet-dev").unwrap();
        assert!(testnet_spec.is_testnet());

        // Test stagenet loading (default)
        let stagenet_spec = cli.load_spec("dev").unwrap();
        assert!(stagenet_spec.is_stagenet());
    }
}
```

### Command Line Testing

Test the node with different chain specifications:

```bash
# Test different runtime selections
./target/release/your-project-node --chain mainnet-dev --tmp
./target/release/your-project-node --chain testnet-dev --tmp
./target/release/your-project-node --chain stagenet-dev --tmp
```

## 8. Additional Considerations

### Runtime Upgrade Compatibility

Ensure runtime upgrade paths are compatible between different network types:

```rust
// In your runtime lib.rs files
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("your-project-mainnet"), // Different per runtime
    impl_name: create_runtime_str!("your-project-mainnet"),
    authoring_version: 1,
    spec_version: 100,        // Coordinate versions across runtimes
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1,
    system_version: 1,
};
```

### Documentation

Document the available chain specifications and their purposes:

```markdown
## Available Chain Specifications

- `mainnet-dev`: Development version of mainnet runtime
- `mainnet-local`: Local testing version of mainnet runtime
- `testnet-dev`: Development version of testnet runtime
- `testnet-local`: Local testing version of testnet runtime
- `stagenet-dev`: Development version of stagenet runtime (default)
- `stagenet-local`: Local testing version of stagenet runtime
```

### Error Handling

Add proper error handling for runtime-specific operations:

```rust
// In your command.rs
fn ensure_runtime_compatibility(config: &Configuration) -> Result<(), sc_cli::Error> {
    match config.chain_spec {
        ref spec if spec.is_mainnet() => {
            // Mainnet-specific validation
        }
        ref spec if spec.is_testnet() => {
            // Testnet-specific validation
        }
        _ => {
            // Stagenet-specific validation
        }
    }
    Ok(())
}
```

## Summary

This implementation provides:

1. **Single Binary**: One executable supports multiple runtime configurations
2. **Type Safety**: Compile-time guarantees for runtime compatibility
3. **Flexible Configuration**: Chain specifications determine runtime selection
4. **Shared Infrastructure**: Common networking, consensus, and service layers
5. **Modular Features**: Runtime-specific features can be enabled conditionally

The architecture allows you to maintain separate runtime logic while sharing the node infrastructure, making it easier to manage different network environments with specific requirements.
