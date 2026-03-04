/**
 * Navigation tests — verifies every page transition in the single-page app.
 */
import { test, expect } from '@playwright/test';
import { clearStorage, seedApiKeys, seedMeeting, SEED_MEETING } from './helpers';

test.describe('Page Navigation', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await clearStorage(page);
    await page.reload();
  });

  test('home page loads and shows app title', async ({ page }) => {
    await expect(page.locator('.logo')).toContainText('Vantage');
    await expect(page.locator('h1')).toContainText('Your meetings');
    await expect(page.locator('.start-btn')).toBeVisible();
  });

  test('home → settings via gear icon, back to home', async ({ page }) => {
    await page.locator('[title="Settings"]').click();
    await expect(page.locator('h2')).toContainText('Settings');
    await expect(page.locator('input[placeholder*="Deepgram"]')).toBeVisible();
    // Back button returns to home
    await page.locator('.btn-back').click();
    await expect(page.locator('h1')).toContainText('Your meetings');
  });

  test('home → history via clock icon, back to home', async ({ page }) => {
    await page.locator('[title="History"]').click();
    await expect(page.locator('h2')).toContainText('Past Meetings');
    await page.locator('.btn-back').click();
    await expect(page.locator('h1')).toContainText('Your meetings');
  });

  test('history → view meeting → back to history', async ({ page }) => {
    await seedMeeting(page);
    await page.reload();
    // Navigate to history
    await page.locator('[title="History"]').click();
    await expect(page.locator('.meeting-card')).toBeVisible();
    // Click the meeting card
    await page.locator('.meeting-card').first().click();
    await expect(page.locator('h2')).toContainText(SEED_MEETING.title);
    // Back from view-meeting goes to history
    await page.locator('.btn-back').click();
    await expect(page.locator('h2')).toContainText('Past Meetings');
  });

  test('home recent meetings card navigates to view-meeting', async ({ page }) => {
    await seedMeeting(page);
    await page.reload();
    // Recent meetings section is on home
    await expect(page.locator('.recent-meetings')).toBeVisible();
    await page.locator('.recent-meetings .meeting-card').first().click();
    await expect(page.locator('h2')).toContainText(SEED_MEETING.title);
  });

  test('setup banner navigates to settings when no keys configured', async ({ page }) => {
    // No keys seeded, so banner should appear
    await expect(page.locator('.setup-banner')).toBeVisible();
    await page.locator('.setup-banner').click();
    await expect(page.locator('h2')).toContainText('Settings');
  });

  test('setup banner is hidden when keys are configured', async ({ page }) => {
    await seedApiKeys(page);
    await page.reload();
    await expect(page.locator('.setup-banner')).not.toBeVisible();
  });
});
