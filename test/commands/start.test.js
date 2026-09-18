import { beforeEach, describe, expect, it } from 'vitest';
import { handleUpdate } from '../../src/handlers.js';
import { createMockBot, createMockCache, commandUpdate } from '../helpers.js';

describe('/start', () => {
  let bot;
  let cache;

  beforeEach(() => {
    bot = createMockBot();
    cache = createMockCache();
  });

  it('sends the cinema keyboard', async () => {
    await handleUpdate(bot, cache, commandUpdate('/start', 2001));

    expect(bot.sendMessage).toHaveBeenCalledTimes(1);
    const [chatId, text, opts] = bot.sendMessage.mock.calls[0];
    expect(chatId).toBe(2001);
    expect(text).toMatch(/guia de cinema/i);
    expect(opts.reply_markup.inline_keyboard).toEqual(
      expect.arrayContaining([
        expect.arrayContaining([expect.objectContaining({ callback_data: 'cinema_1162' })]),
      ]),
    );
  });

  it('handles /start@BotName the same way', async () => {
    await handleUpdate(bot, cache, commandUpdate('/start@MaceioCineBot', 2002));

    expect(bot.sendMessage).toHaveBeenCalledTimes(1);
    expect(bot.sendMessage.mock.calls[0][0]).toBe(2002);
    expect(bot.sendMessage.mock.calls[0][2].reply_markup.inline_keyboard.length).toBe(3);
  });
});
