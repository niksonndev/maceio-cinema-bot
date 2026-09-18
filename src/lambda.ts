#!/usr/bin/env node
/**
 * AWS Lambda entry point — Telegram webhook mode.
 *
 * Substitui o polling (`src/bot.ts`) por API Gateway HTTP API + Lambda.
 * O Telegram envia POSTs para a URL do API Gateway → esta Lambda processa
 * o update via `handleUpdate()` (aguardado até as respostas saírem).
 *
 * Handlers exportados:
 *   - handler()       → invocado pelo API Gateway (webhook)
 *   - fetchHandler()  → invocado pelo EventBridge (cron diário, fetch + S3 cache update)
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
      throw new Error('TELEGRAM_BOT_TOKEN não configurado');
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
    console.log('✅ Menu de comandos configurado');
  } catch (err) {
    commandsSet = false;
    console.error('❌ Erro ao configurar menu de comandos:', errorMessage(err));
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
    console.error('❌ Erro no handler:', errorMessage(err));
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
      console.log(`✅ Webhook definido: ${webhookUrl}`);
    } catch (err) {
      console.warn('⚠️  Erro ao definir webhook:', errorMessage(err));
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
      console.error(`❌ Erro ao atualizar sessões do teatro ${theaterId}:`, errorMessage(err));
    }

    try {
      const result = await fetchUpcoming(theaterId);
      await cache.setUpcoming(result.items, result.fetchedAt, theaterId);
    } catch (err) {
      console.error(`❌ Erro ao atualizar lançamentos do teatro ${theaterId}:`, errorMessage(err));
    }
  }

  console.log(`✅ Cache warming concluído — ${totalMovies} filmes processados`);
  return {
    statusCode: 200,
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ ok: true, totalMovies }),
  };
}
