import { vi } from 'vitest';

export function createMockBot() {
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

export function createMockCache() {
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

export function telegramMessage(text, chatId = 111) {
  return {
    message_id: 10,
    from: { id: chatId, is_bot: false, first_name: 'Test', username: 'tester' },
    chat: { id: chatId, type: 'private' },
    date: 1700000000,
    text,
  };
}

export function commandUpdate(text, chatId = 111) {
  return { update_id: 1, message: telegramMessage(text, chatId) };
}

export function callbackUpdate(data, chatId = 111, extra = {}) {
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

export const SAMPLE_MOVIE = {
  id: 99,
  title: 'Duna',
  name: 'Duna',
  poster: 'https://example.com/duna.jpg',
  sessions: [{ time: '20:00', format: '2D', priceInteira: 30 }],
};

export const SAMPLE_MOVIE_NO_POSTER = {
  id: 100,
  title: 'Filme Sem Poster',
  name: 'Filme Sem Poster',
  poster: null,
  sessions: [{ time: '18:00', format: '2D', priceInteira: 20 }],
};

export const SAMPLE_UPCOMING = {
  id: 200,
  title: 'Lançamento',
  poster: 'https://example.com/up.jpg',
  inPreSale: true,
  firstDateFormatted: '25 de setembro de 2026',
  firstDateDayOfWeek: 'sexta-feira',
};
