import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../src/data.js', () => ({
  getDateString: vi.fn((offset = 0) => (offset === 0 ? '2026-09-18' : '2026-09-19')),
  getMoviesForDate: vi.fn(),
  getUpcomingMovies: vi.fn(),
}));

vi.mock('../../src/format.js', () => ({
  formatSingleMovieCard: vi.fn(async (filme) => `card:${filme.title}`),
  formatSingleUpcomingCard: vi.fn(async (item) => `upcoming:${item.title}`),
}));

import { getMoviesForDate } from '../../src/data.js';
import { handleUpdate } from '../../src/handlers.js';
import { clearPrefs, setUserCinema } from '../../src/cinemas.js';
import {
  createMockBot,
  createMockCache,
  commandUpdate,
  SAMPLE_MOVIE,
  SAMPLE_MOVIE_NO_POSTER,
} from '../helpers.js';

const CHAT = 3001;

describe('/hoje', () => {
  let bot;
  let cache;

  beforeEach(async () => {
    bot = createMockBot();
    cache = createMockCache();
    await clearPrefs();
    getMoviesForDate.mockReset();
  });

  it('asks for a cinema when no preference is stored', async () => {
    await handleUpdate(bot, cache, commandUpdate('/hoje', CHAT));

    expect(getMoviesForDate).not.toHaveBeenCalled();
    const text = bot.sendMessage.mock.calls[0][1];
    expect(text).toMatch(/não escolheu um cinema/i);
  });

  it('sends a photo carousel when a cinema is selected', async () => {
    await setUserCinema(CHAT, '1162');
    getMoviesForDate.mockResolvedValue({ movies: [SAMPLE_MOVIE], date: '2026-09-18' });

    await handleUpdate(bot, cache, commandUpdate('/hoje', CHAT));

    expect(getMoviesForDate).toHaveBeenCalled();
    expect(bot.sendMessage).toHaveBeenCalledWith(CHAT, expect.stringMatching(/Buscando filmes/));
    expect(bot.sendPhoto).toHaveBeenCalledWith(
      CHAT,
      SAMPLE_MOVIE.poster,
      expect.objectContaining({ caption: 'card:Duna', parse_mode: 'Markdown' }),
    );
    expect(bot.deleteMessage).toHaveBeenCalled();
  });

  it('sends a text card when the movie has no poster', async () => {
    await setUserCinema(CHAT, '1230');
    getMoviesForDate.mockResolvedValue({
      movies: [SAMPLE_MOVIE_NO_POSTER],
      date: '2026-09-18',
    });

    await handleUpdate(bot, cache, commandUpdate('/hoje', CHAT));

    expect(bot.sendPhoto).not.toHaveBeenCalled();
    const texts = bot.sendMessage.mock.calls.map((c) => c[1]);
    expect(texts.some((t) => t === 'card:Filme Sem Poster')).toBe(true);
  });

  it('reports an error when the data layer fails', async () => {
    await setUserCinema(CHAT, '924');
    getMoviesForDate.mockRejectedValue(new Error('ingresso down'));

    await handleUpdate(bot, cache, commandUpdate('/hoje', CHAT));

    const texts = bot.sendMessage.mock.calls.map((c) => c[1]);
    expect(texts.some((t) => String(t).includes('ingresso down'))).toBe(true);
  });
});
