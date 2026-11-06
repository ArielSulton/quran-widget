#!/usr/bin/env node
// Sapuan tata letak: merender SEMUA ayat Juz 30 (564) di Chrome tanpa kepala pada beberapa ukuran
// jendela dan menghitung masalah: ayat terpotong dan teks yang bertabrakan dengan pil kontrol,
// pegangan ubah ukuran, atau footer (kontrol dipaksa terlihat lewat `controls=1`). Tanpa
// dependensi npm: Chrome dikendalikan lewat protokol DevTools memakai WebSocket bawaan Node 22.
//
// Pemakaian:
//   node scripts/layout-sweep.mjs [--sizes 430x270,520x320,720x420,960x560] [--assert <ukuran,...>]
//        [--url http://localhost:1420] [--shots DIR] [--shot-ayat 112:1,87:7,98:8] [--theme light|dark]
//        [--chrome google-chrome]
//
// Bawaan: ukuran minimum dari src-tauri/tauri.conf.json plus 520x320, 720x420, 960x560, semuanya
// di-assert. Tanpa --url, skrip menyalakan server vite sendiri di port acak (tidak menyentuh server
// lain). Ukuran adalah ukuran JENDELA (viewport); kartu = jendela dikurangi inset bayangan (40x40).
// Kode keluar: 0 bersih, 1 ada masalah pada ukuran yang di-assert, 2 galat skrip.
//
// Pembersihan (scripts/lib/devtools.mjs): semua proses anak berjalan di grup prosesnya sendiri dan
// dihentikan (SIGTERM, lalu SIGKILL) saat selesai, galat, SIGINT/SIGTERM; profil Chrome sementara
// dihapus setelah Chrome benar-benar keluar.

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { sleep, startChrome, startVite, teardown, waitFor } from "./lib/devtools.mjs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const args = new Map();
for (let i = 2; i < process.argv.length; i++) {
  const a = process.argv[i];
  if (a.startsWith("--")) {
    const next = process.argv[i + 1];
    args.set(a.slice(2), next === undefined || next.startsWith("--") ? "1" : (i++, next));
  }
}
const conf = JSON.parse(readFileSync(join(root, "src-tauri/tauri.conf.json"), "utf8"));
const win0 = conf.app.windows[0];
const parseSizes = (s) =>
  s.split(",").map((x) => x.trim()).filter(Boolean).map((x) => {
    const [w, h] = x.split("x").map(Number);
    return { w, h, key: `${w}x${h}` };
  });
const sizes = parseSizes(args.get("sizes") ?? `${win0.minWidth}x${win0.minHeight},520x320,720x420,960x560`);
const assertKeys = new Set(
  args.has("assert")
    ? args.get("assert").split(",").map((s) => s.trim()).filter(Boolean)
    : sizes.map((s) => s.key),
);
const shotsDir = args.get("shots");
const shotAyat = (args.get("shot-ayat") ?? "112:1,87:7,98:8").split(",").filter(Boolean);
const theme = args.get("theme") ?? "light";
const chromeBin = args.get("chrome") ?? "google-chrome";

const quran = JSON.parse(readFileSync(join(root, "data/quran/juz30.json"), "utf8"));
const ayat = [];
for (const [surah, s] of Object.entries(quran)) s.ayat.forEach((_, i) => ayat.push([Number(surah), i + 1]));

// ---- Fungsi di dalam halaman: render tiap ayat dan ukur ----
// Tanpa animasi (reduced-motion) agar sinkron. Masalah = terpotong (tepi kartu) ATAU tumpang tindih:
// kotak baris teks Arab/terjemahan memotong kotak pil kontrol, pegangan, atau footer (jarak >= 2 px).
const PAGE_SWEEP = `(async (list) => {
  const render = window.__simakRender;
  const widget = document.getElementById("widget");
  const content = document.getElementById("content");
  const arabic = document.getElementById("arabic");
  const translation = document.getElementById("translation");
  const footer = document.getElementById("footer");
  const reserved = [document.getElementById("controls"), document.getElementById("resize-grip"), footer];
  const paint = (surah, ayah) => render({ kind: "showing", surah, part: { kind: "ayah", ayah }, playing: true });
  const lineRects = (el) => {
    const out = [];
    const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
    for (let n = walker.nextNode(); n; n = walker.nextNode()) {
      const r = document.createRange();
      r.selectNodeContents(n);
      for (const q of r.getClientRects()) if (q.width > 0 && q.height > 0) out.push(q);
    }
    return out;
  };
  const CLEAR = 2;
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
    const texts = [...lineRects(arabic), ...lineRects(translation)];
    let overlap = false;
    let gap = Infinity;
    for (const el of reserved) {
      if (!el.textContent.trim() && el !== reserved[0] && el !== reserved[1]) continue;
      const p = el.getBoundingClientRect();
      for (const q of texts) {
        const horiz = q.right > p.left && q.left < p.right;
        if (!horiz) continue;
        gap = Math.min(gap, p.top - q.bottom);
        if (q.bottom > p.top - CLEAR && q.top < p.bottom) overlap = true;
      }
    }
    out.push({
      surah, ayah,
      clipped: vertical || horizontal,
      overlap,
      gap: gap === Infinity ? null : Math.round(gap * 10) / 10,
      tight: widget.hasAttribute("data-tight"),
      density: content.dataset.density,
      scale: Number(content.style.getPropertyValue("--arabic-scale")),
      size: Number(widget.style.getPropertyValue("--size-scale")),
      fill: Math.round((c.height / widget.clientHeight) * 1000) / 1000,
    });
  }
  return out;
})`;

async function main() {
  const baseUrl = args.get("url") ?? (await startVite(root));
  const { send, evaluate } = await startChrome(chromeBin);

  await send("Page.enable");
  const query = new URLSearchParams({ preview: "ayah", material: "solid", controls: "1", backdrop: "1", theme });
  await send("Page.navigate", { url: `${baseUrl}/?${query}` });
  await waitFor(async () => (await evaluate("typeof window.__simakRender === 'function'")) === true, "kait __simakRender");
  await send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "reduce" }] });

  if (shotsDir) mkdirSync(shotsDir, { recursive: true });
  const results = {};
  let failed = false;
  console.log(`Sapuan ${ayat.length} ayat Juz 30 (tema ${theme}); kartu = jendela - 40x40; kontrol dipaksa terlihat`);
  console.log("ukuran     masalah  terpotong  tumpang-tindih  ketat  rapat  skala-ukuran  langkah-font-min  terburuk");
  for (const { w, h, key } of sizes) {
    await send("Emulation.setDeviceMetricsOverride", { width: w, height: h, deviceScaleFactor: 1, mobile: false });
    await sleep(150);
    const rows = await evaluate(`${PAGE_SWEEP}(${JSON.stringify(ayat)})`);
    results[key] = rows;
    const clipped = rows.filter((r) => r.clipped);
    const overlap = rows.filter((r) => r.overlap);
    const problems = rows.filter((r) => r.clipped || r.overlap);
    const tight = rows.filter((r) => r.tight);
    const compact = rows.filter((r) => r.density === "compact");
    const minScale = Math.min(...rows.map((r) => r.scale));
    // Terburuk = font terkecil yang terpaksa dipakai, lalu jarak ke kontrol paling sempit.
    const worst = [...rows].sort((a, b) => a.scale - b.scale || (a.gap ?? 1e9) - (b.gap ?? 1e9))[0];
    console.log(
      `${key.padEnd(10)} ${String(problems.length).padStart(7)}  ${String(clipped.length).padStart(9)}  ${String(overlap.length).padStart(14)}  ${String(tight.length).padStart(5)}  ${String(compact.length).padStart(5)}  ${String(rows[0].size).padStart(12)}  ${String(minScale).padStart(16)}  ${worst.surah}:${worst.ayah} (langkah=${worst.scale}, jarak=${worst.gap})`,
    );
    for (const r of problems.slice(0, 10)) console.log(`   masalah: ${r.surah}:${r.ayah} terpotong=${r.clipped} tumpang=${r.overlap} jarak=${r.gap} langkah=${r.scale}`);
    if (assertKeys.has(key) && problems.length > 0) failed = true;

    if (shotsDir) {
      const wanted = [...shotAyat, `${worst.surah}:${worst.ayah}`, ...problems.slice(0, 3).map((r) => `${r.surah}:${r.ayah}`)];
      for (const id of [...new Set(wanted)]) {
        const [s, a] = id.split(":").map(Number);
        await evaluate(`(() => { window.__simakRender({ kind: "showing", surah: ${s}, part: { kind: "ayah", ayah: ${a} }, playing: true }); return true; })()`);
        await sleep(120);
        const shot = await send("Page.captureScreenshot", { format: "png" });
        writeFileSync(join(shotsDir, `${theme}-${key}-${s}-${a}.png`), Buffer.from(shot.data, "base64"));
      }
    }
  }
  const missing = [...assertKeys].filter((k) => !results[k]);
  if (missing.length) console.log(`Peringatan: ukuran di --assert tidak disapu: ${missing.join(", ")}`);
  const asserted = [...assertKeys].filter((k) => results[k]);
  console.log(failed ? `GAGAL: ada masalah (terpotong atau tumpang tindih dengan kontrol) pada ${asserted.join(", ")}.` : `OK: 0 masalah pada ${asserted.join(", ")}.`);
  process.exitCode = failed ? 1 : 0;
}

try {
  await main();
} catch (e) {
  console.error(e);
  process.exitCode = 2;
} finally {
  await teardown();
}
