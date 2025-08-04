import { test, expect } from './wallet.fixture';

const PAGE_URL = 'http://localhost:5173';

test('sign message and transaction', async ({ page, wallet }) => {
    await page.goto(PAGE_URL);
    await page.getByTestId('connect').click();
    await wallet.approve();

    // sign message
    const msg = 'Hello StorageHub';
    await page.getByTestId('msg-input').fill(msg);
    await page.getByTestId('sign-msg').click();
    const msgSig = page.getByTestId('msg-sig');
    await expect(msgSig).not.toHaveText('');

    // transaction: use same address (already autofilled)
    await page.getByTestId('value-input').fill('0.01');
    await page.getByTestId('sign-txn').click();
    await wallet.approve();
    const txnSig = page.getByTestId('txn-sig');
    await expect(txnSig).not.toHaveText('');
});