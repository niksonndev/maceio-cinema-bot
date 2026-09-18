#!/usr/bin/env node

/**
 * Bot Telegram — Entry Point
 *
 * Inicializa o bot, registra handlers e sobe o health-check HTTP.
 * Uso: npm run bot:listen
 */

import TelegramBot from 'node-telegram-bot-api';
import express from 'express';
import { config } from 'dotenv';
import NormalizedCache from './cache.js';
import { loadPrefs } from './cinemas.js';
import { registerHandlers } from './handlers.js';

config();

const token = process.env.TELEGRAM_BOT_TOKEN;
if (!token) {
  throw new Error('TELEGRAM_BOT_TOKEN não configurado no .env');
}

const bot = new TelegramBot(token, { polling: false });
const cache = new NormalizedCache();
const PORT = process.env.PORT || 10000;
const app = express();
let server;

// --- Health check ---

app.get('/', (_req, res) => {
  const mem = process.memoryUsage();
  const memMB = {
    heapUsed: (mem.heapUsed / 1024 / 1024).toFixed(2),
    heapTotal: (mem.heapTotal / 1024 / 1024).toFixed(2),
    rss: (mem.rss / 1024 / 1024).toFixed(2),
  };
  console.log('📡 Health check recebido', memMB);
  res.json({
    status: '✅ Bot está online!',
    timestamp: new Date().toISOString(),
    memory: memMB,
  });
});

// --- Configuração de comandos do Telegram ---

async function setCommands() {
  try {
    await bot.setMyCommands([
      { command: 'start', description: 'Iniciar o bot e escolher cinema' },
      { command: 'hoje', description: 'Filmes em cartaz no cinema selecionado' },
      { command: 'proximos', description: 'Lançamentos futuros e pré-vendas' },
      { command: 'cinemas', description: 'Trocar de cinema selecionado' },
    ]);
    console.log('✅ Menu de comandos configurado');
  } catch (err) {
    console.error('❌ Erro ao configurar menu de comandos:', err.message);
  }
}

// --- Polling error handler ---

let pollingRetries = 0;
const MAX_POLLING_RETRIES = 5;

bot.on('polling_error', (err) => {
  console.error('❌ Erro de polling:', err.message);

  if (err.code === 409 || err.message.includes('terminated by other')) {
    pollingRetries++;
    if (pollingRetries > MAX_POLLING_RETRIES) {
      console.error(
        `💀 Falha após ${MAX_POLLING_RETRIES} tentativas. Outra instância continua ativa — encerrando.`,
      );
      shutdown('POLLING_CONFLICT');
      return;
    }
    const delay = Math.min(pollingRetries * 5, 30) * 1000;
    console.log(
      `⏳ Outra instância detectada (tentativa ${pollingRetries}/${MAX_POLLING_RETRIES}). Reconectando em ${delay / 1000}s...`,
    );
    bot.stopPolling().then(() => {
      setTimeout(async () => {
        try {
          await bot.deleteWebHook({ drop_pending_updates: true });
          bot.startPolling({ restart: true });
          console.log('🔄 Polling reiniciado.');
        } catch (retryErr) {
          console.error('❌ Erro ao reiniciar polling:', retryErr.message);
        }
      }, delay);
    });
  }
});

bot.on('polling', () => {
  if (pollingRetries > 0) {
    console.log('✅ Polling restabelecido com sucesso.');
    pollingRetries = 0;
  }
});

// --- Inicialização ---

(async () => {
  await cache.load();
  await loadPrefs();
  await setCommands();
  registerHandlers(bot, cache);

  try {
    await bot.deleteWebHook({ drop_pending_updates: true });
    console.log('✅ Webhook removido, polling liberado.');
  } catch (err) {
    console.warn('⚠️ Erro ao remover webhook:', err.message);
  }

  bot.startPolling({ restart: true });

  server = app.listen(PORT, '0.0.0.0', () => {
    console.log(`✅ Bot subiu na porta ${PORT} (host 0.0.0.0)`);
    console.log(`📡 Health check: http://0.0.0.0:${PORT}/`);
  });

  console.log('🚀 Bot iniciado em modo polling (local/dev)...');
  console.log('Aguardando mensagens. Envie /start para começar.');
})();

// --- Graceful shutdown ---

let shuttingDown = false;
function shutdown(signal) {
  if (shuttingDown) return;
  shuttingDown = true;
  console.log(`\n👋 Desligando bot (sinal recebido: ${signal})...`);
  bot.stopPolling();
  if (server) {
    server.close(() => {
      console.log('✅ Servidor encerrado');
      process.exit(0);
    });
  } else {
    process.exit(0);
  }
}
process.on('SIGINT', () => shutdown('SIGINT'));
process.on('SIGTERM', () => shutdown('SIGTERM'));
