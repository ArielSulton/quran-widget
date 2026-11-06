import { describe, expect, it } from "vitest";
import { parsePreview } from "./preview";

describe("parsePreview", () => {
  it("tanpa parameter preview => null (mode normal)", () => {
    expect(parsePreview("")).toBeNull();
    expect(parsePreview("?theme=dark")).toBeNull();
    expect(parsePreview("?preview=nonsense")).toBeNull();
  });

  it("ayah: bawaan 112:1", () => {
    expect(parsePreview("?preview=ayah")).toEqual({
      state: { kind: "showing", surah: 112, part: { kind: "ayah", ayah: 1 }, playing: true },
      theme: undefined,
      syncMs: undefined,
      controls: false,
      backdrop: false,
      material: undefined,
    });
  });

  it("ayah dengan surah dan ayat eksplisit", () => {
    const p = parsePreview("?preview=ayah&surah=98&ayah=8");
    expect(p?.state).toEqual({
      kind: "showing",
      surah: 98,
      part: { kind: "ayah", ayah: 8 },
      playing: true,
    });
  });

  it("intro dan waiting", () => {
    expect(parsePreview("?preview=intro")?.state).toEqual({
      kind: "showing",
      surah: 112,
      part: { kind: "intro" },
      playing: true,
    });
    expect(parsePreview("?preview=waiting")?.state).toEqual({ kind: "waiting" });
  });

  it("theme, sync, controls, backdrop, material", () => {
    const p = parsePreview("?preview=waiting&theme=dark&sync=300&controls=1&backdrop=1&material=solid");
    expect(p).toMatchObject({
      theme: "dark",
      syncMs: 300,
      controls: true,
      backdrop: true,
      material: "solid",
    });
  });

  it("nilai yang tidak valid diabaikan", () => {
    const p = parsePreview("?preview=ayah&theme=blue&sync=abc&controls=0&material=glass&surah=x&ayah=0");
    expect(p).toMatchObject({
      theme: undefined,
      syncMs: undefined,
      controls: false,
      material: undefined,
    });
    expect(p?.state).toEqual({
      kind: "showing",
      surah: 112,
      part: { kind: "ayah", ayah: 1 },
      playing: true,
    });
  });

  it("sync dijepit seperti pengaturan biasa", () => {
    expect(parsePreview("?preview=ayah&sync=-99999")?.syncMs).toBe(-2000);
  });
});
