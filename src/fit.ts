/** Ukuran font Arab penuh untuk ayat pendek. */
export const FONT_MAX_PX = 34;
/** Lantai ukuran font: ayat terpanjang Juz 30 (98:8, 183 karakter dengan waqf sign) tetap terbaca. */
export const FONT_MIN_PX = 20;

/** Sampai panjang ini (karakter) ayat memakai ukuran penuh. */
const SHORT_CHARS = 60;
/** Mulai panjang ini ayat memakai ukuran lantai. */
const LONG_CHARS = 180;

/**
 * Ukuran font (px) untuk teks Arab: penuh (34) sampai 60 karakter, lalu mengecil
 * linear sampai lantai 20 px pada 180 karakter ke atas. Dibulatkan ke px bulat.
 */
export function arabicFontPx(text: string): number {
  const len = text.length;
  if (len <= SHORT_CHARS) return FONT_MAX_PX;
  if (len >= LONG_CHARS) return FONT_MIN_PX;
  const t = (len - SHORT_CHARS) / (LONG_CHARS - SHORT_CHARS);
  return Math.round(FONT_MAX_PX - t * (FONT_MAX_PX - FONT_MIN_PX));
}
