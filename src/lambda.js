#!/usr/bin/env node
/**
 * AWS Lambda entry point — Telegram webhook mode.
 *
 * Substitui o polling (`src/bot.js`) por API Gateway HTTP API + Lambda.
 * O Telegram envia POSTs para a URL do API Gateway → esta Lambda processa
 * o update via `handleUpdate()` (aguardado até as respostas saírem).
 *
 * Handlers exportados:
 *   - handler()       → invocado pelo API Gateway (webhook)
 *   - fetchHandler()  → invocado pelo EventBridge (cron diário, fetch + S3 cache update)
 *
 * Uso:
 *   sam build && sam deploy --guided
 *   (ou) npm run bot:listen  ← modo polling local (src/bot.js)
 */

import TelegramBot from 'node-telegram-bot-api';
import { config } from 'dotenv';
import NormalizedCache from './cache.js';
import { CINEMAS, loadPrefs } from './cinemas.js';
import { handleUpdate } from './handlers.js';
import { fetchNormalized, fetchUpcoming } from './api.js';

config();

const cache = new NormalizedCache();
let bot;
let commandsSet = false;

function getBot() {
  if (!bot) {
    const token = process.env.TELEGRAM_BOT_TOKEN;
    if (!token) {
      throw new Error('TELEGRAM_BOT_TOKEN não configurado');
    }
    bot = new TelegramBot(token, { polling: false });
  }
  return bot;
}

async function setCommandsOnce() {
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
    console.error('❌ Erro ao configurar menu de comandos:', err.message);
  }
}

/**
 * Handler principal — invocado pelo API Gateway a cada update do Telegram.
 * Recarrega cache e prefs a cada invoke para sobreviver a cold start / multi-instance.
 *
 * @param {object} event - Evento do API Gateway HTTP API (event.body = JSON string)
 * @returns {object} Resposta HTTP 200/500
 */
export async function handler(event) {
  try {
    await cache.load();
    await loadPrefs();
    await setCommandsOnce();

    const body = typeof event.body === 'string' ? JSON.parse(event.body) : event.body;
    await handleUpdate(getBot(), cache, body);
    return {
      statusCode: 200,
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ok: true }),
    };
  } catch (err) {
    console.error('❌ Erro no handler:', err.message);
    return {
      statusCode: 500,
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ok: false, error: err.message }),
    };
  }
}

/**
 * Handler de fetch + cache update — invocado pelo EventBridge (cron diário).
 * Também registra o webhook do Telegram (WEBHOOK_URL só existe nesta função).
 *
 * @returns {object} Resposta 200
 */
export async function fetchHandler() {
  await cache.load();

  const webhookUrl = process.env.WEBHOOK_URL;
  if (webhookUrl) {
    try {
      await getBot().setWebHook(webhookUrl);
      console.log(`✅ Webhook definido: ${webhookUrl}`);
    } catch (err) {
      console.warn('⚠️  Erro ao definir webhook:', err.message);
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
        normalized.date,
        normalized.sessions,
        normalized.fetchedAt,
        theaterId,
      );
      totalMovies += Object.keys(normalized.movies).length;
    } catch (err) {
      console.error(`❌ Erro ao atualizar sessões do teatro ${theaterId}:`, err.message);
    }

    try {
      const result = await fetchUpcoming(theaterId);
      await cache.setUpcoming(result.items, result.fetchedAt, theaterId);
    } catch (err) {
      console.error(`❌ Erro ao atualizar lançamentos do teatro ${theaterId}:`, err.message);
    }
  }

  console.log(`✅ Cache warming concluído — ${totalMovies} filmes processados`);
  return {
    statusCode: 200,
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ ok: true, totalMovies }),
  };
}
