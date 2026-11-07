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
 * Nomor ayat dalam angka Arab-Hindi untuk penanda akhir ayat. Lingkarannya (U+06DD) dan
 * penempatan angka di dalamnya digambar UI (main.ts, style.css), bukan oleh ligature font,
 * karena WebKitGTK tidak memasukkan angka ke dalam lingkaran. Teks Tanzil di data tetap utuh.
 */
export function ayahNumber(n: number): string {
  return String(n).replace(/\d/g, (d) => ARABIC_INDIC_DIGITS[Number(d)]);
}

export type ViewModel = { arabic: string; ayahNo: string; translation: string; footer: string };

export function viewModel(s: WidgetState, q: QuranData): ViewModel {
  if (s.kind === "waiting") {
    return { arabic: "", ayahNo: "", translation: "Menunggu bacaan Al-Qur'an", footer: "" };
  }

  const surah = q[s.surah];
  const name = surah?.name ?? `Surah ${s.surah}`;

  if (s.part.kind === "intro") {
    return {
      arabic: BASMALAH.ar,
      ayahNo: "",
      translation: BASMALAH.id,
      footer: `${name} · mukadimah`,
    };
  }

  const ayah = surah?.ayat[s.part.ayah - 1];
  return {
    arabic: ayah?.ar ?? "",
    ayahNo: ayah ? ayahNumber(s.part.ayah) : "",
    translation: ayah?.id ?? "",
    footer: `${name} · ${s.surah}:${s.part.ayah}`,
  };
}
