// Pembantu bersama untuk skrip yang memakai server vite dan Chrome tanpa kepala lewat protokol
// DevTools (scripts/layout-sweep.mjs, scripts/e2e-window-mock.mjs). Tanpa dependensi npm.
//
// Disiplin pembersihan: tiap proses anak berjalan di grup prosesnya sendiri dan dihentikan
// (SIGTERM, lalu SIGKILL) oleh `teardown()`; profil Chrome sementara dihapus setelah Chrome benar-benar
// keluar. `teardown()` idempoten dan juga dipicu oleh SIGINT/SIGTERM/SIGHUP. Panggil di `finally`.

import { spawn } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const children = new Set();
const cleanupFns = [];
let fatal = null;
let tornDown = null;

export function freePort() {
  return new Promise((resolve, reject) => {
    const srv = createServer();
    srv.listen(0, "127.0.0.1", () => {
      const { port } = srv.address();
      srv.close(() => resolve(port));
    });
    srv.on("error", reject);
  });
}

function spawnTracked(name, cmd, cmdArgs, opts = {}) {
  const child = spawn(cmd, cmdArgs, { stdio: "ignore", detached: true, ...opts });
  const entry = { child, name, done: false };
  entry.exited = new Promise((resolve) => {
    child.once("exit", () => ((entry.done = true), resolve()));
    child.once("error", (e) => {
      entry.done = true;
      fatal ??= new Error(`gagal menjalankan ${name}: ${e.message}`);
      resolve();
    });
  });
  children.add(entry);
  return entry;
}

function killGroup(entry, signal) {
  const { pid } = entry.child;
  if (!pid || entry.done) return;
  try {
    process.kill(-pid, signal); // seluruh grup proses (Chrome dan anak-anaknya, vite)
  } catch {
    try {
      entry.child.kill(signal);
    } catch {
      /* sudah mati */
    }
  }
}

async function stopChild(entry) {
  if (entry.done) return;
  killGroup(entry, "SIGTERM");
  await Promise.race([entry.exited, sleep(3000)]);
  if (!entry.done) {
    killGroup(entry, "SIGKILL");
    await Promise.race([entry.exited, sleep(3000)]);
  }
}

/** Hentikan semua proses anak, lalu hapus berkas sementara. Aman dipanggil berulang kali. */
export function teardown() {
  tornDown ??= (async () => {
    for (const entry of [...children].reverse()) await stopChild(entry);
    for (const fn of cleanupFns.splice(0).reverse()) {
      try {
        await fn();
      } catch {
        /* abaikan */
      }
    }
  })();
  return tornDown;
}

for (const sig of ["SIGINT", "SIGTERM", "SIGHUP"]) {
  process.once(sig, () => {
    teardown().finally(() => process.exit(130));
  });
}
// Jaring terakhir (sinkron): bila proses keluar sebelum teardown selesai, matikan grup yang tersisa.
process.on("exit", () => {
  for (const entry of children) killGroup(entry, "SIGKILL");
});

/** Ulangi `fn` sampai mengembalikan nilai benar; gagal cepat bila proses anak gagal dijalankan. */
export async function waitFor(fn, what, ms = 30000) {
  const end = Date.now() + ms;
  for (;;) {
    if (fatal) throw fatal;
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

/** Nyalakan vite di port acak dari `root` (node langsung ke vite.js, bukan npx). Mengembalikan URL dasar. */
export async function startVite(root) {
  const port = await freePort();
  spawnTracked("vite", process.execPath, [join(root, "node_modules/vite/bin/vite.js"), "--port", String(port), "--strictPort", "--host", "127.0.0.1"], { cwd: root });
  const baseUrl = `http://127.0.0.1:${port}`;
  await waitFor(async () => (await fetch(baseUrl)).ok, "server vite");
  return baseUrl;
}

/** Nyalakan Chrome tanpa kepala (profil sementara) dan sambungkan klien CDP ke halamannya. */
export async function startChrome(chromeBin = "google-chrome") {
  const profile = mkdtempSync(join(tmpdir(), "simak-sweep-"));
  cleanupFns.push(() => rmSync(profile, { recursive: true, force: true, maxRetries: 20, retryDelay: 100 }));
  spawnTracked("Chrome", chromeBin, ["--headless=new", "--remote-debugging-port=0", `--user-data-dir=${profile}`, "--no-first-run", "--no-default-browser-check", "--disable-gpu", "--hide-scrollbars", "about:blank"]);
  const dtPort = await waitFor(() => {
    const f = join(profile, "DevToolsActivePort");
    return existsSync(f) ? Number(readFileSync(f, "utf8").split("\n")[0]) : 0;
  }, "port DevTools");
  const targets = await waitFor(async () => {
    const list = await (await fetch(`http://127.0.0.1:${dtPort}/json/list`)).json();
    const pages = list.filter((t) => t.type === "page");
    return pages.length ? pages : null;
  }, "halaman Chrome");

  const ws = new WebSocket(targets[0].webSocketDebuggerUrl);
  cleanupFns.push(() => ws.close());
  await new Promise((res, rej) => {
    ws.onopen = res;
    ws.onerror = () => rej(new Error("gagal membuka WebSocket DevTools"));
  });
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
  return { send, evaluate };
}
