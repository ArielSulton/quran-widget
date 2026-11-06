import "./style.css";
import juz30 from "../data/quran/juz30.json";
import { initControls } from "./controls";
import { layoutCandidates, pickLayout, type Layout } from "./density";
import { arabicFontPx } from "./fit";
import { detectMaterial, isMacPlatform } from "./material";
import { parsePreview } from "./preview";
import { loadSync, saveSync } from "./sync";
import { loadTheme, resolveTheme, saveTheme, type KeyValueStore, type ThemePref } from "./theme";
import { viewModel, type QuranData, type WidgetState } from "./viewModel";

const quran = juz30 as unknown as QuranData;

const root = document.documentElement;
const isTauri = "__TAURI_INTERNALS__" in window;
/** Pratinjau desain hanya di browser biasa; di dalam Tauri selalu null (inert). */
const preview = isTauri ? null : parsePreview(window.location.search);

function el(id: string): HTMLElement {
  const node = document.getElementById(id);
  if (!node) throw new Error(`Elemen #${id} tidak ditemukan`);
  return node;
}

const widgetEl = el("widget");
const contentEl = el("content");
const arabicEl = el("arabic");
const translationEl = el("translation");
const footerEl = el("footer");

// ---- Render ayat (dengan cross-fade singkat saat teks berganti) ----

const FADE_OUT_MS = 90;
const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
let shownKey: string | null = null;
let fadeTimer = 0;

let shownVm: ReturnType<typeof viewModel> | null = null;

function setLayout(layout: Layout): void {
  // Tulis hanya bila berubah (idempoten; tidak memicu kerja yang tak perlu).
  if (contentEl.dataset.density !== layout.density) contentEl.dataset.density = layout.density;
  const scale = String(layout.scale);
  if (contentEl.style.getPropertyValue("--arabic-scale") !== scale) {
    contentEl.style.setProperty("--arabic-scale", scale);
  }
}

/**
 * Pilih tata letak yang muat di kartu: biasa -> rapat -> font Arab mengecil bertahap (lantai 0,7).
 * Dipanggil saat teks berganti, saat font selesai dimuat, dan saat ukuran jendela berubah.
 */
function applyLayout(vm: ReturnType<typeof viewModel>): void {
  const preferCompact = vm.arabic.length > 100 || vm.translation.length > 150;
  const chosen = pickLayout(layoutCandidates(preferCompact), (candidate) => {
    setLayout(candidate);
    return contentEl.getBoundingClientRect().height > widgetEl.clientHeight;
  });
  setLayout(chosen);
  // Bila bahkan lantai masih melebihi kartu, sembunyikan footer agar tidak menimpa teks.
  const tight = contentEl.getBoundingClientRect().height > widgetEl.clientHeight;
  if (widgetEl.hasAttribute("data-tight") !== tight) widgetEl.toggleAttribute("data-tight", tight);
}

function paintText(vm: ReturnType<typeof viewModel>): void {
  shownVm = vm;
  arabicEl.textContent = vm.arabic;
  arabicEl.style.setProperty("--arabic-px", `${arabicFontPx(vm.arabic)}px`);
  translationEl.textContent = vm.translation;
  applyLayout(vm);
  footerEl.textContent = vm.footer;
}

// Font Arab yang baru selesai dimuat dan perubahan ukuran jendela mengubah muat-tidaknya teks.
// Pengamat hanya melihat kotak kartu (ditentukan jendela), bukan isi, jadi tidak ada umpan balik.
document.fonts?.addEventListener("loadingdone", () => {
  if (shownVm) applyLayout(shownVm);
});
new ResizeObserver(() => {
  if (shownVm) applyLayout(shownVm);
}).observe(widgetEl);

export function render(state: WidgetState): void {
  const vm = viewModel(state, quran);
  const key = JSON.stringify(vm);
  if (key === shownKey) return;
  const first = shownKey === null;
  shownKey = key;

  window.clearTimeout(fadeTimer);
  if (first || reducedMotion.matches) {
    paintText(vm);
    contentEl.classList.remove("changing");
    return;
  }
  contentEl.classList.add("changing");
  fadeTimer = window.setTimeout(() => {
    paintText(vm);
    contentEl.classList.remove("changing");
  }, FADE_OUT_MS);
}

// ---- Penyimpanan, tema, bahan permukaan ----

/** localStorage bisa melempar (mis. diblokir); jatuh ke penyimpanan memori. */
function safeStorage(): KeyValueStore {
  try {
    return window.localStorage;
  } catch {
    const mem = new Map<string, string>();
    return { getItem: (k) => mem.get(k) ?? null, setItem: (k, v) => void mem.set(k, v) };
  }
}
const store = safeStorage();

const systemDark = window.matchMedia("(prefers-color-scheme: dark)");
let themePref: ThemePref = preview?.theme ?? loadTheme(store);
let syncMs = preview?.syncMs ?? loadSync(store);

/** API jendela Tauri (null di luar Tauri); dimuat sekali. */
const tauriApi = isTauri
  ? Promise.all([import("@tauri-apps/api/core"), import("@tauri-apps/api/window")])
  : null;

function applyTheme(): void {
  const resolved = resolveTheme(themePref, systemDark.matches);
  root.dataset.theme = resolved;
  root.style.colorScheme = resolved;
  // Samakan tampilan jendela native (vibrancy macOS) dengan pilihan di aplikasi; null = ikuti sistem.
  tauriApi
    ?.then(([, win]) => win.getCurrentWindow().setTheme(themePref === "auto" ? null : themePref))
    .catch((e) => console.error("gagal mengatur tema jendela:", e));
}
systemDark.addEventListener("change", () => {
  if (themePref === "auto") applyTheme();
});

const backdropFilter =
  typeof CSS !== "undefined" &&
  (CSS.supports("backdrop-filter", "blur(1px)") || CSS.supports("-webkit-backdrop-filter", "blur(1px)"));
const material =
  preview?.material ??
  detectMaterial({
    tauri: isTauri,
    mac: isMacPlatform(navigator.platform, navigator.userAgent),
    backdropFilter,
  });
root.dataset.material = material;
if (preview?.backdrop) root.dataset.backdrop = "";
if (preview?.controls) root.dataset.controls = "";

applyTheme();
if (material === "native") {
  // Bayangan jendela dari OS hanya dipakai bila kartu = seluruh jendela (macOS).
  tauriApi
    ?.then(([, win]) => win.getCurrentWindow().setShadow(true))
    .catch((e) => console.error("gagal mengaktifkan bayangan jendela:", e));
}

function sendSync(ms: number): void {
  tauriApi
    ?.then(([core]) => core.invoke("set_sync_shift_ms", { ms }))
    .catch((e) => console.error("gagal mengirim geser sinkron:", e));
}

initControls({
  theme: themePref,
  syncMs,
  onTheme(pref) {
    themePref = pref;
    if (!preview) saveTheme(store, pref);
    applyTheme();
  },
  onSync(ms) {
    syncMs = ms;
    if (!preview) saveSync(store, ms);
    sendSync(ms);
  },
});
sendSync(syncMs);

// ---- Keadaan ----

// Keadaan awal saat aplikasi dibuka: menunggu sampai event pertama dari backend.
render(preview?.state ?? { kind: "waiting" });

/** Hubungkan ke backend Tauri; di luar Tauri (vite dev di browser / vitest) tidak melakukan apa-apa. */
async function connectBackend(): Promise<void> {
  if (!isTauri) return;
  const { listen } = await import("@tauri-apps/api/event");
  const { invoke } = await import("@tauri-apps/api/core");

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
