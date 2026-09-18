/**
 * Busca notas de filmes:
 * - IMDb e Rotten Tomatoes via OMDb
 * - Fallback via TMDb (quando OMDb não retorna nada)
 */

import axios from 'axios';
import type { RatingsResult } from './types.js';
import { errorMessage } from './types.js';

const OMDb_BASE = 'https://www.omdbapi.com/';
const TMDB_BASE = 'https://api.themoviedb.org/3';
const CACHE_TTL_MS = 24 * 60 * 60 * 1000; // 24h

type CacheEntry = { at: number; data: RatingsResult | null };
const memoryCache = new Map<string, CacheEntry>();

function cacheKey(title: string, year: string | number | null | undefined): string {
  const t = (title || '').trim().toLowerCase();
  const y = year ? String(year) : '';
  return `${t}|${y}`;
}

function extractRottenTomatoes(ratings: unknown): string | null {
  if (!Array.isArray(ratings)) return null;
  const rt = ratings.find(
    (r: { Source?: string; Value?: string }) =>
      r.Source && r.Source.toLowerCase().includes('rotten tomatoes'),
  ) as { Value?: string } | undefined;
  if (!rt || !rt.Value) return null;
  const value = String(rt.Value).trim();
  if (value === 'N/A') return null;
  return value;
}

type OmdbResponse = {
  Response?: string;
  imdbRating?: string;
  Ratings?: Array<{ Source?: string; Value?: string }>;
};

async function fetchFromOmdb(
  title: string,
  year: string | number | null | undefined,
): Promise<RatingsResult | null> {
  const apiKey = process.env.OMDb_API_KEY;
  if (!apiKey) return null;

  const params: Record<string, string> = {
    apikey: apiKey,
    t: title,
    type: 'movie',
    r: 'json',
  };
  if (year) params.y = String(year);

  const { data } = await axios.get<OmdbResponse>(OMDb_BASE, {
    params,
    timeout: 5000,
    headers: {
      'User-Agent': 'Mozilla/5.0 (compatible; MaceioCineBot/1.0)',
    },
  });

  if (!data || data.Response === 'False') {
    return null;
  }

  const imdb = data.imdbRating && data.imdbRating !== 'N/A' ? String(data.imdbRating).trim() : null;
  const rottenTomatoes = extractRottenTomatoes(data.Ratings || []);

  return imdb || rottenTomatoes
    ? { imdb: imdb || null, rottenTomatoes: rottenTomatoes || null, tmdb: null }
    : null;
}

type TmdbSearchResponse = {
  results?: Array<{ vote_average?: number }>;
};

async function fetchFromTmdb(
  title: string,
  year: string | number | null | undefined,
): Promise<RatingsResult | null> {
  const apiKey = process.env.TMDB_API_KEY;
  if (!apiKey) return null;

  const params: Record<string, string | boolean | number> = {
    api_key: apiKey,
    query: title,
    include_adult: false,
    language: 'en-US',
  };
  if (year) params.year = year;

  const { data } = await axios.get<TmdbSearchResponse>(`${TMDB_BASE}/search/movie`, {
    params,
    timeout: 5000,
  });

  if (!data || !Array.isArray(data.results) || data.results.length === 0) {
    return null;
  }

  const best = data.results[0];
  if (!best || !best.vote_average) return null;

  const tmdb = Number(best.vote_average).toFixed(1);
  return { imdb: null, rottenTomatoes: null, tmdb };
}

export async function getMovieRatings(
  title: string,
  year: string | number | null = null,
): Promise<RatingsResult | null> {
  const key = cacheKey(title, year);
  const cached = memoryCache.get(key);
  if (cached && Date.now() - cached.at < CACHE_TTL_MS) {
    return cached.data;
  }

  let result: RatingsResult | null = null;

  try {
    result = await fetchFromOmdb(title, year);
  } catch (err) {
    console.warn(`⚠️ OMDb: erro ao buscar "${title}":`, errorMessage(err));
  }

  if (!result) {
    try {
      result = await fetchFromTmdb(title, year);
    } catch (err) {
      console.warn(`⚠️ TMDb: erro ao buscar "${title}":`, errorMessage(err));
    }
  }

  memoryCache.set(key, { at: Date.now(), data: result || null });
  return result || null;
}

export function formatRatingsLine(ratings: RatingsResult | null): string {
  if (!ratings) return '';
  const parts: string[] = [];

  if (ratings.imdb) parts.push(`⭐ IMDb: ${ratings.imdb}/10`);
  if (ratings.rottenTomatoes) parts.push(`🍅 RT: ${ratings.rottenTomatoes}`);
  if (ratings.tmdb) parts.push(`⭐ TMDb: ${ratings.tmdb}/10`);

  if (parts.length === 0) return '';

  return `   📊 Avaliações: ${parts.join(' | ')}\n\n`;
}
