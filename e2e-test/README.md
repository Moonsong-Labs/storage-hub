# Storage Hub E2E Testing Framework

This crate provides end-to-end testing utilities for Storage Hub using subxt 0.41.0 to interact with a running node.

## Features

- Test basic connectivity and functionality
- Test Storage Hub-specific features like the file system
- Manage node lifecycle for testing
- Helper utilities for common testing operations

## Prerequisites

- A running Storage Hub node (local or remote)
- Rust toolchain

## Quick Start

### Running Tests with an External Node

If you already have a node running:

```bash
# Run all tests
cargo test -p sh-e2e

# Run specific test
cargo test -p sh-e2e test_can_connect
```

### Running Tests with CLI Tool

The package includes a CLI tool for more advanced usage:

```bash
# Build the CLI tool
cargo build -p sh-e2e

# Start a node and connect to it
cargo run -p sh-e2e -- start-node --wait-ready

# Run all tests, starting a node automatically
cargo run -p sh-e2e -- run

# Run tests against an existing node
cargo run -p sh-e2e -- run --no-node --node-url ws://127.0.0.1:9944
```

## Writing Tests

Tests are organized by functionality in the `src/tests` directory:

- `basic.rs` - Basic connectivity and functionality tests
- `file_system.rs` - Storage Hub file system tests

To add new tests:

1. Create a new file in `src/tests` if it's a new category, or add to an existing file
2. Use the provided utilities for common operations
3. Use the async test helpers from `tokio`

Example:

```rust
#[tokio::test]
async fn test_my_feature() -> Result<()> {
    let config = TestConfig::default();
    let client = create_client(&config).await?;
    
    // Test implementation...
    
    Ok(())
}
```

## Utility Functions

The framework provides several utility functions to help with testing:

- `create_client` - Create a new subxt client
- `get_keypair` - Get a testing keypair
- `wait_for_blocks` - Wait for a specific number of blocks
- `wait_for_event` - Wait for a specific event to occur

## Metadata Generation

The tests use Subxt's code generation to create a statically typed interface.

If the runtime changes, you'll need to update the metadata:

```bash
# Make sure a node is running locally
cargo run -p sh-e2e -- metadata-download --output storage-hub.scale
```

You can then use this metadata in your test code with:

```rust
#[subxt::subxt(runtime_metadata_path = "./storage-hub.scale")]
pub mod storage_hub {}
```

This approach is recommended as it doesn't require a running node during testing and ensures consistency.

Alternatively, for development purposes, you can fetch the metadata at runtime if you add the "web" feature:

```toml
subxt = { version = "0.41.0", features = ["native", "web"] }
```

```rust
#[subxt::subxt(runtime_metadata_url = "ws://127.0.0.1:9944")]
pub mod storage_hub {}
```

## Updating Subxt

To update subxt to the latest version:

1. Update the version numbers in `Cargo.toml`:
   ```toml
   subxt = { version = "0.41.0", features = ["native"] }
   subxt-signer = { version = "0.41.0" }
   ```

2. Run `cargo update -p subxt` to update the dependency.

3. If needed, update the code to handle API changes between versions:
   - The Block struct now takes two generic parameters
   - Keypair creation uses `from_secret_key` or `from_phrase` instead of `from_seed`
   - For a simple testing keypair, use `dev::alice()` from `subxt_signer::sr25519::dev`
