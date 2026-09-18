# Cache — Strategy and Expiration Rules

> The cache avoids unnecessary requests to the Ingresso.com API, reducing
> latency and external dependency.

## Storage backend

| Environment | Where | How |
| ----------- | ----- | --- |
| Local (`npm start` / `bot:listen`) | `data/cache.json` + `data/prefs.json` | File on disk |
| Production (SAM / Lambda) | S3 | Objects `CACHE_KEY` (`cache.json`) and `PREFS_KEY` (`prefs.json`) in bucket `S3_BUCKET` |
| Tests (`npm test`) | LocalStack S3 | Same production code; `AWS_ENDPOINT_URL` points at the container |

Choice is automatic in `src/cache.ts` / `src/cinemas.ts`: if `process.env.S3_BUCKET` is
set, uses the AWS SDK (`GetObject` / `PutObject`); otherwise uses the local file.
`AWS_ENDPOINT_URL` (with `forcePathStyle`) enables LocalStack without forking the code.

BotFunction **reloads** cache and prefs from S3 on **every invoke**, so it does not serve
data from another instance or a stale warm.

## Logical storage

| Type              | Where (logical)    | Persisted? | Expires?                                      |
| ----------------- | ------------------ | ---------- | --------------------------------------------- |
| Static movies     | `movies`           | ✅ Yes     | Never (updated on demand)                     |
| Sessions          | `sessions`         | ✅ Yes     | Expires at midnight (`America/Maceio` timezone) |
| Upcoming          | `upcoming`         | ✅ Yes     | Expires at midnight (`America/Maceio` timezone) |
| Ratings           | In-memory Map      | ❌ No      | 24h (`CACHE_TTL_MS`)                          |
| Preferences       | `prefs.json`       | ✅ Yes     | Does not expire (write-through per chatId)    |

## JSON structure

```
cache.json
├── movies        → { movieId: MovieStatic }    (static)
├── sessions      → { theaterId: { date: { fetchedAt, items } } }  (dynamic)
├── upcoming      → { theaterId: { fetchedAt, items } }            (dynamic)
└── moviesUpdatedAt → ISO string

prefs.json
└── { "<chatId>": "<theaterId>" }
```

## Expiration rules

### Sessions and upcoming — daily expiration

Implemented in `src/cache.ts`:

- `getSessions(date, theaterId)` → compares `fetchedAt` with the current day in `America/Maceio`. If the day differs, **deletes the entry** and returns `null` (cache miss → forces a new request).
- `getUpcoming(theaterId)` → same logic.
- `purgeOldSessions()` → removes all sessions with `date < today` (called after `setSessions`).

```js
// cache.js — simplified excerpt
const cachedDay = this.toMaceioDateStr(cached.fetchedAt); // convert to YYYY-MM-DD in Maceió
const today = this.getMaceioDate(0);                      // today in Maceió
if (cachedDay !== today) {
  delete theaterSessions[date];
  return null; // expired
}
return cached;  // valid
```

### Static movies — on-demand update

- `mergeMovies(movies)` **never overwrites** an existing movie.
- Only adds **new** `movieId`s and updates `moviesUpdatedAt`.

## Cache hit/miss logic

Implemented in `src/data.ts`:

```js
// getMoviesForDate()
const cached = cache.getSessions(targetDate, theaterId);
if (cached) {
  // ✅ CACHE HIT — no API request
  return { movies: denormalize(...), fromCache: true };
}
// ❌ CACHE MISS — fetch + normalize + save
const normalized = await fetchNormalized(date, theaterId);
cache.mergeMovies(normalized.movies);
await cache.setSessions(normalized.date, normalized.sessions, normalized.fetchedAt, theaterId);
```

## Daily warm (production)

The `fetchHandler` Lambda (EventBridge, `cron(0 3 * * ? *)` = midnight in Maceió)
preloads sessions and upcoming for all 3 theaters and writes to S3, reducing cold-start
fetch on user interactions. After deploy, run `npm run sam:warm` once
to register the webhook and warm the cache (otherwise wait for the 03:00 UTC cron).

## 3-cinema strategy

The cache is **indexed by `theaterId`**, so the 3 cinemas (Cinesystem `1162`,
Centerplex `1230`, Kinoplex `924`) keep independent sessions/upcoming, each with
its own daily expiration.

## Ratings cache (in-memory)

```js
// src/ratings.ts
const CACHE_TTL_MS = 24 * 60 * 60 * 1000; // 24h
const memoryCache = new Map();             // key: "title|year" → { at, data }
```

- Fetches IMDb/RT via OMDb; on failure, falls back to TMDb.
- Result cached for 24h to avoid repeated calls.
- Lost on restart / cold start (does not affect functionality — just re-fetches).

## `data/` directory (local)

- Created automatically (`fs.mkdirSync('data', { recursive: true })`) on the first local `save()`.
- Contains `cache.json` and `prefs.json`. On ephemeral containers (e.g. legacy Render) these files
  do not survive restarts — that is why production uses S3.
