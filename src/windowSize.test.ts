import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  DEFAULT_WINDOW_SIZE,
  MAX_WINDOW_SIZE,
  MIN_WINDOW_SIZE,
  WINDOW_SIZE_KEY,
  clampWindowSize,
  MONITOR_MARGIN,
  debounce,
  fitToMonitor,
  monitorLogicalSize,
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
  it("bawaan 520x320, minimum 430x270, maksimum 2000x1200", () => {
    expect(DEFAULT_WINDOW_SIZE).toEqual({ w: 520, h: 320 });
    expect(MIN_WINDOW_SIZE).toEqual({ w: 430, h: 270 });
    expect(MAX_WINDOW_SIZE).toEqual({ w: 2000, h: 1200 });
    expect(WINDOW_SIZE_KEY).toBe("simak.windowSize");
  });
});

describe("clampWindowSize", () => {
  it("menjepit ke [430,2000] x [270,1200] dan membulatkan", () => {
    expect(clampWindowSize({ w: 100, h: 100 })).toEqual({ w: 430, h: 270 });
    expect(clampWindowSize({ w: 5000, h: 5000 })).toEqual({ w: 2000, h: 1200 });
    expect(clampWindowSize({ w: 700.4, h: 400.6 })).toEqual({ w: 700, h: 401 });
    expect(clampWindowSize({ w: 430, h: 270 })).toEqual({ w: 430, h: 270 });
    expect(clampWindowSize({ w: 2000, h: 1200 })).toEqual({ w: 2000, h: 1200 });
  });
});

describe("parseWindowSize", () => {
  it("menerima JSON {w,h} yang sah", () => {
    expect(parseWindowSize('{"w":800,"h":500}')).toEqual({ w: 800, h: 500 });
  });

  it("di luar rentang tetapi masuk akal: dijepit", () => {
    expect(parseWindowSize('{"w":300,"h":100}')).toEqual({ w: 430, h: 270 });
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
    expect(store.data[WINDOW_SIZE_KEY]).toBe('{"w":430,"h":1200}');
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
    expect(stepWindowSize({ w: 435, h: 275 }, "ArrowLeft", true)).toEqual({ w: 430, h: 275 });
    expect(stepWindowSize({ w: 435, h: 275 }, "ArrowUp", true)).toEqual({ w: 435, h: 270 });
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

describe("fitToMonitor", () => {
  const min = MIN_WINDOW_SIZE;
  const margin = MONITOR_MARGIN;

  it("margin bawaan: 80 px lebar, 120 px tinggi (panel dan dok)", () => {
    expect(MONITOR_MARGIN).toEqual({ w: 80, h: 120 });
  });

  it("ukuran tersimpan lebih besar dari monitor diperkecil ke monitor dikurangi margin", () => {
    expect(fitToMonitor({ w: 2000, h: 1200 }, { w: 1366, h: 768 }, min, margin)).toEqual({ w: 1286, h: 648 });
    expect(fitToMonitor({ w: 1500, h: 400 }, { w: 1920, h: 1080 }, min, margin)).toEqual({ w: 1500, h: 400 });
    expect(fitToMonitor({ w: 1900, h: 1000 }, { w: 1920, h: 1080 }, min, margin)).toEqual({ w: 1840, h: 960 });
  });

  it("ukuran yang sudah muat tidak diubah", () => {
    expect(fitToMonitor({ w: 800, h: 500 }, { w: 1920, h: 1080 }, min, margin)).toEqual({ w: 800, h: 500 });
  });

  it("monitor sangat kecil tidak pernah menurunkan di bawah minimum", () => {
    expect(fitToMonitor({ w: 800, h: 500 }, { w: 480, h: 300 }, min, margin)).toEqual({ w: 430, h: 270 });
    expect(fitToMonitor({ w: 800, h: 500 }, { w: 100, h: 100 }, min, margin)).toEqual({ w: 430, h: 270 });
  });

  it("monitor null atau tak sah: ukuran dilewatkan apa adanya", () => {
    expect(fitToMonitor({ w: 2000, h: 1200 }, null, min, margin)).toEqual({ w: 2000, h: 1200 });
    expect(fitToMonitor({ w: 2000, h: 1200 }, { w: Number.NaN, h: 768 }, min, margin)).toEqual({ w: 2000, h: 1200 });
    expect(fitToMonitor({ w: 2000, h: 1200 }, { w: 0, h: 0 }, min, margin)).toEqual({ w: 2000, h: 1200 });
  });

  it("HiDPI: skala 2 dan 1,25 (monitor fisik dibagi skala)", () => {
    // 3840x2160 fisik, skala 2 => 1920x1080 logis => batas 1840x960
    const m2 = monitorLogicalSize({ size: { width: 3840, height: 2160 }, scaleFactor: 2 });
    expect(m2).toEqual({ w: 1920, h: 1080 });
    expect(fitToMonitor({ w: 2000, h: 1200 }, m2, min, margin)).toEqual({ w: 1840, h: 960 });
    // 1920x1200 fisik, skala 1,25 => 1536x960 logis => batas 1456x840
    const m125 = monitorLogicalSize({ size: { width: 1920, height: 1200 }, scaleFactor: 1.25 });
    expect(m125).toEqual({ w: 1536, h: 960 });
    expect(fitToMonitor({ w: 2000, h: 1200 }, m125, min, margin)).toEqual({ w: 1456, h: 840 });
  });

  it("monitorLogicalSize: skala tak sah dianggap 1, ukuran nol/NaN => null", () => {
    expect(monitorLogicalSize({ size: { width: 1920, height: 1080 }, scaleFactor: 0 })).toEqual({ w: 1920, h: 1080 });
    expect(monitorLogicalSize({ size: { width: 0, height: 1080 }, scaleFactor: 1 })).toBeNull();
    expect(monitorLogicalSize(null)).toBeNull();
  });
});
