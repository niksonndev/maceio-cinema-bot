/**
 * Cinema definitions for Maceió and per-user preferences.
 */

import fs from 'fs';
import path from 'path';
import { GetObjectCommand, PutObjectCommand } from '@aws-sdk/client-s3';
import { createS3Client, isS3NotFound, streamToString } from './cache.js';
import type { Cinema } from './types.js';
import { errorMessage } from './types.js';

export const CINEMAS: Cinema[] = [
  {
    id: '1162',
    name: 'Cinesystem',
    label: 'Cinesystem (Parque Shopping Maceió)',
    url: 'https://www.ingresso.com/cinema/cinesystem-maceio?city=maceio',
  },
  {
    id: '1230',
    name: 'Centerplex',
    label: 'Centerplex (Shopping Pátio Maceió)',
    url: 'https://www.ingresso.com/cinema/centerplex-shopping-patio-maceio?city=maceio',
  },
  {
    id: '924',
    name: 'Kinoplex',
    label: 'Kinoplex (Maceió Shopping)',
    url: 'https://www.ingresso.com/cinema/kinoplex-maceio?city=maceio',
  },
];

const PREFS_FILE = process.env.PREFS_FILE || 'data/prefs.json';
const USE_S3 = !!process.env.S3_BUCKET;
const PREFS_KEY = process.env.PREFS_KEY || 'prefs.json';

const s3 = USE_S3 ? createS3Client() : undefined;

let prefs: Record<string, string> = {};

function isPrefsMap(value: unknown): value is Record<string, string> {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}

export async function loadPrefs(): Promise<void> {
  try {
    if (USE_S3) {
      if (!s3) throw new Error('S3 client not initialized');
      const res = await s3.send(
        new GetObjectCommand({
          Bucket: process.env.S3_BUCKET,
          Key: PREFS_KEY,
        }),
      );
      const parsed: unknown = JSON.parse(await streamToString(res.Body));
      prefs = isPrefsMap(parsed) ? parsed : {};
    } else if (fs.existsSync(PREFS_FILE)) {
      const parsed: unknown = JSON.parse(fs.readFileSync(PREFS_FILE, 'utf-8'));
      prefs = isPrefsMap(parsed) ? parsed : {};
    } else {
      prefs = {};
    }
  } catch (err) {
    prefs = {};
    if (!isS3NotFound(err)) {
      console.warn('⚠️  Prefs corrupted, reinitializing:', errorMessage(err));
    }
  }
}

async function savePrefs(): Promise<void> {
  const body = JSON.stringify(prefs, null, 2);
  try {
    if (USE_S3) {
      if (!s3) throw new Error('S3 client not initialized');
      await s3.send(
        new PutObjectCommand({
          Bucket: process.env.S3_BUCKET,
          Key: PREFS_KEY,
          Body: body,
        }),
      );
    } else {
      const dir = path.dirname(PREFS_FILE);
      if (!fs.existsSync(dir)) {
        fs.mkdirSync(dir, { recursive: true });
      }
      fs.writeFileSync(PREFS_FILE, body, 'utf-8');
    }
  } catch (err) {
    console.error('❌ Failed to save prefs:', errorMessage(err));
    throw err;
  }
}

export async function setUserCinema(
  chatId: number | string,
  theaterId: string | number,
): Promise<void> {
  prefs[String(chatId)] = String(theaterId);
  await savePrefs();
}

export async function getUserCinema(chatId: number | string): Promise<Cinema | null> {
  const theaterId = prefs[String(chatId)];
  if (!theaterId) return null;
  return CINEMAS.find((c) => c.id === theaterId) || null;
}

export function findCinemaById(theaterId: string | number): Cinema | null {
  return CINEMAS.find((c) => c.id === String(theaterId)) || null;
}

export function resetPrefsMemory(): void {
  prefs = {};
}

export async function clearPrefs(): Promise<void> {
  prefs = {};
  await savePrefs();
}
