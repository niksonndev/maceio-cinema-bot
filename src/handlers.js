/**
 * Handlers de comandos e callbacks do Telegram.
 *
 * `handleUpdate(bot, cache, update)` é o dispatcher compartilhado por
 * polling (bot.js) e webhook (lambda.js). Lambda aguarda o retorno antes
 * de responder 200 ao API Gateway.
 */

import { fetchNormalized } from './api.js';
import { getUserCinema, setUserCinema, findCinemaById } from './cinemas.js';
import { getDateString, getMoviesForDate, getUpcomingMovies } from './data.js';
import { formatSingleMovieCard, formatSingleUpcomingCard } from './format.js';
import {
  getCinemaKeyboard,
  getMainKeyboard,
  getBackButtonMarkup,
  getCarouselKeyboard,
} from './keyboards.js';

export const COMMAND_RE = /^\/(start|hoje|proximos|cinemas|atualizar)(?:@\S+)?(?:\s|$)/;

function askCinemaFirst(bot, chatId) {
  return bot.sendMessage(
    chatId,
    '⚠️ Você ainda não escolheu um cinema. Escolha abaixo qual cinema deseja consultar:',
    { reply_markup: getCinemaKeyboard() },
  );
}

function sendWithBackButton(bot, chatId, text, cinemaUrl) {
  return bot.sendMessage(chatId, text, {
    parse_mode: 'Markdown',
    reply_markup: getBackButtonMarkup(cinemaUrl),
  });
}

function emptyListMessage(type) {
  return type === 'proximos'
    ? '📭 *Nenhum lançamento próximo encontrado.*'
    : '📭 *Nenhum filme em cartaz para esta data.*';
}

async function loadCarouselList(cache, type, cinema) {
  let list = [];
  let dateStr = null;

  if (type === 'hoje') {
    const result = await getMoviesForDate(cache, null, cinema.id);
    list = result.movies || [];
    dateStr = result.date;
  } else if (type === 'amanha') {
    const result = await getMoviesForDate(cache, getDateString(1), cinema.id);
    list = result.movies || [];
    dateStr = result.date;
  } else if (type === 'proximos') {
    const result = await getUpcomingMovies(cache, cinema.id);
    list = result.items || [];
  }

  return { list, dateStr };
}

async function sendCarouselPage(bot, cache, chatId, type, index, cinema) {
  const { list, dateStr } = await loadCarouselList(cache, type, cinema);
  const total = list.length;
  if (total === 0) {
    await sendWithBackButton(bot, chatId, emptyListMessage(type), cinema.url);
    return;
  }

  const safeIndex = Math.max(0, Math.min(index, total - 1));
  const item = list[safeIndex];
  const isUpcoming = type === 'proximos';
  const text = isUpcoming
    ? await formatSingleUpcomingCard(item, cinema.label)
    : await formatSingleMovieCard(item, cinema.label, dateStr);
  const posterUrl = item.poster || null;
  const reply_markup = getCarouselKeyboard(type, safeIndex, total, cinema.url);

  if (posterUrl) {
    await bot.sendPhoto(chatId, posterUrl, {
      caption: text,
      parse_mode: 'Markdown',
      reply_markup,
    });
  } else {
    await bot.sendMessage(chatId, text, {
      parse_mode: 'Markdown',
      reply_markup,
    });
  }
}

async function editCarouselPage(bot, cache, chatId, messageId, type, index, cinema, hasPhoto) {
  const { list, dateStr } = await loadCarouselList(cache, type, cinema);
  const total = list.length;
  if (total === 0) {
    await sendWithBackButton(bot, chatId, emptyListMessage(type), cinema.url);
    return;
  }

  const safeIndex = Math.max(0, Math.min(index, total - 1));
  const item = list[safeIndex];
  const isUpcoming = type === 'proximos';
  const text = isUpcoming
    ? await formatSingleUpcomingCard(item, cinema.label)
    : await formatSingleMovieCard(item, cinema.label, dateStr);
  const posterUrl = item.poster || null;
  const reply_markup = getCarouselKeyboard(type, safeIndex, total, cinema.url);

  if (hasPhoto && posterUrl) {
    await bot.editMessageMedia(
      { type: 'photo', media: posterUrl, caption: text, parse_mode: 'Markdown' },
      { chat_id: chatId, message_id: messageId, reply_markup },
    );
  } else {
    await bot.editMessageText(text, {
      chat_id: chatId,
      message_id: messageId,
      parse_mode: 'Markdown',
    });
    await bot.editMessageReplyMarkup(reply_markup, {
      chat_id: chatId,
      message_id: messageId,
    });
  }
}

async function withLoading(bot, chatId, loadingText, fn) {
  const loadingMsg = await bot.sendMessage(chatId, loadingText);
  try {
    await fn();
  } finally {
    await bot.deleteMessage(chatId, loadingMsg.message_id).catch(() => {});
  }
}

export async function handleStart(bot, msg) {
  const chatId = msg.chat.id;
  try {
    await bot.sendMessage(
      chatId,
      'Olá! Eu sou o seu guia de cinema em Maceió. 🍿\nEscolha abaixo qual cinema você deseja consultar:',
      { reply_markup: getCinemaKeyboard() },
    );
    console.log(`✅ /start enviado para ${msg.from?.username || chatId}`);
  } catch (err) {
    console.error(`❌ Erro em /start para ${chatId}:`, err.message);
  }
}

export async function handleHoje(bot, cache, msg) {
  const chatId = msg.chat.id;
  const cinema = await getUserCinema(chatId);
  if (!cinema) return askCinemaFirst(bot, chatId);

  try {
    await withLoading(bot, chatId, '⏳ Buscando filmes de hoje...', () =>
      sendCarouselPage(bot, cache, chatId, 'hoje', 0, cinema),
    );
    console.log(`✅ /hoje enviado para ${msg.from?.username || chatId} (${cinema.name})`);
  } catch (err) {
    await bot.sendMessage(chatId, `❌ Erro ao buscar filmes: ${err.message}`);
    console.error(`❌ Erro em /hoje para ${chatId}:`, err.message);
  }
}

export async function handleProximos(bot, cache, msg) {
  const chatId = msg.chat.id;
  const cinema = await getUserCinema(chatId);
  if (!cinema) return askCinemaFirst(bot, chatId);

  try {
    await withLoading(bot, chatId, '⏳ Buscando próximos lançamentos...', () =>
      sendCarouselPage(bot, cache, chatId, 'proximos', 0, cinema),
    );
    console.log(`✅ /proximos enviado para ${msg.from?.username || chatId} (${cinema.name})`);
  } catch (err) {
    await bot.sendMessage(chatId, `❌ Erro ao buscar lançamentos: ${err.message}`);
    console.error(`❌ Erro em /proximos para ${chatId}:`, err.message);
  }
}

export async function handleCinemas(bot, msg) {
  const chatId = msg.chat.id;
  const current = await getUserCinema(chatId);
  const text = current
    ? `🎬 Cinema atual: *${current.label}*\nEscolha outro cinema:`
    : '🎬 Escolha o cinema que deseja consultar:';

  await bot.sendMessage(chatId, text, {
    parse_mode: 'Markdown',
    reply_markup: getCinemaKeyboard(),
  });
}

export async function handleAtualizar(bot, cache, msg) {
  const chatId = msg.chat.id;
  const cinema = await getUserCinema(chatId);
  if (!cinema) return askCinemaFirst(bot, chatId);

  try {
    await withLoading(bot, chatId, '🔄 Atualizando programação de hoje...', async () => {
      const normalized = await fetchNormalized(null, cinema.id);
      cache.mergeMovies(normalized.movies);
      await cache.setSessions(
        normalized.date,
        normalized.sessions,
        normalized.fetchedAt,
        cinema.id,
      );
      await sendCarouselPage(bot, cache, chatId, 'hoje', 0, cinema);
    });
    console.log(`✅ /atualizar enviado para ${msg.from?.username || chatId} (${cinema.name})`);
  } catch (err) {
    await bot.sendMessage(chatId, `❌ Erro ao atualizar: ${err.message}`);
    console.error(`❌ Erro em /atualizar para ${chatId}:`, err.message);
  }
}

export async function handleCallbackQuery(bot, cache, query) {
  const chatId = query.message.chat.id;
  const callbackData = query.data;

  try {
    await bot.answerCallbackQuery(query.id);
  } catch (err) {
    console.error('❌ Erro ao responder callback:', err.message);
  }

  try {
    if (callbackData.startsWith('cinema_')) {
      const theaterId = callbackData.replace('cinema_', '');
      const cinema = findCinemaById(theaterId);
      if (!cinema) {
        await bot.sendMessage(chatId, '❌ Cinema não encontrado.');
        return;
      }
      await setUserCinema(chatId, theaterId);
      await bot.sendMessage(
        chatId,
        `✅ Cinema selecionado: *${cinema.label}*\n\nEscolha uma opção:`,
        { parse_mode: 'Markdown', reply_markup: getMainKeyboard() },
      );
      console.log(`🎬 ${query.from?.username || chatId} selecionou ${cinema.name}`);
      return;
    }

    if (callbackData === 'trocar_cinema') {
      const current = await getUserCinema(chatId);
      const text = current
        ? `🎬 Cinema atual: *${current.label}*\nEscolha outro cinema:`
        : '🎬 Escolha o cinema que deseja consultar:';
      await bot.sendMessage(chatId, text, {
        parse_mode: 'Markdown',
        reply_markup: getCinemaKeyboard(),
      });
      return;
    }

    const cinema = await getUserCinema(chatId);
    if (!cinema) {
      await askCinemaFirst(bot, chatId);
      return;
    }

    const carouselMatch = callbackData.match(/^carousel_(hoje|amanha|proximos)_(\d+)_(\d+)$/);
    if (carouselMatch) {
      const [, type, indexStr] = carouselMatch;
      const index = parseInt(indexStr, 10);
      const messageId = query.message.message_id;
      const hasPhoto = Array.isArray(query.message.photo) && query.message.photo.length > 0;
      try {
        await editCarouselPage(bot, cache, chatId, messageId, type, index, cinema, hasPhoto);
      } catch (err) {
        console.error(`❌ Erro ao editar carrossel ${type}:`, err.message);
      }
      return;
    }

    switch (callbackData) {
      case 'filmes_hoje': {
        const hasCached = cache.getSessions(getDateString(0), cinema.id);
        if (!hasCached) {
          await withLoading(bot, chatId, '⏳ Buscando filmes de hoje... Aguarde um momento!', () =>
            sendCarouselPage(bot, cache, chatId, 'hoje', 0, cinema),
          );
        } else {
          await sendCarouselPage(bot, cache, chatId, 'hoje', 0, cinema);
        }
        return;
      }

      case 'filmes_amanha': {
        const tomorrowDate = getDateString(1);
        const hasCached = cache.getSessions(tomorrowDate, cinema.id);
        if (!hasCached) {
          await withLoading(
            bot,
            chatId,
            '⏳ Buscando filmes de amanhã... Aguarde um momento!',
            () => sendCarouselPage(bot, cache, chatId, 'amanha', 0, cinema),
          );
        } else {
          await sendCarouselPage(bot, cache, chatId, 'amanha', 0, cinema);
        }
        return;
      }

      case 'proximos_lancamentos': {
        const hasCached = cache.getUpcoming(cinema.id);
        if (!hasCached) {
          await withLoading(bot, chatId, '⏳ Buscando próximos lançamentos...', () =>
            sendCarouselPage(bot, cache, chatId, 'proximos', 0, cinema),
          );
        } else {
          await sendCarouselPage(bot, cache, chatId, 'proximos', 0, cinema);
        }
        return;
      }

      case 'voltar_menu':
        await bot.sendMessage(chatId, `*🎬 ${cinema.label}*\n\nEscolha uma opção:`, {
          parse_mode: 'Markdown',
          reply_markup: getMainKeyboard(),
        });
        return;

      case 'como_funciona':
        await sendWithBackButton(
          bot,
          chatId,
          '❓ *Como Funciona*\n\n' +
            'Este bot fornece informações sobre filmes em cartaz nos cinemas de Maceió.\n\n' +
            '💡 *Funcionalidades:*\n' +
            '🎬 Filmes de Hoje — Veja os filmes em exibição hoje\n' +
            '📅 Filmes de Amanhã — Veja os filmes em exibição amanhã\n' +
            '🆕 Próximos Lançamentos — Veja o que está chegando\n' +
            '🔄 Trocar Cinema — Mude o cinema selecionado\n' +
            '💰 Preços — Extraídos automaticamente da API\n\n',
          cinema.url,
        );
        return;

      default:
        await sendWithBackButton(bot, chatId, '❓ Opção não reconhecida.', cinema.url);
    }
  } catch (err) {
    console.error(`❌ Erro ao processar ${callbackData}:`, err.message);
    await bot.sendMessage(chatId, `❌ Erro ao processar: ${err.message}`).catch(() => {});
  }
}

export async function handleUpdate(bot, cache, update) {
  if (!update) return;

  if (update.callback_query) {
    return handleCallbackQuery(bot, cache, update.callback_query);
  }

  const msg = update.message;
  if (!msg?.text) return;

  const match = msg.text.match(COMMAND_RE);
  if (!match) return;

  switch (match[1]) {
    case 'start':
      return handleStart(bot, msg);
    case 'hoje':
      return handleHoje(bot, cache, msg);
    case 'proximos':
      return handleProximos(bot, cache, msg);
    case 'cinemas':
      return handleCinemas(bot, msg);
    case 'atualizar':
      return handleAtualizar(bot, cache, msg);
    default:
  }
}

export function registerHandlers(bot, cache) {
  bot.on('callback_query', (query) => handleCallbackQuery(bot, cache, query));
  bot.on('message', (msg) => {
    if (msg.text && !COMMAND_RE.test(msg.text)) {
      console.log(`📨 Mensagem recebida de ${msg.from?.username || msg.chat.id}: "${msg.text}"`);
      return;
    }
    return handleUpdate(bot, cache, { message: msg });
  });
}
