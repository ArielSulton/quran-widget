import { describe, expect, it } from "vitest";
import { FONT_SCALE_STEPS, layoutCandidates, pickLayout, type Layout } from "./density";

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
