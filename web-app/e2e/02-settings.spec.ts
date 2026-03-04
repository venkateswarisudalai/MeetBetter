/**
 * Settings page tests — API key persistence, UI, and validation behaviour.
 */
import { test, expect } from '@playwright/test';
import { clearStorage, seedApiKeys, FAKE_DG_KEY, FAKE_GROQ_KEY, KEYS_STORAGE } from './helpers';

test.describe('Settings — API keys', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await clearStorage(page);
    await page.reload();
    // Navigate to settings
    await page.locator('[title="Settings"]').click();
  });

  test('settings page shows both API key inputs', async ({ page }) => {
    await expect(page.locator('input[placeholder*="Deepgram"]')).toBeVisible();
    await expect(page.locator('input[placeholder*="Groq"]')).toBeVisible();
  });

  test('inputs are type=password (keys are hidden)', async ({ page }) => {
    const dgInput = page.locator('input[placeholder*="Deepgram"]');
    const groqInput = page.locator('input[placeholder*="Groq"]');
    await expect(dgInput).toHaveAttribute('type', 'password');
    await expect(groqInput).toHaveAttribute('type', 'password');
  });

  test('saving keys persists to localStorage and returns to home', async ({ page }) => {
    await page.locator('input[placeholder*="Deepgram"]').fill(FAKE_DG_KEY);
    await page.locator('input[placeholder*="Groq"]').fill(FAKE_GROQ_KEY);
    await page.locator('button', { hasText: 'Save Keys' }).click();

    // Should redirect to home
    await expect(page.locator('h1')).toContainText('Your meetings');

    // Keys persisted in localStorage
    const stored = await page.evaluate((key) => {
      return JSON.parse(localStorage.getItem(key) || '{}');
    }, KEYS_STORAGE);
    expect(stored.deepgram).toBe(FAKE_DG_KEY);
    expect(stored.groq).toBe(FAKE_GROQ_KEY);
  });

  test('setup banner disappears after saving valid keys', async ({ page }) => {
    await page.locator('input[placeholder*="Deepgram"]').fill(FAKE_DG_KEY);
    await page.locator('input[placeholder*="Groq"]').fill(FAKE_GROQ_KEY);
    await page.locator('button', { hasText: 'Save Keys' }).click();
    await expect(page.locator('.setup-banner')).not.toBeVisible();
  });

  test('pre-existing keys are loaded into settings inputs', async ({ page }) => {
    // Seed keys directly, navigate back to settings
    await seedApiKeys(page, 'deepgram_test_key_xyz', 'groq_test_key_abc');
    await page.reload();
    await page.locator('[title="Settings"]').click();

    // Values appear (obfuscated but readable via .inputValue())
    const dgVal = await page.locator('input[placeholder*="Deepgram"]').inputValue();
    expect(dgVal).toBe('deepgram_test_key_xyz');
    const groqVal = await page.locator('input[placeholder*="Groq"]').inputValue();
    expect(groqVal).toBe('groq_test_key_abc');
  });

  test('saving empty keys stores empty strings', async ({ page }) => {
    // Clear both and save
    await page.locator('input[placeholder*="Deepgram"]').fill('');
    await page.locator('input[placeholder*="Groq"]').fill('');
    await page.locator('button', { hasText: 'Save Keys' }).click();

    const stored = await page.evaluate((key) => {
      return JSON.parse(localStorage.getItem(key) || '{}');
    }, KEYS_STORAGE);
    expect(stored.deepgram).toBe('');
    expect(stored.groq).toBe('');
  });

  test('saving empty keys shows setup banner on home', async ({ page }) => {
    await page.locator('input[placeholder*="Deepgram"]').fill('');
    await page.locator('input[placeholder*="Groq"]').fill('');
    await page.locator('button', { hasText: 'Save Keys' }).click();
    await expect(page.locator('.setup-banner')).toBeVisible();
  });

  test('settings page shows privacy note', async ({ page }) => {
    await expect(page.locator('.settings-note')).toContainText('stored locally');
  });
});
