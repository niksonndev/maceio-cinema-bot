import { vi } from 'vitest';
import type {
  BotLike,
  CacheLike,
  DenormalizedMovie,
  TelegramMessage,
  UpcomingItem,
} from '../src/types.js';

export function createMockBot(): BotLike & {
  sendMessage: ReturnType<typeof vi.fn>;
  sendPhoto: ReturnType<typeof vi.fn>;
  deleteMessage: ReturnType<typeof vi.fn>;
  answerCallbackQuery: ReturnType<typeof vi.fn>;
  editMessageMedia: ReturnType<typeof vi.fn>;
  editMessageText: ReturnType<typeof vi.fn>;
  editMessageReplyMarkup: ReturnType<typeof vi.fn>;
} {
  return {
    sendMessage: vi.fn().mockResolvedValue({ message_id: 1 }),
    sendPhoto: vi.fn().mockResolvedValue({ message_id: 2 }),
    deleteMessage: vi.fn().mockResolvedValue(true),
    answerCallbackQuery: vi.fn().mockResolvedValue(true),
    editMessageMedia: vi.fn().mockResolvedValue(true),
    editMessageText: vi.fn().mockResolvedValue(true),
    editMessageReplyMarkup: vi.fn().mockResolvedValue(true),
  };
}

export function createMockCache(): CacheLike & {
  getSessions: ReturnType<typeof vi.fn>;
  getUpcoming: ReturnType<typeof vi.fn>;
  mergeMovies: ReturnType<typeof vi.fn>;
  setSessions: ReturnType<typeof vi.fn>;
  setUpcoming: ReturnType<typeof vi.fn>;
  getAllMovies: ReturnType<typeof vi.fn>;
  load: ReturnType<typeof vi.fn>;
} {
  return {
    getSessions: vi.fn().mockReturnValue(null),
    getUpcoming: vi.fn().mockReturnValue(null),
    mergeMovies: vi.fn(),
    setSessions: vi.fn().mockResolvedValue(undefined),
    setUpcoming: vi.fn().mockResolvedValue(undefined),
    getAllMovies: vi.fn().mockReturnValue({}),
    load: vi.fn().mockResolvedValue(undefined),
  };
}

export function telegramMessage(text: string, chatId = 111): TelegramMessage {
  return {
    message_id: 10,
    from: { id: chatId, is_bot: false, first_name: 'Test', username: 'tester' },
    chat: { id: chatId, type: 'private' },
    date: 1700000000,
    text,
  };
}

export function commandUpdate(text: string, chatId = 111) {
  return { update_id: 1, message: telegramMessage(text, chatId) };
}

export function callbackUpdate(
  data: string,
  chatId = 111,
  extra: { photo?: Array<{ file_id: string }> } = {},
) {
  return {
    update_id: 2,
    callback_query: {
      id: 'cb1',
      from: { id: chatId, username: 'tester' },
      message: {
        message_id: 20,
        chat: { id: chatId, type: 'private' },
        photo: extra.photo,
      },
      data,
    },
  };
}

export const SAMPLE_MOVIE: DenormalizedMovie = {
  id: 99,
  title: 'Duna',
  name: 'Duna',
  originalTitle: null,
  urlKey: 'duna',
  duration: null,
  contentRating: null,
  ratingColor: null,
  genres: [],
  distributor: null,
  poster: 'https://example.com/duna.jpg',
  backdrop: null,
  trailer: null,
  tags: [],
  isReexhibition: false,
  inPreSale: false,
  sessions: [
    {
      time: '20:00',
      sessionId: 's1',
      format: '2D',
      priceInteira: 30,
      priceMeia: 15,
      gratuito: false,
      room: null,
      audio: null,
    },
  ],
};

export const SAMPLE_MOVIE_NO_POSTER: DenormalizedMovie = {
  ...SAMPLE_MOVIE,
  id: 100,
  title: 'Filme Sem Poster',
  name: 'Filme Sem Poster',
  poster: null,
  sessions: [
    {
      time: '18:00',
      sessionId: 's2',
      format: '2D',
      priceInteira: 20,
      priceMeia: 10,
      gratuito: false,
      room: null,
      audio: null,
    },
  ],
};

export const SAMPLE_UPCOMING: UpcomingItem = {
  id: 200,
  title: 'Lançamento',
  originalTitle: null,
  contentRating: null,
  genres: [],
  poster: 'https://example.com/up.jpg',
  inPreSale: true,
  formats: [],
  priceFrom: null,
  firstDate: '2026-09-25',
  firstDateFormatted: '25 de setembro de 2026',
  firstDateDayOfWeek: 'sexta-feira',
  siteURL: null,
};
