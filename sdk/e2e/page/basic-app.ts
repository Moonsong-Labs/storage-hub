import { MetamaskWallet } from '@storagehub-sdk/core';
import { parseEther, Transaction, getBytes } from 'ethers';

// Expose StorageHub SDK to window for E2E test access
(window as any).MetamaskWallet = MetamaskWallet;

// DOM elements
const connectBtn = document.getElementById('connectButton') as HTMLButtonElement;
const switchNetworkBtn = document.getElementById('switch-network-button') as HTMLButtonElement;
const walletInfo = document.getElementById('walletInfo') as HTMLDivElement;
const walletAddressSpan = document.getElementById('walletAddress') as HTMLSpanElement;
const connectStatus = document.getElementById('connect-status') as HTMLInputElement;
const networkStatus = document.getElementById('network-status') as HTMLInputElement;

// Signing elements
const messageInput = document.getElementById('messageInput') as HTMLInputElement;
const signMessageBtn = document.getElementById('signMessageBtn') as HTMLButtonElement;
const messageSignature = document.getElementById('messageSignature') as HTMLElement;

// Transaction signing elements
const recipientInput = document.getElementById('recipientInput') as HTMLInputElement;
const amountInput = document.getElementById('amountInput') as HTMLInputElement;
const signTxnBtn = document.getElementById('signTxnBtn') as HTMLButtonElement;
const txSignature = document.getElementById('txSignature') as HTMLElement;

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
        connectStatus.value = 'connected';
        walletInfo.classList.remove('hidden');
        connectBtn.classList.add('hidden');
        switchNetworkBtn.classList.remove('hidden');

        // Update network status
        try {
            const chainId = await (window as any).ethereum.request({ method: 'eth_chainId' });
            networkStatus.value = parseInt(chainId, 16).toString();
        } catch (e) {
            console.error('Failed to get chain ID:', e);
        }

        // DISABLED: Auto-fill recipient (signing disabled)
        // recipientInput.value = address;

    } catch (e: unknown) {
        const msg = e instanceof Error ? e.message : String(e);
        console.error('❌ Wallet connection failed:', msg);
        showError(msg);
    }
});

// Network switching handler
switchNetworkBtn?.addEventListener('click', async () => {
    console.log('🔄 Attempting to switch to Hardhat network...');

    try {
        await (window as any).ethereum.request({
            method: 'wallet_switchEthereumChain',
            params: [{ chainId: '0x7a69' }], // 31337 in hex
        });

        // Update network status after switch
        setTimeout(async () => {
            try {
                const chainId = await (window as any).ethereum.request({ method: 'eth_chainId' });
                networkStatus.value = parseInt(chainId, 16).toString();
                console.log(`✅ Switched to network: ${networkStatus.value}`);
            } catch (e) {
                console.error('Failed to get updated chain ID:', e);
            }
        }, 1000);

    } catch (e: unknown) {
        const msg = e instanceof Error ? e.message : String(e);
        console.error('❌ Network switch failed:', msg);
        showError(`Network switch failed: ${msg}`);
    }
});

// Message signing handler
signMessageBtn?.addEventListener('click', async () => {
    console.log('🖊️ Signing message...');
    messageSignature.textContent = 'Signing...';

    try {
        const message = messageInput.value || 'Hello StorageHub!';

        let signature: string;

        console.log('Wallet status:', wallet ? 'exists' : 'null');

        // Use StorageHub SDK's MetamaskWallet for signing
        if (!wallet) {
            console.log('No wallet instance, creating new MetamaskWallet...');
            // Create a new MetamaskWallet instance using the existing ethereum provider
            // This ensures we use the provider that dappwright has already connected
            const { BrowserProvider } = await import('ethers');
            const provider = new BrowserProvider(window.ethereum as any);
            wallet = new (MetamaskWallet as any)(provider); // Using private constructor
        }

        console.log('Using StorageHub SDK MetamaskWallet.signMessage()...');
        signature = await wallet.signMessage(message);
        console.log('StorageHub SDK signMessage completed with signature length:', signature.length);

        console.log('✅ Message signed successfully!');
        console.log('📝 Message:', message);
        console.log('📋 Signature:', signature);

        messageSignature.textContent = signature;

    } catch (e: unknown) {
        const msg = e instanceof Error ? e.message : String(e);
        console.error('❌ Message signing failed:', msg);
        showError(msg);
        messageSignature.textContent = 'Error signing message';
    }
});

// Transaction signing handler
signTxnBtn?.addEventListener('click', async () => {
    // Use StorageHub SDK's MetamaskWallet for transaction signing
    if (!wallet) {
        console.log('No wallet instance for transaction, creating new MetamaskWallet...');
        // Create a new MetamaskWallet instance using the existing ethereum provider
        const { BrowserProvider } = await import('ethers');
        const provider = new BrowserProvider(window.ethereum as any);
        wallet = new (MetamaskWallet as any)(provider); // Using private constructor
    }

    console.log('💸 Signing transaction...');
    txSignature.textContent = 'Signing...';

    try {
        const to = recipientInput.value.trim();
        const amount = amountInput.value.trim() || '0.001';

        if (!/^0x[0-9a-fA-F]{40}$/.test(to)) {
            throw new Error('Invalid recipient address');
        }

        // Create transaction object
        const txRequest = {
            to,
            value: parseEther(amount).toString(),
            gasLimit: 21000
        };

        console.log('📋 Transaction:', txRequest);

        // Convert to Transaction object and get raw bytes
        const txObj = Transaction.from(txRequest);
        const rawBytes = getBytes(txObj.unsignedSerialized);

        console.log('🔧 About to call StorageHub SDK wallet.signTxn()...');
        // Sign transaction using StorageHub SDK's MetamaskWallet
        const signature = await wallet.signTxn(rawBytes);

        console.log('✅ Transaction signed & sent successfully!');
        console.log('📋 Signature:', signature);

        txSignature.textContent = signature;

    } catch (e: unknown) {
        const msg = e instanceof Error ? e.message : String(e);
        console.error('❌ Transaction signing failed:', msg);
        showError(msg);
        txSignature.textContent = 'Error signing transaction';
    }
});