import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
    testDir: './tests',
    testMatch: '**/metamask-sdk-sign.spec.ts',
    timeout: 45000, // Reduced timeout per request
    fullyParallel: false, // MetaMask tests should run sequentially
    forbidOnly: !!process.env.CI,
    retries: process.env.CI ? 2 : 0,
    workers: 1, // Run tests one at a time for MetaMask
    reporter: [['html'], ['list']],
    use: {
        trace: 'on-first-retry',
        screenshot: 'only-on-failure',
    },
    projects: [
        {
            name: 'MetaMask dAppwright Tests',
            use: {
                ...devices['Desktop Chrome'],
                // headless mode controlled by environment variable
                headless: process.env.HEADLESS === 'true',
            },
        },
    ],
});
