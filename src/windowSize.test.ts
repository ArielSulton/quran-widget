import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  DEFAULT_WINDOW_SIZE,
  MAX_WINDOW_SIZE,
  MIN_WINDOW_SIZE,
  WINDOW_SIZE_KEY,
  clampWindowSize,
  debounce,
  loadWindowSize,
  parseWindowSize,
  physicalToLogical,
  saveWindowSize,
  stepWindowSize,
} from "./windowSize";
import type { KeyValueStore } from "./theme";

function memStore(initial: Record<string, string> = {}): KeyValueStore & { data: Record<string, string> } {
  const data = { ...initial };
  return {
    data,
    getItem: (k) => (k in data ? data[k] : null),
    setItem: (k, v) => void (data[k] = v),
  };
}

describe("konstanta", () => {
  it("bawaan 520x320, minimum 420x260, maksimum 2000x1200", () => {
    expect(DEFAULT_WINDOW_SIZE).toEqual({ w: 520, h: 320 });
    expect(MIN_WINDOW_SIZE).toEqual({ w: 420, h: 260 });
    expect(MAX_WINDOW_SIZE).toEqual({ w: 2000, h: 1200 });
    expect(WINDOW_SIZE_KEY).toBe("simak.windowSize");
  });
});

describe("clampWindowSize", () => {
  it("menjepit ke [420,2000] x [260,1200] dan membulatkan", () => {
    expect(clampWindowSize({ w: 100, h: 100 })).toEqual({ w: 420, h: 260 });
    expect(clampWindowSize({ w: 5000, h: 5000 })).toEqual({ w: 2000, h: 1200 });
    expect(clampWindowSize({ w: 700.4, h: 400.6 })).toEqual({ w: 700, h: 401 });
    expect(clampWindowSize({ w: 420, h: 260 })).toEqual({ w: 420, h: 260 });
    expect(clampWindowSize({ w: 2000, h: 1200 })).toEqual({ w: 2000, h: 1200 });
  });
});

describe("parseWindowSize", () => {
  it("menerima JSON {w,h} yang sah", () => {
    expect(parseWindowSize('{"w":800,"h":500}')).toEqual({ w: 800, h: 500 });
  });

  it("di luar rentang tetapi masuk akal: dijepit", () => {
    expect(parseWindowSize('{"w":300,"h":100}')).toEqual({ w: 420, h: 260 });
    expect(parseWindowSize('{"w":3000,"h":2000}')).toEqual({ w: 2000, h: 1200 });
  });

  it("rusak, kosong, bertipe salah atau tak masuk akal: ukuran bawaan", () => {
    const bad = [
      null,
      "",
      "not json",
      "{",
      "null",
      "[]",
      "42",
      '"x"',
      "{}",
      '{"w":800}',
      '{"h":500}',
      '{"w":"800","h":"500"}',
      '{"w":null,"h":500}',
      '{"w":-5,"h":500}',
      '{"w":0,"h":0}',
      '{"w":800,"h":1e9}',
      '{"w":1e400,"h":500}',
    ];
    for (const raw of bad) expect(parseWindowSize(raw)).toEqual(DEFAULT_WINDOW_SIZE);
  });
});

describe("loadWindowSize / saveWindowSize", () => {
  it("hilang => bawaan", () => {
    expect(loadWindowSize(memStore())).toEqual({ w: 520, h: 320 });
  });

  it("bulat-balik, ditulis sebagai JSON {w,h}", () => {
    const store = memStore();
    saveWindowSize(store, { w: 801, h: 499 });
    expect(store.data[WINDOW_SIZE_KEY]).toBe('{"w":801,"h":499}');
    expect(loadWindowSize(store)).toEqual({ w: 801, h: 499 });
  });

  it("menjepit saat menyimpan", () => {
    const store = memStore();
    saveWindowSize(store, { w: 10, h: 99999 });
    expect(store.data[WINDOW_SIZE_KEY]).toBe('{"w":420,"h":1200}');
  });

  it("penyimpanan yang melempar tidak merusak (baca => bawaan, tulis diabaikan)", () => {
    const throwing: KeyValueStore = {
      getItem() {
        throw new Error("diblokir");
      },
      setItem() {
        throw new Error("kuota");
      },
    };
    expect(loadWindowSize(throwing)).toEqual(DEFAULT_WINDOW_SIZE);
    expect(() => saveWindowSize(throwing, { w: 800, h: 500 })).not.toThrow();
  });
});

describe("stepWindowSize", () => {
  const start = { w: 600, h: 400 };

  it("panah: 10 px; Shift: 50 px", () => {
    expect(stepWindowSize(start, "ArrowRight", false)).toEqual({ w: 610, h: 400 });
    expect(stepWindowSize(start, "ArrowLeft", false)).toEqual({ w: 590, h: 400 });
    expect(stepWindowSize(start, "ArrowDown", false)).toEqual({ w: 600, h: 410 });
    expect(stepWindowSize(start, "ArrowUp", false)).toEqual({ w: 600, h: 390 });
    expect(stepWindowSize(start, "ArrowRight", true)).toEqual({ w: 650, h: 400 });
    expect(stepWindowSize(start, "ArrowUp", true)).toEqual({ w: 600, h: 350 });
  });

  it("dijepit ke minimum dan maksimum", () => {
    expect(stepWindowSize({ w: 425, h: 265 }, "ArrowLeft", true)).toEqual({ w: 420, h: 265 });
    expect(stepWindowSize({ w: 425, h: 265 }, "ArrowUp", true)).toEqual({ w: 425, h: 260 });
    expect(stepWindowSize({ w: 1990, h: 1195 }, "ArrowRight", true)).toEqual({ w: 2000, h: 1195 });
    expect(stepWindowSize({ w: 1990, h: 1195 }, "ArrowDown", true)).toEqual({ w: 1990, h: 1200 });
  });

  it("tombol lain: null (tidak ada perubahan)", () => {
    expect(stepWindowSize(start, "Enter", false)).toBeNull();
    expect(stepWindowSize(start, "a", true)).toBeNull();
  });
});

describe("physicalToLogical", () => {
  it("membagi dengan faktor skala dan membulatkan", () => {
    expect(physicalToLogical({ width: 1040, height: 640 }, 2)).toEqual({ w: 520, h: 320 });
    expect(physicalToLogical({ width: 780, height: 480 }, 1.5)).toEqual({ w: 520, h: 320 });
    expect(physicalToLogical({ width: 521, height: 321 }, 1)).toEqual({ w: 521, h: 321 });
  });

  it("faktor tak sah dianggap 1", () => {
    expect(physicalToLogical({ width: 500, height: 300 }, 0)).toEqual({ w: 500, h: 300 });
    expect(physicalToLogical({ width: 500, height: 300 }, Number.NaN)).toEqual({ w: 500, h: 300 });
  });
});

describe("debounce", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("hanya memanggil sekali dengan argumen terakhir setelah jeda", () => {
    const calls: number[] = [];
    const d = debounce((n: number) => calls.push(n), 400);
    d(1);
    vi.advanceTimersByTime(300);
    d(2);
    vi.advanceTimersByTime(300);
    d(3);
    expect(calls).toEqual([]);
    vi.advanceTimersByTime(399);
    expect(calls).toEqual([]);
    vi.advanceTimersByTime(1);
    expect(calls).toEqual([3]);
  });

  it("cancel membatalkan panggilan yang tertunda", () => {
    const calls: number[] = [];
    const d = debounce((n: number) => calls.push(n), 400);
    d(1);
    d.cancel();
    vi.advanceTimersByTime(1000);
    expect(calls).toEqual([]);
  });
});
