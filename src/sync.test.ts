import { describe, expect, it } from "vitest";
import {
  clampSync,
  formatSync,
  loadSync,
  saveSync,
  stepSync,
  SYNC_KEY,
  SYNC_MAX_MS,
  SYNC_MIN_MS,
  SYNC_COARSE_STEP_MS,
  SYNC_STEP_MS,
} from "./sync";
import type { KeyValueStore } from "./theme";

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

const MINUS = "−";

describe("konstanta", () => {
  it("langkah 0,1 dtk, rentang ±2,0 dtk, kunci penyimpanan", () => {
    expect(SYNC_STEP_MS).toBe(100);
    expect(SYNC_COARSE_STEP_MS).toBe(1000);
    expect(SYNC_MIN_MS).toBe(-2000);
    expect(SYNC_MAX_MS).toBe(2000);
    expect(SYNC_KEY).toBe("simak.syncMs");
  });
});

describe("clampSync", () => {
  it("membulatkan ke langkah 100 ms dan menjepit ke rentang", () => {
    expect(clampSync(0)).toBe(0);
    expect(clampSync(300)).toBe(300);
    expect(clampSync(349)).toBe(300);
    expect(clampSync(350)).toBe(400);
    expect(clampSync(-349)).toBe(-300);
    expect(clampSync(-350)).toBe(-400); // simetris terhadap nol
    expect(clampSync(5000)).toBe(2000);
    expect(clampSync(-5000)).toBe(-2000);
  });

  it("nilai tak terhingga atau NaN menjadi 0", () => {
    expect(clampSync(Number.NaN)).toBe(0);
    expect(clampSync(Number.POSITIVE_INFINITY)).toBe(0);
    expect(clampSync(Number.NEGATIVE_INFINITY)).toBe(0);
  });

  it("tidak pernah menghasilkan -0", () => {
    expect(Object.is(clampSync(-10), 0)).toBe(true);
  });
});

describe("formatSync", () => {
  it("koma desimal, tanda eksplisit, satu desimal", () => {
    expect(formatSync(0)).toBe("0,0 dtk");
    expect(formatSync(300)).toBe("+0,3 dtk");
    expect(formatSync(100)).toBe("+0,1 dtk");
    expect(formatSync(2000)).toBe("+2,0 dtk");
    expect(formatSync(1230)).toBe("+1,2 dtk");
  });

  it(`memakai tanda minus asli U+2212 untuk nilai negatif`, () => {
    expect(formatSync(-1500)).toBe(`${MINUS}1,5 dtk`);
    expect(formatSync(-100)).toBe(`${MINUS}0,1 dtk`);
    expect(formatSync(-2000)).toBe(`${MINUS}2,0 dtk`);
    expect(formatSync(-1500)).not.toContain("-");
    expect(formatSync(-1500).codePointAt(0)).toBe(0x2212);
  });

  it("nol tidak diberi tanda, termasuk -0 dan nilai yang membulat ke nol", () => {
    expect(formatSync(-0)).toBe("0,0 dtk");
    expect(formatSync(-20)).toBe("0,0 dtk");
    expect(formatSync(20)).toBe("0,0 dtk");
  });

  it("menjepit nilai di luar rentang", () => {
    expect(formatSync(9999)).toBe("+2,0 dtk");
    expect(formatSync(-9999)).toBe(`${MINUS}2,0 dtk`);
  });
});

describe("stepSync", () => {
  it("naik dan turun 100 ms", () => {
    expect(stepSync(0, 1)).toBe(100);
    expect(stepSync(0, -1)).toBe(-100);
    expect(stepSync(300, -1)).toBe(200);
    expect(stepSync(-100, 1)).toBe(0);
  });

  it("berhenti di batas rentang", () => {
    expect(stepSync(2000, 1)).toBe(2000);
    expect(stepSync(-2000, -1)).toBe(-2000);
    expect(stepSync(1900, 1)).toBe(2000);
  });

  it("nilai yang tidak sejajar langkah dirapikan dulu", () => {
    expect(stepSync(340, 1)).toBe(400);
  });

  it("langkah kasar (Shift) = 1000 ms", () => {
    expect(stepSync(0, 1, true)).toBe(1000);
    expect(stepSync(0, -1, true)).toBe(-1000);
    expect(stepSync(300, 1, true)).toBe(1300);
    expect(stepSync(-300, -1, true)).toBe(-1300);
  });

  it("langkah kasar tetap dijepit ke ±2000", () => {
    expect(stepSync(1500, 1, true)).toBe(2000);
    expect(stepSync(-1500, -1, true)).toBe(-2000);
    expect(stepSync(2000, 1, true)).toBe(2000);
  });

  it("langkah halus eksplisit sama dengan bawaan", () => {
    expect(stepSync(0, -1, false)).toBe(-100);
    expect(stepSync(0, -1)).toBe(-100);
  });
});

describe("loadSync / saveSync", () => {
  it("bawaan 0 bila kosong", () => {
    expect(loadSync(memoryStore())).toBe(0);
  });

  it("membaca bilangan bulat valid", () => {
    expect(loadSync(memoryStore({ [SYNC_KEY]: "300" }))).toBe(300);
    expect(loadSync(memoryStore({ [SYNC_KEY]: "-1500" }))).toBe(-1500);
    expect(loadSync(memoryStore({ [SYNC_KEY]: "0" }))).toBe(0);
  });

  it("menjepit dan membulatkan ke langkah", () => {
    expect(loadSync(memoryStore({ [SYNC_KEY]: "99999" }))).toBe(2000);
    expect(loadSync(memoryStore({ [SYNC_KEY]: "-99999" }))).toBe(-2000);
    expect(loadSync(memoryStore({ [SYNC_KEY]: "1234" }))).toBe(1200);
  });

  it("rusak atau bukan bilangan bulat jatuh ke 0", () => {
    for (const v of ["", "abc", "NaN", "Infinity", "1e2", "0.3", "300ms", " 300", "--5", "{}"]) {
      expect(loadSync(memoryStore({ [SYNC_KEY]: v }))).toBe(0);
    }
  });

  it("penyimpanan yang melempar galat jatuh ke 0 dan saveSync tidak melempar", () => {
    const broken: KeyValueStore = {
      getItem: () => {
        throw new Error("akses ditolak");
      },
      setItem: () => {
        throw new Error("penuh");
      },
    };
    expect(loadSync(broken)).toBe(0);
    expect(() => saveSync(broken, 300)).not.toThrow();
  });

  it("saveSync menyimpan bilangan bulat ms yang sudah dijepit", () => {
    const store = memoryStore();
    saveSync(store, 340);
    expect(store.data[SYNC_KEY]).toBe("300");
    saveSync(store, -9999);
    expect(store.data[SYNC_KEY]).toBe("-2000");
    expect(loadSync(store)).toBe(-2000);
  });
});
