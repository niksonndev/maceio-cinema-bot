/**
 * Data-access layer with cache for movies and upcoming releases.
 */

import { fetchNormalized, fetchUpcoming } from './api.js';
import { denormalize } from './normalize.js';
import type { CacheLike, DenormalizedMovie, UpcomingItem } from './types.js';

export function getDateString(daysOffset = 0): string {
  const now = new Date();
  const maceio = new Date(now.toLocaleString('en-US', { timeZone: 'America/Maceio' }));
  maceio.setDate(maceio.getDate() + daysOffset);
  return maceio.toISOString().split('T')[0];
}

export async function getMoviesForDate(
  cache: CacheLike,
  date: string | null = null,
  theaterId = '1162',
): Promise<{ movies: DenormalizedMovie[]; date: string; fromCache: boolean }> {
  const targetDate = date || getDateString(0);

  const cached = cache.getSessions(targetDate, theaterId);
  if (cached) {
    const movies = denormalize(cache.getAllMovies(), cached.items);
    return { movies, date: targetDate, fromCache: true };
  }

  const normalized = await fetchNormalized(date, theaterId);
  cache.mergeMovies(normalized.movies);
  await cache.setSessions(
    normalized.date ?? targetDate,
    normalized.sessions,
    normalized.fetchedAt,
    theaterId,
  );

  const movies = denormalize(normalized.movies, normalized.sessions);
  return { movies, date: normalized.date ?? targetDate, fromCache: false };
}

export async function getUpcomingMovies(
  cache: CacheLike,
  theaterId = '1162',
): Promise<{ items: UpcomingItem[]; fromCache: boolean }> {
  const cached = cache.getUpcoming(theaterId);
  if (cached) {
    return { items: cached.items, fromCache: true };
  }

  const result = await fetchUpcoming(theaterId);
  await cache.setUpcoming?.(result.items, result.fetchedAt, theaterId);
  return { items: result.items, fromCache: false };
}
