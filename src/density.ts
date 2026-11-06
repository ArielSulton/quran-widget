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
