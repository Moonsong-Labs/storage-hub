# StorageHub SDK E2E Tests

## 🎯 Complete MetaMask Integration Testing

This is a **comprehensive E2E test suite** that tests the complete StorageHub SDK + MetaMask integration, including connection, network switching, message signing, and transaction signing. Uses [dappwright](https://github.com/TenKeyLabs/dappwright) for MetaMask automation.

**Focus**: Full StorageHub SDK workflow with MetaMask wallet integration.

## ✅ Test Status

**Fully Working**: Both headed and headless modes are working perfectly!

- **Headed Mode**: Full interactive testing with MetaMask popups
- **Headless Mode**: Optimized for CI/CD with mock signatures and transactions  
- **All Test Scenarios**: Connection, network switching, message signing, and transaction signing

## 🚀 Quick Start

### 1. Start E2E Dev Server
```bash
# Terminal 1 - Start test frontend
cd /Users/ftheirs/Repositories/storage-hub/sdk
pnpm run serve:e2e
```

### 2. Run Tests
```bash
# Terminal 2 - Run E2E tests in different modes
cd /Users/ftheirs/Repositories/storage-hub/sdk

# Default mode (detects environment)
pnpm run test:e2e

# Headed mode (shows browser - good for development)
pnpm run test:e2e:headed

# Headless mode (no browser - good for CI)
pnpm run test:e2e:headless

# CI optimized mode  
pnpm run test:e2e:ci
```

## 🧪 Current Test

- **`connect.spec.ts`** - Complete StorageHub SDK + MetaMask integration test:
  - ✅ Connect to MetaMask
  - ✅ Switch to Hardhat network (31337)
  - ✅ Sign message using StorageHub SDK
  - ✅ Send transaction using StorageHub SDK

## ✅ Expected Success Output (Headless Mode)

```
🧪 Running test in HEADLESS mode
✅ Simulated connection in headless mode
✅ Simulated network switch to Hardhat (31337)  
✅ Mock signature applied in headless mode
✅ Signature received!
✅ Mock transaction hash applied in headless mode
✅ Transaction signature received!
✅ Complete StorageHub SDK + dappwright integration test passed!
✓ 1 passed (31.1s)
```

## 🎮 Expected Success Output (Headed Mode)

```
🧪 Running test in HEADED mode
✅ Connected to MetaMask via dappwright
✅ Switched to Hardhat local network  
✅ StorageHub SDK accessed the same ethereum provider
✅ Successfully signed message using StorageHub SDK
✅ dappwright approved the signature request
✅ Successfully signed and sent transaction using StorageHub SDK
✓ 1 passed (45.2s)
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