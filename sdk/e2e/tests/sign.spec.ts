// DISABLED: Complex test - use basic.spec.ts instead
// import { test, expect } from './wallet.fixture';

// const PAGE_URL = 'http://localhost:5173';

// test('sign message and transaction', async ({ page, wallet }) => {
//   await page.goto(PAGE_URL);
//   await page.getByTestId('connect').click();
//   await wallet.approve();

//   // Get the wallet address to use as recipient  
//   const address = await page.getByTestId('address').textContent();

//   // sign message
//   const msg = 'Hello StorageHub';
//   await page.getByTestId('msg-input').fill(msg);
//   await page.getByTestId('sign-msg').click();
//   await wallet.approve();
//   const msgSig = page.getByTestId('msg-sig');
//   await expect(msgSig).not.toHaveText('');

//   // transaction: send to self (common testing pattern)
//   await page.getByTestId('to-input').fill(address || '0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266');
//   await page.getByTestId('value-input').fill('0.01');
//   await page.getByTestId('sign-txn').click();
//   await wallet.approve();
//   const txnSig = page.getByTestId('txn-sig');
//   await expect(txnSig).not.toHaveText('');
// });