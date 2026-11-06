import type { KeyValueStore } from "./theme";

/** Geser sinkron pengguna, dalam ms. Positif = pergantian ayat muncul LEBIH LAMBAT. */
export const SYNC_KEY = "quran-widget.syncMs";
export const SYNC_STEP_MS = 100;
/** Langkah kasar (Shift): satu detik penuh. */
export const SYNC_COARSE_STEP_MS = 1000;
export const SYNC_MIN_MS = -2000;
export const SYNC_MAX_MS = 2000;

const MINUS = "−";

/** Bulatkan ke langkah 100 ms (simetris terhadap nol) lalu jepit ke ±2000. NaN/∞ => 0. */
export function clampSync(ms: number): number {
  if (!Number.isFinite(ms)) return 0;
  const steps = Math.sign(ms) * Math.round(Math.abs(ms) / SYNC_STEP_MS);
  const rounded = steps * SYNC_STEP_MS;
  const clamped = Math.min(SYNC_MAX_MS, Math.max(SYNC_MIN_MS, rounded));
  return clamped === 0 ? 0 : clamped; // buang -0
}

/** "+0,3 dtk" / "0,0 dtk" / "−1,5 dtk" (koma desimal, minus asli U+2212). */
export function formatSync(ms: number): string {
  const v = clampSync(ms);
  const tenths = Math.abs(v) / SYNC_STEP_MS;
  const body = `${Math.floor(tenths / 10)},${tenths % 10} dtk`;
  if (v === 0) return body;
  return `${v > 0 ? "+" : MINUS}${body}`;
}

/** Naik (+1) atau turun (-1) satu langkah (0,1 dtk; `coarse` = 1,0 dtk), tetap di dalam rentang. */
export function stepSync(ms: number, dir: 1 | -1, coarse = false): number {
  const step = coarse ? SYNC_COARSE_STEP_MS : SYNC_STEP_MS;
  return clampSync(clampSync(ms) + dir * step);
}

/** Baca geser tersimpan; hilang, rusak, atau bukan bilangan bulat => 0. */
export function loadSync(store: KeyValueStore): number {
  try {
    const v = store.getItem(SYNC_KEY);
    if (v === null || !/^-?\d+$/.test(v)) return 0;
    return clampSync(Number(v));
  } catch {
    return 0;
  }
}

export function saveSync(store: KeyValueStore, ms: number): void {
  try {
    store.setItem(SYNC_KEY, String(clampSync(ms)));
  } catch {
    /* tidak kritis */
  }
}
