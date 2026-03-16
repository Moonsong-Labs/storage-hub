# StorageHub SDK Demo - Quick Launch

## Prerequisites
- Docker installed and running
- Bun installed
- MetaMask browser extension

## Launch Commands

```bash
# Navigate to demo app
cd sdk/examples/demo-app

# Install dependencies
bun install

# 1. Build StorageHub environment (first time only)
bun run env:build

# 2. Start StorageHub services (in separate terminal)
bun run env:start

# 3. Run the demo app (in another terminal)
bun run dev
```

## Access
- **Demo App**: http://localhost:3001
- **Blockchain RPC**: ws://127.0.0.1:9888
- **MSP Backend**: http://127.0.0.1:8080

## Stop Environment
```bash
bun run env:stop
```
Or press `Ctrl+C` in the terminal running `bun run env:start`

## MetaMask Network Setup
The demo will automatically add and switch to the StorageHub network when you connect your wallet.

**Network Details** (added automatically):
- **Network Name**: StorageHub Solochain EVM
- **RPC URL**: http://127.0.0.1:9888
- **Chain ID**: 181222
- **Currency Symbol**: SH

**Manual Setup** (optional): You can also add the network manually in MetaMask if preferred.
