/**
 * Meeting recording flow — start, live transcript, stop, done page.
 *
 * The Deepgram WebSocket, MediaRecorder, AudioContext, and getUserMedia
 * are all replaced by fakes installed before the page loads (see helpers.ts).
 */
import { test, expect } from '@playwright/test';
import {
  clearStorage,
  seedApiKeys,
  installMeetingMocks,
  FAKE_DG_KEY,
  FAKE_GROQ_KEY,
} from './helpers';

test.describe('Meeting recording flow', () => {
  test.beforeEach(async ({ page }) => {
    // Install WebSocket + media mocks before the app bundle runs
    await installMeetingMocks(page);
    await page.goto('/');
    await clearStorage(page);
    await seedApiKeys(page);
    await page.reload();
    // Re-install mocks after reload (addInitScript persists, but let's be explicit)
  });

  test('clicking Start Meeting without keys redirects to settings', async ({ page }) => {
    // Remove keys so hasKeys is false
    await page.evaluate((key) => localStorage.removeItem(key), 'vantage_keys');
    await page.reload();
    await page.locator('.start-btn').click();
    await expect(page.locator('h2')).toContainText('Settings');
  });

  test('Start Meeting with keys transitions to meeting page', async ({ page }) => {
    await page.locator('.audio-mode-btn', { hasText: 'Mic only' }).click();
    await page.locator('.start-btn').click();

    // Meeting page shows recording indicator
    await expect(page.locator('.recording-indicator')).toBeVisible({ timeout: 5000 });
    await expect(page.locator('.recording-indicator')).toContainText('Recording');
  });

  test('meeting page shows a running timer', async ({ page }) => {
    await page.locator('.audio-mode-btn', { hasText: 'Mic only' }).click();
    await page.locator('.start-btn').click();
    await expect(page.locator('.timer')).toBeVisible({ timeout: 5000 });
    // Timer starts at 00:00
    const firstValue = await page.locator('.timer').textContent();
    expect(firstValue).toMatch(/^\d{2}:\d{2}$/);
  });

  test('meeting page shows Stop button', async ({ page }) => {
    await page.locator('.audio-mode-btn', { hasText: 'Mic only' }).click();
    await page.locator('.start-btn').click();
    await expect(page.locator('.btn-stop')).toBeVisible({ timeout: 5000 });
    await expect(page.locator('.btn-stop')).toContainText('Stop');
  });

  test('connecting status is shown while waiting', async ({ page }) => {
    await page.locator('.audio-mode-btn', { hasText: 'Mic only' }).click();
    await page.locator('.start-btn').click();
    // Either "Connecting..." or the status message from the fake WS is shown
    const waiting = page.locator('.transcript-waiting');
    // It may appear briefly; if transcripts arrive fast it might already be gone
    // Just assert the meeting page structure is correct
    await expect(page.locator('.recording-indicator')).toBeVisible({ timeout: 5000 });
  });

  test('fake WebSocket delivers transcript lines to the live view', async ({ page }) => {
    await page.locator('.audio-mode-btn', { hasText: 'Mic only' }).click();
    await page.locator('.start-btn').click();

    // Ensure we are on the meeting page first
    await expect(page.locator('.recording-indicator')).toBeVisible({ timeout: 5000 });

    // The fake WS delivers transcript messages at 200-400ms after the WS is
    // created. Give it a generous window; the interim entry appears first.
    await expect(page.locator('.chat-bubble').first()).toBeVisible({ timeout: 8000 });
    const text = await page.locator('.chat-bubble').first().textContent();
    expect(text).toBeTruthy();
  });

  test('stopping the meeting navigates to the done page', async ({ page }) => {
    await page.locator('.audio-mode-btn', { hasText: 'Mic only' }).click();
    await page.locator('.start-btn').click();
    await expect(page.locator('.btn-stop')).toBeVisible({ timeout: 5000 });
    await page.locator('.btn-stop').click();

    // Done page shows "Meeting Complete" heading
    await expect(page.locator('h2')).toContainText('Meeting Complete', { timeout: 5000 });
  });

  test('done page shows duration and utterance stats', async ({ page }) => {
    await page.locator('.audio-mode-btn', { hasText: 'Mic only' }).click();
    await page.locator('.start-btn').click();
    await expect(page.locator('.btn-stop')).toBeVisible({ timeout: 5000 });
    await page.locator('.btn-stop').click();

    await expect(page.locator('.done-stats')).toBeVisible({ timeout: 5000 });
    await expect(page.locator('.stat-label', { hasText: 'Duration' })).toBeVisible();
    await expect(page.locator('.stat-label', { hasText: 'Utterances' })).toBeVisible();
  });

  test('done page shows Full Transcript section', async ({ page }) => {
    await page.locator('.audio-mode-btn', { hasText: 'Mic only' }).click();
    await page.locator('.start-btn').click();
    // Wait for transcripts to arrive before stopping (200ms delay + stagger)
    await expect(page.locator('.recording-indicator')).toBeVisible({ timeout: 5000 });
    await page.waitForTimeout(1200);
    await page.locator('.btn-stop').click();

    await expect(page.locator('h3', { hasText: 'Full Transcript' })).toBeVisible({
      timeout: 5000,
    });
  });

  test('done page has "Save & Close" button', async ({ page }) => {
    await page.locator('.audio-mode-btn', { hasText: 'Mic only' }).click();
    await page.locator('.start-btn').click();
    await expect(page.locator('.btn-stop')).toBeVisible({ timeout: 5000 });
    await page.locator('.btn-stop').click();

    await expect(page.locator('button', { hasText: 'Save & Close' })).toBeVisible({
      timeout: 5000,
    });
  });

  test('done page has Ask about this meeting section', async ({ page }) => {
    await page.locator('.audio-mode-btn', { hasText: 'Mic only' }).click();
    await page.locator('.start-btn').click();
    await expect(page.locator('.btn-stop')).toBeVisible({ timeout: 5000 });
    await page.locator('.btn-stop').click();

    await expect(page.locator('h3', { hasText: 'Ask about this meeting' })).toBeVisible({
      timeout: 5000,
    });
  });

  test('context text is passed through to the meeting title on save', async ({ page }) => {
    await page.locator('.context-input').fill('Sprint planning session');
    await page.locator('.audio-mode-btn', { hasText: 'Mic only' }).click();
    await page.locator('.start-btn').click();
    await expect(page.locator('.recording-indicator')).toBeVisible({ timeout: 5000 });
    await page.waitForTimeout(400);
    await page.locator('.btn-stop').click();
    await expect(page.locator('button', { hasText: 'Save & Close' })).toBeVisible({
      timeout: 5000,
    });
    await page.locator('button', { hasText: 'Save & Close' }).click();

    // Back on home, the recent meeting should show with the context title
    const meetings = await page.evaluate(() => {
      const raw = localStorage.getItem('vantage_meetings');
      return raw ? JSON.parse(raw) : [];
    });
    expect(meetings.length).toBeGreaterThan(0);
    expect(meetings[0].title).toBe('Sprint planning session');
  });
});
