/**
 * Definições de cinemas em Maceió e preferência por usuário.
 *
 * Persistência dual (igual ao cache):
 * - Local: data/prefs.json
 * - Produção / testes S3: objeto PREFS_KEY (padrão prefs.json) no bucket S3_BUCKET
 */

import fs from 'fs';
import path from 'path';
import { GetObjectCommand, PutObjectCommand } from '@aws-sdk/client-s3';
import { createS3Client, isS3NotFound, streamToString } from './cache.js';

export const CINEMAS = [
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

let s3;
if (USE_S3) {
  s3 = createS3Client();
}

/** @type {Record<string, string>} chatId → theaterId */
let prefs = {};

export async function loadPrefs() {
  try {
    if (USE_S3) {
      const res = await s3.send(
        new GetObjectCommand({
          Bucket: process.env.S3_BUCKET,
          Key: PREFS_KEY,
        }),
      );
      const parsed = JSON.parse(await streamToString(res.Body));
      prefs = parsed && typeof parsed === 'object' && !Array.isArray(parsed) ? parsed : {};
    } else if (fs.existsSync(PREFS_FILE)) {
      const parsed = JSON.parse(fs.readFileSync(PREFS_FILE, 'utf-8'));
      prefs = parsed && typeof parsed === 'object' && !Array.isArray(parsed) ? parsed : {};
    } else {
      prefs = {};
    }
  } catch (err) {
    prefs = {};
    if (!isS3NotFound(err)) {
      console.warn('⚠️  Prefs corrompidas, reinicializando:', err.message);
    }
  }
}

async function savePrefs() {
  const body = JSON.stringify(prefs, null, 2);
  try {
    if (USE_S3) {
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
    console.error('❌ Erro ao salvar prefs:', err.message);
    throw err;
  }
}

export async function setUserCinema(chatId, theaterId) {
  prefs[String(chatId)] = String(theaterId);
  await savePrefs();
}

export async function getUserCinema(chatId) {
  const theaterId = prefs[String(chatId)];
  if (!theaterId) return null;
  return CINEMAS.find((c) => c.id === theaterId) || null;
}

export function findCinemaById(theaterId) {
  return CINEMAS.find((c) => c.id === String(theaterId)) || null;
}

/** Test helper: drop in-memory copy without touching storage (simulates a new Lambda). */
export function resetPrefsMemory() {
  prefs = {};
}

/** Test helper: wipe prefs in memory and storage. */
export async function clearPrefs() {
  prefs = {};
  await savePrefs();
}
