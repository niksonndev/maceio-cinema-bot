# API Reference — Ingresso.com

> Referência da API pública do Ingresso.com consumida pelo bot.

## Configuração base

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

A API é **pública e não exige key**. Os headers browser-like evitam bloqueios.

## Endpoints

### 1. `GET /v0/sessions/city/{cityId}/theater/{theaterId}/partnership/home/groupBy/sessionType?date={date}`

Retorna sessões de um cinema para uma data específica, agrupadas por tipo de sessão.

| Parâmetro  | Tipo   | Descrição                              | Exemplo          |
| ---------- | ------ | -------------------------------------- | ---------------- |
| `cityId`   | number | ID da cidade (53 = Maceió)            | `53`             |
| `theaterId`| number | ID do cinema                          | `1162`, `1230`, `924` |
| `date`     | string | Data no formato `YYYY-MM-DD`          | `2026-08-19`     |

**Consumido por:** `fetchNormalized()` → `normalizeSessionsResponse()`

---

### 2. `GET /v0/sessions/city/{cityId}/theater/{theaterId}`

Retorna todas as sessões de um cinema — para todas as datas (passado, hoje e futuro).

**Consumido por:** `fetchUpcoming()` → identifica lançamentos futuros em pré-venda.

## Funções exportadas (`src/api.ts`)

### `fetchNormalized(date = null, theaterId = 1162)`

- Resolve a data alvo: usa `date` se informado, senão data atual no fuso `America/Maceio`.
- Faz o request ao endpoint 1.
- Retorna dados normalizados:
  ```ts
  {
    movies: Record<string, MovieStatic>,
    sessions: Session[],
    date: string,          // YYYY-MM-DD
    fetchedAt: string      // ISO timestamp
  }
  ```

### `fetchUpcoming(theaterId = 1162)`

- Pega todas as datas do endpoint 2.
- Filtra filmes **não em cartaz hoje** (`todayMovieIds`).
- Filtra apenas os em **pré-venda** (`inPreSale === true`).
- Retorna:
  ```ts
  { items: UpcomingItem[], fetchedAt: string }
  ```

---

## Funções de normalização (`src/normalize.ts`)

### `extractMovieStatic(raw)` → `MovieStatic`
Extrai dados estáticos (imutáveis) de um filme cru da API.

### `extractSessions(movieId, sessionTypes)` → `Session[]`
Extrai sessões dinâmicas (horário, preço, sala, formato, áudio) de um filme.

### `normalizeSessionsResponse(apiResponse)`
Normaliza a resposta completa do endpoint 1 em `{ movies, sessions, date, fetchedAt }`.

### `normalizeUpcomingFromSessions(futureDates, todayMovieIds)` → `UpcomingItem[]`
Identifica novos lançamentos a partir de datas futuras, excluindo filmes já em cartaz hoje.

### `denormalize(movies, sessions)` → `DisplayMovie[]`
Reconstitui a visão de exibição: junta dados estáticos + sessões em um array pronto para a UI.

---

## Teatros suportados

| `theaterId` | Cinema     | Shopping                    |
| ----------- | ---------- | --------------------------- |
| `1162`      | Cinesystem | Parque Shopping Maceió      |
| `1230`      | Centerplex | Shopping Pátio Maceió       |
| `924`       | Kinoplex   | Maceió Shopping             |

## Utils de data (fuso `America/Maceio`)

- `getMaceioDate(offsetDays = 0)` → `YYYY-MM-DD` do dia atual em Maceió (com offset).
- `toMaceioDateStr(isoString)` → converte qualquer ISO string para `YYYY-MM-DD` em Maceió.
- `getTodayInMaceioISO()` (em `api.js`) → data de hoje no formato `en-CA` (YYYY-MM-DD).
