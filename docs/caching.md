# Cache — Strategy and Expiration Rules

> The cache avoids unnecessary requests to the Ingresso.com API, reducing
> latency and external dependency.

## Storage backend

| Environment | Where | How |
| ----------- | ----- | --- |
| Local (`telegram-poll`) | `data/cache.json` + `data/prefs.json` | File on disk |
| Production (SAM / Lambda) | S3 | Objects `CACHE_KEY` (`cache.json`) and `PREFS_KEY` (`prefs.json`) in bucket `S3_BUCKET` |
| Local S3 testing | LocalStack | `AWS_ENDPOINT_URL` points at the custom endpoint |

Choice is automatic in `telegram/rust/src/store.rs` / `telegram/rust/src/prefs.rs`: if `S3_BUCKET` is
set, uses the AWS SDK (`GetObject` / `PutObject`); otherwise uses the local file.
`AWS_ENDPOINT_URL` (with `forcePathStyle`) enables LocalStack without forking the code.

### Read policy inside BotFunction

| Object | Read behavior | Why |
| ------ | ------------- | --- |
| `cache.json` | Memoized for **60s per warm container** (`S3_LOAD_TTL` in `store.rs`), then `GetObject` again | Saves one S3 round trip on every update without serving data older than a minute |
| `prefs.json` | `GetObject` on **every invoke** (`from_env_with_ttl(..., Duration::ZERO)`) | Preferences change at any time; a stale read could send a user to the wrong cinema |

Writes (`save`) update the memoized payload immediately, so a container never reads back its
own stale copy. Other containers converge within 60s for the schedule cache and immediately for
prefs.

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

Implemented in `telegram/rust/src/store.rs`:

- `getSessions(date, theaterId)` → compares `fetchedAt` with the current day in `America/Maceio`. If the day differs, **deletes the entry** and returns `null` (cache miss → forces a new request).
- `getUpcoming(theaterId)` → same logic.
- `purgeOldSessions()` → removes all sessions with `date < today` (called after `setSessions`).

```rust
if iso_to_maceio_date(&cached.fetched_at) != maceio_date(0) {
    return None;
}
Some(cached.clone())
```

### Static movies — on-demand update

- `mergeMovies(movies)` **never overwrites** an existing movie.
- Only adds **new** `movieId`s and updates `moviesUpdatedAt`.

## Cache hit/miss logic

Implemented in `telegram/rust/src/data.rs`:

Cache hits return denormalized movies without writing S3. On a miss, the service fetches and normalizes Ingresso data, merges static movies, stores sessions, and saves the changed cache before replying.

## Daily warm (production)

The Rust `telegram-fetch` Lambda (EventBridge, `cron(0 3 * * ? *)` = midnight in Maceió)
preloads sessions and upcoming for **today and tomorrow** in all 3 theaters and writes to S3,
so the first user of the day — and every user asking for tomorrow — hits a warm cache.
After deploy, run `bash scripts/sam-warm.sh` once to register the webhook and warm the cache
(otherwise wait for the 03:00 UTC cron).

## 3-cinema strategy

The cache is **indexed by `theaterId`**, so the 3 cinemas (Cinesystem `1162`,
Centerplex `1230`, Kinoplex `924`) keep independent sessions/upcoming, each with
its own daily expiration.

## Ratings cache (in-memory)

```rust
// telegram/rust/src/ratings.rs
const CACHE_TTL = Duration::from_secs(24 * 60 * 60);
static CACHE: OnceLock<DashMap<String, CacheEntry>>;
```

- Fetches IMDb/RT via OMDb; on failure, falls back to TMDb.
- Result cached for 24h to avoid repeated calls.
- Lost on restart / cold start (does not affect functionality — just re-fetches).

## `data/` directory (local)

- Created automatically when the Rust local store first saves.
- Contains `cache.json` and `prefs.json`. On ephemeral containers (e.g. legacy Render) these files
  do not survive restarts — that is why production uses S3.
