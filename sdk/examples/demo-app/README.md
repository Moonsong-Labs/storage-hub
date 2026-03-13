# StorageHub SDK Demo App

A comprehensive demonstration of all StorageHub SDK features including MSP client operations, blockchain interactions, and file management.

## Features

### Phase 1 (Completed)
- ✅ **Environment Setup**: Check and manage Docker services
- ✅ **Configuration Panel**: Configure MSP backend and blockchain connections
- ✅ **MetaMask Integration**: Connect wallet and manage network switching
- ✅ **Status Monitoring**: Real-time service health monitoring

### Phase 2 (Coming Soon)
- 🔄 **File Management**: Upload, download, and file operations
- 🔄 **MSP Operations**: Authentication, bucket browsing, file info

### Phase 3 (Coming Soon)
- 🔄 **Bucket Operations**: Create, manage, and explore buckets
- 🔄 **Blockchain Operations**: On-chain transactions and precompile calls
- 🔄 **Advanced Features**: Gas optimization, transaction monitoring

## Prerequisites

### 1. Environment Setup

Build the required Docker images:
```bash
bun run env:build
```

Start the StorageHub environment:

**Recommended: Clean Environment (Matches SDK precompiles tests):**
```bash
bun run env:start
```

**Alternative: Pre-initialized Environment (with demo data):**
```bash
bun run env:start:initialized
```

This will start:
- StorageHub blockchain node (ws://127.0.0.1:9888)
- MSP backend service (http://127.0.0.1:8080)
- PostgreSQL database for indexing

**Environment Differences:**
- **Clean Environment**: Fresh state, ALITH account properly funded, matches sdk-precompiles test setup
- **Pre-initialized Environment**: Contains demo bucket and storage request, may have balance/value proposition issues
- Auto-sealing blocks every 6 seconds

### 2. MetaMask Setup

Install MetaMask browser extension and add the StorageHub network:

- **Network Name**: StorageHub Solochain EVM
- **RPC URL**: http://127.0.0.1:9888
- **Chain ID**: 181222
- **Currency Symbol**: SH

## Development

Install dependencies:
```bash
bun install
```

Run the development server:
```bash
bun run dev
```

The demo will be available at [http://localhost:3001](http://localhost:3001).

## Usage

1. **Environment**: Verify all services are running
2. **Configuration**: Test MSP and blockchain connections
3. **Wallet**: Connect MetaMask and switch to StorageHub network
4. **Features**: Use the SDK features (file operations, buckets, blockchain)

## Architecture

- **Next.js 15** with App Router and TypeScript
- **Tailwind CSS** for styling
- **Radix UI** components for accessibility
- **StorageHub SDK** packages:
  - `@storagehub-sdk/core` - Core utilities and blockchain operations
  - `@storagehub-sdk/msp-client` - MSP backend operations
- **Viem** for Ethereum interactions

## Development Notes

- Uses port 3001 to avoid conflicts with other Next.js apps
- MetaMask-only wallet integration (no LocalWallet)
- Real-time service monitoring with auto-refresh
- Responsive design for desktop and mobile
- Type-safe with comprehensive TypeScript coverage

## Stopping the Environment

To stop the StorageHub environment:
```bash
bun run env:stop
```

Or press Ctrl+C in the terminal where `bun run env:start` is running.