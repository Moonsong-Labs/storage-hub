# 🧪 Automating MetaMask with Dappwright and Playwright

This guide walks you through automating MetaMask interactions (connect, network switch, message signing) using **Dappwright** and **Playwright**, connected to a local **Hardhat/Anvil** node.

---

## ✅ Goal

1. Launch MetaMask in a Playwright browser.
2. Connect to a local Hardhat/Anvil node (`http://127.0.0.1:8545`).
3. Load a minimal local dApp.
4. Request MetaMask to sign a message.
5. Automatically approve the signature via MetaMask.

---

## 📦 Requirements

- Node.js (v16+)
- Running Hardhat/Anvil node (default: `http://127.0.0.1:8545`)
- Seed phrase from local node (default provided below)
- Local static server to host dApp (`serve` or similar)

---

## 🛠️ Setup

### 1. Install dependencies

```bash
npm install -D @tenkeylabs/dappwright playwright
npx playwright install
```

---

### 2. Default Hardhat Mnemonic

Hardhat/Anvil typically uses:

```
test test test test test test test test test test test junk
```

---

### 3. Create a local test dApp

Save this as `public/index.html`:

```html
<!DOCTYPE html>
<html>
  <body>
    <button id="connect">Connect Wallet</button>
    <button id="sign">Sign Message</button>

    <script>
      const connectBtn = document.getElementById("connect");
      const signBtn = document.getElementById("sign");

      let account;

      connectBtn.onclick = async () => {
        const [addr] = await window.ethereum.request({ method: "eth_requestAccounts" });
        account = addr;
        console.log("Connected:", addr);
      };

      signBtn.onclick = async () => {
        const msg = "Hello from Dappwright!";
        const signature = await window.ethereum.request({
          method: "personal_sign",
          params: [msg, account],
        });
        console.log("Signature:", signature);
      };
    </script>
  </body>
</html>
```

Serve this file locally:

```bash
npx serve public
```

---

### 4. Dappwright Test Script

Save as `test-dappwright.js`:

```js
import { launch } from '@tenkeylabs/dappwright';

const seed = 'test test test test test test test test test test test junk';

const dappwright = await launch('metamask', {
  metaMaskVersion: 'latest',
  seed,
  password: 'password123',
  headless: false,
  args: ['--remote-debugging-port=9222'],
});

const { page, metaMask } = dappwright;

await metaMask.addNetwork({
  networkName: 'Localhost 8545',
  rpc: 'http://127.0.0.1:8545',
  chainId: 31337,
  symbol: 'ETH',
});

await page.goto('http://localhost:3000');

await page.click('#connect');
await metaMask.acceptAccess();

await page.click('#sign');
await metaMask.sign();

await page.waitForTimeout(2000);
await dappwright.close();
```

---

## ✅ Summary

- This setup allows for full automation of dApp-to-wallet interaction using MetaMask.
- Great for CI or local test environments.
- Easily extensible to support transactions, network switching, etc.

---

## 🧯 Troubleshooting

- Use `headless: false` while debugging.
- Ensure local server (on `localhost:3000`) is live.
- Check chain ID (`31337`) matches Anvil config.
- Make sure `metaMask.acceptAccess()` and `.sign()` are called **after** page triggers.

---

Let me know if you'd like a repo template or CI setup.