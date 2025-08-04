import { MetamaskWallet, FileManager } from '@storagehub-sdk/core';
import initWasm from '@storagehub/wasm';
import { parseEther, Transaction, getBytes } from 'ethers';

const wasmReady = initWasm();

// DOM refs
const connectBtn = document.getElementById('connectButton') as HTMLButtonElement;
const walletInfo = document.getElementById('walletInfo') as HTMLDivElement;
const walletAddressSpan = document.getElementById('walletAddress') as HTMLSpanElement;
const errorEl = document.getElementById('error') as HTMLElement;

// File hashing elements
const fileInput = document.getElementById('fileInput') as HTMLInputElement;
const fileNameEl = document.getElementById('fileNameDisplay') as HTMLElement;
const rootHashEl = document.getElementById('rootHashDisplay') as HTMLElement;

// Sign message elements
const msgInput = document.getElementById('msgInput') as HTMLInputElement;
const signMsgBtn = document.getElementById('signMsgBtn') as HTMLButtonElement;
const msgSigEl = document.getElementById('msgSig') as HTMLElement;

// Sign txn elements
const toInput = document.getElementById('toInput') as HTMLInputElement;
const valueInput = document.getElementById('valueInput') as HTMLInputElement;
const signTxnBtn = document.getElementById('signTxnBtn') as HTMLButtonElement;
const txnSigEl = document.getElementById('txnSig') as HTMLElement;

let wallet: MetamaskWallet | null = null;

function showError(msg: string) {
    errorEl.textContent = msg;
    errorEl.classList.remove('hidden');
}
function hideError() { errorEl.classList.add('hidden'); }

function enablePostConnectActions() {
    signMsgBtn.disabled = false;
    signTxnBtn.disabled = false;
}

// Wallet connect
connectBtn?.addEventListener('click', async () => {
    hideError();
    try {
        wallet = await MetamaskWallet.connect();
        const address = await wallet.getAddress();
        walletAddressSpan.textContent = address;
        walletInfo.classList.remove('hidden');
        connectBtn.classList.add('hidden');
        enablePostConnectActions();
        // Autofill recipient with own address for convenience
        toInput.value = address;
    } catch (e: unknown) {
        const msg = e instanceof Error ? e.message : String(e);
        showError(msg);
    }
});

// File hashing handler
fileInput?.addEventListener('change', async () => {
    const files = fileInput.files;
    if (!files || files.length === 0) return;
    hideError();
    const file = files[0];
    fileNameEl.textContent = file.name;
    rootHashEl.textContent = 'Computing…';
    try {
        await wasmReady;
        const fm = new FileManager({ size: file.size, stream: () => file.stream() });
        const fp = await fm.getFingerprint();
        rootHashEl.textContent = (fp as any).toHex ? (fp as any).toHex() : fp.toString();
    } catch (e: unknown) {
        showError(e instanceof Error ? e.message : String(e));
        rootHashEl.textContent = '';
    }
});

// Sign message
signMsgBtn?.addEventListener('click', async () => {
    if (!wallet) return showError('Connect wallet first');
    msgSigEl.textContent = '';
    hideError();
    try {
        const signature = await wallet.signMessage(msgInput.value);
        msgSigEl.textContent = signature;
    } catch (e: unknown) {
        showError(e instanceof Error ? e.message : String(e));
    }
});

// Send / sign transaction
signTxnBtn?.addEventListener('click', async () => {
    if (!wallet) return showError('Connect wallet first');
    txnSigEl.textContent = '';
    hideError();
    try {
        const to = toInput.value.trim();
        if (!/^0x[0-9a-fA-F]{40}$/.test(to)) throw new Error('Invalid recipient address');
        const value = valueInput.value.trim() || '0';
        const unsigned = { to, value: parseEther(value).toString(), gasLimit: 21000 };
        const txObj = Transaction.from(unsigned);
        const rawBytes = getBytes(txObj.unsignedSerialized);
        const signature = await wallet.signTxn(rawBytes);
        txnSigEl.textContent = signature;
    } catch (e: unknown) {
        showError(e instanceof Error ? e.message : String(e));
    }
});