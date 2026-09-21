#!/usr/bin/env node

/**
 * CLI to query Cinesystem Maceió schedules via the official API.
 *
 * Usage:
 *   npx tsx src/index.ts [date]
 *
 * Examples:
 *   npm start                 → today (per API / Maceió timezone)
 *   npm start -- 2026-02-23   → specific date (YYYY-MM-DD)
 */

import { fetchNormalized } from './api.js';
import { denormalize } from './normalize.js';
import { errorMessage } from './types.js';

async function main(): Promise<void> {
  const date = process.argv[2] || null;

  console.log('📡 Fetching Cinesystem Maceió schedule via API...');
  if (date) {
    console.log(`📅 Requested date: ${date} (YYYY-MM-DD)`);
  } else {
    console.log('📅 No date provided, using current API date.');
  }

  const normalized = await fetchNormalized(date);
  const movies = denormalize(normalized.movies, normalized.sessions);

  console.log(`📽️  Movies: ${movies.length}`);

  if (movies.length === 0) {
    console.log('⚠️  No sessions found for this date');
    return;
  }

  movies.forEach((m) => {
    const sessionsList = (m.sessions || [])
      .map((s) => {
        let str = s.time || '';
        if (s.priceInteira != null) {
          str += ` (R$ ${Number(s.priceInteira).toFixed(2)})`;
        }
        return str;
      })
      .join(', ');
    console.log(`  🎬 ${m.name}: ${(m.sessions || []).length} session(s)`);
    console.log(`     ${sessionsList}`);
  });
}

main().catch((err: unknown) => {
  console.error('❌ Error:', errorMessage(err));
  process.exit(2);
});
