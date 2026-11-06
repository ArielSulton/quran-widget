import type { Material } from "./material";
import { clampSync } from "./sync";
import type { ThemePref } from "./theme";
import type { WidgetState } from "./viewModel";

/** Pratinjau khusus pengembangan: dipakai hanya di browser biasa (bukan Tauri). */
export interface Preview {
  state: WidgetState;
  theme: ThemePref | undefined;
  syncMs: number | undefined;
  /** Paksa pil kontrol terlihat (untuk tangkapan layar). */
  controls: boolean;
  /** Tambahkan latar warna-warni di belakang kartu agar efek kaca terlihat. */
  backdrop: boolean;
  material: Material | undefined;
}

function oneOf<T extends string>(v: string | null, allowed: readonly T[]): T | undefined {
  return allowed.find((a) => a === v);
}

function intIn(v: string | null, min: number, max: number): number | undefined {
  if (v === null || !/^\d+$/.test(v)) return undefined;
  const n = Number(v);
  return n >= min && n <= max ? n : undefined;
}

/**
 * `?preview=ayah|waiting|intro` [`&surah=98&ayah=8`] [`&theme=light|dark|auto`] [`&sync=300`]
 * [`&controls=1`] [`&backdrop=1`] [`&material=native|css|solid`]. Tanpa `preview` valid => null.
 */
export function parsePreview(search: string): Preview | null {
  const q = new URLSearchParams(search);
  const kind = oneOf(q.get("preview"), ["ayah", "waiting", "intro"] as const);
  if (!kind) return null;

  const surah = intIn(q.get("surah"), 78, 114) ?? 112;
  const ayah = intIn(q.get("ayah"), 1, 286) ?? 1;
  const state: WidgetState =
    kind === "waiting"
      ? { kind: "waiting" }
      : {
          kind: "showing",
          surah,
          part: kind === "intro" ? { kind: "intro" } : { kind: "ayah", ayah },
          playing: true,
        };

  const syncRaw = q.get("sync");
  return {
    state,
    theme: oneOf(q.get("theme"), ["auto", "light", "dark"] as const),
    syncMs: syncRaw !== null && /^-?\d+$/.test(syncRaw) ? clampSync(Number(syncRaw)) : undefined,
    controls: q.get("controls") === "1",
    backdrop: q.get("backdrop") === "1",
    material: oneOf(q.get("material"), ["native", "css", "solid"] as const),
  };
}
