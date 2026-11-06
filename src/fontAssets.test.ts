import { describe, expect, it } from "vitest";
import { existsSync, readFileSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";

/** Pelindung kabel aset: jsdom tidak bisa merender font, jadi yang diuji berkas dan aturan CSS-nya. */
const here = (rel: string) => fileURLToPath(new URL(rel, import.meta.url));
const FONT = here("./assets/fonts/AmiriQuran-Regular.woff2");
const css = readFileSync(here("./style.css"), "utf8");
const html = readFileSync(here("../index.html"), "utf8");

describe("font Arab yang dibundel (Amiri Quran)", () => {
  it("berkas font ada, berformat WOFF2, dan kecil (< 400 KB)", () => {
    expect(existsSync(FONT)).toBe(true);
    expect(statSync(FONT).size).toBeLessThan(400 * 1024);
    expect(statSync(FONT).size).toBeGreaterThan(10 * 1024);
    expect(readFileSync(FONT).subarray(0, 4).toString("latin1")).toBe("wOF2");
  });

  it("teks lisensi OFL 1.1 dan README asal font tersedia", () => {
    const ofl = readFileSync(here("./assets/fonts/OFL.txt"), "utf8");
    expect(ofl).toContain("SIL OPEN FONT LICENSE Version 1.1");
    expect(ofl).toContain("Copyright 2010-2022 The Amiri Project Authors");
    const readme = readFileSync(here("./assets/fonts/README.md"), "utf8");
    expect(readme).toMatch(/SHA-256 `[0-9a-f]{64}`/);
  });

  it("@font-face Amiri Quran menunjuk ke berkas itu dengan font-display: block", () => {
    const rule = css.match(/@font-face\s*{[^}]*}/)?.[0] ?? "";
    expect(rule).toContain('font-family: "Amiri Quran"');
    expect(rule).toContain("./assets/fonts/AmiriQuran-Regular.woff2");
    expect(rule).toContain('format("woff2")');
    expect(rule).toContain("font-display: block");
  });

  it("--font-arab memakai Amiri Quran lebih dulu, lalu cadangan sistem", () => {
    const stack = css.match(/--font-arabic:\s*([^;]+);/)?.[1] ?? "";
    expect(stack.startsWith('"Amiri Quran"')).toBe(true);
    expect(stack).toContain("serif");
  });

  it("index.html memuat-awal font yang sama (hash ditangani Vite)", () => {
    expect(html).toContain('rel="preload"');
    expect(html).toContain("/src/assets/fonts/AmiriQuran-Regular.woff2");
    expect(html).toContain('as="font"');
  });

  it("teks OFL ikut terpasang bersama aplikasi (bundle.resources)", () => {
    const conf = JSON.parse(readFileSync(here("../src-tauri/tauri.conf.json"), "utf8"));
    expect(conf.bundle.resources["../src/assets/fonts/OFL.txt"]).toBe("licenses/AmiriQuran-OFL.txt");
    const notices = readFileSync(here("../THIRD-PARTY-NOTICES.md"), "utf8");
    expect(notices).toContain("licenses/AmiriQuran-OFL.txt");
  });

  it("CSP Tauri tidak diperketat sehingga memblokir font lokal", () => {
    const conf = JSON.parse(readFileSync(here("../src-tauri/tauri.conf.json"), "utf8"));
    const csp = conf.app.security.csp as string | null | Record<string, string>;
    if (csp === null) return; // tanpa CSP: tidak ada pembatasan
    const fontSrc = typeof csp === "string" ? csp.match(/font-src([^;]*)/)?.[1] ?? "" : csp["font-src"] ?? "";
    expect(fontSrc).toContain("'self'");
  });
});
