import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { MIN_WINDOW_SIZE, DEFAULT_WINDOW_SIZE } from "./windowSize";

/** Pelindung kabel: konfigurasi jendela, izin Tauri, HTML dan CSS pegangan ubah ukuran harus selaras. */
const here = (rel: string) => fileURLToPath(new URL(rel, import.meta.url));
const html = readFileSync(here("../index.html"), "utf8");
const css = readFileSync(here("./style.css"), "utf8");
const conf = JSON.parse(readFileSync(here("../src-tauri/tauri.conf.json"), "utf8")) as {
  app: { windows: { width: number; height: number; minWidth: number; minHeight: number; resizable: boolean }[] };
};
const caps = JSON.parse(readFileSync(here("../src-tauri/capabilities/default.json"), "utf8")) as {
  permissions: string[];
};

describe("ukuran jendela di tauri.conf.json", () => {
  const win = conf.app.windows[0];

  it("bawaan 520x320 dan minimum 420x260, sama dengan konstanta antarmuka", () => {
    expect({ w: win.width, h: win.height }).toEqual(DEFAULT_WINDOW_SIZE);
    expect({ w: win.minWidth, h: win.minHeight }).toEqual(MIN_WINDOW_SIZE);
    expect(win.resizable).toBe(true);
  });
});

describe("izin jendela", () => {
  it("memuat izin seret-ubah-ukuran dan set-size, tanpa izin jendela yang tak perlu", () => {
    expect(caps.permissions).toContain("core:window:allow-start-resize-dragging");
    expect(caps.permissions).toContain("core:window:allow-set-size");
    const windowPerms = caps.permissions.filter((p) => p.startsWith("core:window:")).sort();
    expect(windowPerms).toEqual([
      "core:window:allow-set-shadow",
      "core:window:allow-set-size",
      "core:window:allow-set-theme",
      "core:window:allow-start-dragging",
      "core:window:allow-start-resize-dragging",
    ]);
  });
});

describe("pegangan ubah ukuran", () => {
  it("tombol sungguhan di dalam kartu, berlabel dan bertooltip, bukan area seret", () => {
    const m = html.match(/<button[^>]*id="resize-grip"[^>]*>/);
    expect(m).not.toBeNull();
    const tag = m![0];
    expect(tag).toContain('type="button"');
    expect(tag).toContain('aria-label="Ubah ukuran jendela"');
    expect(tag).toContain('title="Tarik untuk mengubah ukuran. Panah: 10 px, Shift+panah: 50 px"');
    expect(tag).not.toContain("data-tauri-drag-region");
    expect(html.indexOf('id="resize-grip"')).toBeGreaterThan(html.indexOf('id="widget"'));
    expect(html.indexOf('id="resize-grip"')).toBeLessThan(html.indexOf("</main>"));
  });

  it("memakai kursor nwse-resize dan muncul bersama pil kontrol (hover, fokus, controls=1)", () => {
    expect(css).toMatch(/#resize-grip\s*{[^}]*cursor:\s*nwse-resize/);
    expect(css).toContain("#widget:hover #resize-grip");
    expect(css).toContain("html[data-controls] #resize-grip");
    expect(css).toContain("#widget:has(:focus-visible) #resize-grip");
  });
});
