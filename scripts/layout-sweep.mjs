#!/usr/bin/env node
// Sapuan tata letak: merender SEMUA ayat Juz 30 (564) di Chrome tanpa kepala pada beberapa ukuran
// jendela dan menghitung ayat yang terpotong. Tanpa dependensi npm: Chrome dikendalikan lewat
// protokol DevTools memakai WebSocket bawaan Node 22.
//
// Pemakaian:
//   node scripts/layout-sweep.mjs [--sizes 420x260,520x320,720x420,960x560] [--assert 420x260,520x320]
//        [--url http://localhost:1420] [--shots DIR] [--shot-ayat 112:1,87:7,98:8] [--theme light|dark]
//        [--chrome google-chrome]
//
// Tanpa --url, skrip menyalakan server vite sendiri di port acak (tidak menyentuh server lain).
// Ukuran adalah ukuran JENDELA (viewport); kartu = jendela dikurangi inset bayangan (40x40).
// Keluar dengan kode 1 bila ada ayat terpotong pada ukuran di --assert (bawaan 420x260 dan 520x320).
// `--shots` menyimpan PNG ayat pilihan plus ayat terburuk pada tiap ukuran.

import { spawn } from "node:child_process";
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync, existsSync } from "node:fs";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const args = new Map();
for (let i = 2; i < process.argv.length; i++) {
  const a = process.argv[i];
  if (a.startsWith("--")) args.set(a.slice(2), process.argv[i + 1]?.startsWith("--") || i + 1 >= process.argv.length ? "1" : process.argv[++i]);
}
const parseSizes = (s) => s.split(",").map((x) => x.trim().split("x").map(Number)).map(([w, h]) => ({ w, h, key: `${w}x${h}` }));
const sizes = parseSizes(args.get("sizes") ?? "420x260,520x320,720x420,960x560");
const assertKeys = new Set(
  (args.get("assert") ?? "420x260,520x320").split(",").map((s) => s.trim()).filter((k) => k && sizes.some((z) => z.key === k)),
);
const shotsDir = args.get("shots");
const shotAyat = (args.get("shot-ayat") ?? "112:1,87:7,98:8").split(",").filter(Boolean);
const theme = args.get("theme") ?? "light";
const chromeBin = args.get("chrome") ?? "google-chrome";

const quran = JSON.parse(readFileSync(join(root, "data/quran/juz30.json"), "utf8"));
const ayat = [];
for (const [surah, s] of Object.entries(quran)) s.ayat.forEach((_, i) => ayat.push([Number(surah), i + 1]));

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const freePort = () =>
  new Promise((resolve, reject) => {
    const srv = createServer();
    srv.listen(0, "127.0.0.1", () => {
      const { port } = srv.address();
      srv.close(() => resolve(port));
    });
    srv.on("error", reject);
  });

async function waitFor(fn, what, ms = 30000) {
  const end = Date.now() + ms;
  for (;;) {
    try {
      const v = await fn();
      if (v) return v;
    } catch {
      /* ulangi */
    }
    if (Date.now() > end) throw new Error(`waktu habis menunggu ${what}`);
    await sleep(100);
  }
}

const cleanups = [];
async function cleanup() {
  for (const fn of cleanups.reverse()) {
    try {
      await fn();
    } catch {
      /* abaikan */
    }
  }
}

// ---- Server vite (hanya bila --url tidak diberikan) ----
let baseUrl = args.get("url");
if (!baseUrl) {
  const port = await freePort();
  const vite = spawn("npx", ["vite", "--port", String(port), "--strictPort", "--host", "127.0.0.1"], { cwd: root, stdio: "ignore" });
  cleanups.push(() => vite.kill("SIGTERM"));
  baseUrl = `http://127.0.0.1:${port}`;
  await waitFor(async () => (await fetch(baseUrl)).ok, "server vite");
}

// ---- Chrome tanpa kepala ----
const profile = mkdtempSync(join(tmpdir(), "simak-sweep-"));
cleanups.push(() => rmSync(profile, { recursive: true, force: true }));
const chrome = spawn(
  chromeBin,
  ["--headless=new", "--remote-debugging-port=0", `--user-data-dir=${profile}`, "--no-first-run", "--no-default-browser-check", "--disable-gpu", "--hide-scrollbars", "about:blank"],
  { stdio: "ignore" },
);
cleanups.push(() => chrome.kill("SIGTERM"));
const dtPort = await waitFor(() => {
  const f = join(profile, "DevToolsActivePort");
  return existsSync(f) ? Number(readFileSync(f, "utf8").split("\n")[0]) : 0;
}, "port DevTools");
const targets = await waitFor(async () => {
  const list = await (await fetch(`http://127.0.0.1:${dtPort}/json/list`)).json();
  return list.filter((t) => t.type === "page");
}, "halaman Chrome");

// ---- Klien CDP minimal ----
const ws = new WebSocket(targets[0].webSocketDebuggerUrl);
await new Promise((res, rej) => {
  ws.onopen = res;
  ws.onerror = () => rej(new Error("gagal membuka WebSocket DevTools"));
});
cleanups.push(() => ws.close());
let nextId = 1;
const pending = new Map();
ws.onmessage = (ev) => {
  const msg = JSON.parse(ev.data);
  if (msg.id && pending.has(msg.id)) {
    const { resolve, reject } = pending.get(msg.id);
    pending.delete(msg.id);
    msg.error ? reject(new Error(`${msg.error.message}`)) : resolve(msg.result);
  }
};
const send = (method, params = {}) =>
  new Promise((resolve, reject) => {
    const id = nextId++;
    pending.set(id, { resolve, reject });
    ws.send(JSON.stringify({ id, method, params }));
  });
const evaluate = async (expression) => {
  const r = await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
  if (r.exceptionDetails) throw new Error(r.exceptionDetails.exception?.description ?? r.exceptionDetails.text);
  return r.result.value;
};

// Fungsi di dalam halaman: render tiap ayat dan ukur. Tanpa animasi (reduced-motion) agar sinkron.
const PAGE_SWEEP = `(async (list) => {
  const render = window.__simakRender;
  const widget = document.getElementById("widget");
  const content = document.getElementById("content");
  const arabic = document.getElementById("arabic");
  const translation = document.getElementById("translation");
  const paint = (surah, ayah) => render({ kind: "showing", surah, part: { kind: "ayah", ayah }, playing: true });
  paint(112, 1);
  await document.fonts.ready;
  const out = [];
  for (const [surah, ayah] of list) {
    paint(surah, ayah);
    const w = widget.getBoundingClientRect();
    const c = content.getBoundingClientRect();
    const a = arabic.getBoundingClientRect();
    const t = translation.getBoundingClientRect();
    const eps = 0.5;
    const vertical = c.height > widget.clientHeight + eps || t.bottom > w.bottom + eps || a.top < w.top - eps;
    const horizontal = arabic.scrollWidth > arabic.clientWidth + 1 || translation.scrollWidth > translation.clientWidth + 1 || c.right > w.right + eps || c.left < w.left - eps;
    out.push({
      surah, ayah,
      clipped: vertical || horizontal,
      tight: widget.hasAttribute("data-tight"),
      density: content.dataset.density,
      scale: Number(content.style.getPropertyValue("--arabic-scale")),
      size: Number(widget.style.getPropertyValue("--size-scale")),
      fill: Math.round((c.height / widget.clientHeight) * 1000) / 1000,
      arabicPx: Math.round(parseFloat(getComputedStyle(arabic).fontSize) * 10) / 10,
      translationPx: Math.round(parseFloat(getComputedStyle(translation).fontSize) * 10) / 10,
    });
  }
  return out;
})`;

try {
  await send("Page.enable");
  const query = new URLSearchParams({ preview: "ayah", material: "solid", controls: "1", backdrop: "1", theme });
  await send("Page.navigate", { url: `${baseUrl}/?${query}` });
  await waitFor(async () => (await evaluate("typeof window.__simakRender === 'function'")) === true, "kait __simakRender");
  await send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "reduce" }] });

  if (shotsDir) mkdirSync(shotsDir, { recursive: true });
  const results = {};
  let failed = false;
  console.log(`Sapuan ${ayat.length} ayat Juz 30 (tema ${theme}); kartu = jendela - 40x40`);
  console.log("ukuran     terpotong  ketat  rapat  skala-ukuran  skala-min  ayat terburuk (isi kartu)");
  for (const { w, h, key } of sizes) {
    await send("Emulation.setDeviceMetricsOverride", { width: w, height: h, deviceScaleFactor: 1, mobile: false });
    await sleep(150);
    const rows = await evaluate(`${PAGE_SWEEP}(${JSON.stringify(ayat)})`);
    results[key] = rows;
    const clipped = rows.filter((r) => r.clipped);
    const tight = rows.filter((r) => r.tight);
    const compact = rows.filter((r) => r.density === "compact");
    const minScale = Math.min(...rows.map((r) => r.scale));
    // Terburuk = skala font terkecil yang terpaksa dipakai, lalu kartu paling penuh.
    const worst = [...rows].sort((a, b) => a.scale - b.scale || b.fill - a.fill)[0];
    const sizeScale = rows[0].size;
    console.log(
      `${key.padEnd(10)} ${String(clipped.length).padStart(9)}  ${String(tight.length).padStart(5)}  ${String(compact.length).padStart(5)}  ${String(sizeScale).padStart(12)}  ${String(minScale).padStart(9)}  ${worst.surah}:${worst.ayah} (${worst.fill}, rapat=${worst.density === "compact"}, skala=${worst.scale})`,
    );
    for (const r of clipped.slice(0, 10)) console.log(`   terpotong: ${r.surah}:${r.ayah} skala=${r.scale} isi=${r.fill}`);
    if (assertKeys.has(key) && clipped.length > 0) failed = true;

    if (shotsDir) {
      const wanted = [...shotAyat, `${worst.surah}:${worst.ayah}`];
      for (const id of [...new Set(wanted)]) {
        const [s, a] = id.split(":").map(Number);
        await evaluate(`(() => { window.__simakRender({ kind: "showing", surah: ${s}, part: { kind: "ayah", ayah: ${a} }, playing: true }); return true; })()`);
        await sleep(120);
        const shot = await send("Page.captureScreenshot", { format: "png" });
        const file = join(shotsDir, `${theme}-${key}-${s}-${a}.png`);
        writeFileSync(file, Buffer.from(shot.data, "base64"));
      }
    }
  }
  const total = sizes.reduce((n, s) => n + results[s.key].filter((r) => r.clipped).length, 0);
  console.log(failed ? "GAGAL: ada ayat terpotong pada ukuran yang di-assert." : `OK: tidak ada ayat terpotong pada ${[...assertKeys].join(", ")} (total terpotong semua ukuran: ${total}).`);
  process.exitCode = failed ? 1 : 0;
} catch (e) {
  console.error(e);
  process.exitCode = 2;
} finally {
  await cleanup();
}
