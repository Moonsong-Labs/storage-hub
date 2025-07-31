import { MetamaskWallet } from '@storagehub-sdk/core';
import { parseEther, Transaction, getBytes } from 'ethers';

// DOM Elements
const connectButton = document.getElementById('connectButton');
const walletInfoDiv = document.getElementById('walletInfo');
const walletAddressSpan = document.getElementById('walletAddress');
const errorDiv = document.getElementById('error');

const messageInput = document.getElementById('messageInput');
const signMessageButton = document.getElementById('signMessageButton');
const signatureOutput = document.getElementById('signatureOutput');

const toInput = document.getElementById('toInput');
const valueInput = document.getElementById('valueInput');
const sendTxButton = document.getElementById('sendTxButton');
const txSigOutput = document.getElementById('txSigOutput');

let wallet = null;

function showError(message) {
    errorDiv.textContent = message;
    errorDiv.classList.remove('hidden');
}

function hideError() {
    errorDiv.classList.add('hidden');
}

connectButton.addEventListener('click', async () => {
    hideError();
    try {
        wallet = await MetamaskWallet.connect();
        const address = await wallet.getAddress();

        walletAddressSpan.textContent = address;
        walletInfoDiv.classList.remove('hidden');
        connectButton.classList.add('hidden');

        // Populate the 'to' address with a default value for convenience
        if (!toInput.value) {
            toInput.value = address;
        }

    } catch (err) {
        showError(err.message);
    }
});

signMessageButton.addEventListener('click', async () => {
    hideError();
    signatureOutput.textContent = '';
    if (!wallet) {
        showError('Please connect your wallet first.');
        return;
    }
    try {
        const message = messageInput.value;
        const signature = await wallet.signMessage(message);
        signatureOutput.textContent = signature;
    } catch (err) {
        showError(err.message);
    }
});

sendTxButton.addEventListener('click', async () => {
    hideError();
    txSigOutput.textContent = '';
    if (!wallet) {
        showError('Please connect your wallet first.');
        return;
    }
    try {
        const to = toInput.value.trim();
        const valueEth = valueInput.value.trim();

        if (!to || !/^0x[0-9a-fA-F]{40}$/.test(to)) {
            throw new Error('Please enter a valid recipient address');
        }
        const unsignedTx = {
            to,
            value: parseEther(valueEth || '0').toString(),
            gasLimit: 21_000,
        };

        const txObj = Transaction.from(unsignedTx);
        const rawBytes = getBytes(txObj.unsignedSerialized);
        const signature = await wallet.signTxn(rawBytes);
        txSigOutput.textContent = signature;
    } catch (err) {
        showError(err.message);
    }
}); 