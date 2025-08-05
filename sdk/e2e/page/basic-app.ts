import { MetamaskWallet } from '@storagehub-sdk/core';

// DOM elements
const connectBtn = document.getElementById('connectButton') as HTMLButtonElement;
const walletInfo = document.getElementById('walletInfo') as HTMLDivElement;
const walletAddressSpan = document.getElementById('walletAddress') as HTMLSpanElement;

let wallet: MetamaskWallet | null = null;

function showError(msg: string) {
    console.error('Error:', msg);
    alert(`Error: ${msg}`);
}

// Wallet connect handler
connectBtn?.addEventListener('click', async () => {
    console.log('🔄 Attempting to connect wallet...');

    try {
        wallet = await MetamaskWallet.connect();
        const address = await wallet.getAddress();

        console.log('✅ Wallet connected successfully!');
        console.log('📍 Address:', address);

        // Update UI
        walletAddressSpan.textContent = address;
        walletInfo.classList.remove('hidden');
        connectBtn.classList.add('hidden');

    } catch (e: unknown) {
        const msg = e instanceof Error ? e.message : String(e);
        console.error('❌ Wallet connection failed:', msg);
        showError(msg);
    }
});