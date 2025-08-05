import { test, expect } from './wallet.fixture';

const PAGE_URL = 'http://localhost:5173/basic.html';

test('basic wallet connection', async ({ page, wallet }) => {
    // Navigate to the page
    await page.goto(PAGE_URL);

    // Click the connect button
    await page.getByTestId('connect').click();

    // Approve connection in MetaMask
    await wallet.approve();

    // Verify wallet connected successfully
    await expect(page.getByTestId('address')).not.toHaveText('');

    // Verify wallet info is shown
    await expect(page.locator('#walletInfo')).toBeVisible();

    // Verify connect button is hidden after connection
    await expect(page.getByTestId('connect')).toBeHidden();

    console.log('✅ Basic wallet connection test passed!');
});