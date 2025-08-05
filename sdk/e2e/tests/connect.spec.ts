// DISABLED: Complex test - use basic.spec.ts instead
// import { test, expect } from './wallet.fixture';

// const PAGE_URL = 'http://localhost:5173';

// test('connect wallet and compute file hash', async ({ page, wallet }) => {
//   await page.goto(PAGE_URL);

//   // Click connect and then approve (NO page parameter needed)
//   await page.getByTestId('connect').click();
//   await wallet.approve(); // FIXED: No page parameter

//   await expect(page.getByTestId('address')).not.toHaveText('');

//   const testFile = await page.evaluateHandle(() => {
//     return new File([new Uint8Array([1, 2, 3, 4])], 'test.bin');
//   });
//   await page.getByTestId('file-input').setInputFiles(testFile);
//   await expect(page.getByTestId('root-hash')).not.toHaveText('');
// });