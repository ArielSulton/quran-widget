import { describe, expect, it } from "vitest";
import { detectMaterial, isMacPlatform } from "./material";

describe("isMacPlatform", () => {
  it("mengenali macOS dari navigator.platform atau userAgent", () => {
    expect(isMacPlatform("MacIntel", "")).toBe(true);
    expect(isMacPlatform("", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)")).toBe(true);
    expect(isMacPlatform("Win32", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")).toBe(false);
    expect(isMacPlatform("Linux x86_64", "Mozilla/5.0 (X11; Linux x86_64)")).toBe(false);
  });
});

describe("detectMaterial", () => {
  it("macOS di dalam Tauri memakai efek native (vibrancy)", () => {
    expect(detectMaterial({ tauri: true, mac: true, backdropFilter: true })).toBe("native");
    expect(detectMaterial({ tauri: true, mac: true, backdropFilter: false })).toBe("native");
  });

  it("browser biasa dengan backdrop-filter memakai kaca CSS", () => {
    expect(detectMaterial({ tauri: false, mac: false, backdropFilter: true })).toBe("css");
    expect(detectMaterial({ tauri: false, mac: true, backdropFilter: true })).toBe("css");
  });

  it("Windows/Linux di dalam Tauri: tidak ada blur di belakang jendela => solid", () => {
    expect(detectMaterial({ tauri: true, mac: false, backdropFilter: true })).toBe("solid");
  });

  it("tanpa dukungan backdrop-filter => solid", () => {
    expect(detectMaterial({ tauri: false, mac: false, backdropFilter: false })).toBe("solid");
  });
});
