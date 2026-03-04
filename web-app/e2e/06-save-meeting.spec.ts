/**
 * Save meeting to history — exercises the full "record → stop → save" loop,
 * then verifies the saved meeting appears in history and on the home page.
 */
import { test, expect } from '@playwright/test';
import {
  clearStorage,
  seedApiKeys,
  installMeetingMocks,
  getStoredMeetings,
} from './helpers';

test.describe('Save meeting to history', () => {
  test.beforeEach(async ({ page }) => {
    await installMeetingMocks(page);
    await page.goto('/');
    await clearStorage(page);
    await seedApiKeys(page);
    await page.reload();
  });

  async function startAndStop(page: any, contextText?: string) {
    if (contextText) {
      await page.locator('.context-input').fill(contextText);
    }
    await page.locator('.audio-mode-btn', { hasText: 'Mic only' }).click();
    await page.locator('.start-btn').click();
    // Wait for the meeting recording page to appear
    await expect(page.locator('.recording-indicator')).toBeVisible({ timeout: 5000 });
    // Allow all fake WS transcript messages to arrive (200ms delay + 3 * 200ms stagger = 800ms)
    await page.waitForTimeout(1200);
    await page.locator('.btn-stop').click();
    await expect(page.locator('h2')).toContainText('Meeting Complete', { timeout: 5000 });
  }

  test('"Save & Close" saves meeting and goes to home', async ({ page }) => {
    await startAndStop(page, 'Quarterly review');
    await page.locator('button', { hasText: 'Save & Close' }).click();
    await expect(page.locator('h1')).toContainText('Your meetings');
  });

  test('saved meeting appears in localStorage', async ({ page }) => {
    await startAndStop(page, 'Q4 planning');
    await page.locator('button', { hasText: 'Save & Close' }).click();

    const meetings = await getStoredMeetings(page);
    expect(meetings.length).toBe(1);
    expect(meetings[0].title).toBe('Q4 planning');
  });

  test('saved meeting appears in the recent-meetings section on home', async ({ page }) => {
    await startAndStop(page, 'Design review');
    await page.locator('button', { hasText: 'Save & Close' }).click();

    await expect(page.locator('.recent-meetings')).toBeVisible();
    await expect(page.locator('.recent-meetings .meeting-card h4')).toContainText('Design review');
  });

  test('saved meeting has transcript stored', async ({ page }) => {
    await startAndStop(page);
    await page.locator('button', { hasText: 'Save & Close' }).click();

    const meetings = await getStoredMeetings(page);
    // Transcript should be non-empty (fake WS delivered final lines)
    expect(meetings[0].transcript.length).toBeGreaterThan(0);
  });

  test('saved meeting title defaults to date when no context given', async ({ page }) => {
    await startAndStop(page); // no context
    await page.locator('button', { hasText: 'Save & Close' }).click();

    const meetings = await getStoredMeetings(page);
    expect(meetings[0].title).toMatch(/Meeting on/i);
  });

  test('saved meeting appears in history page', async ({ page }) => {
    await startAndStop(page, 'Sprint retro');
    await page.locator('button', { hasText: 'Save & Close' }).click();

    await page.locator('[title="History"]').click();
    await expect(page.locator('.meeting-card h4')).toContainText('Sprint retro');
  });

  test('can save multiple meetings and all appear in history', async ({ page }) => {
    await startAndStop(page, 'First meeting');
    await page.locator('button', { hasText: 'Save & Close' }).click();
    await expect(page.locator('h1')).toContainText('Your meetings');

    // Second meeting
    await startAndStop(page, 'Second meeting');
    await page.locator('button', { hasText: 'Save & Close' }).click();

    await page.locator('[title="History"]').click();
    await expect(page.locator('.meeting-card')).toHaveCount(2);
  });
});
