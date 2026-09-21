#!/usr/bin/env node

/**
 * Telegram bot — local entry point.
 *
 * Initializes the bot, registers handlers, and starts the HTTP health check.
 * Usage: npm run bot:listen
 */

import TelegramBot from 'node-telegram-bot-api';
import express from 'express';
import type { Server } from 'http';
import { config } from 'dotenv';
import NormalizedCache from './cache.js';
import { loadPrefs } from './cinemas.js';
import { registerHandlers } from './handlers.js';
import type { BotLike } from './types.js';
import { errorMessage } from './types.js';

config();

const token = process.env.TELEGRAM_BOT_TOKEN;
if (!token) {
  throw new Error('TELEGRAM_BOT_TOKEN is not set in .env');
}

const bot = new TelegramBot(token, { polling: false });
const cache = new NormalizedCache();
const PORT = Number(process.env.PORT) || 10000;
const app = express();
let server: Server | undefined;

app.get('/', (_req, res) => {
  const mem = process.memoryUsage();
  const memMB = {
    heapUsed: (mem.heapUsed / 1024 / 1024).toFixed(2),
    heapTotal: (mem.heapTotal / 1024 / 1024).toFixed(2),
    rss: (mem.rss / 1024 / 1024).toFixed(2),
  };
  console.log('📡 Health check received', memMB);
  res.json({
    status: '✅ Bot is online!',
    timestamp: new Date().toISOString(),
    memory: memMB,
  });
});

async function deleteWebhookDropPending(): Promise<void> {
  await (bot.deleteWebHook as (opts?: { drop_pending_updates?: boolean }) => Promise<unknown>)({
    drop_pending_updates: true,
  });
}

async function setCommands(): Promise<void> {
  try {
    await bot.setMyCommands([
      { command: 'start', description: 'Iniciar o bot e escolher cinema' },
      { command: 'hoje', description: 'Filmes em cartaz no cinema selecionado' },
      { command: 'proximos', description: 'Lançamentos futuros e pré-vendas' },
      { command: 'cinemas', description: 'Trocar de cinema selecionado' },
    ]);
    console.log('✅ Command menu configured');
  } catch (err) {
    console.error('❌ Failed to configure command menu:', errorMessage(err));
  }
}

let pollingRetries = 0;
const MAX_POLLING_RETRIES = 5;

bot.on('polling_error', (err) => {
  const message = errorMessage(err);
  const code = (err as Error & { code?: string | number }).code;
  console.error('❌ Polling error:', message);

  if (code === 409 || message.includes('terminated by other')) {
    pollingRetries++;
    if (pollingRetries > MAX_POLLING_RETRIES) {
      console.error(
        `💀 Failed after ${MAX_POLLING_RETRIES} attempts. Another instance is still active — shutting down.`,
      );
      shutdown('POLLING_CONFLICT');
      return;
    }
    const delay = Math.min(pollingRetries * 5, 30) * 1000;
    console.log(
      `⏳ Another instance detected (attempt ${pollingRetries}/${MAX_POLLING_RETRIES}). Reconnecting in ${delay / 1000}s...`,
    );
    bot.stopPolling().then(() => {
      setTimeout(async () => {
        try {
          await deleteWebhookDropPending();
          bot.startPolling({ restart: true });
          console.log('🔄 Polling restarted.');
        } catch (retryErr) {
          console.error('❌ Failed to restart polling:', errorMessage(retryErr));
        }
      }, delay);
    });
  }
});

bot.on('polling', () => {
  if (pollingRetries > 0) {
    console.log('✅ Polling restored successfully.');
    pollingRetries = 0;
  }
});

void (async () => {
  await cache.load();
  await loadPrefs();
  await setCommands();
  registerHandlers(bot as unknown as BotLike, cache);

  try {
    await deleteWebhookDropPending();
    console.log('✅ Webhook removed, polling enabled.');
  } catch (err) {
    console.warn('⚠️ Failed to remove webhook:', errorMessage(err));
  }

  bot.startPolling({ restart: true });

  server = app.listen(PORT, '0.0.0.0', () => {
    console.log(`✅ Bot listening on port ${PORT} (host 0.0.0.0)`);
    console.log(`📡 Health check: http://0.0.0.0:${PORT}/`);
  });

  console.log('🚀 Bot started in polling mode (local/dev)...');
  console.log('Waiting for messages. Send /start to begin.');
})();

let shuttingDown = false;
function shutdown(signal: string): void {
  if (shuttingDown) return;
  shuttingDown = true;
  console.log(`\n👋 Shutting down bot (signal: ${signal})...`);
  void bot.stopPolling();
  if (server) {
    server.close(() => {
      console.log('✅ Server closed');
      process.exit(0);
    });
  } else {
    process.exit(0);
  }
}
process.on('SIGINT', () => shutdown('SIGINT'));
process.on('SIGTERM', () => shutdown('SIGTERM'));
