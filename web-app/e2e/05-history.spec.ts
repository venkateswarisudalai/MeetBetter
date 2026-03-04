/**
 * History page tests — empty state, meeting list, view, and delete.
 */
import { test, expect } from '@playwright/test';
import {
  clearStorage,
  seedMeeting,
  SEED_MEETING,
  MEETINGS_STORAGE,
} from './helpers';

test.describe('History page', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await clearStorage(page);
    await page.reload();
  });

  // ---- Empty state ----

  test('empty state message shown when no meetings exist', async ({ page }) => {
    await page.locator('[title="History"]').click();
    await expect(page.locator('.empty-state')).toBeVisible();
    await expect(page.locator('.empty-state h3')).toContainText('No meetings yet');
    await expect(page.locator('.empty-state p')).toContainText('Your meeting recordings');
  });

  test('empty state icon is visible', async ({ page }) => {
    await page.locator('[title="History"]').click();
    await expect(page.locator('.empty-icon')).toBeVisible();
  });

  // ---- Meeting list ----

  test('seeded meeting appears in history list', async ({ page }) => {
    await seedMeeting(page);
    await page.reload();
    await page.locator('[title="History"]').click();
    await expect(page.locator('.meeting-card')).toBeVisible();
    await expect(page.locator('.meeting-card h4')).toContainText(SEED_MEETING.title);
  });

  test('meeting card shows date and duration', async ({ page }) => {
    await seedMeeting(page);
    await page.reload();
    await page.locator('[title="History"]').click();
    const card = page.locator('.meeting-card').first();
    // Date is formatted by toLocaleDateString(), check for separator
    await expect(card.locator('p')).toContainText('·');
  });

  test('multiple seeded meetings all appear', async ({ page }) => {
    const m1 = { ...SEED_MEETING, id: '111', title: 'Meeting Alpha' };
    const m2 = { ...SEED_MEETING, id: '222', title: 'Meeting Beta' };
    await seedMeeting(page, m1);
    await seedMeeting(page, m2);
    await page.reload();
    await page.locator('[title="History"]').click();
    await expect(page.locator('.meeting-card')).toHaveCount(2);
  });

  // ---- View meeting ----

  test('clicking a meeting card opens the view-meeting page', async ({ page }) => {
    await seedMeeting(page);
    await page.reload();
    await page.locator('[title="History"]').click();
    await page.locator('.meeting-card').first().click();
    await expect(page.locator('h2')).toContainText(SEED_MEETING.title);
  });

  test('view-meeting page shows transcript', async ({ page }) => {
    await seedMeeting(page);
    await page.reload();
    await page.locator('[title="History"]').click();
    await page.locator('.meeting-card').first().click();
    await expect(page.locator('.transcript-section')).toBeVisible();
    await expect(page.locator('.transcript-section h3')).toContainText('Transcript');
  });

  test('view-meeting page shows summary when present', async ({ page }) => {
    await seedMeeting(page); // SEED_MEETING has a summary
    await page.reload();
    await page.locator('[title="History"]').click();
    await page.locator('.meeting-card').first().click();
    await expect(page.locator('.summary-card')).toBeVisible();
    await expect(page.locator('.summary-card h3')).toContainText('Summary');
  });

  test('view-meeting page has Ask about this meeting section', async ({ page }) => {
    await seedMeeting(page);
    await page.reload();
    await page.locator('[title="History"]').click();
    await page.locator('.meeting-card').first().click();
    await expect(page.locator('.ask-section')).toBeVisible();
    await expect(page.locator('.ask-section h3')).toContainText('Ask about this meeting');
  });

  test('ask-section input accepts text', async ({ page }) => {
    await seedMeeting(page);
    await page.reload();
    await page.locator('[title="History"]').click();
    await page.locator('.meeting-card').first().click();
    const input = page.locator('.ask-input-row input');
    await input.fill('What were the action items?');
    await expect(input).toHaveValue('What were the action items?');
  });

  // ---- Delete from history page ----

  test('delete icon in history list removes the meeting', async ({ page }) => {
    await seedMeeting(page);
    await page.reload();
    await page.locator('[title="History"]').click();
    await expect(page.locator('.meeting-card')).toBeVisible();

    // Click the trash icon button (not the card itself)
    await page.locator('.meeting-card .btn-icon').click();

    // List is now empty
    await expect(page.locator('.empty-state')).toBeVisible();
    await expect(page.locator('.meeting-card')).toHaveCount(0);
  });

  test('delete removes the meeting from localStorage', async ({ page }) => {
    await seedMeeting(page);
    await page.reload();
    await page.locator('[title="History"]').click();
    await page.locator('.meeting-card .btn-icon').click();

    const stored = await page.evaluate((key) => {
      return JSON.parse(localStorage.getItem(key) || '[]');
    }, MEETINGS_STORAGE);
    expect(stored.length).toBe(0);
  });

  test('delete from view-meeting page returns to history', async ({ page }) => {
    await seedMeeting(page);
    await page.reload();
    await page.locator('[title="History"]').click();
    await page.locator('.meeting-card').first().click();
    await expect(page.locator('h2')).toContainText(SEED_MEETING.title);

    // Delete button in view-meeting
    await page.locator('button', { hasText: 'Delete' }).click();

    // Should land back on history with empty state
    await expect(page.locator('h2')).toContainText('Past Meetings');
    await expect(page.locator('.empty-state')).toBeVisible();
  });

  test('delete from view-meeting removes item from localStorage', async ({ page }) => {
    await seedMeeting(page);
    await page.reload();
    await page.locator('[title="History"]').click();
    await page.locator('.meeting-card').first().click();
    await page.locator('button', { hasText: 'Delete' }).click();

    const stored = await page.evaluate((key) => {
      return JSON.parse(localStorage.getItem(key) || '[]');
    }, MEETINGS_STORAGE);
    expect(stored.length).toBe(0);
  });
});
