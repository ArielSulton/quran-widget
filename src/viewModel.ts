export type Part = { kind: "intro" } | { kind: "ayah"; ayah: number };

export type WidgetState =
  | { kind: "waiting" }
  | { kind: "showing"; surah: number; part: Part; playing: boolean };

export type QuranData = Record<number, { name: string; ayat: { ar: string; id: string }[] }>;

export const BASMALAH: { ar: string; id: string } = {
  ar: "بِسْمِ ٱللَّهِ ٱلرَّحْمَٰنِ ٱلرَّحِيمِ",
  id: "Dengan nama Allah Yang Maha Pengasih, Maha Penyayang.",
};

const ARABIC_INDIC_DIGITS = "٠١٢٣٤٥٦٧٨٩";

/**
 * Penanda akhir ayat: U+06DD (tanda akhir ayat) langsung disusul nomor ayat dalam angka
 * Arab-Hindi. Ini hanya tampilan; teks Tanzil di data tetap persis seperti aslinya.
 */
export function endOfAyah(n: number): string {
  const digits = String(n).replace(/\d/g, (d) => ARABIC_INDIC_DIGITS[Number(d)]);
  return `\u06DD${digits}`;
}

export type ViewModel = { arabic: string; translation: string; footer: string };

export function viewModel(s: WidgetState, q: QuranData): ViewModel {
  if (s.kind === "waiting") {
    return { arabic: "", translation: "Menunggu bacaan Al-Qur'an", footer: "" };
  }

  const surah = q[s.surah];
  const name = surah?.name ?? `Surah ${s.surah}`;

  if (s.part.kind === "intro") {
    return {
      arabic: BASMALAH.ar,
      translation: BASMALAH.id,
      footer: `${name} · mukadimah`,
    };
  }

  const ayah = surah?.ayat[s.part.ayah - 1];
  return {
    // NBSP menjaga penanda tetap menempel pada kata terakhir saat baris dipatahkan.
    arabic: ayah ? `${ayah.ar}\u00A0${endOfAyah(s.part.ayah)}` : "",
    translation: ayah?.id ?? "",
    footer: `${name} · ${s.surah}:${s.part.ayah}`,
  };
}
