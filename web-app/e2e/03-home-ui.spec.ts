/**
 * Home page UI tests — audio mode selector, meeting context textarea,
 * empty states, and the setup banner.
 */
import { test, expect } from '@playwright/test';
import { clearStorage, seedApiKeys } from './helpers';

test.describe('Home page UI', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await clearStorage(page);
    await page.reload();
  });

  // ---- Audio mode selector ----

  test('audio mode selector renders all three options', async ({ page }) => {
    await expect(page.locator('.audio-mode-btn', { hasText: 'Mic only' })).toBeVisible();
    await expect(page.locator('.audio-mode-btn', { hasText: 'Tab audio' })).toBeVisible();
    await expect(page.locator('.audio-mode-btn', { hasText: 'Mic + Tab' })).toBeVisible();
  });

  test('"Mic + Tab" is active by default', async ({ page }) => {
    await expect(
      page.locator('.audio-mode-btn.active', { hasText: 'Mic + Tab' }),
    ).toBeVisible();
  });

  test('selecting "Mic only" activates it and removes active from others', async ({ page }) => {
    await page.locator('.audio-mode-btn', { hasText: 'Mic only' }).click();
    await expect(page.locator('.audio-mode-btn.active')).toHaveCount(1);
    await expect(
      page.locator('.audio-mode-btn.active', { hasText: 'Mic only' }),
    ).toBeVisible();
  });

  test('selecting "Tab audio" activates it', async ({ page }) => {
    await page.locator('.audio-mode-btn', { hasText: 'Tab audio' }).click();
    await expect(
      page.locator('.audio-mode-btn.active', { hasText: 'Tab audio' }),
    ).toBeVisible();
  });

  test('tab-audio hint appears when "Tab audio" selected', async ({ page }) => {
    await page.locator('.audio-mode-btn', { hasText: 'Tab audio' }).click();
    await expect(page.locator('.audio-mode-info')).toBeVisible();
    await expect(page.locator('.audio-mode-info')).toContainText('Chrome Tab');
  });

  test('tab-audio hint appears when "Mic + Tab" selected', async ({ page }) => {
    // Default is Mic + Tab, so hint should already be visible
    await expect(page.locator('.audio-mode-info')).toBeVisible();
  });

  test('mic-only info shows microphone description', async ({ page }) => {
    await page.locator('.audio-mode-btn', { hasText: 'Mic only' }).click();
    await expect(page.locator('.audio-mode-info')).toBeVisible();
    await expect(page.locator('.audio-mode-info')).toContainText('microphone');
  });

  test('can cycle through all three audio modes', async ({ page }) => {
    const modes = ['Mic only', 'Tab audio', 'Mic + Tab'];
    for (const mode of modes) {
      await page.locator('.audio-mode-btn', { hasText: mode }).click();
      await expect(
        page.locator('.audio-mode-btn.active', { hasText: mode }),
      ).toBeVisible();
    }
  });

  // ---- Meeting context textarea ----

  test('context textarea is present with placeholder', async ({ page }) => {
    const ta = page.locator('.context-input');
    await expect(ta).toBeVisible();
    await expect(ta).toHaveAttribute('placeholder', /Describe the meeting context/);
  });

  test('typing in context textarea updates its value', async ({ page }) => {
    const ta = page.locator('.context-input');
    await ta.fill('Sprint planning with the backend team');
    await expect(ta).toHaveValue('Sprint planning with the backend team');
  });

  test('clearing context textarea leaves it empty', async ({ page }) => {
    const ta = page.locator('.context-input');
    await ta.fill('some text');
    await ta.fill('');
    await expect(ta).toHaveValue('');
  });

  // ---- Setup banner / empty state ----

  test('setup banner is visible when no API keys are stored', async ({ page }) => {
    await expect(page.locator('.setup-banner')).toBeVisible();
    await expect(page.locator('.setup-banner')).toContainText('Set up your free API keys');
  });

  test('no recent meetings section when history is empty', async ({ page }) => {
    await expect(page.locator('.recent-meetings')).not.toBeVisible();
  });

  test('Start Meeting button is visible', async ({ page }) => {
    await expect(page.locator('.start-btn')).toBeVisible();
    await expect(page.locator('.start-btn')).toContainText('Start Meeting');
  });

  // ---- Feature cards ----

  test('three feature cards are displayed', async ({ page }) => {
    await expect(page.locator('.feature')).toHaveCount(3);
  });
});
