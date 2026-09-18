/**
 * Normalize Ingresso.com API data.
 *
 * Separates static movie data (rarely changes) from dynamic session data
 * (changes by day/time), removing the massive redundancy of the sessions endpoint.
 */

import type {
  DenormalizedMovie,
  IngressoDateEntry,
  IngressoRawMovie,
  IngressoSessionGroup,
  MovieStatic,
  NormalizedSessions,
  Session,
  UpcomingItem,
} from './types.js';

export function extractMovieStatic(raw: IngressoRawMovie): MovieStatic {
  const poster = raw.images?.find((i) => i.type === 'PosterPortrait')?.url ?? null;
  const backdrop = raw.images?.find((i) => i.type === 'PosterHorizontal')?.url ?? null;
  const trailer = raw.trailers?.[0]?.url ?? null;

  return {
    id: raw.id,
    title: raw.title,
    originalTitle: raw.originalTitle || null,
    urlKey: raw.urlKey ?? '',
    duration: Number(raw.duration) || null,
    contentRating: raw.contentRating || null,
    ratingColor: raw.ratingDetails?.color ?? null,
    genres: raw.genres ?? [],
    distributor: raw.distributor || null,
    poster,
    backdrop,
    trailer,
    tags: (raw.completeTags || raw.tags || []).map((t) => (typeof t === 'string' ? t : t.name)),
    isReexhibition: raw.isReexhibition ?? false,
    inPreSale: raw.inPreSale ?? false,
  };
}

export function extractSessions(
  movieId: number,
  sessionTypes: IngressoSessionGroup[] | undefined,
): Session[] {
  const sessions: Session[] = [];

  for (const group of sessionTypes || []) {
    for (const s of group.sessions || []) {
      const format =
        s.types?.find((t) => t.name !== 'Dublado' && t.name !== 'Legendado')?.alias ?? '2D';
      const audio =
        s.types?.find((t) => t.name === 'Dublado' || t.name === 'Legendado')?.alias ?? null;

      sessions.push({
        id: s.id,
        movieId,
        time: s.time,
        price: s.price ?? null,
        room: s.room || null,
        format,
        audio,
        checkoutUrl: s.siteURL || null,
      });
    }
  }

  return sessions;
}

export function normalizeSessionsResponse(
  apiResponse: IngressoDateEntry | IngressoDateEntry[] | null | undefined,
): NormalizedSessions {
  const data = Array.isArray(apiResponse) ? apiResponse[0] : apiResponse;

  if (!data?.movies) {
    return { movies: {}, sessions: [], date: null, fetchedAt: new Date().toISOString() };
  }

  const movies: Record<string, MovieStatic> = {};
  const sessions: Session[] = [];

  for (const rawMovie of data.movies) {
    if (!movies[rawMovie.id]) {
      movies[rawMovie.id] = extractMovieStatic(rawMovie);
    }

    const movieSessions = extractSessions(rawMovie.id, rawMovie.sessionTypes);
    sessions.push(...movieSessions);
  }

  return {
    movies,
    sessions,
    date: data.date || null,
    fetchedAt: new Date().toISOString(),
  };
}

export function normalizeUpcomingFromSessions(
  futureDates: IngressoDateEntry[],
  todayMovieIds: Set<number>,
): UpcomingItem[] {
  const seen = new Map<number, UpcomingItem>();

  for (const dateEntry of futureDates) {
    const movies = dateEntry.movies || [];

    for (const raw of movies) {
      if (todayMovieIds.has(raw.id) || seen.has(raw.id)) continue;

      const poster = raw.images?.find((i) => i.type === 'PosterPortrait')?.url ?? null;

      const formats = new Set<string>();
      let minPrice: number | null = null;

      const sessionGroups = raw.sessionTypes || raw.rooms || [];
      for (const group of sessionGroups) {
        for (const s of group.sessions || []) {
          for (const t of s.types || []) {
            if (t.name !== 'Dublado' && t.name !== 'Legendado' && t.alias) {
              formats.add(t.alias);
            }
          }
          if (s.price && (minPrice === null || s.price < minPrice)) {
            minPrice = s.price;
          }
        }
      }

      seen.set(raw.id, {
        id: raw.id,
        title: raw.title,
        originalTitle: raw.originalTitle || null,
        contentRating: raw.contentRating || null,
        genres: raw.genres ?? [],
        poster,
        inPreSale: raw.inPreSale ?? false,
        formats: [...formats],
        priceFrom: minPrice,
        firstDate: dateEntry.date,
        firstDateFormatted: dateEntry.dateFormatted ?? dateEntry.date,
        firstDateDayOfWeek: dateEntry.dayOfWeek ?? '',
        siteURL: raw.siteURLByTheater || raw.siteURL || null,
      });
    }
  }

  return [...seen.values()];
}

export function denormalize(
  movies: Record<string, MovieStatic>,
  sessions: Session[],
): DenormalizedMovie[] {
  const grouped = new Map<number, DenormalizedMovie>();

  for (const session of sessions) {
    if (!grouped.has(session.movieId)) {
      const movie = movies[session.movieId];
      if (!movie) continue;
      grouped.set(session.movieId, {
        ...movie,
        name: movie.title,
        sessions: [],
      });
    }

    const entry = grouped.get(session.movieId);
    if (!entry) continue;
    entry.sessions.push({
      time: session.time,
      sessionId: session.id,
      priceInteira: session.price,
      priceMeia: session.price ? +(session.price / 2).toFixed(2) : null,
      gratuito: !session.price,
      room: session.room,
      format: session.format,
      audio: session.audio,
    });
  }

  return Array.from(grouped.values());
}
