#!/usr/bin/env node
// Uji end-to-end jalur jendela Tauri TANPA aplikasi asli: halaman dimuat di Chrome tanpa kepala
// dengan `__TAURI_INTERNALS__` palsu yang mencatat perintah IPC. Memeriksa pemulihan ukuran
// (termasuk pembatasan ke monitor), seret ubah ukuran, panah papan ketik, dan simpan ukuran.
// Bukan pengganti uji pada jendela sungguhan. Tanpa dependensi npm; Chrome dan vite dibersihkan
// seperti pada scripts/layout-sweep.mjs.
//
// Pemakaian: node scripts/e2e-window-mock.mjs [--url http://localhost:1420] [--chrome google-chrome]
// Kode keluar: 0 semua lolos, 1 ada yang gagal, 2 galat skrip.

import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { sleep, startChrome, startVite, teardown, waitFor } from "./lib/devtools.mjs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const args = new Map();
for (let i = 2; i < process.argv.length; i += 2) args.set(process.argv[i]?.replace(/^--/, ""), process.argv[i + 1]);

/** monitor: objek Monitor palsu, null, atau "throw". Ukuran fisik; jendela awal 1040x640 fisik (skala 2). */
const mockSource = (monitor) => `
(() => {
  window.__log = []; const cbs = new Map(); let n = 1; const handlers = {};
  window.__emit = (event, payload) => { for (const h of handlers[event] ?? []) cbs.get(h)?.({ event, id: 0, payload }); };
  window.__size = { width: 1040, height: 640 };
  const monitor = ${JSON.stringify(monitor)};
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: "main" }, currentWebview: { windowLabel: "main", label: "main" } },
    transformCallback(cb) { const i = n++; cbs.set(i, cb); return i; },
    unregisterCallback(i) { cbs.delete(i); },
    convertFileSrc: (x) => x,
    async invoke(cmd, args) {
      window.__log.push([cmd, JSON.parse(JSON.stringify(args ?? {}))]);
      switch (cmd) {
        case "plugin:window|inner_size": return window.__size;
        case "plugin:window|scale_factor": return 2;
        case "plugin:window|current_monitor": if (monitor === "throw") throw new Error("monitor gagal"); return monitor;
        case "plugin:window|set_size": {
          const s = JSON.parse(JSON.stringify(args.value)); const L = s.Logical ?? s.Physical; const f = s.Logical ? 2 : 1;
          window.__size = { width: L.width * f, height: L.height * f };
          setTimeout(() => window.__emit("tauri://resize", window.__size), 20); return null;
        }
        case "plugin:event|listen": (handlers[args.event] ??= []).push(args.handler); return n++;
        case "current_state": return { kind: "waiting" };
        default: return null;
      }
    },
  };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
})();`;

const monitorOf = (width, height, scaleFactor) => ({ name: "m", size: { width, height }, position: { x: 0, y: 0 }, workArea: { position: { x: 0, y: 0 }, size: { width, height } }, scaleFactor });

let failures = 0;
const check = (name, ok, detail = "") => {
  if (!ok) failures++;
  console.log(`${ok ? "OK   " : "GAGAL"} ${name}${detail ? ` (${detail})` : ""}`);
};

async function main() {
  const baseUrl = args.get("url") ?? (await startVite(root));
  const { send, evaluate: ev } = await startChrome(args.get("chrome") ?? "google-chrome");
  await send("Page.enable");
  const setSizes = (log) => log.filter(([c]) => c === "plugin:window|set_size").map(([, a]) => a.value.Logical);

  async function load(monitor, seed) {
    await send("Page.navigate", { url: "about:blank" });
    const ids = [];
    ids.push((await send("Page.addScriptToEvaluateOnNewDocument", { source: mockSource(monitor) })).identifier);
    if (seed !== null) ids.push((await send("Page.addScriptToEvaluateOnNewDocument", { source: `localStorage.setItem("simak.windowSize", ${JSON.stringify(seed)});` })).identifier);
    else ids.push((await send("Page.addScriptToEvaluateOnNewDocument", { source: `localStorage.removeItem("simak.windowSize");` })).identifier);
    await send("Page.navigate", { url: baseUrl });
    // Tunggu sampai aplikasi memanggil `current_state` (akhir inisialisasi), lalu beri waktu pemulihan.
    await waitFor(async () => (await ev(`window.__log?.some(([c]) => c === "current_state") ?? false`)) === true, "inisialisasi halaman", 20000);
    await sleep(900);
    for (const id of ids) await send("Page.removeScriptToEvaluateOnNewDocument", { identifier: id });
  }
  const log = async () => JSON.parse(await ev("JSON.stringify(window.__log)"));
  const stored = () => ev(`localStorage.getItem("simak.windowSize")`);
  const key = (k, shift = false) => ev(`document.getElementById("resize-grip").dispatchEvent(new KeyboardEvent("keydown", { key: "${k}", shiftKey: ${shift}, bubbles: true, cancelable: true })); 1`);

  // Pemanasan: muat pertama dengan Tauri palsu memicu pra-bundel vite (dan muat ulang); abaikan hasilnya.
  await load(monitorOf(3840, 2160, 2), null);

  // 1. Pemulihan biasa pada monitor besar HiDPI (3840x2160 @2 = 1920x1080 logis).
  await load(monitorOf(3840, 2160, 2), '{"w":800,"h":500}');
  check("pemulihan 800x500 pada monitor besar", JSON.stringify(setSizes(await log())) === JSON.stringify([{ width: 800, height: 500 }]), JSON.stringify(setSizes(await log())));

  // 2. Ukuran besar tersimpan dari monitor besar, monitor sekarang 1366x768 skala 1 => 1286x648.
  await load(monitorOf(1366, 768, 1), '{"w":2000,"h":1200}');
  check("2000x1200 diperkecil ke monitor 1366x768 (1286x648)", JSON.stringify(setSizes(await log())) === JSON.stringify([{ width: 1286, height: 648 }]), JSON.stringify(setSizes(await log())));

  // 3. Skala 1,25: 1920x1200 fisik = 1536x960 logis => 1456x840.
  await load(monitorOf(1920, 1200, 1.25), '{"w":2000,"h":1200}');
  check("skala 1,25 => 1456x840", JSON.stringify(setSizes(await log())) === JSON.stringify([{ width: 1456, height: 840 }]), JSON.stringify(setSizes(await log())));

  // 4. Monitor null dan galat monitor: ukuran tersimpan (terjepit 2000x1200) tetap dipakai.
  await load(null, '{"w":2000,"h":1200}');
  check("monitor null => 2000x1200 apa adanya", JSON.stringify(setSizes(await log())) === JSON.stringify([{ width: 2000, height: 1200 }]));
  await load("throw", '{"w":2000,"h":1200}');
  check("galat currentMonitor => 2000x1200 apa adanya", JSON.stringify(setSizes(await log())) === JSON.stringify([{ width: 2000, height: 1200 }]));

  // 5. Tanpa nilai tersimpan, rusak, atau tak masuk akal: tidak ada setSize saat mulai; tidak menimpa.
  for (const seed of [null, "garbage", '{"w":99999,"h":99999}']) {
    await load(monitorOf(3840, 2160, 2), seed);
    check(`seed ${seed === null ? "kosong" : seed}: tanpa setSize saat mulai`, setSizes(await log()).length === 0);
    check(`seed ${seed === null ? "kosong" : seed}: nilai tersimpan tidak ditimpa`, (await stored()) === seed);
  }

  // 6. Seret, fokus, papan ketik, simpan, 0x0, debounce (monitor 1920x1080 skala 1 => batas 1840x960).
  await load(monitorOf(1920, 1080, 1), '{"w":800,"h":500}');
  await sleep(800); // pemantauan baru aktif sekitar 650 ms setelah pemulihan
  await ev(`document.getElementById("resize-grip").dispatchEvent(new PointerEvent("pointerdown", { button: 0, bubbles: true, cancelable: true })); 1`);
  await sleep(100);
  check("pointerdown memulai seret SouthEast", (await log()).some(([c, a]) => c === "plugin:window|start_resize_dragging" && a.value === "SouthEast"));
  const before = (await log()).length;
  await ev(`document.getElementById("resize-grip").focus(); 1`);
  check("fokus saja tidak memanggil apa pun", (await log()).length === before);
  for (const [k, s] of [["ArrowRight", false], ["ArrowRight", false], ["ArrowRight", false], ["ArrowDown", true], ["ArrowDown", true]]) await key(k, s);
  await sleep(500);
  const keys = setSizes(await log()).slice(-5);
  check("panah menumpuk dari 800x500 (810,820,830 lalu 550,600)", JSON.stringify(keys) === JSON.stringify([{ width: 810, height: 500 }, { width: 820, height: 500 }, { width: 830, height: 500 }, { width: 830, height: 550 }, { width: 830, height: 600 }]), JSON.stringify(keys));
  await sleep(900);
  check("ukuran tersimpan setelah debounce", (await stored()) === '{"w":830,"h":600}', String(await stored()));
  for (let i = 0; i < 30; i++) await key("ArrowDown", true);
  await sleep(600);
  const last = setSizes(await log()).slice(-1)[0];
  check("tinggi dibatasi monitor (960), bukan hanya 1200", last.height === 960 && last.width === 830, JSON.stringify(last));
  const s0 = await stored();
  await ev(`window.__emit("tauri://resize", { width: 0, height: 0 }); 1`);
  await sleep(700);
  check("resize 0x0 tidak disimpan", (await stored()) === s0);
  await ev(`window.__emit("tauri://resize", { width: 1000, height: 700 }); window.__emit("tauri://resize", { width: 1200, height: 800 }); window.__emit("tauri://resize", { width: 1300, height: 900 }); 1`);
  await sleep(200);
  check("debounce: belum menulis sebelum 400 ms", (await stored()) === s0);
  await sleep(500);
  check("debounce: satu tulis dengan nilai terakhir (1300x900 fisik / 2)", (await stored()) === '{"w":650,"h":450}', String(await stored()));

  console.log(failures === 0 ? "SEMUA LOLOS" : `${failures} PEMERIKSAAN GAGAL`);
  process.exitCode = failures === 0 ? 0 : 1;
}

try {
  await main();
} catch (e) {
  console.error(e);
  process.exitCode = 2;
} finally {
  await teardown();
}
