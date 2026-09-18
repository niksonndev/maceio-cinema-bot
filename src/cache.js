/**
 * Cache normalizado para dados do Ingresso.com
 *
 * Estrutura do arquivo:
 * {
 *   movies: { [movieId]: MovieStatic },           // dados estáticos (raro mudar)
 *   sessions: { [date]: { fetchedAt, items } },   // dados dinâmicos por data
 *   upcoming: { fetchedAt, items },               // próximos lançamentos
 *   moviesUpdatedAt: ISO string
 * }
 *
 * Regras de expiração:
 * - Sessões expiram na virada do dia (fuso America/Maceio)
 * - Filmes estáticos são atualizados apenas quando uma nova sessão traz um filme desconhecido
 */

import fs from 'fs';
import { S3Client, GetObjectCommand, PutObjectCommand } from '@aws-sdk/client-s3';

const CACHE_FILE = 'data/cache.json';
const USE_S3 = !!process.env.S3_BUCKET;

/** S3 client shared with prefs — honors AWS_ENDPOINT_URL (LocalStack). */
export function createS3Client() {
  const config = {
    region: process.env.AWS_REGION || 'us-east-1',
  };
  if (process.env.AWS_ENDPOINT_URL) {
    config.endpoint = process.env.AWS_ENDPOINT_URL;
    config.forcePathStyle = true;
    config.credentials = {
      accessKeyId: process.env.AWS_ACCESS_KEY_ID || 'test',
      secretAccessKey: process.env.AWS_SECRET_ACCESS_KEY || 'test',
    };
  }
  return new S3Client(config);
}

export function isS3NotFound(err) {
  return (
    err?.name === 'NoSuchKey' ||
    err?.name === 'NotFound' ||
    err?.Code === 'NoSuchKey' ||
    err?.$metadata?.httpStatusCode === 404
  );
}

/** Converte um stream do S3 em string (Node 20-compatible). */
export async function streamToString(stream) {
  const chunks = [];
  for await (const chunk of stream) {
    chunks.push(typeof chunk === 'string' ? Buffer.from(chunk) : chunk);
  }
  return Buffer.concat(chunks).toString('utf-8');
}

let s3;
if (USE_S3) {
  s3 = createS3Client();
}

class NormalizedCache {
  constructor() {
    this.data = { movies: {}, sessions: {}, upcoming: {}, moviesUpdatedAt: null };
  }

  getMaceioDate(offsetDays = 0) {
    const now = new Date();
    const maceio = new Date(now.toLocaleString('en-US', { timeZone: 'America/Maceio' }));
    maceio.setDate(maceio.getDate() + offsetDays);
    return maceio.toISOString().split('T')[0];
  }

  toMaceioDateStr(isoString) {
    return new Date(isoString).toLocaleString('en-CA', {
      timeZone: 'America/Maceio',
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
    });
  }

  async load() {
    try {
      if (USE_S3) {
        const res = await s3.send(
          new GetObjectCommand({
            Bucket: process.env.S3_BUCKET,
            Key: process.env.CACHE_KEY || 'cache.json',
          }),
        );
        this.data = JSON.parse(await streamToString(res.Body));
      } else if (fs.existsSync(CACHE_FILE)) {
        this.data = JSON.parse(fs.readFileSync(CACHE_FILE, 'utf-8'));
      }
    } catch (err) {
      this.data = { movies: {}, sessions: {}, upcoming: {}, moviesUpdatedAt: null };
      if (!isS3NotFound(err)) {
        console.warn('⚠️  Cache corrompido, reinicializando:', err.message);
      }
    }
  }

  async save() {
    try {
      if (USE_S3) {
        await s3.send(
          new PutObjectCommand({
            Bucket: process.env.S3_BUCKET,
            Key: process.env.CACHE_KEY || 'cache.json',
            Body: JSON.stringify(this.data, null, 2),
          }),
        );
      } else {
        if (!fs.existsSync('data')) {
          fs.mkdirSync('data', { recursive: true });
        }
        fs.writeFileSync(CACHE_FILE, JSON.stringify(this.data, null, 2), 'utf-8');
      }
    } catch (err) {
      console.error('❌ Erro ao salvar cache:', err.message);
    }
  }

  /**
   * Mescla filmes estáticos no cache.
   * Só sobrescreve se o filme ainda não existe — evita writes desnecessários.
   * @returns {number} Quantidade de filmes novos adicionados
   */
  mergeMovies(movies) {
    let added = 0;
    for (const [id, movie] of Object.entries(movies)) {
      if (!this.data.movies[id]) {
        this.data.movies[id] = movie;
        added++;
      }
    }
    if (added > 0) {
      this.data.moviesUpdatedAt = new Date().toISOString();
      console.log(`💾 ${added} filme(s) novo(s) adicionado(s) ao cache estático`);
    }
    return added;
  }

  /**
   * Salva sessões dinâmicas para uma data e teatro específicos.
   */
  async setSessions(date, sessions, fetchedAt, theaterId = '1162') {
    if (!this.data.sessions[theaterId]) this.data.sessions[theaterId] = {};
    this.data.sessions[theaterId][date] = { fetchedAt, items: sessions };
    this.purgeOldSessions();
    await this.save();
    console.log(`💾 ${sessions.length} sessão(ões) salva(s) para ${date} (teatro ${theaterId})`);
  }

  /**
   * Retorna sessões de uma data/teatro se o cache for válido (mesmo dia em Maceió).
   * @returns {{ items: Array, fetchedAt: string } | null}
   */
  getSessions(date, theaterId = '1162') {
    const theaterSessions = this.data.sessions[theaterId];
    if (!theaterSessions) return null;

    const cached = theaterSessions[date];
    if (!cached?.fetchedAt) return null;

    const cachedDay = this.toMaceioDateStr(cached.fetchedAt);
    const today = this.getMaceioDate(0);

    if (cachedDay !== today) {
      console.log(`📅 Cache de sessões para ${date} expirado (${cachedDay} → ${today})`);
      delete theaterSessions[date];
      return null;
    }

    console.log(`✅ Cache hit: sessões de ${date} (teatro ${theaterId})`);
    return cached;
  }

  /**
   * Retorna um filme estático pelo ID.
   */
  getMovie(id) {
    return this.data.movies[id] ?? null;
  }

  /**
   * Retorna todos os filmes estáticos.
   */
  getAllMovies() {
    return this.data.movies;
  }

  /**
   * Salva próximos lançamentos no cache para um teatro específico.
   */
  async setUpcoming(items, fetchedAt, theaterId = '1162') {
    if (!this.data.upcoming || typeof this.data.upcoming !== 'object') {
      this.data.upcoming = {};
    }
    this.data.upcoming[theaterId] = { fetchedAt, items };
    await this.save();
    console.log(`💾 ${items.length} lançamento(s) salvo(s) no cache (teatro ${theaterId})`);
  }

  /**
   * Retorna próximos lançamentos de um teatro se o cache for válido (mesmo dia em Maceió).
   * @returns {{ items: Array, fetchedAt: string } | null}
   */
  getUpcoming(theaterId = '1162') {
    const cached = this.data.upcoming?.[theaterId];
    if (!cached?.fetchedAt) return null;

    const cachedDay = this.toMaceioDateStr(cached.fetchedAt);
    const today = this.getMaceioDate(0);

    if (cachedDay !== today) {
      console.log(
        `📅 Cache de lançamentos expirado para teatro ${theaterId} (${cachedDay} → ${today})`,
      );
      delete this.data.upcoming[theaterId];
      return null;
    }

    console.log(`✅ Cache hit: próximos lançamentos (teatro ${theaterId})`);
    return cached;
  }

  /**
   * Remove sessões de datas passadas para todos os teatros.
   */
  purgeOldSessions() {
    const today = this.getMaceioDate(0);
    for (const theaterId of Object.keys(this.data.sessions)) {
      const theaterSessions = this.data.sessions[theaterId];
      if (typeof theaterSessions !== 'object' || theaterSessions === null) continue;
      for (const date of Object.keys(theaterSessions)) {
        if (date < today) {
          delete theaterSessions[date];
        }
      }
    }
  }
}

export default NormalizedCache;
