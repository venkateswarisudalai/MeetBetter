/**
 * Verification suite — tests realistic conversation rendering in the desktop app.
 *
 * Mocks the Tauri IPC layer so the React app runs in a plain Chromium browser.
 * Emits fake `transcript-update` events to simulate the Rust backend and verifies
 * that "You" bubbles render on the right, "Participant" on the left, interims
 * replace correctly, and done state shows the full transcript.
 */
import { test, expect } from '@playwright/test';
import {
  installTauriMocks,
  emitConversation,
  startMeeting,
  stopMeeting,
} from './helpers';
import {
  MIC_ONLY_MONOLOGUE,
  PARTICIPANT_ONLY,
  FULL_MEETING,
  RAPID_INTERIM,
  totalDurationMs,
} from './fake-conversations';

// ---------------------------------------------------------------------------
// Setup: install Tauri mocks before each test
// ---------------------------------------------------------------------------
test.beforeEach(async ({ page }) => {
  await installTauriMocks(page);
  await page.goto('/');
  // Wait for the app to be in ready state
  await page.waitForSelector('.app-minimal.ready', { timeout: 10000 });
});

// ---------------------------------------------------------------------------
// You-only mode (MIC_ONLY_MONOLOGUE)
// ---------------------------------------------------------------------------
test.describe('Verify: You-only mode', () => {
  test('all bubbles are .transcript-item.you (right side)', async ({ page }) => {
    await startMeeting(page);
    await emitConversation(page, MIC_ONLY_MONOLOGUE);
    // Wait for at least one bubble to appear
    await expect(page.locator('.transcript-item.you')).not.toHaveCount(0, { timeout: 3000 });
    const count = await page.locator('.transcript-item.you').count();
    expect(count).toBeGreaterThan(0);
  });

  test('zero .transcript-item.participant bubbles exist', async ({ page }) => {
    await startMeeting(page);
    await emitConversation(page, MIC_ONLY_MONOLOGUE);
    await expect(page.locator('.transcript-item.you').first()).toBeVisible({ timeout: 3000 });
    await expect(page.locator('.transcript-item.participant')).toHaveCount(0);
  });

  test('speaker labels all say "You"', async ({ page }) => {
    await startMeeting(page);
    await emitConversation(page, MIC_ONLY_MONOLOGUE);
    const speakers = page.locator('.transcript-item.you .speaker-label');
    await expect(speakers.first()).toBeVisible({ timeout: 3000 });
    const count = await speakers.count();
    for (let i = 0; i < count; i++) {
      await expect(speakers.nth(i)).toHaveText('You');
    }
  });

  test('final messages contain expected text', async ({ page }) => {
    await startMeeting(page);
    await emitConversation(page, MIC_ONLY_MONOLOGUE);
    await expect(page.locator('.transcript-item.you').first()).toBeVisible({ timeout: 3000 });
    await expect(page.locator('.message-bubble p', { hasText: 'We shipped the new dashboard' })).toBeVisible();
  });
});

// ---------------------------------------------------------------------------
// Participant-only mode (PARTICIPANT_ONLY)
// ---------------------------------------------------------------------------
test.describe('Verify: Participant-only mode', () => {
  test('all bubbles are .transcript-item.participant (left side)', async ({ page }) => {
    await startMeeting(page);
    await emitConversation(page, PARTICIPANT_ONLY);
    await expect(page.locator('.transcript-item.participant')).not.toHaveCount(0, { timeout: 3000 });
    const count = await page.locator('.transcript-item.participant').count();
    expect(count).toBeGreaterThan(0);
  });

  test('zero .transcript-item.you bubbles exist', async ({ page }) => {
    await startMeeting(page);
    await emitConversation(page, PARTICIPANT_ONLY);
    await expect(page.locator('.transcript-item.participant').first()).toBeVisible({ timeout: 3000 });
    await expect(page.locator('.transcript-item.you')).toHaveCount(0);
  });

  test('speaker labels say "Participant"', async ({ page }) => {
    await startMeeting(page);
    await emitConversation(page, PARTICIPANT_ONLY);
    const speakers = page.locator('.transcript-item.participant .speaker-label');
    await expect(speakers.first()).toBeVisible({ timeout: 3000 });
    const count = await speakers.count();
    for (let i = 0; i < count; i++) {
      await expect(speakers.nth(i)).toHaveText('Participant');
    }
  });

  test('contains expected content', async ({ page }) => {
    await startMeeting(page);
    await emitConversation(page, PARTICIPANT_ONLY);
    await expect(page.locator('.message-bubble p', { hasText: 'Welcome to the presentation' })).toBeVisible({ timeout: 3000 });
    await expect(page.locator('.message-bubble p', { hasText: 'Here is the system diagram' })).toBeVisible();
  });
});

// ---------------------------------------------------------------------------
// Full meeting (FULL_MEETING) — both You + Participant
// ---------------------------------------------------------------------------
test.describe('Verify: Full meeting conversation', () => {
  test('both .you and .participant bubbles appear', async ({ page }) => {
    await startMeeting(page);
    await emitConversation(page, FULL_MEETING);
    await expect(page.locator('.transcript-item.you').first()).toBeVisible({ timeout: 3000 });
    await expect(page.locator('.transcript-item.participant').first()).toBeVisible({ timeout: 3000 });
    expect(await page.locator('.transcript-item.you').count()).toBeGreaterThan(0);
    expect(await page.locator('.transcript-item.participant').count()).toBeGreaterThan(0);
  });

  test('messages appear in correct order', async ({ page }) => {
    await startMeeting(page);
    await emitConversation(page, FULL_MEETING);
    const allTexts = page.locator('.message-bubble p');
    await expect(allTexts.first()).toBeVisible({ timeout: 3000 });
    const texts: string[] = [];
    const count = await allTexts.count();
    for (let i = 0; i < count; i++) {
      texts.push((await allTexts.nth(i).textContent()) || '');
    }
    // The final messages (in order) should include these key phrases
    const expectedOrder = [
      'Hi team lets get started',
      'Sounds good',
      'We migrated to the new database',
      'Nice work on that migration',
      'No issues everything is stable',
      'Looks clean nice job',
    ];
    let lastIndex = -1;
    for (const expected of expectedOrder) {
      const idx = texts.findIndex((t, i) => i > lastIndex && t.includes(expected));
      expect(idx).toBeGreaterThan(lastIndex);
      lastIndex = idx;
    }
  });

  test('You bubbles align right, Participant bubbles align left', async ({ page }) => {
    await startMeeting(page);
    await emitConversation(page, FULL_MEETING);
    // You bubbles should have align-self: flex-end (right)
    const youBubble = page.locator('.transcript-item.you').first();
    await expect(youBubble).toBeVisible({ timeout: 3000 });
    await expect(youBubble).toHaveCSS('align-self', 'flex-end');
    // Participant bubbles should have align-self: flex-start (left)
    const participantBubble = page.locator('.transcript-item.participant').first();
    await expect(participantBubble).toHaveCSS('align-self', 'flex-start');
  });

  test('Stop -> done state shows transcript in collapsible details', async ({ page }) => {
    await startMeeting(page);
    await emitConversation(page, FULL_MEETING);
    await stopMeeting(page);
    // Done state should have the collapsible Full Transcript
    const details = page.locator('.transcript-section details');
    await expect(details).toBeVisible({ timeout: 3000 });
    // Click to open
    await details.locator('summary').click();
    // Verify segment count in the heading
    const finalCount = FULL_MEETING.filter(m => m.is_final).length;
    await expect(details.locator('summary h3')).toContainText(`${finalCount} segments`);
  });

  test('done page transcript has correct segment count', async ({ page }) => {
    await startMeeting(page);
    await emitConversation(page, FULL_MEETING);
    await stopMeeting(page);
    const details = page.locator('.transcript-section details');
    await details.locator('summary').click();
    // Count the actual rendered transcript items in done view
    const items = details.locator('.transcript-item');
    const finalCount = FULL_MEETING.filter(m => m.is_final).length;
    await expect(items).toHaveCount(finalCount);
  });

  test('done page transcript contains all final messages', async ({ page }) => {
    await startMeeting(page);
    await emitConversation(page, FULL_MEETING);
    await stopMeeting(page);
    const details = page.locator('.transcript-section details');
    await details.locator('summary').click();
    const finalMessages = FULL_MEETING.filter(m => m.is_final);
    for (const msg of finalMessages) {
      await expect(details.locator('.text', { hasText: msg.text })).toBeVisible();
    }
  });

  test('Save Meeting stores and returns to home', async ({ page }) => {
    await startMeeting(page);
    await emitConversation(page, FULL_MEETING);
    await stopMeeting(page);
    // Click "Save Meeting" to open modal
    await page.locator('button', { hasText: 'Save Meeting' }).click();
    await expect(page.locator('.save-meeting-modal')).toBeVisible({ timeout: 3000 });
    // Fill title and save
    await page.locator('.save-meeting-modal input').fill('Test Meeting');
    await page.locator('.save-meeting-modal .primary-btn').click();
    // Should return to home
    await expect(page.locator('.app-minimal.ready')).toBeVisible({ timeout: 5000 });
  });
});

// ---------------------------------------------------------------------------
// Interim handling (RAPID_INTERIM)
// ---------------------------------------------------------------------------
test.describe('Verify: Interim result handling', () => {
  test('interim bubbles have .interim class with reduced opacity', async ({ page }) => {
    await startMeeting(page);
    // Emit just the first interim
    await page.evaluate(() => {
      (window as any).__TAURI_TEST_EMIT__('transcript-update', {
        text: 'The',
        timestamp: new Date().toISOString(),
        speaker: 'You',
        is_final: false,
      });
    });
    const interimItem = page.locator('.transcript-item.interim');
    await expect(interimItem.first()).toBeVisible({ timeout: 3000 });
    // Verify reduced opacity via CSS
    await expect(interimItem.first()).toHaveCSS('opacity', '0.7');
  });

  test('interim has interim-badge indicator', async ({ page }) => {
    await startMeeting(page);
    await page.evaluate(() => {
      (window as any).__TAURI_TEST_EMIT__('transcript-update', {
        text: 'The quick',
        timestamp: new Date().toISOString(),
        speaker: 'You',
        is_final: false,
      });
    });
    await expect(page.locator('.interim-badge')).toBeVisible({ timeout: 3000 });
  });

  test('final replaces interim for same speaker', async ({ page }) => {
    await startMeeting(page);
    await emitConversation(page, RAPID_INTERIM);
    // After all messages delivered, the "You" speaker should have exactly one final
    // and "Participant" should have one final
    await expect(page.locator('.message-bubble p', { hasText: 'The quick brown fox jumps' })).toBeVisible({ timeout: 3000 });
    await expect(page.locator('.message-bubble p', { hasText: 'Over the lazy dog' })).toBeVisible();
    // No interims should remain
    await expect(page.locator('.transcript-item.interim')).toHaveCount(0);
  });

  test('final results show correct text after all messages', async ({ page }) => {
    await startMeeting(page);
    await emitConversation(page, RAPID_INTERIM);
    await expect(page.locator('.message-bubble p', { hasText: 'The quick brown fox jumps' })).toBeVisible({ timeout: 3000 });
    await expect(page.locator('.message-bubble p', { hasText: 'Over the lazy dog' })).toBeVisible();
    // Should only have 2 transcript items (one per speaker, finals only)
    await expect(page.locator('.transcript-item')).toHaveCount(2);
  });
});
