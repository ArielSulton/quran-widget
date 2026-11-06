/**
 * Bahan permukaan kartu:
 *  - native: macOS di dalam Tauri; blur ada di tingkat OS (NSVisualEffectView lewat `windowEffects`),
 *            jadi CSS cukup memberi lapisan tint tipis tanpa bayangan sendiri.
 *  - css:    browser biasa (dev/preview) yang mendukung `backdrop-filter`; blur diproses di halaman.
 *  - solid:  selain itu (Windows/Linux di dalam Tauri atau tanpa `backdrop-filter`). Di sana tidak ada
 *            apa pun yang dapat di-blur di belakang jendela transparan, jadi isian dibuat lebih pekat
 *            agar teks tetap terbaca di atas wallpaper apa pun.
 */
export type Material = "native" | "css" | "solid";

export interface MaterialEnv {
  tauri: boolean;
  mac: boolean;
  backdropFilter: boolean;
}

/** Sederhana dan terdokumentasi: `navigator.platform` (atau userAgent bila kosong) berawalan/berisi Mac. */
export function isMacPlatform(platform: string, userAgent: string): boolean {
  return /^Mac/i.test(platform) || /Macintosh|Mac OS X/i.test(userAgent);
}

export function detectMaterial(env: MaterialEnv): Material {
  if (env.tauri && env.mac) return "native";
  if (!env.tauri && env.backdropFilter) return "css";
  return "solid";
}
