// @ts-nocheck
import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
    testDir: './tests',
    timeout: 120000,
    fullyParallel: false,
    forbidOnly: !!process.env.CI,
    retries: process.env.CI ? 2 : 0,
    reporter: [['html', { outputFolder: '/tmp/playwright-report', open: 'never' }], ['list']],
    use: {
        trace: 'on-first-retry',
        video: 'retain-on-failure',
        screenshot: 'only-on-failure',
        headless: process.env.HEADLESS === 'true',
        storageState: undefined,
    },
    outputDir: '/tmp/test-results',
    webServer: {
        command: "/bin/bash -lc 'PID=\$(lsof -ti tcp:3000); if [ -n \"$PID\" ]; then kill -9 \"$PID\"; fi; python3 -m http.server 3000 --directory ..'",
        url: process.env.E2E_BASE_URL || 'http://localhost:3000',
        timeout: 120000,
        reuseExistingServer: true,
    },
    projects: [
        {
            name: 'metamask',
            testMatch: ['wallet/**/metamask-sdk-sign.spec.ts'],
            use: {
                ...devices['Desktop Chrome'],
                baseURL: process.env.E2E_BASE_URL || 'http://localhost:3000',
            },
            workers: 1,
        },
        {
            name: 'msp',
            testMatch: ['msp/**/*.spec.ts'],
            use: {
                ...devices['Desktop Chrome'],
                baseURL: process.env.E2E_BASE_URL || 'http://localhost:3000',
            },
        },
    ],
});
