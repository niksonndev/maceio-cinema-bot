# API Reference — Ingresso.com

> Reference for the public Ingresso.com API consumed by the bot.

## Base configuration

```ts
// telegram/rust/src/api.rs (WhatsApp: whatsapp/src/ingresso.rs)
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

## API functions (`telegram/rust/src/api.rs`)

### `fetch_normalized(date = None, theater_id = "1162")`

- Resolves the target date: uses `date` if provided, otherwise the current date in the `America/Maceio` timezone.
- Requests endpoint 1.
- Returns normalized data:
  ```ts
  {
    movies: HashMap<String, MovieStatic>,
    sessions: Vec<Session>,
    date: Option<String>,  // YYYY-MM-DD
    fetched_at: String     // ISO timestamp
  }
  ```

### `fetch_upcoming(theater_id = "1162")`

- Fetches all dates from endpoint 2.
- Filters movies **not currently showing today** (`todayMovieIds`).
- Keeps only those in **pre-sale** (`inPreSale === true`).
- Returns:
  ```ts
  (Vec<UpcomingItem>, String) // items and fetched_at
  ```

---

## Normalization functions (`telegram/rust/src/normalize.rs`)

### `extract_movie_static(raw)` → `MovieStatic`
Extracts static (immutable) data from a raw API movie.

### `extract_sessions(movie_id, session_types)` → `Vec<Session>`
Extracts dynamic sessions (time, price, room, format, audio) from a movie.

### `normalize_sessions_response(api_response)`
Normalizes the full endpoint-1 response into `{ movies, sessions, date, fetchedAt }`.

### `normalize_upcoming_from_sessions(future_dates, today_movie_ids)` → `Vec<UpcomingItem>`
Identifies new releases from future dates, excluding movies already showing today.

### `denormalize(movies, sessions)` → `Vec<DenormalizedMovie>`
Rebuilds the display view: joins static data + sessions into an array ready for the UI.

---

## Supported theaters

| `theaterId` | Cinema     | Shopping                    |
| ----------- | ---------- | --------------------------- |
| `1162`      | Cinesystem | Parque Shopping Maceió      |
| `1230`      | Centerplex | Shopping Pátio Maceió       |
| `924`       | Kinoplex   | Maceió Shopping             |

## Date utils (`America/Maceio` timezone)

- `maceio_date(days_offset)` → `YYYY-MM-DD` for the current day in Maceió (with offset).
- `iso_to_maceio_date(iso)` → converts an ISO timestamp to `YYYY-MM-DD` in Maceió.
