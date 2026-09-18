# API Reference — Ingresso.com

> Reference for the public Ingresso.com API consumed by the bot.

## Base configuration

```ts
// src/api.ts
const BASE_URL = 'https://api-content.ingresso.com';
const CITY_ID = 53;  // Maceió
const DEFAULT_THEATER_ID = '1162';  // Cinesystem
```

## Headers (browser-like)

```js
const HEADERS = {
  'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 ... Chrome/120.0.0.0 Safari/537.36',
  Accept: 'application/json, text/plain, */*',
  'Accept-Encoding': 'gzip, deflate, br',
  'Accept-Language': 'pt-BR,pt;q=0.9,en;q=0.8',
};
```

The API is **public and requires no key**. Browser-like headers help avoid blocks.

## Endpoints

### 1. `GET /v0/sessions/city/{cityId}/theater/{theaterId}/partnership/home/groupBy/sessionType?date={date}`

Returns sessions for a cinema on a specific date, grouped by session type.

| Parameter  | Type   | Description                            | Example          |
| ---------- | ------ | -------------------------------------- | ---------------- |
| `cityId`   | number | City ID (53 = Maceió)                 | `53`             |
| `theaterId`| number | Cinema ID                             | `1162`, `1230`, `924` |
| `date`     | string | Date in `YYYY-MM-DD` format           | `2026-08-19`     |

**Consumed by:** `fetchNormalized()` → `normalizeSessionsResponse()`

---

### 2. `GET /v0/sessions/city/{cityId}/theater/{theaterId}`

Returns all sessions for a cinema — for all dates (past, today, and future).

**Consumed by:** `fetchUpcoming()` → identifies future pre-sale releases.

## Exported functions (`src/api.ts`)

### `fetchNormalized(date = null, theaterId = 1162)`

- Resolves the target date: uses `date` if provided, otherwise the current date in the `America/Maceio` timezone.
- Requests endpoint 1.
- Returns normalized data:
  ```ts
  {
    movies: Record<string, MovieStatic>,
    sessions: Session[],
    date: string,          // YYYY-MM-DD
    fetchedAt: string      // ISO timestamp
  }
  ```

### `fetchUpcoming(theaterId = 1162)`

- Fetches all dates from endpoint 2.
- Filters movies **not currently showing today** (`todayMovieIds`).
- Keeps only those in **pre-sale** (`inPreSale === true`).
- Returns:
  ```ts
  { items: UpcomingItem[], fetchedAt: string }
  ```

---

## Normalization functions (`src/normalize.ts`)

### `extractMovieStatic(raw)` → `MovieStatic`
Extracts static (immutable) data from a raw API movie.

### `extractSessions(movieId, sessionTypes)` → `Session[]`
Extracts dynamic sessions (time, price, room, format, audio) from a movie.

### `normalizeSessionsResponse(apiResponse)`
Normalizes the full endpoint-1 response into `{ movies, sessions, date, fetchedAt }`.

### `normalizeUpcomingFromSessions(futureDates, todayMovieIds)` → `UpcomingItem[]`
Identifies new releases from future dates, excluding movies already showing today.

### `denormalize(movies, sessions)` → `DisplayMovie[]`
Rebuilds the display view: joins static data + sessions into an array ready for the UI.

---

## Supported theaters

| `theaterId` | Cinema     | Shopping                    |
| ----------- | ---------- | --------------------------- |
| `1162`      | Cinesystem | Parque Shopping Maceió      |
| `1230`      | Centerplex | Shopping Pátio Maceió       |
| `924`       | Kinoplex   | Maceió Shopping             |

## Date utils (`America/Maceio` timezone)

- `getMaceioDate(offsetDays = 0)` → `YYYY-MM-DD` for the current day in Maceió (with offset).
- `toMaceioDateStr(isoString)` → converts any ISO string to `YYYY-MM-DD` in Maceió.
- `getTodayInMaceioISO()` (in `api.ts`) → today's date in `en-CA` format (YYYY-MM-DD).
