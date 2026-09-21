#!/usr/bin/env node
/**
 * AWS Lambda entry point — Telegram webhook mode.
 *
 * Replaces polling (`src/bot.ts`) with API Gateway HTTP API + Lambda.
 * Telegram POSTs to the API Gateway URL → this Lambda processes the update
 * via `handleUpdate()` (awaited until replies are sent).
 *
 * Exported handlers:
 *   - handler()       → invoked by API Gateway (webhook)
 *   - fetchHandler()  → invoked by EventBridge (daily cron, fetch + S3 cache update)
 */

import TelegramBot from 'node-telegram-bot-api';
import { config } from 'dotenv';
import NormalizedCache from './cache.js';
import { CINEMAS, loadPrefs } from './cinemas.js';
import { handleUpdate } from './handlers.js';
import { fetchNormalized, fetchUpcoming } from './api.js';
import type { ApiGatewayEvent, BotLike, LambdaHttpResult, TelegramUpdate } from './types.js';
import { errorMessage } from './types.js';

config();

const cache = new NormalizedCache();
let bot: TelegramBot | undefined;
let commandsSet = false;

function getBot(): TelegramBot {
  if (!bot) {
    const token = process.env.TELEGRAM_BOT_TOKEN;
    if (!token) {
      throw new Error('TELEGRAM_BOT_TOKEN is not set');
    }
    bot = new TelegramBot(token, { polling: false });
  }
  return bot;
}

async function setCommandsOnce(): Promise<void> {
  if (commandsSet) return;
  commandsSet = true;
  try {
    await getBot().setMyCommands([
      { command: 'start', description: 'Iniciar o bot e escolher cinema' },
      { command: 'hoje', description: 'Filmes em cartaz no cinema selecionado' },
      { command: 'proximos', description: 'Lançamentos futuros e pré-vendas' },
      { command: 'cinemas', description: 'Trocar de cinema selecionado' },
    ]);
    console.log('✅ Command menu configured');
  } catch (err) {
    commandsSet = false;
    console.error('❌ Failed to configure command menu:', errorMessage(err));
  }
}

export async function handler(event: ApiGatewayEvent): Promise<LambdaHttpResult> {
  try {
    await cache.load();
    await loadPrefs();
    await setCommandsOnce();

    const body: TelegramUpdate | null | undefined =
      typeof event.body === 'string' ? (JSON.parse(event.body) as TelegramUpdate) : event.body;
    await handleUpdate(getBot() as unknown as BotLike, cache, body);
    return {
      statusCode: 200,
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ok: true }),
    };
  } catch (err) {
    console.error('❌ Handler error:', errorMessage(err));
    return {
      statusCode: 500,
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ok: false, error: errorMessage(err) }),
    };
  }
}

export async function fetchHandler(): Promise<LambdaHttpResult> {
  await cache.load();

  const webhookUrl = process.env.WEBHOOK_URL;
  if (webhookUrl) {
    try {
      await getBot().setWebHook(webhookUrl);
      console.log(`✅ Webhook set: ${webhookUrl}`);
    } catch (err) {
      console.warn('⚠️  Failed to set webhook:', errorMessage(err));
    }
  }

  await setCommandsOnce();

  let totalMovies = 0;

  for (const cinema of CINEMAS) {
    const theaterId = cinema.id;
    try {
      const normalized = await fetchNormalized(null, theaterId);
      cache.mergeMovies(normalized.movies);
      await cache.setSessions(
        normalized.date ?? '',
        normalized.sessions,
        normalized.fetchedAt,
        theaterId,
      );
      totalMovies += Object.keys(normalized.movies).length;
    } catch (err) {
      console.error(`❌ Failed to update sessions for theater ${theaterId}:`, errorMessage(err));
    }

    try {
      const result = await fetchUpcoming(theaterId);
      await cache.setUpcoming(result.items, result.fetchedAt, theaterId);
    } catch (err) {
      console.error(`❌ Failed to update upcoming for theater ${theaterId}:`, errorMessage(err));
    }
  }

  console.log(`✅ Cache warming complete — ${totalMovies} movies processed`);
  return {
    statusCode: 200,
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ ok: true, totalMovies }),
  };
}
