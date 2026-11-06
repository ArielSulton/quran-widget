import { describe, expect, it } from "vitest";
import { loadTheme, resolveTheme, saveTheme, THEME_KEY, type KeyValueStore } from "./theme";

function memoryStore(initial: Record<string, string> = {}): KeyValueStore & { data: Record<string, string> } {
  const data = { ...initial };
  return {
    data,
    getItem: (k) => (k in data ? data[k] : null),
    setItem: (k, v) => {
      data[k] = v;
    },
  };
}

describe("resolveTheme", () => {
  it("light dan dark mengabaikan preferensi sistem", () => {
    expect(resolveTheme("light", true)).toBe("light");
    expect(resolveTheme("light", false)).toBe("light");
    expect(resolveTheme("dark", true)).toBe("dark");
    expect(resolveTheme("dark", false)).toBe("dark");
  });

  it("auto mengikuti sistem", () => {
    expect(resolveTheme("auto", true)).toBe("dark");
    expect(resolveTheme("auto", false)).toBe("light");
  });
});

describe("loadTheme / saveTheme", () => {
  it("kunci penyimpanan", () => {
    expect(THEME_KEY).toBe("simak.theme");
  });

  it("bawaan auto bila kosong", () => {
    expect(loadTheme(memoryStore())).toBe("auto");
  });

  it("membaca nilai valid", () => {
    for (const v of ["auto", "light", "dark"] as const) {
      expect(loadTheme(memoryStore({ [THEME_KEY]: v }))).toBe(v);
    }
  });

  it("nilai rusak atau tak dikenal jatuh ke auto", () => {
    for (const v of ["", "Dark", "blue", "null", "{}", " light"]) {
      expect(loadTheme(memoryStore({ [THEME_KEY]: v }))).toBe("auto");
    }
  });

  it("penyimpanan yang melempar galat jatuh ke auto dan saveTheme tidak melempar", () => {
    const broken: KeyValueStore = {
      getItem: () => {
        throw new Error("akses ditolak");
      },
      setItem: () => {
        throw new Error("penuh");
      },
    };
    expect(loadTheme(broken)).toBe("auto");
    expect(() => saveTheme(broken, "dark")).not.toThrow();
  });

  it("saveTheme lalu loadTheme kembali sama", () => {
    const store = memoryStore();
    saveTheme(store, "dark");
    expect(store.data[THEME_KEY]).toBe("dark");
    expect(loadTheme(store)).toBe("dark");
  });
});
