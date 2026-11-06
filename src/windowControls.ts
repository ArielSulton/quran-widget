import type { KeyValueStore } from "./theme";
import {
  MIN_WINDOW_SIZE,
  MONITOR_MARGIN,
  debounce,
  fitToMonitor,
  monitorLogicalSize,
  loadWindowSize,
  physicalToLogical,
  saveWindowSize,
  stepWindowSize,
  DEFAULT_WINDOW_SIZE,
  type WindowSize,
} from "./windowSize";

/** Modul jendela Tauri (dimuat dinamis; `null` di luar Tauri). */
export type WindowApi = typeof import("@tauri-apps/api/window");

/** Jeda tulis ukuran setelah event resize terakhir (ms). */
const SAVE_DEBOUNCE_MS = 400;
/** Tunggu setelah `setSize` pemulihan sebelum kartu ditampilkan dan penyimpanan diaktifkan (ms). */
const RESTORE_SETTLE_MS = 150;
const ARM_DELAY_MS = 500;
/** Pengaman: kartu tidak pernah tersembunyi lebih lama dari ini bila pemulihan macet (ms). */
const RESTORE_FAILSAFE_MS = 1500;

/** Batasi ke monitor jendela saat ini; bila pembacaan monitor gagal atau null, ukuran apa adanya. */
async function fitToCurrentMonitor(api: WindowApi, size: WindowSize): Promise<WindowSize> {
  try {
    const monitor = await api.currentMonitor();
    return fitToMonitor(size, monitorLogicalSize(monitor), MIN_WINDOW_SIZE, MONITOR_MARGIN);
  } catch (e) {
    console.error("gagal membaca monitor:", e);
    return size;
  }
}

const fail = (what: string) => (e: unknown) => console.error(`gagal ${what}:`, e);

/**
 * Pegangan ubah ukuran di pojok kanan bawah. Mouse/sentuh: seret memulai ubah ukuran native dari
 * sudut tenggara. Papan ketik: panah mengubah ukuran 10 px (Shift 50 px); fokus saja tidak memulai
 * apa pun. Di luar Tauri (`api` null) pegangan hanya tampil dan tidak bereaksi.
 */
export function initResizeGrip(api: Promise<WindowApi> | null): void {
  const grip = document.getElementById("resize-grip");
  if (!grip) throw new Error("Elemen #resize-grip tidak ditemukan");
  if (!api) return;

  // Ukuran yang diminta terakhir lewat papan ketik; dipakai agar tombol yang ditahan menumpuk
  // dengan benar tanpa menunggu jendela selesai berubah. Dibuang saat fokus/seret berganti.
  let target: WindowSize | null = null;
  let chain: Promise<void> = Promise.resolve();

  grip.addEventListener("pointerdown", (e) => {
    if (e.button !== 0) return;
    e.preventDefault(); // tanpa seleksi teks atau seret kartu
    target = null;
    api
      .then((w) => w.getCurrentWindow().startResizeDragging("SouthEast"))
      .catch(fail("memulai ubah ukuran"));
  });

  grip.addEventListener("blur", () => {
    target = null;
  });

  grip.addEventListener("keydown", (e) => {
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    if (stepWindowSize(DEFAULT_WINDOW_SIZE, e.key, e.shiftKey) === null) return;
    e.preventDefault();
    const { key, shiftKey } = e;
    chain = chain
      .then(async () => {
        const { getCurrentWindow, LogicalSize } = await api;
        const win = getCurrentWindow();
        const current =
          target ?? physicalToLogical(await win.innerSize(), await win.scaleFactor());
        const stepped = stepWindowSize(current, key, shiftKey);
        if (!stepped) return;
        const next = await fitToCurrentMonitor(await api, stepped);
        target = next;
        await win.setSize(new LogicalSize(next.w, next.h));
      })
      .catch(fail("mengubah ukuran dengan papan ketik"));
  });
}

/**
 * Pulihkan ukuran jendela tersimpan sekali saat mulai, lalu simpan setiap perubahan ukuran
 * (debounce 400 ms). Penyimpanan baru aktif setelah pemulihan selesai, supaya event resize awal
 * tidak menimpa nilai tersimpan. Tidak melakukan apa pun di luar Tauri.
 */
export function initWindowSizePersistence(api: Promise<WindowApi> | null, store: KeyValueStore): void {
  if (!api) return;
  const root = document.documentElement;
  const saved = loadWindowSize(store);
  const needsRestore = saved.w !== DEFAULT_WINDOW_SIZE.w || saved.h !== DEFAULT_WINDOW_SIZE.h;

  // Sembunyikan kartu selama pemulihan agar lompatan ukuran tidak terlihat.
  let failsafe = 0;
  const reveal = (): void => {
    window.clearTimeout(failsafe);
    root.removeAttribute("data-restoring");
  };
  if (needsRestore) {
    root.setAttribute("data-restoring", "");
    failsafe = window.setTimeout(reveal, RESTORE_FAILSAFE_MS);
  }

  const restored = api.then(async (windowApi) => {
    if (!needsRestore) return;
    const { getCurrentWindow, LogicalSize } = windowApi;
    try {
      // Ukuran tersimpan dari monitor besar tidak boleh melebihi monitor ini (posisi tidak disimpan).
      const target = await fitToCurrentMonitor(windowApi, saved);
      await getCurrentWindow().setSize(new LogicalSize(target.w, target.h));
      await new Promise((r) => window.setTimeout(r, RESTORE_SETTLE_MS));
    } finally {
      reveal();
    }
  });

  // Pemantauan tetap dipasang walau pemulihan gagal (kegagalan hanya dicatat).
  restored
    .catch((e) => {
      reveal();
      fail("memulihkan ukuran jendela")(e);
    })
    .then(() => new Promise((r) => window.setTimeout(r, ARM_DELAY_MS)))
    .then(async () => {
      const win = (await api).getCurrentWindow();
      let last: { width: number; height: number } | null = null;
      const persist = debounce(() => {
        const physical = last;
        if (!physical) return;
        win
          .scaleFactor()
          .then((scale) => saveWindowSize(store, physicalToLogical(physical, scale)))
          .catch(fail("menyimpan ukuran jendela"));
      }, SAVE_DEBOUNCE_MS);
      await win.onResized(({ payload }) => {
        // Diminimalkan/disembunyikan dapat melaporkan 0x0: jangan simpan.
        if (payload.width <= 0 || payload.height <= 0) return;
        last = { width: payload.width, height: payload.height };
        persist();
      });
    })
    .catch(fail("memantau ukuran jendela"));
}
