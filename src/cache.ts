/**
 * Cache normalizado para dados do Ingresso.com
 */

import fs from 'fs';
import { S3Client, GetObjectCommand, PutObjectCommand } from '@aws-sdk/client-s3';
import type {
  CacheData,
  MovieStatic,
  Session,
  SessionDayCache,
  UpcomingCache,
  UpcomingItem,
} from './types.js';
import { emptyCache, errorMessage } from './types.js';

const CACHE_FILE = 'data/cache.json';
const USE_S3 = !!process.env.S3_BUCKET;

type S3ErrorShape = {
  name?: string;
  Code?: string;
  $metadata?: { httpStatusCode?: number };
};

export function createS3Client(): S3Client {
  const config: ConstructorParameters<typeof S3Client>[0] = {
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

export function isS3NotFound(err: unknown): boolean {
  if (!err || typeof err !== 'object') return false;
  const e = err as S3ErrorShape;
  return (
    e.name === 'NoSuchKey' ||
    e.name === 'NotFound' ||
    e.Code === 'NoSuchKey' ||
    e.$metadata?.httpStatusCode === 404
  );
}

export async function streamToString(stream: unknown): Promise<string> {
  if (!stream) return '';
  if (
    typeof stream === 'object' &&
    'transformToString' in stream &&
    typeof (stream as { transformToString?: unknown }).transformToString === 'function'
  ) {
    return (stream as { transformToString: () => Promise<string> }).transformToString();
  }
  const chunks: Buffer[] = [];
  for await (const chunk of stream as AsyncIterable<Uint8Array | string>) {
    chunks.push(typeof chunk === 'string' ? Buffer.from(chunk) : Buffer.from(chunk));
  }
  return Buffer.concat(chunks).toString('utf-8');
}

let s3: S3Client | undefined;
if (USE_S3) {
  s3 = createS3Client();
}

export class NormalizedCache {
  data: CacheData;

  constructor() {
    this.data = emptyCache();
  }

  getMaceioDate(offsetDays = 0): string {
    const now = new Date();
    const maceio = new Date(now.toLocaleString('en-US', { timeZone: 'America/Maceio' }));
    maceio.setDate(maceio.getDate() + offsetDays);
    return maceio.toISOString().split('T')[0];
  }

  toMaceioDateStr(isoString: string): string {
    return new Date(isoString).toLocaleString('en-CA', {
      timeZone: 'America/Maceio',
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
    });
  }

  async load(): Promise<void> {
    try {
      if (USE_S3) {
        if (!s3) throw new Error('S3 client not initialized');
        const res = await s3.send(
          new GetObjectCommand({
            Bucket: process.env.S3_BUCKET,
            Key: process.env.CACHE_KEY || 'cache.json',
          }),
        );
        this.data = {
          ...emptyCache(),
          ...(JSON.parse(await streamToString(res.Body)) as CacheData),
        };
      } else if (fs.existsSync(CACHE_FILE)) {
        this.data = {
          ...emptyCache(),
          ...(JSON.parse(fs.readFileSync(CACHE_FILE, 'utf-8')) as CacheData),
        };
      }
    } catch (err) {
      this.data = emptyCache();
      if (!isS3NotFound(err)) {
        console.warn('⚠️  Cache corrompido, reinicializando:', errorMessage(err));
      }
    }
  }

  async save(): Promise<void> {
    try {
      if (USE_S3) {
        if (!s3) throw new Error('S3 client not initialized');
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
      console.error('❌ Erro ao salvar cache:', errorMessage(err));
    }
  }

  mergeMovies(movies: Record<string, MovieStatic>): number {
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

  async setSessions(
    date: string,
    sessions: Session[],
    fetchedAt: string,
    theaterId = '1162',
  ): Promise<void> {
    if (!this.data.sessions[theaterId]) this.data.sessions[theaterId] = {};
    this.data.sessions[theaterId][date] = { fetchedAt, items: sessions };
    this.purgeOldSessions();
    await this.save();
    console.log(`💾 ${sessions.length} sessão(ões) salva(s) para ${date} (teatro ${theaterId})`);
  }

  getSessions(date: string, theaterId = '1162'): SessionDayCache | null {
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

  getMovie(id: string | number): MovieStatic | null {
    return this.data.movies[id] ?? null;
  }

  getAllMovies(): Record<string, MovieStatic> {
    return this.data.movies;
  }

  async setUpcoming(items: UpcomingItem[], fetchedAt: string, theaterId = '1162'): Promise<void> {
    if (!this.data.upcoming || typeof this.data.upcoming !== 'object') {
      this.data.upcoming = {};
    }
    this.data.upcoming[theaterId] = { fetchedAt, items };
    await this.save();
    console.log(`💾 ${items.length} lançamento(s) salvo(s) no cache (teatro ${theaterId})`);
  }

  getUpcoming(theaterId = '1162'): UpcomingCache | null {
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

  purgeOldSessions(): void {
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
