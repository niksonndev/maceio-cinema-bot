/**
 * Builders de teclados inline para o Telegram.
 */

import { CINEMAS } from './cinemas.js';
import type { InlineKeyboardMarkup } from './types.js';

export const getCinemaKeyboard = (): InlineKeyboardMarkup => ({
  inline_keyboard: CINEMAS.map((c) => [{ text: c.label, callback_data: `cinema_${c.id}` }]),
});

export const getMainKeyboard = (): InlineKeyboardMarkup => ({
  inline_keyboard: [
    [
      { text: '🎬 Filmes de Hoje', callback_data: 'filmes_hoje' },
      { text: '📅 Filmes de Amanhã', callback_data: 'filmes_amanha' },
    ],
    [{ text: '🆕 Próximos Lançamentos', callback_data: 'proximos_lancamentos' }],
    [{ text: '🔄 Trocar de Cinema', callback_data: 'trocar_cinema' }],
  ],
});

export function getBackButtonMarkup(cinemaUrl: string | null | undefined): InlineKeyboardMarkup {
  const rows: InlineKeyboardMarkup['inline_keyboard'] = [];
  if (cinemaUrl) {
    rows.push([{ text: '🎫 Comprar Ingressos', url: cinemaUrl }]);
  }
  rows.push([
    { text: '⬅️ Voltar ao menu', callback_data: 'voltar_menu' },
    { text: '🔄 Trocar cinema', callback_data: 'trocar_cinema' },
  ]);
  return { inline_keyboard: rows };
}

export function getCarouselKeyboard(
  type: string,
  index: number,
  total: number,
  cinemaUrl: string | null | undefined,
): InlineKeyboardMarkup {
  const rows: InlineKeyboardMarkup['inline_keyboard'] = [];

  if (total > 1) {
    const nav: InlineKeyboardMarkup['inline_keyboard'][number] = [];
    if (index > 0) {
      nav.push({
        text: '◀ Anterior',
        callback_data: `carousel_${type}_${index - 1}_${total}`,
      });
    }
    if (index < total - 1) {
      nav.push({
        text: 'Próximo ▶',
        callback_data: `carousel_${type}_${index + 1}_${total}`,
      });
    }
    if (nav.length > 0) rows.push(nav);
  }

  if (cinemaUrl) {
    rows.push([{ text: '🎫 Comprar Ingressos', url: cinemaUrl }]);
  }
  rows.push([
    { text: '⬅️ Voltar ao menu', callback_data: 'voltar_menu' },
    { text: '🔄 Trocar cinema', callback_data: 'trocar_cinema' },
  ]);
  return { inline_keyboard: rows };
}
