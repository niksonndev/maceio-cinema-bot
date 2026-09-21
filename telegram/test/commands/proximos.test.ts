import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../src/data.js', () => ({
  getDateString: vi.fn((offset = 0) => (offset === 0 ? '2026-09-18' : '2026-09-19')),
  getMoviesForDate: vi.fn(),
  getUpcomingMovies: vi.fn(),
}));

vi.mock('../../src/format.js', () => ({
  formatSingleMovieCard: vi.fn(async (filme: { title: string }) => `card:${filme.title}`),
  formatSingleUpcomingCard: vi.fn(async (item: { title: string }) => `upcoming:${item.title}`),
}));

import { getUpcomingMovies } from '../../src/data.js';
import { handleUpdate } from '../../src/handlers.js';
import { clearPrefs, setUserCinema } from '../../src/cinemas.js';
import { createMockBot, createMockCache, commandUpdate, SAMPLE_UPCOMING } from '../helpers.js';

const CHAT = 4001;

describe('/proximos', () => {
  let bot: ReturnType<typeof createMockBot>;
  let cache: ReturnType<typeof createMockCache>;

  beforeEach(async () => {
    bot = createMockBot();
    cache = createMockCache();
    await clearPrefs();
    vi.mocked(getUpcomingMovies).mockReset();
  });

  it('asks for a cinema when no preference is stored', async () => {
    await handleUpdate(bot, cache, commandUpdate('/proximos', CHAT));
    expect(getUpcomingMovies).not.toHaveBeenCalled();
    expect(bot.sendMessage.mock.calls[0][1]).toMatch(/não escolheu um cinema/i);
  });

  it('sends an upcoming carousel when a cinema is selected', async () => {
    await setUserCinema(CHAT, '1162');
    vi.mocked(getUpcomingMovies).mockResolvedValue({ items: [SAMPLE_UPCOMING], fromCache: false });

    await handleUpdate(bot, cache, commandUpdate('/proximos', CHAT));

    expect(getUpcomingMovies).toHaveBeenCalled();
    expect(bot.sendPhoto).toHaveBeenCalledWith(
      CHAT,
      SAMPLE_UPCOMING.poster,
      expect.objectContaining({ caption: 'upcoming:Lançamento' }),
    );
  });

  it('sends an empty-state message when there are no upcoming titles', async () => {
    await setUserCinema(CHAT, '1162');
    vi.mocked(getUpcomingMovies).mockResolvedValue({ items: [], fromCache: false });

    await handleUpdate(bot, cache, commandUpdate('/proximos', CHAT));

    const texts = bot.sendMessage.mock.calls.map((c) => c[1]);
    expect(texts.some((t) => String(t).includes('Nenhum lançamento'))).toBe(true);
  });
});
