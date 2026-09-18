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

vi.mock('../../src/api.js', () => ({
  fetchNormalized: vi.fn(),
  fetchUpcoming: vi.fn(),
}));

import { getMoviesForDate } from '../../src/data.js';
import { fetchNormalized } from '../../src/api.js';
import { handleUpdate } from '../../src/handlers.js';
import { clearPrefs, setUserCinema } from '../../src/cinemas.js';
import { createMockBot, createMockCache, commandUpdate, SAMPLE_MOVIE } from '../helpers.js';

const CHAT = 6001;

describe('/atualizar', () => {
  let bot;
  let cache;

  beforeEach(async () => {
    bot = createMockBot();
    cache = createMockCache();
    await clearPrefs();
    fetchNormalized.mockReset();
    getMoviesForDate.mockReset();
  });

  it('asks for a cinema when no preference is stored', async () => {
    await handleUpdate(bot, cache, commandUpdate('/atualizar', CHAT));
    expect(fetchNormalized).not.toHaveBeenCalled();
    expect(bot.sendMessage.mock.calls[0][1]).toMatch(/não escolheu um cinema/i);
  });

  it('refetches sessions and then shows the carousel', async () => {
    await setUserCinema(CHAT, '1162');
    fetchNormalized.mockResolvedValue({
      movies: { 99: SAMPLE_MOVIE },
      sessions: [{ id: 's1', movieId: 99 }],
      date: '2026-09-18',
      fetchedAt: '2026-09-18T12:00:00.000Z',
    });
    getMoviesForDate.mockResolvedValue({ movies: [SAMPLE_MOVIE], date: '2026-09-18' });

    await handleUpdate(bot, cache, commandUpdate('/atualizar', CHAT));

    expect(fetchNormalized).toHaveBeenCalledWith(null, '1162');
    expect(cache.mergeMovies).toHaveBeenCalled();
    expect(cache.setSessions).toHaveBeenCalledWith(
      '2026-09-18',
      expect.any(Array),
      '2026-09-18T12:00:00.000Z',
      '1162',
    );
    expect(bot.sendPhoto).toHaveBeenCalled();
  });
});
