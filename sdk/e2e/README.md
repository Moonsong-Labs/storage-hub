# StorageHub E2E Tests with dAppwright

This directory contains end-to-end tests for StorageHub using **dAppwright** for automated MetaMask integration.

## 🚀 Features

- ✅ **Pure dAppwright integration** - No SDK dependencies
- ✅ **Automated MetaMask setup** - Extension installation & wallet import
- ✅ **Dual execution modes** - Both headed and headless
- ✅ **Real network switching** - Anvil testnet configuration
- ✅ **Message signing** - "StorageHub rocks!" signature test
- ✅ **Clean & extensible** - Foundation for future E2E tests

## 📁 Project Structure

```
sdk/e2e/
├── package.json                          # dAppwright + Playwright dependencies
├── playwright.config.ts                  # Test configuration
├── README.md                            # This file
├── page/
│   └── index.html                       # Simple test dApp (pure HTML/JS)
└── tests/
    └── dappwright-hello-world.spec.ts   # Main E2E test
```

## 🛠️ Setup

1. **Install dependencies:**
   ```bash
   cd sdk/e2e
   npm install
   npx playwright install chromium
   ```

2. **Start Anvil (required):**
   ```bash
   # In a separate terminal
   anvil
   ```
   This should start on `http://127.0.0.1:8545` with chain ID `31337`.

## 🧪 Running Tests

### **The Two Commands You Requested:**

#### **Headed Mode (Interactive - Development)**
```bash
npm run test:headed
```
- Opens browser window
- See MetaMask extension in action
- Great for debugging and development
- Interactive MetaMask popups

#### **Headless Mode (Automated - CI)**
```bash
npm run test:headless
```
- Runs in background
- Perfect for CI/CD pipelines
- Fully automated
- No GUI required

### **Additional Commands:**

```bash
# Default test (headless)
npm test

# Interactive UI mode
npm run test:ui

# Debug mode (step through tests)
npm run test:debug

# Start dev server only
npm run dev
```

## 🔧 How It Works

### **1. dAppwright Magic**
- Automatically downloads and installs MetaMask extension
- Imports test wallet with seed: `"test test test test test test test test test test test junk"`
- Manages extension lifecycle and wallet state

### **2. Test Flow**
1. 🦊 Load MetaMask extension with test seed
2. 🌐 Add Anvil network (localhost:8545, Chain ID: 31337)
3. 🔀 Switch to Anvil network
4. 🌍 Open test dApp (pure HTML/JS)
5. 🔗 Connect MetaMask to dApp
6. ✍️ Sign "StorageHub rocks!" message
7. ✅ Verify signature and wallet address

### **3. Test dApp**
- **Pure HTML/JavaScript** - No framework dependencies
- **Direct `window.ethereum` usage** - Real MetaMask integration
- **Beautiful UI** - Clean interface for testing
- **Real-time logging** - See every step in action

## 🎯 Test Configuration

### **Environment Variables**
- `HEADLESS=true` - Run in headless mode
- `HEADLESS=false` - Run in headed mode (default for development)

### **Network Settings**
- **Network Name:** Anvil Testnet
- **RPC URL:** http://127.0.0.1:8545
- **Chain ID:** 31337
- **Symbol:** ETH

### **Wallet Configuration**
- **Seed Phrase:** `test test test test test test test test test test test junk`
- **Expected Address:** `0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266`
- **Message to Sign:** `"StorageHub rocks!"`

## 🐛 Troubleshooting

**Tests failing to start:**
- Ensure Anvil is running on port 8545
- Check that port 3000 is available for the test dApp

**MetaMask not connecting:**
- dAppwright handles all extension setup automatically
- In headed mode, you can see the MetaMask UI
- Check browser console for any errors

**Network issues:**
- Anvil must be running before tests start
- Chain ID must be exactly 31337
- RPC must be accessible at http://127.0.0.1:8545

## 🔄 Extending Tests

This implementation provides a clean foundation for adding more E2E tests:

1. **Add new test cases** in `tests/dappwright-hello-world.spec.ts`
2. **Create new test files** for different scenarios
3. **Extend the HTML dApp** with more functionality
4. **Test additional MetaMask features** (transactions, contracts, etc.)

## 📊 Expected Output

### **Headed Mode:**
- Browser window opens with MetaMask extension
- Test dApp loads at http://localhost:3000
- Real-time interaction visible
- MetaMask popups appear for approval

### **Headless Mode:**
- All output in terminal
- Step-by-step progress logged
- Perfect for CI/CD automation
- No visual interface needed

## 🎉 Success Criteria

When tests pass, you'll see:
- ✅ MetaMask extension loaded
- ✅ Anvil network added and switched
- ✅ Wallet connected to dApp
- ✅ "StorageHub rocks!" message signed
- ✅ Valid signature received (0x...)
- ✅ Correct wallet address verified

This gives you a robust, maintainable E2E testing foundation that can be extended for all your StorageHub testing needs! 🚀
