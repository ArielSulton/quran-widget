import { describe, expect, it } from "vitest";
import { BASMALAH, viewModel, type QuranData, type WidgetState } from "./viewModel";

const q: QuranData = {
  112: {
    name: "Al-Ikhlas",
    ayat: [
      { ar: "قُلْ هُوَ ٱللَّهُ أَحَدٌ", id: "Katakanlah (Muhammad), Dialah Allah, Yang Maha Esa." },
      { ar: "ٱللَّهُ ٱلصَّمَدُ", id: "Allah tempat meminta segala sesuatu." },
    ],
  },
};

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
      arabic: q[112].ayat[0].ar,
      translation: q[112].ayat[0].id,
      footer: "Al-Ikhlas · 112:1",
    });
  });
});
