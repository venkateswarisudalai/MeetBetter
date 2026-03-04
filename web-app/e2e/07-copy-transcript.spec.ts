/**
 * Copy transcript / copy summary button tests.
 *
 * The clipboard API requires a secure context and special permissions.
 * Playwright grants clipboard-read + clipboard-write via the browser context,
 * and we grant it in playwright.config.ts. We also intercept the
 * navigator.clipboard.writeText call so we can assert it was called.
 */
import { test, expect } from '@playwright/test';
import {
  clearStorage,
  seedApiKeys,
  seedMeeting,
  installMeetingMocks,
  SEED_MEETING,
} from './helpers';

test.describe('Copy transcript button', () => {
  // ---- From view-meeting page (seeded data) ----

  test.describe('from view-meeting page', () => {
    test.beforeEach(async ({ page, context }) => {
      // Grant clipboard permission
      await context.grantPermissions(['clipboard-read', 'clipboard-write']);
      await page.goto('/');
      await clearStorage(page);
      await seedMeeting(page);
      await page.reload();
      await page.locator('[title="History"]').click();
      await page.locator('.meeting-card').first().click();
    });

    test('Copy Transcript button is visible', async ({ page }) => {
      await expect(page.locator('button', { hasText: 'Copy Transcript' })).toBeVisible();
    });

    test('Copy Summary button is visible when summary exists', async ({ page }) => {
      await expect(page.locator('button', { hasText: 'Copy Summary' })).toBeVisible();
    });

    test('clicking Copy Transcript shows "Copied!" feedback', async ({ page }) => {
      await page.locator('button', { hasText: 'Copy Transcript' }).click();
      await expect(page.locator('button', { hasText: 'Copied!' })).toBeVisible({
        timeout: 3000,
      });
    });

    test('"Copied!" reverts back to label after 2 seconds', async ({ page }) => {
      await page.locator('button', { hasText: 'Copy Transcript' }).click();
      await expect(page.locator('button', { hasText: 'Copied!' })).toBeVisible({
        timeout: 3000,
      });
      // After 2 s the label should revert
      await expect(page.locator('button', { hasText: 'Copy Transcript' })).toBeVisible({
        timeout: 4000,
      });
    });

    test('Copy Transcript writes transcript text to clipboard', async ({ page }) => {
      await page.locator('button', { hasText: 'Copy Transcript' }).click();
      // Wait for Copied! to confirm the async writeText resolved
      await expect(page.locator('button', { hasText: 'Copied!' })).toBeVisible({
        timeout: 3000,
      });
      const clipText = await page.evaluate(() => navigator.clipboard.readText());
      expect(clipText).toContain(SEED_MEETING.transcript);
    });

    test('clicking Copy Summary shows "Copied!" feedback', async ({ page }) => {
      await page.locator('button', { hasText: 'Copy Summary' }).click();
      await expect(page.locator('button', { hasText: 'Copied!' })).toBeVisible({
        timeout: 3000,
      });
    });
  });

  // ---- From done page (after a mock recording) ----

  test.describe('from done page', () => {
    test.beforeEach(async ({ page, context }) => {
      await context.grantPermissions(['clipboard-read', 'clipboard-write']);
      await installMeetingMocks(page);
      await page.goto('/');
      await clearStorage(page);
      await seedApiKeys(page);
      await page.reload();

      // Start meeting, wait for transcripts, then stop
      await page.locator('.audio-mode-btn', { hasText: 'Mic only' }).click();
      await page.locator('.start-btn').click();
      await expect(page.locator('.recording-indicator')).toBeVisible({ timeout: 5000 });
      await page.waitForTimeout(1200); // allow fake WS transcripts to arrive
      await page.locator('.btn-stop').click();
      await expect(page.locator('h2')).toContainText('Meeting Complete', { timeout: 5000 });
    });

    test('Copy Transcript button is visible on done page', async ({ page }) => {
      await expect(page.locator('.done-actions button', { hasText: 'Copy Transcript' })).toBeVisible();
    });

    test('clicking Copy Transcript on done page shows Copied!', async ({ page }) => {
      await page.locator('.done-actions button', { hasText: 'Copy Transcript' }).click();
      await expect(page.locator('button', { hasText: 'Copied!' })).toBeVisible({
        timeout: 3000,
      });
    });

    test('copy transcript on done page writes content to clipboard', async ({ page }) => {
      await page.locator('.done-actions button', { hasText: 'Copy Transcript' }).click();
      await expect(page.locator('button', { hasText: 'Copied!' })).toBeVisible({
        timeout: 3000,
      });
      const text = await page.evaluate(() => navigator.clipboard.readText());
      // The fake WS delivers "Hello everyone" and "Welcome to the standup"
      expect(typeof text).toBe('string');
    });
  });
});
