# Data Model

> Complete reference for the data structures used internally and persisted.

## 1. Persistent cache — `data/cache.json`

Managed by the `NormalizedCache` class (`src/cache.ts`). Persisted to disk
(local) or S3 (production), along with `prefs.json`.

```jsonc
{
  "movies": {
    "12345": {
      "id": 12345,
      "title": "Avatar: Fogo E Cinzas",
      "originalTitle": "Avatar: The Way of Water",
      "urlKey": "avatar-fogo-e-cinzas",
      "duration": 192,            // minutes
      "contentRating": "14",     // age rating (e.g. "14", "16", "L")
      "ratingColor": null,
      "genres": ["Ação", "Aventura"],
      "distributor": "20th Century Studios",
      "poster": "https://...",    // PosterPortrait
      "backdrop": "https://...",  // PosterHorizontal
      "trailer": "https://...",
      "tags": ["3D"],
      "isReexhibition": false,
      "inPreSale": false          // in pre-sale
    }
  },

  "sessions": {
    "1162": {                     // theaterId (Cinesystem)
      "2026-08-19": {
        "fetchedAt": "2026-08-19T10:30:00.000Z",
        "items": [
          {
            "id": "sess_001",
            "movieId": 12345,
            "time": "14:30",
            "price": 55.86,       // raw full-price ticket
            "room": "Sala 5",
            "format": "2D",       // 2D | 3D | Cinépic | VIP
            "audio": "Dublado",   // Dublado | Legendado | null
            "checkoutUrl": "https://ingresso.com/..."
          }
        ]
      }
    },
    "1230": { },                    // Centerplex
    "924":  { }                     // Kinoplex
  },

  "upcoming": {
    "1162": {
      "fetchedAt": "2026-08-19T10:31:00.000Z",
      "items": [
        {
          "id": 67890,
          "title": "Vingadores: Ultimato 2",
          "originalTitle": null,
          "contentRating": "14",
          "genres": ["Ação"],
          "poster": "https://...",
          "inPreSale": true,
          "formats": ["2D", "3D"],
          "priceFrom": 45.00,
          "firstDate": "2026-08-25",            // YYYY-MM-DD
          "firstDateFormatted": "25 de agosto de 2026",
          "firstDateDayOfWeek": "segunda-feira",
          "siteURL": "https://ingresso.com/..."
        }
      ]
    }
  },

  "moviesUpdatedAt": "2026-08-19T10:30:00.000Z"  // last static-movies update
}
```

### Fields by type

#### `MovieStatic` (static data — rarely changes)

Extracted by `extractMovieStatic()` in `src/normalize.ts`. Identical regardless of date/cinema.

#### `Session` (dynamic data — changes by day/time)

Extracted by `extractSessions()` in `src/normalize.ts`. Each session has only the data relevant to the queried date/theater.

#### `UpcomingItem` (future releases)

Extracted by `normalizeUpcomingFromSessions()`. Identifies movies that are **not yet showing today** and have active pre-sale.

---

## 2. Display structure (denormalized)

Produced by `denormalize(movies, sessions)` in `src/normalize.ts`. This is the format consumed by the formatting layer (`format.ts`).

```js
{
  id: 12345,
  title: "Avatar: Fogo E Cinzas",
  originalTitle: "Avatar: The Way of Water",
  // ... other MovieStatic fields ...
  duration: 192,
  genres: ["Ação", "Aventura"],
  poster: "https://...",
  backdrop: "https://...",
  // fields added by denormalize():
  name: "Avatar: Fogo E Cinzas",     // alias of title
  sessions: [
    {
      time: "14:30",
      sessionId: "sess_001",
      priceInteira: 55.86,
      priceMeia: 27.93,            // priceInteira / 2 (toFixed 2)
      gratuito: false,             // true when price is null/0
      room: "Sala 5",
      format: "2D",
      audio: "Dublado"
    }
  ]
}
```

> 📌 `denormalize()` groups sessions by `movieId` and joins them with the movie's static data, producing an array ready for bot rendering.

---

## 3. Ratings (in-memory)

Temporary structure held in `src/ratings.ts`:

```js
// memoryCache: Map<string, { at: number, data: RatingsResult | null }>
// cacheKey = `${title.toLowerCase()}|${year ?? ''}`
// TTL: 24h (CACHE_TTL_MS)

RatingsResult = {
  imdb: "7.5" | null,        // from OMDb
  rottenTomatoes: "85%" | null, // from OMDb
  tmdb: "7.3" | null        // TMDb fallback
}
```

⚠️ **Not persisted to disk** — lost on process restart.

---

## 4. User preferences (persisted)

Managed in `src/cinemas.ts`. Same dual backend as the cache:

- Local: `data/prefs.json`
- Production / S3 tests: object `PREFS_KEY` (default `prefs.json`)

```jsonc
{
  "123456789": "1162",
  "987654321": "924"
}
```

Keys are always `String(chatId)`. `setUserCinema` is write-through; BotFunction
calls `loadPrefs()` at the start of **every** invoke so the choice is shared across
Lambda instances.

---

## 5. Environment variables

| Variable            | Required    | Default   | Description                                  |
| ------------------- | ----------- | --------- | -------------------------------------------- |
| `TELEGRAM_BOT_TOKEN` | Yes        | —         | Bot token (via @BotFather).                 |
| `PORT`              | No          | `10000`   | Express port (health check, local polling). |
| `OMDb_API_KEY`      | No          | —         | IMDb/RT lookup.                              |
| `TMDB_API_KEY`      | No          | —         | TMDb fallback when OMDb fails.              |
| `S3_BUCKET`         | No*         | —         | Cache and prefs bucket (*SAM in production). |
| `CACHE_KEY`         | No          | `cache.json` | S3 cache object key.                     |
| `PREFS_KEY`         | No          | `prefs.json` | S3 prefs object key.                     |
| `WEBHOOK_URL`       | No*         | —         | Webhook URL (*SAM on FetchFunction: `…/prod/webhook`). |
| `AWS_REGION`        | No          | —         | S3Client region (injected by Lambda).       |
| `AWS_ENDPOINT_URL`  | No          | —         | Custom S3 endpoint (LocalStack in tests).   |
