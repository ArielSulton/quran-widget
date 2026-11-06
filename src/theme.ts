/** Pilihan tema pengguna dan tema yang benar-benar dipakai. */
export type ThemePref = "auto" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";

export const THEME_KEY = "simak.theme";

/** Bagian minimal dari `Storage` yang dipakai (mudah diganti di tes). */
export interface KeyValueStore {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

/** auto mengikuti sistem; light/dark memaksa. */
export function resolveTheme(pref: ThemePref, systemDark: boolean): ResolvedTheme {
  if (pref === "auto") return systemDark ? "dark" : "light";
  return pref;
}

function isThemePref(v: unknown): v is ThemePref {
  return v === "auto" || v === "light" || v === "dark";
}

/** Baca pilihan tema; hilang, rusak, atau penyimpanan tak dapat diakses => "auto". */
export function loadTheme(store: KeyValueStore): ThemePref {
  try {
    const v = store.getItem(THEME_KEY);
    return isThemePref(v) ? v : "auto";
  } catch {
    return "auto";
  }
}

/** Simpan pilihan tema; kegagalan penyimpanan (mode privat, kuota) diabaikan. */
export function saveTheme(store: KeyValueStore, pref: ThemePref): void {
  try {
    store.setItem(THEME_KEY, pref);
  } catch {
    /* tidak kritis: pilihan hanya tidak bertahan */
  }
}
