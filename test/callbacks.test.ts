import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../src/data.js', () => ({
  getDateString: vi.fn((offset = 0) => (offset === 0 ? '2026-09-18' : '2026-09-19')),
  getMoviesForDate: vi.fn(),
  getUpcomingMovies: vi.fn(),
}));

vi.mock('../src/format.js', () => ({
  formatSingleMovieCard: vi.fn(async (filme: { title: string }) => `card:${filme.title}`),
  formatSingleUpcomingCard: vi.fn(async (item: { title: string }) => `upcoming:${item.title}`),
}));

import { getMoviesForDate, getUpcomingMovies } from '../src/data.js';
import { handleUpdate } from '../src/handlers.js';
import { clearPrefs, getUserCinema } from '../src/cinemas.js';
import {
  createMockBot,
  createMockCache,
  callbackUpdate,
  SAMPLE_MOVIE,
  SAMPLE_UPCOMING,
} from './helpers.js';

const CHAT = 7001;

describe('callback queries', () => {
  let bot: ReturnType<typeof createMockBot>;
  let cache: ReturnType<typeof createMockCache>;

  beforeEach(async () => {
    bot = createMockBot();
    cache = createMockCache();
    await clearPrefs();
    vi.mocked(getMoviesForDate).mockReset();
    vi.mocked(getUpcomingMovies).mockReset();
  });

  it('persists cinema_<id> and shows the main menu', async () => {
    await handleUpdate(bot, cache, callbackUpdate('cinema_1162', CHAT));

    const cinema = await getUserCinema(CHAT);
    expect(cinema?.id).toBe('1162');
    expect(bot.answerCallbackQuery).toHaveBeenCalledWith('cb1');
    const text = bot.sendMessage.mock.calls[0][1] as string;
    expect(text).toMatch(/Cinesystem/);
    expect(
      (bot.sendMessage.mock.calls[0][2] as { reply_markup: { inline_keyboard: unknown } })
        .reply_markup.inline_keyboard,
    ).toEqual(
      expect.arrayContaining([
        expect.arrayContaining([expect.objectContaining({ callback_data: 'filmes_hoje' })]),
      ]),
    );
  });

  it('rejects an unknown cinema id', async () => {
    await handleUpdate(bot, cache, callbackUpdate('cinema_9999', CHAT));
    expect(bot.sendMessage.mock.calls[0][1]).toMatch(/não encontrado/i);
    expect(await getUserCinema(CHAT)).toBeNull();
  });

  it('asks for a cinema on filmes_hoje when none is selected', async () => {
    await handleUpdate(bot, cache, callbackUpdate('filmes_hoje', CHAT));
    expect(getMoviesForDate).not.toHaveBeenCalled();
    expect(bot.sendMessage.mock.calls[0][1]).toMatch(/não escolheu um cinema/i);
  });

  it('shows today movies after a cinema is selected', async () => {
    await handleUpdate(bot, cache, callbackUpdate('cinema_1230', CHAT));
    vi.mocked(getMoviesForDate).mockResolvedValue({
      movies: [SAMPLE_MOVIE],
      date: '2026-09-18',
      fromCache: false,
    });

    await handleUpdate(bot, cache, callbackUpdate('filmes_hoje', CHAT));

    expect(getMoviesForDate).toHaveBeenCalled();
    expect(bot.sendPhoto).toHaveBeenCalled();
  });

  it('shows tomorrow movies via filmes_amanha', async () => {
    await handleUpdate(bot, cache, callbackUpdate('cinema_1162', CHAT));
    vi.mocked(getMoviesForDate).mockResolvedValue({
      movies: [SAMPLE_MOVIE],
      date: '2026-09-19',
      fromCache: false,
    });

    await handleUpdate(bot, cache, callbackUpdate('filmes_amanha', CHAT));

    expect(getMoviesForDate).toHaveBeenCalledWith(cache, '2026-09-19', '1162');
  });

  it('shows upcoming via proximos_lancamentos', async () => {
    await handleUpdate(bot, cache, callbackUpdate('cinema_1162', CHAT));
    vi.mocked(getUpcomingMovies).mockResolvedValue({ items: [SAMPLE_UPCOMING], fromCache: false });

    await handleUpdate(bot, cache, callbackUpdate('proximos_lancamentos', CHAT));

    expect(getUpcomingMovies).toHaveBeenCalled();
    expect(bot.sendPhoto).toHaveBeenCalled();
  });

  it('edits the carousel on next-page callback', async () => {
    await handleUpdate(bot, cache, callbackUpdate('cinema_1162', CHAT));
    const second = {
      ...SAMPLE_MOVIE,
      id: 101,
      title: 'Segundo',
      poster: 'https://example.com/2.jpg',
    };
    vi.mocked(getMoviesForDate).mockResolvedValue({
      movies: [SAMPLE_MOVIE, second],
      date: '2026-09-18',
      fromCache: false,
    });

    await handleUpdate(
      bot,
      cache,
      callbackUpdate('carousel_hoje_1_2', CHAT, {
        photo: [{ file_id: 'p1' }],
      }),
    );

    expect(bot.editMessageMedia).toHaveBeenCalled();
  });
});
