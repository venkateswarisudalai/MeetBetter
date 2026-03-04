import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright E2E configuration for the MeetBetter / Vantage web app.
 *
 * Tests run against the deployed Vercel URL by default.
 * Set BASE_URL env var to point at a local dev server instead.
 */
export default defineConfig({
  testDir: './e2e',
  fullyParallel: false,       // keep serial so localStorage state is predictable
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  reporter: [['list'], ['html', { open: 'never', outputFolder: 'e2e-report' }]],

  use: {
    baseURL: process.env.BASE_URL || 'https://web-app-eta-beige.vercel.app',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    // Fake microphone so media-permission prompts don't block tests
    permissions: ['microphone'],
    launchOptions: {
      args: [
        '--use-fake-ui-for-media-stream',
        '--use-fake-device-for-media-stream',
        '--allow-file-access-from-files',
      ],
    },
  },

  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
});
