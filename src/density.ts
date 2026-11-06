/** Tata letak teks: kepadatan (biasa/rapat) dan skala font Arab (1 = ukuran dari `arabicFontPx`). */
export type Density = "regular" | "compact";
export interface Layout {
  density: Density;
  scale: number;
}

/** Skala font Arab yang dicoba berurutan; 0,7 adalah lantai terakhir. */
export const FONT_SCALE_STEPS: readonly number[] = [1, 0.92, 0.85, 0.78, 0.7];

/**
 * Kandidat dari yang paling longgar ke yang paling rapat: biasa, lalu rapat dengan font Arab yang
 * mengecil bertahap. Ayat panjang (`preferCompact`) langsung mulai dari rapat.
 */
export function layoutCandidates(preferCompact: boolean): Layout[] {
  const compact = FONT_SCALE_STEPS.map((scale): Layout => ({ density: "compact", scale }));
  return preferCompact ? compact : [{ density: "regular", scale: 1 }, ...compact];
}

/** Kandidat pertama yang tidak melebihi kartu; bila tidak ada, yang terakhir (lantai). */
export function pickLayout(candidates: readonly Layout[], overflows: (layout: Layout) => boolean): Layout {
  for (const layout of candidates) {
    if (!overflows(layout)) return layout;
  }
  return candidates[candidates.length - 1];
}

/** Kartu bawaan (jendela 520x320 dikurangi inset bayangan 20+20 dan 16+24) = skala 1,0. */
export const CARD_REF_WIDTH = 480;
export const CARD_REF_HEIGHT = 280;
/** Batas atas pembesaran font (jendela 960x560 ke atas). */
export const SIZE_SCALE_MAX = 1.6;

const round = (n: number, digits: number): number => {
  const f = 10 ** digits;
  return Math.round(n * f) / f;
};

/**
 * Pengali ukuran font menurut ukuran kartu (px CSS): 1,0 pada kartu bawaan, tumbuh mulus untuk
 * kartu lebih besar sampai 1,6. Memakai rasio yang LEBIH KECIL antara lebar dan tinggi agar jendela
 * tinggi-sempit atau lebar-pendek tidak meluap. Tidak pernah di bawah 1,0: pengecilan di bawah
 * ukuran bawaan dikerjakan pemilih bertahap (`pickLayout`). Dibulatkan ke 0,01 agar penarikan
 * jendela tidak menulis gaya terus-menerus.
 */
export function sizeScale(cardWidth: number, cardHeight: number): number {
  const ratio = Math.min(cardWidth / CARD_REF_WIDTH, cardHeight / CARD_REF_HEIGHT);
  if (!(ratio > 1)) return 1; // juga menangkap NaN
  return round(Math.min(ratio, SIZE_SCALE_MAX), 2);
}

/** Terjemahan ikut tumbuh 60% dari laju font Arab, paling banyak 1,3x. */
export function translationScale(size: number): number {
  return round(Math.min(1.3, 1 + (size - 1) * 0.6), 3);
}

/** Footer ikut tumbuh 30% dari laju font Arab, paling banyak 1,2x. */
export function footerScale(size: number): number {
  return round(Math.min(1.2, 1 + (size - 1) * 0.3), 3);
}
