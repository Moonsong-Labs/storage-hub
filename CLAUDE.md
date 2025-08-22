# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## StorageHub Overview

StorageHub is a Substrate-based parachain for the Polkadot ecosystem, focused on decentralized storage. It implements two types of storage providers:
- **MSP (Main Storage Providers)**: Primary data retrieval services
- **BSP (Backup Storage Providers)**: Redundancy and backup services

## Build Commands

### Rust/Node Build
```bash
# Standard build
cargo build --release

# macOS cross-build (requires zig)
pnpm i
pnpm crossbuild:mac

# Build Docker image
pnpm docker:build
```

### Tests
```bash
# Rust unit tests
cargo test

# Integration tests (requires Docker)
pnpm test:node      # Solo node tests
pnpm test:bspnet    # BSP network tests
pnpm test:fullnet   # Full network tests
pnpm test:user      # User interaction tests

# Run specific test with filter
pnpm test:node:single # with FILTER env var

# Zombienet tests
pnpm zombie:test:native
```

### Linting and Formatting
```bash
# Rust
cargo fmt --all -- --check
cargo clippy --all-targets

# JavaScript/TypeScript
pnpm lint
pnpm fmt
pnpm fmt:fix
pnpm typecheck
```

### Type Generation
```bash
# In /test directory
pnpm typegen  # Generate TypeScript types from runtime
```

## Architecture

### Core Components
- `/runtime`: StorageHub runtime implementation
- `/pallets`: Custom Substrate pallets (bucket-nfts, file-system, payment-streams, proofs-dealer, providers, randomness)
- `/node`: Parachain node implementation
- `/client`: Storage hub client modules using actors-framework
- `/primitives`: Shared types and traits
- `/test`: Comprehensive test suite

### Client Architecture
The client uses an actor-based architecture (`/client/actors-framework`) with specialized services:
- `blockchain-service`: Blockchain interaction
- `file-manager`: File operations and chunking
- `forest-manager`: Merkle tree management
- `indexer-service`: Blockchain event indexing

### Telemetry System
The telemetry system (`/client/common/src/telemetry.rs`) provides production-grade observability:
- **Bounded channels**: Prevents resource exhaustion with 10K event buffer
- **Automatic batching**: Network-efficient with 100 events or 5-second flush intervals
- **Graceful shutdown**: Ensures all events are flushed on shutdown
- **Delivery strategies**: BestEffort, Guaranteed, and Critical event handling
- **Health metrics**: Atomic counters for events sent/dropped/failed
- **Backend abstraction**: Supports Axiom and custom backends via async traits

Configuration via environment variables:
```bash
AXIOM_TOKEN=xaat-...
AXIOM_DATASET=storage-hub-prod
TELEMETRY_BUFFER_SIZE=10000  # Optional
TELEMETRY_BATCH_SIZE=100     # Optional
TELEMETRY_FLUSH_INTERVAL=5   # Optional
```

### Testing Infrastructure
- **Docker-based**: Most tests run in Docker containers for isolation
- **Zombienet**: Network topology testing
- **BSPNet**: Small dev network for file merklisation testing
- **Integration tests**: TypeScript-based using Node.js test runner

## Development Workflow

1. **Local Development**:
   ```bash
   # Start dev node
   ../target/release/storage-hub --dev
   # or with Docker
   pnpm docker:start
   ```

2. **Running Networks**:
   ```bash
   # BSPNet (small test network)
   pnpm docker:start:bspnet
   
   # Full Zombienet
   pnpm zombie:run:full:native
   pnpm zombie:setup:native
   ```

3. **Before Committing**:
   - Run `cargo fmt --all`
   - Run `cargo clippy --all-targets`
   - Run relevant tests for your changes
   - Update TypeScript types if runtime APIs changed: `pnpm typegen`

## Key Development Notes

- The project uses a monorepo structure with both Rust (Cargo workspace) and TypeScript (pnpm workspace)
- Docker is heavily used for testing to ensure consistency and isolation
- When updating RuntimeAPIs or RPC calls, update `/types-bundle/src/rpc.ts` and `/types-bundle/src/runtime.ts`
- BSP selection can be "gamed" in tests by choosing BSP IDs that match file fingerprints
- The client implements a sophisticated actor system for handling storage operations