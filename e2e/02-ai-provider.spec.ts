/**
 * Settings → AI provider: choosing Ollama runs summaries and suggestions on this Mac.
 * The Tauri IPC is mocked (see helpers.ts); get_ollama_status reports two installed models.
 */
import { test, expect } from '@playwright/test';
import { installTauriMocks } from './helpers';

const calls = (page: import('@playwright/test').Page, cmd: string) =>
  page.evaluate((c) => ((window as any).__TAURI_TEST_CALLS__ ?? []).filter((x: any) => x.cmd === c).map((x: any) => x.args), cmd);

test.beforeEach(async ({ page }) => {
  await installTauriMocks(page);
  await page.goto('/');
  await page.waitForSelector('.app-minimal.ready', { timeout: 10000 });
  await page.getByTitle('Settings').first().click();
});

test('Groq is the default AI provider', async ({ page }) => {
  await expect(page.getByRole('radio', { name: 'Groq (cloud)' })).toHaveAttribute('aria-checked', 'true');
  await expect(page.getByLabel('Ollama model')).toHaveCount(0);
});

test('choosing Ollama lists installed models and saves the choice', async ({ page }) => {
  await page.getByRole('radio', { name: 'Ollama (on this Mac)' }).click();
  await expect(page.getByRole('radio', { name: 'Ollama (on this Mac)' })).toHaveAttribute('aria-checked', 'true');

  const picker = page.getByLabel('Ollama model');
  await expect(picker).toHaveValue('qwen3:14b'); // best installed model is preselected
  await expect(picker.locator('option')).toHaveText(['llama3.2:latest', 'qwen3:14b']);
  expect(await calls(page, 'set_ai_provider')).toContainEqual({ provider: 'ollama', ollamaModel: '' });

  await picker.selectOption('llama3.2:latest');
  expect(await calls(page, 'set_ai_provider')).toContainEqual({ provider: 'ollama', ollamaModel: 'llama3.2:latest' });
});

test('explains how to set up Ollama when it is not running', async ({ page }) => {
  await page.evaluate(() => {
    const t = (window as any).__TAURI_INTERNALS__;
    const invoke = t.invoke.bind(t);
    t.invoke = (cmd: string, args: any) =>
      cmd === 'get_ollama_status' ? Promise.resolve({ running: false, models: [], chosen: null }) : invoke(cmd, args);
  });
  await page.getByRole('radio', { name: 'Ollama (on this Mac)' }).click();
  await expect(page.getByText("Ollama isn't running")).toBeVisible();
  await expect(page.getByText('ollama pull qwen3:14b')).toBeVisible();
});
