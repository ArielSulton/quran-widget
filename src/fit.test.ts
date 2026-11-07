import { describe, expect, it } from "vitest";
import juz30 from "../data/quran/juz30.json";
import { arabicFontPx, FONT_MAX_PX, FONT_MIN_PX } from "./fit";
import type { QuranData } from "./viewModel";

const quran = juz30 as unknown as QuranData;
const allAyat = Object.values(quran).flatMap((s) => s.ayat.map((a) => a.ar));
const longest = allAyat.reduce((a, b) => (b.length > a.length ? b : a));

describe("arabicFontPx", () => {
  it("memakai ukuran penuh untuk ayat pendek (<= 60 karakter)", () => {
    expect(arabicFontPx("")).toBe(FONT_MAX_PX);
    expect(arabicFontPx("قُلْ هُوَ ٱللَّهُ أَحَدٌ")).toBe(34);
    expect(arabicFontPx("a".repeat(60))).toBe(34);
  });

  it("memberi 20 px (lantai) untuk ayat terpanjang Juz 30 (Al-Bayyinah 98:8)", () => {
    expect(longest.length).toBe(183);
    expect(arabicFontPx(longest)).toBe(FONT_MIN_PX);
    expect(FONT_MIN_PX).toBe(20);
  });

  it("tidak pernah di bawah lantai maupun di atas ukuran penuh untuk seluruh ayat Juz 30", () => {
    expect(allAyat.length).toBeGreaterThan(500);
    for (const ar of allAyat) {
      const px = arabicFontPx(ar);
      expect(px).toBeGreaterThanOrEqual(FONT_MIN_PX);
      expect(px).toBeLessThanOrEqual(FONT_MAX_PX);
    }
  });

  it("tidak pernah di bawah lantai untuk teks jauh lebih panjang dari ayat mana pun", () => {
    expect(arabicFontPx("a".repeat(5000))).toBe(FONT_MIN_PX);
  });

  it("tidak naik ketika teks bertambah panjang (monoton non-increasing)", () => {
    let prev = Infinity;
    for (let n = 0; n <= 400; n++) {
      const px = arabicFontPx("a".repeat(n));
      expect(px).toBeLessThanOrEqual(prev);
      prev = px;
    }
  });

  it("mengecil bertahap di antara 60 dan 180 karakter", () => {
    expect(arabicFontPx("a".repeat(90))).toBeLessThan(34);
    expect(arabicFontPx("a".repeat(90))).toBeGreaterThan(arabicFontPx("a".repeat(150)));
  });
});
