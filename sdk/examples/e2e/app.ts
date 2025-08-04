import { MetamaskWallet, FileManager } from '@storagehub-sdk/core';
import initWasm from '@storagehub/wasm';

// Initialise the WASM module once at start-up
const wasmReady = initWasm();

// ────────────────────────────────────────────────────────────
// DOM ELEMENTS
// ────────────────────────────────────────────────────────────
const connectBtn = document.getElementById('connectButton') as HTMLButtonElement;
const walletInfo = document.getElementById('walletInfo') as HTMLDivElement;
const walletAddressSpan = document.getElementById('walletAddress') as HTMLSpanElement;
const errorEl = document.getElementById('error') as HTMLElement;

const fileInput = document.getElementById('fileInput') as HTMLInputElement;
const fileNameEl = document.getElementById('fileNameDisplay') as HTMLElement;
const rootHashEl = document.getElementById('rootHashDisplay') as HTMLElement;

let wallet: MetamaskWallet | null = null;

function showError(msg: string) {
    errorEl.textContent = msg;
    errorEl.classList.remove('hidden');
}

function hideError() {
    errorEl.classList.add('hidden');
}

// ────────────────────────────────────────────────────────────
// WALLET CONNECT
// ────────────────────────────────────────────────────────────
connectBtn?.addEventListener('click', async () => {
    hideError();
    try {
        wallet = await MetamaskWallet.connect();
        const address = await wallet.getAddress();

        walletAddressSpan.textContent = address;
        walletInfo.classList.remove('hidden');
        connectBtn.classList.add('hidden');
    } catch (e: unknown) {
        const msg = e instanceof Error ? e.message : String(e);
        showError(msg);
    }
});

// ────────────────────────────────────────────────────────────
// FILE HASHING
// ────────────────────────────────────────────────────────────
fileInput?.addEventListener('change', async () => {
    const files = fileInput.files;
    if (!files || files.length === 0) {
        return;
    }

    hideError();
    const file = files[0];
    fileNameEl.textContent = file.name;
    rootHashEl.textContent = 'Computing…';

    try {
        await wasmReady;
        const fm = new FileManager({
            size: file.size,
            stream: () => file.stream(),
        });
        const fp = await fm.getFingerprint();
        // fp may expose toHex() depending on TypeRegistry version
        rootHashEl.textContent = (fp as any).toHex ? (fp as any).toHex() : fp.toString();
    } catch (e: unknown) {
        const msg = e instanceof Error ? e.message : String(e);
        showError(msg);
        rootHashEl.textContent = '';
    }
});
