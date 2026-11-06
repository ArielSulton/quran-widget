import type { KeyValueStore } from "./theme";

/** Ukuran jendela dalam piksel logis (CSS). */
export interface WindowSize {
  w: number;
  h: number;
}

export const WINDOW_SIZE_KEY = "simak.windowSize";
export const DEFAULT_WINDOW_SIZE: WindowSize = { w: 520, h: 320 };
/** Sama dengan `minWidth`/`minHeight` di tauri.conf.json. */
export const MIN_WINDOW_SIZE: WindowSize = { w: 430, h: 270 };
export const MAX_WINDOW_SIZE: WindowSize = { w: 2000, h: 1200 };

/** Nilai di atas ini dianggap rusak (bukan sekadar jendela yang sangat besar). */
const ABSURD_PX = 10000;

const clamp = (n: number, min: number, max: number): number => Math.min(max, Math.max(min, n));

/** Bulatkan lalu jepit ke [430,2000] x [270,1200]. */
export function clampWindowSize(size: WindowSize): WindowSize {
  return {
    w: clamp(Math.round(size.w), MIN_WINDOW_SIZE.w, MAX_WINDOW_SIZE.w),
    h: clamp(Math.round(size.h), MIN_WINDOW_SIZE.h, MAX_WINDOW_SIZE.h),
  };
}

function sane(n: unknown): n is number {
  return typeof n === "number" && Number.isFinite(n) && n > 0 && n <= ABSURD_PX;
}

/**
 * Baca JSON `{"w":int,"h":int}`. Hilang, rusak, bertipe salah, nol/negatif atau tak masuk akal
 * (> 10000) => ukuran bawaan; angka masuk akal di luar rentang dijepit.
 */
export function parseWindowSize(raw: string | null): WindowSize {
  if (raw === null) return { ...DEFAULT_WINDOW_SIZE };
  try {
    const v: unknown = JSON.parse(raw);
    if (typeof v !== "object" || v === null) return { ...DEFAULT_WINDOW_SIZE };
    const { w, h } = v as Record<string, unknown>;
    if (!sane(w) || !sane(h)) return { ...DEFAULT_WINDOW_SIZE };
    return clampWindowSize({ w, h });
  } catch {
    return { ...DEFAULT_WINDOW_SIZE };
  }
}

/** Baca ukuran tersimpan; penyimpanan yang tak dapat diakses => bawaan. */
export function loadWindowSize(store: KeyValueStore): WindowSize {
  try {
    return parseWindowSize(store.getItem(WINDOW_SIZE_KEY));
  } catch {
    return { ...DEFAULT_WINDOW_SIZE };
  }
}

/** Simpan (sudah dijepit); kegagalan penyimpanan diabaikan, ukuran hanya tidak bertahan. */
export function saveWindowSize(store: KeyValueStore, size: WindowSize): void {
  const { w, h } = clampWindowSize(size);
  try {
    store.setItem(WINDOW_SIZE_KEY, JSON.stringify({ w, h }));
  } catch {
    /* tidak kritis */
  }
}

/** Langkah panah pada pegangan ubah ukuran: 10 px, Shift 50 px. */
export const KEY_STEP_PX = 10;
export const KEY_STEP_COARSE_PX = 50;

/** Ukuran baru untuk tombol panah (sudah dijepit); `null` bila bukan tombol panah. */
export function stepWindowSize(size: WindowSize, key: string, coarse: boolean): WindowSize | null {
  const d = coarse ? KEY_STEP_COARSE_PX : KEY_STEP_PX;
  switch (key) {
    case "ArrowRight":
      return clampWindowSize({ w: size.w + d, h: size.h });
    case "ArrowLeft":
      return clampWindowSize({ w: size.w - d, h: size.h });
    case "ArrowDown":
      return clampWindowSize({ w: size.w, h: size.h + d });
    case "ArrowUp":
      return clampWindowSize({ w: size.w, h: size.h - d });
    default:
      return null;
  }
}

/** Piksel fisik (dari event resize Tauri) ke piksel logis; faktor tak sah dianggap 1. */
export function physicalToLogical(size: { width: number; height: number }, scaleFactor: number): WindowSize {
  const f = Number.isFinite(scaleFactor) && scaleFactor > 0 ? scaleFactor : 1;
  return { w: Math.round(size.width / f), h: Math.round(size.height / f) };
}

export interface Debounced<A extends unknown[]> {
  (...args: A): void;
  cancel(): void;
}

/** Panggil `fn` sekali dengan argumen terakhir setelah `ms` tanpa panggilan baru. */
export function debounce<A extends unknown[]>(fn: (...args: A) => void, ms: number): Debounced<A> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const wrapped = ((...args: A) => {
    clearTimeout(timer);
    timer = setTimeout(() => fn(...args), ms);
  }) as Debounced<A>;
  wrapped.cancel = () => clearTimeout(timer);
  return wrapped;
}

/** Ruang yang disisakan dari monitor untuk panel/dok (px logis): lebar, tinggi. */
export const MONITOR_MARGIN: WindowSize = { w: 80, h: 120 };

/** Ukuran monitor dalam px logis dari `Monitor` Tauri (fisik / skala); `null` bila tak sah. */
export function monitorLogicalSize(
  monitor: { size: { width: number; height: number }; scaleFactor: number } | null,
): WindowSize | null {
  if (!monitor) return null;
  const f = Number.isFinite(monitor.scaleFactor) && monitor.scaleFactor > 0 ? monitor.scaleFactor : 1;
  const w = monitor.size.width / f;
  const h = monitor.size.height / f;
  if (!(w > 0) || !(h > 0) || !Number.isFinite(w) || !Number.isFinite(h)) return null;
  return { w: Math.round(w), h: Math.round(h) };
}

/**
 * Batasi `size` ke monitor dikurangi `margin`, tetapi tidak pernah di bawah `min`. Monitor `null`
 * (tidak diketahui) meloloskan ukuran apa adanya.
 */
export function fitToMonitor(
  size: WindowSize,
  monitorLogical: WindowSize | null,
  min: WindowSize,
  margin: WindowSize,
): WindowSize {
  if (!monitorLogical || !(monitorLogical.w > 0) || !(monitorLogical.h > 0)) return size;
  if (!Number.isFinite(monitorLogical.w) || !Number.isFinite(monitorLogical.h)) return size;
  return {
    w: Math.max(min.w, Math.min(size.w, monitorLogical.w - margin.w)),
    h: Math.max(min.h, Math.min(size.h, monitorLogical.h - margin.h)),
  };
}
