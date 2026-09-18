import { beforeEach, describe, expect, it, vi } from 'vitest';

const { botMocks } = vi.hoisted(() => ({
  botMocks: {
    sendDone: false,
    sendMessage: vi.fn(),
    setMyCommands: vi.fn().mockResolvedValue(true),
    setWebHook: vi.fn().mockResolvedValue(true),
  },
}));

vi.mock('node-telegram-bot-api', () => ({
  default: class TelegramBot {
    constructor() {
      this.sendMessage = botMocks.sendMessage;
      this.setMyCommands = botMocks.setMyCommands;
      this.setWebHook = botMocks.setWebHook;
    }
  },
}));

process.env.TELEGRAM_BOT_TOKEN = process.env.TELEGRAM_BOT_TOKEN || 'test-token';

import { handler } from '../src/lambda.js';
import { commandUpdate } from './helpers.js';

function httpApiEvent(update) {
  return {
    version: '2.0',
    routeKey: 'POST /webhook',
    rawPath: '/webhook',
    body: JSON.stringify(update),
    isBase64Encoded: false,
  };
}

describe('lambda handler', () => {
  beforeEach(() => {
    botMocks.sendDone = false;
    botMocks.sendMessage.mockReset();
    botMocks.sendMessage.mockImplementation(async () => {
      await new Promise((resolve) => setTimeout(resolve, 40));
      botMocks.sendDone = true;
      return { message_id: 1 };
    });
  });

  it('awaits sendMessage before returning 200', async () => {
    const promise = handler(httpApiEvent(commandUpdate('/start', 9001)));
    await new Promise((resolve) => setTimeout(resolve, 10));
    expect(botMocks.sendDone).toBe(false);

    const result = await promise;
    expect(botMocks.sendDone).toBe(true);
    expect(result.statusCode).toBe(200);
    expect(JSON.parse(result.body)).toEqual({ ok: true });
    expect(botMocks.sendMessage).toHaveBeenCalled();
  });

  it('parses a JSON string body from API Gateway', async () => {
    const result = await handler(httpApiEvent(commandUpdate('/start', 9002)));
    expect(result.statusCode).toBe(200);
    expect(botMocks.sendMessage.mock.calls[0][0]).toBe(9002);
  });
});
