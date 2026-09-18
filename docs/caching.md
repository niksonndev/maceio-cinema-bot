# Cache — Estratégia e Regras de Expiração

> O cache evita requisições desnecessárias à API do Ingresso.com, reduzindo
> latência e dependência externa.

## Backend de armazenamento

| Ambiente | Onde | Como |
| -------- | ---- | ---- |
| Local (`npm start` / `bot:listen`) | `data/cache.json` + `data/prefs.json` | Arquivo no disco |
| Produção (SAM / Lambda) | S3 | Objetos `CACHE_KEY` (`cache.json`) e `PREFS_KEY` (`prefs.json`) no bucket `S3_BUCKET` |
| Testes (`npm test`) | LocalStack S3 | Mesmo código de produção, `AWS_ENDPOINT_URL` aponta para o container |

A escolha é automática em `src/cache.js` / `src/cinemas.js`: se `process.env.S3_BUCKET` estiver
definido, usa o AWS SDK (`GetObject` / `PutObject`); senão, usa o arquivo local.
`AWS_ENDPOINT_URL` (com `forcePathStyle`) habilita LocalStack sem fork do código.

A BotFunction **recarrega** cache e prefs do S3 a **cada invoke**, para não servir
dados de outra instância ou de um warm antigo.

## Armazenamento lógico

| Tipo              | Onde (lógico)      | Persistido? | Expira?                                   |
| ----------------- | ------------------ | ----------- | ----------------------------------------- |
| Filmes estáticos  | `movies`           | ✅ Sim      | Nunca expira (atualizado sob demanda)     |
| Sessões           | `sessions`         | ✅ Sim      | Expira à meia-noite (fuso `America/Maceio`) |
| Lançamentos       | `upcoming`         | ✅ Sim      | Expira à meia-noite (fuso `America/Maceio`) |
| Ratings           | Map em memória     | ❌ Não      | 24h (`CACHE_TTL_MS`)                      |
| Preferências      | `prefs.json`       | ✅ Sim      | Não expira (write-through por chatId)     |

## Estrutura do JSON

```
cache.json
├── movies        → { movieId: MovieStatic }    (estático)
├── sessions      → { theaterId: { date: { fetchedAt, items } } }  (dinâmico)
├── upcoming      → { theaterId: { fetchedAt, items } }            (dinâmico)
└── moviesUpdatedAt → ISO string

prefs.json
└── { "<chatId>": "<theaterId>" }
```

## Regras de expiração

### Sessões e lançamentos — expiração diária

Implementada em `src/cache.js`:

- `getSessions(date, theaterId)` → compara `fetchedAt` com o dia atual em `America/Maceio`. Se o dia for diferente, **deleta a entrada** e retorna `null` (cache miss → força nova requisição).
- `getUpcoming(theaterId)` → mesma lógica.
- `purgeOldSessions()` → remove todas as sessões com `date < today` (chamado após `setSessions`).

```js
// cache.js — trecho simplificado
const cachedDay = this.toMaceioDateStr(cached.fetchedAt); // converte para YYYY-MM-DD em Maceió
const today = this.getMaceioDate(0);                      // hoje em Maceió
if (cachedDay !== today) {
  delete theaterSessions[date];
  return null; // expirado
}
return cached;  // válido
```

### Filmes estáticos — atualização sob demanda

- `mergeMovies(movies)` **nunca sobrescreve** um filme já existente.
- Só adiciona `movieId`s **novos** e atualiza `moviesUpdatedAt`.

## Lógica de cache hit/miss

Implementada em `src/data.js`:

```js
// getMoviesForDate()
const cached = cache.getSessions(targetDate, theaterId);
if (cached) {
  // ✅ CACHE HIT — sem requisição à API
  return { movies: denormalize(...), fromCache: true };
}
// ❌ CACHE MISS — fetch + normalize + save
const normalized = await fetchNormalized(date, theaterId);
cache.mergeMovies(normalized.movies);
await cache.setSessions(normalized.date, normalized.sessions, normalized.fetchedAt, theaterId);
```

## Warm diário (produção)

A Lambda `fetchHandler` (EventBridge, `cron(0 3 * * ? *)` = meia-noite em Maceió)
pré-carrega sessões e lançamentos dos 3 teatros e grava no S3, reduzindo cold-start
fetch nas interações do usuário. Após o deploy, rode `npm run sam:warm` uma vez
para registrar o webhook e aquecer o cache (senão espera o cron das 03:00 UTC).

## Estratégia de 3 cinemas

O cache é **indexado por `theaterId`**, permitindo que os 3 cinemas (Cinesystem `1162`,
Centerplex `1230`, Kinoplex `924`) mantenham sessões/lançamentos independentes, cada um
com sua própria expiração diária.

## Cache de ratings (in-memory)

```js
// src/ratings.js
const CACHE_TTL_MS = 24 * 60 * 60 * 1000; // 24h
const memoryCache = new Map();             // key: "title|year" → { at, data }
```

- Busca IMDb/RT via OMDb; se falhar, fallback TMDb.
- Resultado cacheado por 24h para evitar chamadas repetidas.
- Perdido em reinício / cold start (não afeta funcionalidade — apenas re-faz a busca).

## Diretório `data/` (local)

- Criado automaticamente (`fs.mkdirSync('data', { recursive: true })`) no primeiro `save()` local.
- Contém `cache.json` e `prefs.json`. Em containers efêmeros (ex.: Render legado) esses arquivos
  não sobrevivem a reinícios — por isso a produção usa S3.
