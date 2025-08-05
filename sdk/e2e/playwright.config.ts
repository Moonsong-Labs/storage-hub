import { defineConfig } from '@playwright/test';

export default defineConfig({
    workers: 1,
    testDir: './tests',
    timeout: 60_000,
    webServer: {
        command: 'pnpm run serve:e2e',
        port: 5173,
        reuseExistingServer: !process.env.CI,
        timeout: 30_000,
    },
    use: {
        headless: false, // Changed to false for easier debugging
        viewport: { width: 1280, height: 720 },
        trace: 'retain-on-failure',
    },
    // FOCUS: Run only the connect test
    testMatch: ['**/connect.spec.ts']
});