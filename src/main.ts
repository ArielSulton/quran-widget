import "./style.css";
import juz30 from "../data/quran/juz30.json";
import { arabicFontPx } from "./fit";
import { viewModel, type QuranData, type WidgetState } from "./viewModel";

const quran = juz30 as unknown as QuranData;

function el(id: string): HTMLElement {
  const node = document.getElementById(id);
  if (!node) throw new Error(`Elemen #${id} tidak ditemukan`);
  return node;
}

const arabicEl = el("arabic");
const translationEl = el("translation");
const footerEl = el("footer");

export function render(state: WidgetState): void {
  const vm = viewModel(state, quran);
  arabicEl.textContent = vm.arabic;
  arabicEl.style.fontSize = `${arabicFontPx(vm.arabic)}px`;
  translationEl.textContent = vm.translation;
  footerEl.textContent = vm.footer;
}

// Keadaan awal saat aplikasi dibuka: menunggu sampai event pertama dari backend.
render({ kind: "waiting" });

/** Hubungkan ke backend Tauri; di luar Tauri (vite dev di browser / vitest) tidak melakukan apa-apa. */
async function connectBackend(): Promise<void> {
  if (!("__TAURI_INTERNALS__" in window)) return;
  const [{ listen }, { invoke }] = await Promise.all([
    import("@tauri-apps/api/event"),
    import("@tauri-apps/api/core"),
  ]);

  let receivedEvent = false;
  await listen<WidgetState>("state", (event) => {
    receivedEvent = true;
    render(event.payload);
  });

  // Event pertama bisa terbit sebelum listener terdaftar: ambil keadaan terakhir sekali.
  const latest = await invoke<WidgetState>("current_state");
  if (!receivedEvent) render(latest);
}

connectBackend().catch((e) => console.error("gagal terhubung ke backend Tauri:", e));
