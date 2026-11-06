import { describe, expect, it } from "vitest";
import {
  CARD_REF_HEIGHT,
  CARD_REF_WIDTH,
  FONT_SCALE_STEPS,
  SIZE_SCALE_MAX,
  footerScale,
  layoutCandidates,
  pickLayout,
  sizeScale,
  translationScale,
  type Layout,
} from "./density";

describe("layoutCandidates", () => {
  it("urutan: biasa -> rapat -> rapat dengan font Arab mengecil sampai lantai 0,7", () => {
    expect(layoutCandidates(false)).toEqual([
      { density: "regular", scale: 1 },
      { density: "compact", scale: 1 },
      { density: "compact", scale: 0.92 },
      { density: "compact", scale: 0.85 },
      { density: "compact", scale: 0.78 },
      { density: "compact", scale: 0.7 },
    ]);
  });

  it("ayat panjang langsung mulai dari rapat", () => {
    const list = layoutCandidates(true);
    expect(list[0]).toEqual({ density: "compact", scale: 1 });
    expect(list.every((l) => l.density === "compact")).toBe(true);
    expect(list).toHaveLength(FONT_SCALE_STEPS.length);
  });

  it("langkah skala menurun ketat dan berakhir di lantai 0,7", () => {
    expect(FONT_SCALE_STEPS[0]).toBe(1);
    expect(FONT_SCALE_STEPS[FONT_SCALE_STEPS.length - 1]).toBe(0.7);
    for (let i = 1; i < FONT_SCALE_STEPS.length; i++) {
      expect(FONT_SCALE_STEPS[i]).toBeLessThan(FONT_SCALE_STEPS[i - 1]);
    }
  });
});

describe("pickLayout", () => {
  const tried: Layout[] = [];
  const spy = (fitsFrom: number) => (l: Layout) => {
    tried.push(l);
    return l.density === "regular" || l.scale > fitsFrom; // true = melebihi kartu
  };

  it("memilih kandidat pertama yang tidak melebihi kartu", () => {
    tried.length = 0;
    const chosen = pickLayout(layoutCandidates(false), spy(1));
    expect(chosen).toEqual({ density: "compact", scale: 1 });
    expect(tried).toHaveLength(2); // berhenti di yang pertama muat
  });

  it("mengecilkan font bertahap bila rapat 1,0 masih melebihi", () => {
    tried.length = 0;
    expect(pickLayout(layoutCandidates(false), spy(0.85))).toEqual({ density: "compact", scale: 0.85 });
  });

  it("bila semua melebihi: lantai (rapat, 0,7), tanpa error", () => {
    expect(pickLayout(layoutCandidates(false), () => true)).toEqual({ density: "compact", scale: 0.7 });
    expect(pickLayout(layoutCandidates(true), () => true)).toEqual({ density: "compact", scale: 0.7 });
  });

  it("bila biasa sudah muat, tidak mengubah apa pun", () => {
    expect(pickLayout(layoutCandidates(false), (l) => !(l.density === "regular" && l.scale === 1))).toEqual({
      density: "regular",
      scale: 1,
    });
  });

  it("idempoten: dua kali dengan fungsi yang sama memberi hasil sama", () => {
    const overflows = (l: Layout) => l.scale > 0.85;
    expect(pickLayout(layoutCandidates(true), overflows)).toEqual(pickLayout(layoutCandidates(true), overflows));
  });
});

describe("sizeScale", () => {
  it("1,0 pada kartu bawaan (jendela 520x320 = kartu 480x280)", () => {
    expect(CARD_REF_WIDTH).toBe(480);
    expect(CARD_REF_HEIGHT).toBe(280);
    expect(sizeScale(480, 280)).toBe(1);
  });

  it("tumbuh mulus untuk jendela lebih besar, mengikuti rasio yang lebih kecil", () => {
    expect(sizeScale(540, 315)).toBe(1.13); // lebar 1,125; tinggi 1,125
    expect(sizeScale(680, 380)).toBe(1.36); // jendela 720x420: lebar 1,417; tinggi 1,357
    expect(sizeScale(920, 300)).toBe(1.07); // lebar-pendek: tinggi 1,071 yang menentukan
    expect(sizeScale(500, 520)).toBe(1.04); // tinggi-sempit: lebar 1,042 yang menentukan
  });

  it("dibatasi 1,6 di atas (jendela 960x560 dan lebih besar)", () => {
    expect(SIZE_SCALE_MAX).toBe(1.6);
    expect(sizeScale(920, 520)).toBe(1.6);
    expect(sizeScale(1960, 1160)).toBe(1.6);
  });

  it("tidak pernah di bawah 1,0 (penyusutan di bawah ukuran bawaan dilakukan pemilih bertahap)", () => {
    expect(sizeScale(380, 220)).toBe(1); // jendela minimum 420x260
    expect(sizeScale(100, 100)).toBe(1);
    expect(sizeScale(0, 0)).toBe(1);
  });

  it("nilai tak wajar tidak merusak: NaN/Infinity jatuh ke 1,0 atau batas", () => {
    expect(sizeScale(Number.NaN, 280)).toBe(1);
    expect(sizeScale(480, Number.NaN)).toBe(1);
    expect(sizeScale(Infinity, Infinity)).toBe(1.6);
  });

  it("monoton tak menurun terhadap lebar dan tinggi dan dibulatkan ke 0,01", () => {
    let prev = 0;
    for (let w = 380; w <= 1960; w += 13) {
      const s = sizeScale(w, 520);
      expect(s).toBeGreaterThanOrEqual(prev);
      expect(Math.round(s * 100) / 100).toBe(s);
      prev = s;
    }
  });
});

describe("skala teks sekunder", () => {
  it("terjemahan tumbuh 60% dari laju Arab, dibatasi 1,3", () => {
    expect(translationScale(1)).toBe(1);
    expect(translationScale(1.3)).toBe(1.18);
    expect(translationScale(1.6)).toBe(1.3);
  });

  it("footer tumbuh 30% dari laju Arab, dibatasi 1,2", () => {
    expect(footerScale(1)).toBe(1);
    expect(footerScale(1.3)).toBe(1.09);
    expect(footerScale(1.6)).toBe(1.18);
  });
});
