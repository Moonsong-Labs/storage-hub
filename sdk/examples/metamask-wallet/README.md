# MetaMask Wallet Quick-test

This example lets you confirm that the **StorageHub SDK → MetaMask** integration works end-to-end.  In less than a minute you can:

1. Connect the browser to your MetaMask account.
2. Sign an arbitrary message.
3. Sign **and** broadcast a small ETH transaction and view the raw ECDSA signature that MetaMask returns.

---

## Prerequisites

* A modern browser (Chrome / Firefox / Brave / Edge).
* The [MetaMask extension](https://metamask.io/download/) installed and unlocked.
* [Node.js](https://nodejs.org/) & [pnpm](https://pnpm.io/) – StorageHub is a pnpm monorepo.

> **Directory layout reminder**  
> The SDK lives in `storage-hub/sdk/` and this example is in `storage-hub/sdk/examples/metamask-wallet/`.

---

## 1. Build the SDK (from *sdk/*)

```bash
cd storage-hub/sdk        # ALWAYS run the following commands from here
pnpm install              # one-time – installs dependencies for every workspace
pnpm run build            # builds core → creates core/dist/index.js
```

While developing you can keep the build running in watch mode:

```bash
pnpm --filter ./core run dev  # rebuilds on every file change
```

---

## 2. Serve the files (still in *sdk/*)

MetaMask only injects its provider on pages served via `http:` or `https:`.  Any static server works – the tiny **serve** CLI is convenient:

```bash
# install once if you don’t already have it
npm i -g serve

# IMPORTANT: run *from the sdk directory*
serve -l 3000              # or any free port
```

The web root is now the *sdk/* folder, so the import map entry `/core/dist/index.js` resolves correctly.

---

## 3. Open the demo

Visit: <http://localhost:3000/examples/metamask-wallet/>  
Click **“Connect to MetaMask”** and authorise the site.

* **Sign a message** – enter text, click **Sign Message** and the signature appears below.
* **Sign & Send a transaction** – the form is pre-filled with your own address; press **Send Transaction**. After confirming in MetaMask the page displays the **raw signature**.  You can copy it for contract calls or testing.

---