import { beforeEach, describe, expect, it } from 'vitest';
import {
  clearPrefs,
  getUserCinema,
  loadPrefs,
  resetPrefsMemory,
  setUserCinema,
} from '../src/cinemas.js';

describe('cinema preferences', () => {
  beforeEach(async () => {
    await clearPrefs();
  });

  it('round-trips a theaterId through S3 (LocalStack)', async () => {
    await setUserCinema(8001, '1162');
    const cinema = await getUserCinema(8001);
    expect(cinema).not.toBeNull();
    expect(cinema?.id).toBe('1162');
    expect(cinema?.name).toBe('Cinesystem');
  });

  it('reloads from storage after a simulated new process', async () => {
    await setUserCinema(8002, '924');
    resetPrefsMemory();
    expect(await getUserCinema(8002)).toBeNull();

    await loadPrefs();
    const cinema = await getUserCinema(8002);
    expect(cinema?.id).toBe('924');
  });

  it('returns null for an unknown theaterId', async () => {
    await setUserCinema(8003, '0000');
    expect(await getUserCinema(8003)).toBeNull();
  });

  it('stringifies numeric chat ids as keys', async () => {
    await setUserCinema(8004, '1230');
    const cinema = await getUserCinema('8004');
    expect(cinema?.id).toBe('1230');
  });
});
