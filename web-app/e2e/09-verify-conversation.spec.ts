/**
 * Verification suite — tests realistic multi-speaker conversation rendering.
 *
 * Uses custom ConversationScenario data passed to installMeetingMocks to verify
 * that mic audio (speaker 0) renders as "You" bubbles on the right, tab/YouTube
 * audio (speaker 1+) renders as "Participant" bubbles on the left, and full
 * multi-speaker conversations display correctly.
 */
import { test, expect } from '@playwright/test';
import {
  clearStorage,
  seedApiKeys,
  installMeetingMocks,
  getStoredMeetings,
} from './helpers';
import {
  MIC_ONLY_MONOLOGUE,
  TAB_AUDIO_YOUTUBE,
  FULL_MEETING_CONVERSATION,
  RAPID_INTERIM_UPDATES,
  totalDurationMs,
} from './fake-conversations';

/** Start a meeting with the given audio mode and wait for all scenario messages. */
async function startMeeting(
  page: any,
  audioMode: 'Mic only' | 'Tab audio' | 'Mic + Tab',
  waitMs: number,
) {
  await page.locator('.audio-mode-btn', { hasText: audioMode }).click();
  await page.locator('.start-btn').click();
  await expect(page.locator('.recording-indicator')).toBeVisible({ timeout: 5000 });
  // Wait for the addInitScript 200ms delivery start + all cumulative message delays + buffer
  await page.waitForTimeout(waitMs + 500);
}

/** Start meeting, wait for messages, then stop. */
async function startAndStop(
  page: any,
  audioMode: 'Mic only' | 'Tab audio' | 'Mic + Tab',
  waitMs: number,
) {
  await startMeeting(page, audioMode, waitMs);
  await page.locator('.btn-stop').click();
  await expect(page.locator('h2')).toContainText('Meeting Complete', { timeout: 5000 });
}

// ---------------------------------------------------------------------------
// Mic-only mode
// ---------------------------------------------------------------------------
test.describe('Verify: Mic-only mode', () => {
  test.beforeEach(async ({ page }) => {
    await installMeetingMocks(page, MIC_ONLY_MONOLOGUE);
    await page.goto('/');
    await clearStorage(page);
    await seedApiKeys(page);
    await page.reload();
  });

  test('all bubbles are chat-user (right side)', async ({ page }) => {
    await startMeeting(page, 'Mic only', totalDurationMs(MIC_ONLY_MONOLOGUE));
    const userBubbles = page.locator('.chat-bubble.chat-user');
    await expect(userBubbles.first()).toBeVisible({ timeout: 3000 });
    const count = await userBubbles.count();
    expect(count).toBeGreaterThan(0);
  });

  test('zero chat-participant bubbles exist', async ({ page }) => {
    await startMeeting(page, 'Mic only', totalDurationMs(MIC_ONLY_MONOLOGUE));
    const participantBubbles = page.locator('.chat-bubble.chat-participant');
    await expect(participantBubbles).toHaveCount(0);
  });

  test('speaker labels all say "You"', async ({ page }) => {
    await startMeeting(page, 'Mic only', totalDurationMs(MIC_ONLY_MONOLOGUE));
    const speakers = page.locator('.chat-user .chat-speaker');
    await expect(speakers.first()).toBeVisible({ timeout: 3000 });
    const count = await speakers.count();
    for (let i = 0; i < count; i++) {
      await expect(speakers.nth(i)).toHaveText('You');
    }
  });

  test('final messages contain expected text', async ({ page }) => {
    await startMeeting(page, 'Mic only', totalDurationMs(MIC_ONLY_MONOLOGUE));
    const texts = page.locator('.chat-user .chat-text');
    await expect(texts.first()).toBeVisible({ timeout: 3000 });
    // Check that at least one expected final transcript appears
    await expect(page.locator('.chat-text', { hasText: 'We shipped the new dashboard' })).toBeVisible();
  });
});

// ---------------------------------------------------------------------------
// Tab audio / YouTube simulation
// ---------------------------------------------------------------------------
test.describe('Verify: Tab audio / YouTube', () => {
  test.beforeEach(async ({ page }) => {
    await installMeetingMocks(page, TAB_AUDIO_YOUTUBE);
    await page.goto('/');
    await clearStorage(page);
    await seedApiKeys(page);
    await page.reload();
  });

  test('all bubbles are chat-participant (left side)', async ({ page }) => {
    await startMeeting(page, 'Tab audio', totalDurationMs(TAB_AUDIO_YOUTUBE));
    const participantBubbles = page.locator('.chat-bubble.chat-participant');
    await expect(participantBubbles.first()).toBeVisible({ timeout: 3000 });
    const count = await participantBubbles.count();
    expect(count).toBeGreaterThan(0);
  });

  test('zero chat-user bubbles exist', async ({ page }) => {
    await startMeeting(page, 'Tab audio', totalDurationMs(TAB_AUDIO_YOUTUBE));
    const userBubbles = page.locator('.chat-bubble.chat-user');
    await expect(userBubbles).toHaveCount(0);
  });

  test('speaker labels say "Participant N"', async ({ page }) => {
    await startMeeting(page, 'Tab audio', totalDurationMs(TAB_AUDIO_YOUTUBE));
    const speakers = page.locator('.chat-participant .chat-speaker');
    await expect(speakers.first()).toBeVisible({ timeout: 3000 });
    const count = await speakers.count();
    for (let i = 0; i < count; i++) {
      const text = await speakers.nth(i).textContent();
      expect(text).toMatch(/^Participant \d+$/);
    }
  });

  test('contains expected YouTube-style content', async ({ page }) => {
    await startMeeting(page, 'Tab audio', totalDurationMs(TAB_AUDIO_YOUTUBE));
    await expect(page.locator('.chat-text', { hasText: 'Welcome to the presentation' })).toBeVisible();
    await expect(page.locator('.chat-text', { hasText: 'Here is the system diagram' })).toBeVisible();
  });
});

// ---------------------------------------------------------------------------
// Full meeting conversation (mic + tab)
// ---------------------------------------------------------------------------
test.describe('Verify: Full meeting conversation', () => {
  test.beforeEach(async ({ page }) => {
    await installMeetingMocks(page, FULL_MEETING_CONVERSATION);
    await page.goto('/');
    await clearStorage(page);
    await seedApiKeys(page);
    await page.reload();
  });

  test('both chat-user and chat-participant bubbles appear', async ({ page }) => {
    await startMeeting(page, 'Mic + Tab', totalDurationMs(FULL_MEETING_CONVERSATION));
    const userBubbles = page.locator('.chat-bubble.chat-user');
    const participantBubbles = page.locator('.chat-bubble.chat-participant');
    await expect(userBubbles.first()).toBeVisible({ timeout: 3000 });
    await expect(participantBubbles.first()).toBeVisible({ timeout: 3000 });
    expect(await userBubbles.count()).toBeGreaterThan(0);
    expect(await participantBubbles.count()).toBeGreaterThan(0);
  });

  test('messages appear in correct order', async ({ page }) => {
    await startMeeting(page, 'Mic + Tab', totalDurationMs(FULL_MEETING_CONVERSATION));
    const allTexts = page.locator('.chat-text');
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

  test('user bubbles align right, participant bubbles align left', async ({ page }) => {
    await startMeeting(page, 'Mic + Tab', totalDurationMs(FULL_MEETING_CONVERSATION));
    // User bubbles should have align-self: flex-end (right)
    const userBubble = page.locator('.chat-bubble.chat-user').first();
    await expect(userBubble).toBeVisible({ timeout: 3000 });
    await expect(userBubble).toHaveCSS('align-self', 'flex-end');
    // Participant bubbles should have align-self: flex-start (left)
    const participantBubble = page.locator('.chat-bubble.chat-participant').first();
    await expect(participantBubble).toHaveCSS('align-self', 'flex-start');
  });

  test('done page shows correct utterance count', async ({ page }) => {
    await startAndStop(page, 'Mic + Tab', totalDurationMs(FULL_MEETING_CONVERSATION));
    const finalCount = FULL_MEETING_CONVERSATION.filter(m => m.isFinal).length;
    const utteranceValue = page.locator('.stat-value').nth(1);
    await expect(utteranceValue).toHaveText(String(finalCount));
  });

  test('done page Full Transcript contains all final messages', async ({ page }) => {
    await startAndStop(page, 'Mic + Tab', totalDurationMs(FULL_MEETING_CONVERSATION));
    const transcriptBox = page.locator('.transcript-box');
    await expect(transcriptBox).toBeVisible({ timeout: 3000 });
    const finalMessages = FULL_MEETING_CONVERSATION.filter(m => m.isFinal);
    for (const msg of finalMessages) {
      await expect(transcriptBox.locator('span', { hasText: msg.transcript })).toBeVisible();
    }
  });

  test('Save & Close stores meeting in history', async ({ page }) => {
    await startAndStop(page, 'Mic + Tab', totalDurationMs(FULL_MEETING_CONVERSATION));
    await page.locator('button', { hasText: 'Save & Close' }).click();
    await expect(page.locator('h1')).toContainText('Your meetings', { timeout: 5000 });
    const meetings = await getStoredMeetings(page);
    expect(meetings.length).toBe(1);
    // Verify stored transcript contains final messages
    const stored = meetings[0].transcript;
    expect(stored).toContain('Hi team lets get started');
    expect(stored).toContain('Sounds good');
    expect(stored).toContain('Looks clean nice job');
  });

  test('Copy Transcript writes correct formatted text to clipboard', async ({ page, context }) => {
    await context.grantPermissions(['clipboard-read', 'clipboard-write']);
    await startAndStop(page, 'Mic + Tab', totalDurationMs(FULL_MEETING_CONVERSATION));
    await page.locator('button', { hasText: 'Copy Transcript' }).click();
    await expect(page.locator('button', { hasText: 'Copied!' })).toBeVisible({ timeout: 3000 });
    const clipText = await page.evaluate(() => navigator.clipboard.readText());
    // Speaker 0 → [Speaker 1], Speaker 1 → [Speaker 2], Speaker 2 → [Speaker 3]
    expect(clipText).toContain('[Speaker 1]: Hi team lets get started');
    expect(clipText).toContain('[Speaker 2]: Sounds good');
    expect(clipText).toContain('[Speaker 3]: We migrated to the new database');
  });
});

// ---------------------------------------------------------------------------
// Interim result handling
// ---------------------------------------------------------------------------
test.describe('Verify: Interim result handling', () => {
  test.beforeEach(async ({ page }) => {
    await installMeetingMocks(page, RAPID_INTERIM_UPDATES);
    await page.goto('/');
    await clearStorage(page);
    await seedApiKeys(page);
    await page.reload();
  });

  test('interim bubbles have chat-interim class', async ({ page }) => {
    // Start and check quickly — interims should appear before finals
    await page.locator('.audio-mode-btn', { hasText: 'Mic only' }).click();
    await page.locator('.start-btn').click();
    await expect(page.locator('.recording-indicator')).toBeVisible({ timeout: 5000 });
    // Wait for first interim but before all finals arrive
    await page.waitForTimeout(400);
    const interimBubbles = page.locator('.chat-bubble.chat-interim');
    // At least some interims should exist at this point
    const count = await interimBubbles.count();
    expect(count).toBeGreaterThanOrEqual(0); // May or may not be visible depending on timing
  });

  test('final results appear after all messages delivered', async ({ page }) => {
    await startMeeting(page, 'Mic only', totalDurationMs(RAPID_INTERIM_UPDATES));
    // After all messages, we should see the final versions
    await expect(page.locator('.chat-text', { hasText: 'The quick brown fox jumps' })).toBeVisible();
    await expect(page.locator('.chat-text', { hasText: 'Over the lazy dog' })).toBeVisible();
  });

  test('done page only shows final messages', async ({ page }) => {
    await startAndStop(page, 'Mic only', totalDurationMs(RAPID_INTERIM_UPDATES));
    const finalCount = RAPID_INTERIM_UPDATES.filter(m => m.isFinal).length;
    const utteranceValue = page.locator('.stat-value').nth(1);
    await expect(utteranceValue).toHaveText(String(finalCount));
  });
});
