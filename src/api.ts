import axios from 'axios';
import { normalizeSessionsResponse, normalizeUpcomingFromSessions } from './normalize.js';
import type { IngressoDateEntry, NormalizedSessions, UpcomingItem } from './types.js';

const BASE_URL = 'https://api-content.ingresso.com';
const CITY_ID = 53; // Maceió
const DEFAULT_THEATER_ID = '1162';

const HEADERS = {
  'User-Agent':
    'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
  Accept: 'application/json, text/plain, */*',
  'Accept-Encoding': 'gzip, deflate, br',
  'Accept-Language': 'pt-BR,pt;q=0.9,en;q=0.8',
};

function getTodayInMaceioISO(): string {
  return new Date().toLocaleString('en-CA', {
    timeZone: 'America/Maceio',
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  });
}

function resolveTargetDate(date: string | null): string {
  return date || getTodayInMaceioISO();
}

export async function fetchNormalized(
  date: string | null = null,
  theaterId: string | number = DEFAULT_THEATER_ID,
): Promise<NormalizedSessions> {
  const targetDate = resolveTargetDate(date);
  const theater = String(theaterId);

  console.log(`🎬 Fetching sessions for ${targetDate} (theater ${theater})...`);
  const { data: response } = await axios.get<IngressoDateEntry | IngressoDateEntry[]>(
    `${BASE_URL}/v0/sessions/city/${CITY_ID}/theater/${theater}/partnership/home/groupBy/sessionType`,
    { params: { date: targetDate }, headers: HEADERS },
  );

  const normalized = normalizeSessionsResponse(response);
  normalized.date = targetDate;

  console.log(
    `✅ ${Object.keys(normalized.movies).length} movies, ${normalized.sessions.length} sessions`,
  );

  return normalized;
}

export async function fetchUpcoming(
  theaterId: string | number = DEFAULT_THEATER_ID,
): Promise<{ items: UpcomingItem[]; fetchedAt: string }> {
  const theater = String(theaterId);
  console.log(`🆕 Fetching upcoming releases — pre-sale (theater ${theater})...`);

  const { data: response } = await axios.get<IngressoDateEntry | IngressoDateEntry[]>(
    `${BASE_URL}/v0/sessions/city/${CITY_ID}/theater/${theater}`,
    { headers: HEADERS },
  );

  const allDates = Array.isArray(response) ? response : [];
  const today = getTodayInMaceioISO();

  const todayLikeEntries = allDates.filter((d) => d.date <= today);
  const todayMovieIds = new Set<number>();
  for (const entry of todayLikeEntries) {
    for (const m of entry.movies || []) {
      todayMovieIds.add(m.id);
    }
  }

  const futureDates = allDates.filter((d) => d.date > today);

  let items = normalizeUpcomingFromSessions(futureDates, todayMovieIds);
  items = items.filter((item) => item.inPreSale === true);
  console.log(`✅ ${items.length} pre-sale release(s)`);

  return { items, fetchedAt: new Date().toISOString() };
}
