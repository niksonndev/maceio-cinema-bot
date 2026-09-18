import { beforeEach, describe, expect, it } from 'vitest';
import { handleUpdate } from '../../src/handlers.js';
import { clearPrefs, setUserCinema } from '../../src/cinemas.js';
import { createMockBot, createMockCache, commandUpdate } from '../helpers.js';

const CHAT = 5001;

describe('/cinemas', () => {
  let bot: ReturnType<typeof createMockBot>;
  let cache: ReturnType<typeof createMockCache>;

  beforeEach(async () => {
    bot = createMockBot();
    cache = createMockCache();
    await clearPrefs();
  });

  it('asks the user to pick a cinema when none is selected', async () => {
    await handleUpdate(bot, cache, commandUpdate('/cinemas', CHAT));

    const [chatId, text, opts] = bot.sendMessage.mock.calls[0] as [
      number,
      string,
      { parse_mode: string; reply_markup: { inline_keyboard: unknown[] } },
    ];
    expect(chatId).toBe(CHAT);
    expect(text).toMatch(/Escolha o cinema/i);
    expect(opts.parse_mode).toBe('Markdown');
    expect(opts.reply_markup.inline_keyboard).toHaveLength(3);
  });

  it('shows the current cinema label when a preference exists', async () => {
    await setUserCinema(CHAT, '924');
    await handleUpdate(bot, cache, commandUpdate('/cinemas', CHAT));

    const text = bot.sendMessage.mock.calls[0][1] as string;
    expect(text).toMatch(/Cinema atual/);
    expect(text).toMatch(/Kinoplex/);
  });
});
