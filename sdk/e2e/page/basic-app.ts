import { MetamaskWallet } from '@storagehub-sdk/core';
import { parseEther, Transaction, getBytes } from 'ethers';

// DOM elements
const connectBtn = document.getElementById('connectButton') as HTMLButtonElement;
const switchNetworkBtn = document.getElementById('switch-network-button') as HTMLButtonElement;
const walletInfo = document.getElementById('walletInfo') as HTMLDivElement;
const walletAddressSpan = document.getElementById('walletAddress') as HTMLSpanElement;
const connectStatus = document.getElementById('connect-status') as HTMLInputElement;
const networkStatus = document.getElementById('network-status') as HTMLInputElement;

// DISABLED: Signing elements until network switching works
// const messageInput = document.getElementById('messageInput') as HTMLInputElement;
// const signMessageBtn = document.getElementById('signMessageBtn') as HTMLButtonElement;
// const messageSignature = document.getElementById('messageSignature') as HTMLElement;

// const recipientInput = document.getElementById('recipientInput') as HTMLInputElement;
// const amountInput = document.getElementById('amountInput') as HTMLInputElement;
// const signTxnBtn = document.getElementById('signTxnBtn') as HTMLButtonElement;
// const txSignature = document.getElementById('txSignature') as HTMLElement;

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

/* DISABLED: Signing handlers until network switching works

// Message signing handler
signMessageBtn?.addEventListener('click', async () => {
    if (!wallet) {
        showError('Please connect wallet first');
        return;
    }
    
    console.log('🖊️ Signing message...');
    messageSignature.textContent = 'Signing...';
    
    try {
        const message = messageInput.value || 'Hello StorageHub!';
        const signature = await wallet.signMessage(message);
        
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
    if (!wallet) {
        showError('Please connect wallet first');
        return;
    }
    
    console.log('💸 Signing transaction...');
    txSignature.textContent = 'Signing...';
    
    try {
        const to = recipientInput.value.trim();
        const amount = amountInput.value.trim() || '0.01';
        
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
        
        // Sign transaction (this will also send it via MetaMask)
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

*/