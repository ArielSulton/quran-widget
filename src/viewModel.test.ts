import { describe, expect, it } from "vitest";
import { BASMALAH, endOfAyah, viewModel, type QuranData, type WidgetState } from "./viewModel";

const q: QuranData = {
  112: {
    name: "Al-Ikhlas",
    ayat: [
      { ar: "قُلْ هُوَ ٱللَّهُ أَحَدٌ", id: "Katakanlah (Muhammad), Dialah Allah, Yang Maha Esa." },
      { ar: "ٱللَّهُ ٱلصَّمَدُ", id: "Allah tempat meminta segala sesuatu." },
    ],
  },
};

describe("endOfAyah", () => {
  it("memberi U+06DD lalu nomor ayat dalam angka Arab-Hindi", () => {
    expect(endOfAyah(1)).toBe("\u06DD١");
    expect(endOfAyah(40)).toBe("\u06DD٤٠");
    expect(endOfAyah(179)).toBe("\u06DD١٧٩");
  });
});

describe("viewModel", () => {
  it("menampilkan teks menunggu saat state waiting", () => {
    const s: WidgetState = { kind: "waiting" };
    expect(viewModel(s, q)).toEqual({
      arabic: "",
      translation: "Menunggu bacaan Al-Qur'an",
      footer: "",
    });
  });

  it("menampilkan basmalah dan footer mukadimah saat intro", () => {
    const s: WidgetState = { kind: "showing", surah: 112, part: { kind: "intro" }, playing: true };
    expect(viewModel(s, q)).toEqual({
      arabic: BASMALAH.ar,
      translation: BASMALAH.id,
      footer: "Al-Ikhlas · mukadimah",
    });
  });

  it("menampilkan teks ayat dan footer surah:ayat saat ayah", () => {
    const s: WidgetState = {
      kind: "showing",
      surah: 112,
      part: { kind: "ayah", ayah: 1 },
      playing: true,
    };
    expect(viewModel(s, q)).toEqual({
      arabic: `${q[112].ayat[0].ar}\u00A0\u06DD١`,
      translation: q[112].ayat[0].id,
      footer: "Al-Ikhlas · 112:1",
    });
  });
  describe("fallback saat data surah/ayat tidak ada", () => {
    it("surah tidak ada di data: teks kosong, footer memakai 'Surah N'", () => {
      const s: WidgetState = {
        kind: "showing",
        surah: 99,
        part: { kind: "ayah", ayah: 1 },
        playing: true,
      };
      expect(viewModel(s, q)).toEqual({ arabic: "", translation: "", footer: "Surah 99 · 99:1" });
    });

    it("ayat di luar jumlah ayat surah: teks kosong, footer tetap tampil", () => {
      const s: WidgetState = {
        kind: "showing",
        surah: 112,
        part: { kind: "ayah", ayah: 9 },
        playing: true,
      };
      expect(viewModel(s, q)).toEqual({ arabic: "", translation: "", footer: "Al-Ikhlas · 112:9" });
    });

    it("intro untuk surah yang tidak ada di data tetap menampilkan basmalah dengan nama cadangan", () => {
      const s: WidgetState = { kind: "showing", surah: 99, part: { kind: "intro" }, playing: false };
      expect(viewModel(s, q)).toEqual({
        arabic: BASMALAH.ar,
        translation: BASMALAH.id,
        footer: "Surah 99 · mukadimah",
      });
    });
  });
});
