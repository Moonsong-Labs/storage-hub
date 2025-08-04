import { defineConfig } from '@playwright/test';

export default defineConfig({
    testDir: './tests',
    timeout: 60_000,
    webServer: {
        command: 'pnpm run serve:e2e',
        port: 5173,
        reuseExistingServer: !process.env.CI,
        timeout: 30_000,
    },
    use: {
        headless: true,
        viewport: { width: 1280, height: 720 },
        trace: 'retain-on-failure',
    },
});