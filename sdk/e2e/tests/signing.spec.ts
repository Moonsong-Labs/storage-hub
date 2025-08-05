import { test, expect } from './wallet.fixture';

const PAGE_URL = 'http://localhost:5173/basic.html';

test('wallet signing methods', async ({ page, wallet }) => {
    // Navigate and connect
    await page.goto(PAGE_URL);
    await page.getByTestId('connect').click();
    await wallet.approve();

    // Verify connection
    await expect(page.getByTestId('address')).not.toHaveText('');
    const walletAddress = await page.getByTestId('address').textContent();
    console.log('🔗 Connected to wallet:', walletAddress);

    // Test 1: Sign Message
    console.log('🖊️ Testing message signing...');

    const testMessage = 'Hello StorageHub E2E Test!';
    await page.getByTestId('message-input').fill(testMessage);
    await page.getByTestId('sign-message').click();

    // Approve the signing in MetaMask
    await wallet.approve();

    // Verify message signature appears
    await expect(page.getByTestId('message-signature')).not.toHaveText('');
    await expect(page.getByTestId('message-signature')).not.toHaveText('Signing...');
    await expect(page.getByTestId('message-signature')).not.toHaveText('Error signing message');

    const messageSignature = await page.getByTestId('message-signature').textContent();
    console.log('✅ Message signature:', messageSignature);

    // Verify signature format (should be hex string starting with 0x)
    expect(messageSignature).toMatch(/^0x[0-9a-fA-F]{130}$/); // Standard Ethereum signature length

    // Test 2: Sign Transaction  
    console.log('💸 Testing transaction signing...');

    // Verify recipient is auto-filled with wallet address
    const recipientValue = await page.getByTestId('recipient-input').inputValue();
    expect(recipientValue).toBe(walletAddress);

    // Set a small amount
    await page.getByTestId('amount-input').fill('0.001');
    await page.getByTestId('sign-transaction').click();

    // Approve the transaction in MetaMask
    await wallet.approve();

    // Verify transaction signature appears
    await expect(page.getByTestId('tx-signature')).not.toHaveText('');
    await expect(page.getByTestId('tx-signature')).not.toHaveText('Signing...');
    await expect(page.getByTestId('tx-signature')).not.toHaveText('Error signing transaction');

    const txSignature = await page.getByTestId('tx-signature').textContent();
    console.log('✅ Transaction signature:', txSignature);

    // Verify transaction signature format
    expect(txSignature).toMatch(/^0x[0-9a-fA-F]+$/); // Should be hex string

    console.log('🎉 All signing tests passed!');
});

test('message signing with custom message', async ({ page, wallet }) => {
    await page.goto(PAGE_URL);
    await page.getByTestId('connect').click();
    await wallet.approve();

    // Test with a different message
    const customMessage = 'StorageHub rocks! 🚀';
    await page.getByTestId('message-input').fill(customMessage);
    await page.getByTestId('sign-message').click();
    await wallet.approve();

    const signature = await page.getByTestId('message-signature').textContent();
    expect(signature).toMatch(/^0x[0-9a-fA-F]{130}$/);
    console.log('✅ Custom message signed:', customMessage);
});

test('transaction validation', async ({ page, wallet }) => {
    await page.goto(PAGE_URL);
    await page.getByTestId('connect').click();
    await wallet.approve();

    // Test with invalid recipient address
    await page.getByTestId('recipient-input').fill('invalid-address');
    await page.getByTestId('sign-transaction').click();

    // Should show error without trying to sign
    await expect(page.getByTestId('tx-signature')).toHaveText('Error signing transaction');
    console.log('✅ Invalid address correctly rejected');
});