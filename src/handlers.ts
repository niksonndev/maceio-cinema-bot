/**
 * Handlers de comandos e callbacks do Telegram.
 *
 * `handleUpdate(bot, cache, update)` é o dispatcher compartilhado por
 * polling (bot.ts) e webhook (lambda.ts). Lambda aguarda o retorno antes
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
import type {
  BotLike,
  CacheLike,
  CarouselType,
  Cinema,
  DenormalizedMovie,
  TelegramCallbackQuery,
  TelegramMessage,
  TelegramUpdate,
  UpcomingItem,
} from './types.js';
import { errorMessage } from './types.js';

export const COMMAND_RE = /^\/(start|hoje|proximos|cinemas|atualizar)(?:@\S+)?(?:\s|$)/;

function askCinemaFirst(bot: BotLike, chatId: number | string) {
  return bot.sendMessage(
    chatId,
    '⚠️ Você ainda não escolheu um cinema. Escolha abaixo qual cinema deseja consultar:',
    { reply_markup: getCinemaKeyboard() },
  );
}

function sendWithBackButton(
  bot: BotLike,
  chatId: number | string,
  text: string,
  cinemaUrl: string,
) {
  return bot.sendMessage(chatId, text, {
    parse_mode: 'Markdown',
    reply_markup: getBackButtonMarkup(cinemaUrl),
  });
}

function emptyListMessage(type: string): string {
  return type === 'proximos'
    ? '📭 *Nenhum lançamento próximo encontrado.*'
    : '📭 *Nenhum filme em cartaz para esta data.*';
}

function isUpcomingType(type: string): type is 'proximos' {
  return type === 'proximos';
}

async function loadCarouselList(
  cache: CacheLike,
  type: string,
  cinema: Cinema,
): Promise<{ list: Array<DenormalizedMovie | UpcomingItem>; dateStr: string | null }> {
  let list: Array<DenormalizedMovie | UpcomingItem> = [];
  let dateStr: string | null = null;

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

async function formatCarouselItem(
  type: string,
  item: DenormalizedMovie | UpcomingItem,
  cinema: Cinema,
  dateStr: string | null,
): Promise<string> {
  if (isUpcomingType(type)) {
    return formatSingleUpcomingCard(item as UpcomingItem, cinema.label);
  }
  return formatSingleMovieCard(item as DenormalizedMovie, cinema.label, dateStr);
}

async function sendCarouselPage(
  bot: BotLike,
  cache: CacheLike,
  chatId: number | string,
  type: string,
  index: number,
  cinema: Cinema,
): Promise<void> {
  const { list, dateStr } = await loadCarouselList(cache, type, cinema);
  const total = list.length;
  if (total === 0) {
    await sendWithBackButton(bot, chatId, emptyListMessage(type), cinema.url);
    return;
  }

  const safeIndex = Math.max(0, Math.min(index, total - 1));
  const item = list[safeIndex];
  const text = await formatCarouselItem(type, item, cinema, dateStr);
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

async function editCarouselPage(
  bot: BotLike,
  cache: CacheLike,
  chatId: number | string,
  messageId: number,
  type: string,
  index: number,
  cinema: Cinema,
  hasPhoto: boolean,
): Promise<void> {
  const { list, dateStr } = await loadCarouselList(cache, type, cinema);
  const total = list.length;
  if (total === 0) {
    await sendWithBackButton(bot, chatId, emptyListMessage(type), cinema.url);
    return;
  }

  const safeIndex = Math.max(0, Math.min(index, total - 1));
  const item = list[safeIndex];
  const text = await formatCarouselItem(type, item, cinema, dateStr);
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

async function withLoading(
  bot: BotLike,
  chatId: number | string,
  loadingText: string,
  fn: () => Promise<void>,
): Promise<void> {
  const loadingMsg = await bot.sendMessage(chatId, loadingText);
  try {
    await fn();
  } finally {
    await bot.deleteMessage(chatId, loadingMsg.message_id).catch(() => {});
  }
}

export async function handleStart(bot: BotLike, msg: TelegramMessage): Promise<void> {
  const chatId = msg.chat.id;
  try {
    await bot.sendMessage(
      chatId,
      'Olá! Eu sou o seu guia de cinema em Maceió. 🍿\nEscolha abaixo qual cinema você deseja consultar:',
      { reply_markup: getCinemaKeyboard() },
    );
    console.log(`✅ /start enviado para ${msg.from?.username || chatId}`);
  } catch (err) {
    console.error(`❌ Erro em /start para ${chatId}:`, errorMessage(err));
  }
}

export async function handleHoje(
  bot: BotLike,
  cache: CacheLike,
  msg: TelegramMessage,
): Promise<void> {
  const chatId = msg.chat.id;
  const cinema = await getUserCinema(chatId);
  if (!cinema) {
    await askCinemaFirst(bot, chatId);
    return;
  }

  try {
    await withLoading(bot, chatId, '⏳ Buscando filmes de hoje...', () =>
      sendCarouselPage(bot, cache, chatId, 'hoje', 0, cinema),
    );
    console.log(`✅ /hoje enviado para ${msg.from?.username || chatId} (${cinema.name})`);
  } catch (err) {
    await bot.sendMessage(chatId, `❌ Erro ao buscar filmes: ${errorMessage(err)}`);
    console.error(`❌ Erro em /hoje para ${chatId}:`, errorMessage(err));
  }
}

export async function handleProximos(
  bot: BotLike,
  cache: CacheLike,
  msg: TelegramMessage,
): Promise<void> {
  const chatId = msg.chat.id;
  const cinema = await getUserCinema(chatId);
  if (!cinema) {
    await askCinemaFirst(bot, chatId);
    return;
  }

  try {
    await withLoading(bot, chatId, '⏳ Buscando próximos lançamentos...', () =>
      sendCarouselPage(bot, cache, chatId, 'proximos', 0, cinema),
    );
    console.log(`✅ /proximos enviado para ${msg.from?.username || chatId} (${cinema.name})`);
  } catch (err) {
    await bot.sendMessage(chatId, `❌ Erro ao buscar lançamentos: ${errorMessage(err)}`);
    console.error(`❌ Erro em /proximos para ${chatId}:`, errorMessage(err));
  }
}

export async function handleCinemas(bot: BotLike, msg: TelegramMessage): Promise<void> {
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

export async function handleAtualizar(
  bot: BotLike,
  cache: CacheLike,
  msg: TelegramMessage,
): Promise<void> {
  const chatId = msg.chat.id;
  const cinema = await getUserCinema(chatId);
  if (!cinema) {
    await askCinemaFirst(bot, chatId);
    return;
  }

  try {
    await withLoading(bot, chatId, '🔄 Atualizando programação de hoje...', async () => {
      const normalized = await fetchNormalized(null, cinema.id);
      cache.mergeMovies(normalized.movies);
      await cache.setSessions(
        normalized.date ?? getDateString(0),
        normalized.sessions,
        normalized.fetchedAt,
        cinema.id,
      );
      await sendCarouselPage(bot, cache, chatId, 'hoje', 0, cinema);
    });
    console.log(`✅ /atualizar enviado para ${msg.from?.username || chatId} (${cinema.name})`);
  } catch (err) {
    await bot.sendMessage(chatId, `❌ Erro ao atualizar: ${errorMessage(err)}`);
    console.error(`❌ Erro em /atualizar para ${chatId}:`, errorMessage(err));
  }
}

export async function handleCallbackQuery(
  bot: BotLike,
  cache: CacheLike,
  query: TelegramCallbackQuery,
): Promise<void> {
  const chatId = query.message?.chat.id;
  const callbackData = query.data;
  if (chatId == null || !callbackData) return;

  try {
    await bot.answerCallbackQuery(query.id);
  } catch (err) {
    console.error('❌ Erro ao responder callback:', errorMessage(err));
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
      const type = carouselMatch[1] as CarouselType;
      const index = parseInt(carouselMatch[2], 10);
      const messageId = query.message?.message_id;
      if (messageId == null) return;
      const hasPhoto = Array.isArray(query.message?.photo) && query.message.photo.length > 0;
      try {
        await editCarouselPage(bot, cache, chatId, messageId, type, index, cinema, hasPhoto);
      } catch (err) {
        console.error(`❌ Erro ao editar carrossel ${type}:`, errorMessage(err));
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
    console.error(`❌ Erro ao processar ${callbackData}:`, errorMessage(err));
    await bot.sendMessage(chatId, `❌ Erro ao processar: ${errorMessage(err)}`).catch(() => {});
  }
}

export async function handleUpdate(
  bot: BotLike,
  cache: CacheLike,
  update: TelegramUpdate | null | undefined,
): Promise<void> {
  if (!update) return;

  if (update.callback_query) {
    await handleCallbackQuery(bot, cache, update.callback_query);
    return;
  }

  const msg = update.message;
  if (!msg?.text) return;

  const match = msg.text.match(COMMAND_RE);
  if (!match) return;

  switch (match[1]) {
    case 'start':
      await handleStart(bot, msg);
      return;
    case 'hoje':
      await handleHoje(bot, cache, msg);
      return;
    case 'proximos':
      await handleProximos(bot, cache, msg);
      return;
    case 'cinemas':
      await handleCinemas(bot, msg);
      return;
    case 'atualizar':
      await handleAtualizar(bot, cache, msg);
      return;
    default:
  }
}

export function registerHandlers(bot: BotLike, cache: CacheLike): void {
  bot.on?.('callback_query', ((query: TelegramCallbackQuery) =>
    handleCallbackQuery(bot, cache, query)) as (...args: never[]) => unknown);
  bot.on?.('message', ((msg: TelegramMessage) => {
    if (msg.text && !COMMAND_RE.test(msg.text)) {
      console.log(`📨 Mensagem recebida de ${msg.from?.username || msg.chat.id}: "${msg.text}"`);
      return;
    }
    return handleUpdate(bot, cache, { message: msg });
  }) as (...args: never[]) => unknown);
}
