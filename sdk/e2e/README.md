# StorageHub SDK E2E Tests

## 🎯 Simple MetaMask Connection Testing

This is a **basic E2E test** that connects to MetaMask and demonstrates wallet functionality. Following the [dappwright example pattern](https://github.com/TenKeyLabs/dappwright).

**Focus**: Just connect to MetaMask and verify basic functionality works.

## 🚀 Quick Start

### 1. Start E2E Dev Server
```bash
# Terminal 1 - Start test frontend
cd /Users/ftheirs/Repositories/storage-hub/sdk
pnpm run serve:e2e
```

### 2. Run Tests
```bash
# Terminal 2 - Run E2E tests
cd /Users/ftheirs/Repositories/storage-hub/sdk
pnpm run test:e2e
```

## 🧪 Current Test

- **`basic.spec.ts`** - Basic MetaMask connection + network info logging

## ✅ Expected Success Output

```
🚀 Setting up MetaMask with dappwright...
✅ MetaMask bootstrap complete
🎬 Starting basic MetaMask connection test...
📡 Network: Chain ID 1 (0x1)
ℹ️  Connected to different network (that's okay for basic testing)
💡 To test with Anvil: run `pnpm run node:e2e` first
✅ Basic wallet connection test passed!
```

## 💡 Optional: Local Anvil Testing

If you want to test with a local blockchain:

```bash
# Terminal 1 - Start Anvil (optional)
pnpm run node:e2e
# Wait for: "Listening on 127.0.0.1:8545"

# Terminal 2 - Start frontend
pnpm run serve:e2e

# Terminal 3 - Run tests (will detect Anvil)
pnpm run test:e2e
```

## 🔧 Commands Reference

| Command | Purpose |
|---------|---------|
| `pnpm run serve:e2e` | Start test frontend server |
| `pnpm run test:e2e` | Run E2E tests |
| `pnpm run node:e2e` | (Optional) Start Anvil blockchain |

## 📝 Notes

- Tests work on **any network** (mainnet, testnets, local)
- **No network requirements** - just tests basic connection
- Follows **dappwright best practices**
- One simple test to verify MetaMask integration works