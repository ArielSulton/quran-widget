export type Part = { kind: "intro" } | { kind: "ayah"; ayah: number };

export type WidgetState =
  | { kind: "waiting" }
  | { kind: "showing"; surah: number; part: Part; playing: boolean };

export type QuranData = Record<number, { name: string; ayat: { ar: string; id: string }[] }>;

export const BASMALAH: { ar: string; id: string } = {
  ar: "بِسْمِ ٱللَّهِ ٱلرَّحْمَٰنِ ٱلرَّحِيمِ",
  id: "Dengan nama Allah Yang Maha Pengasih, Maha Penyayang.",
};

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
    arabic: ayah?.ar ?? "",
    translation: ayah?.id ?? "",
    footer: `${name} · ${s.surah}:${s.part.ayah}`,
  };
}
