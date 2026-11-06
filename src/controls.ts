import { formatSync, stepSync, SYNC_MAX_MS, SYNC_MIN_MS } from "./sync";
import type { ThemePref } from "./theme";

export interface ControlsOptions {
  theme: ThemePref;
  syncMs: number;
  onTheme(pref: ThemePref): void;
  onSync(ms: number): void;
}

/** Jeda sebelum tombol +/− menahan dan mengulang, dan jarak antar ulangan (ms). */
const HOLD_DELAY_MS = 450;
const HOLD_REPEAT_MS = 90;

function query<T extends Element>(root: ParentNode, selector: string): T {
  const node = root.querySelector<T>(selector);
  if (!node) throw new Error(`Elemen ${selector} tidak ditemukan`);
  return node;
}

/** Hubungkan pil kontrol di `index.html`: pilihan tema dan stepper geser sinkron. */
export function initControls(opts: ControlsOptions): void {
  const root = query<HTMLElement>(document, "#controls");
  const themeButtons = Array.from(root.querySelectorAll<HTMLButtonElement>("[data-theme-choice]"));
  const dec = query<HTMLButtonElement>(root, "#sync-dec");
  const inc = query<HTMLButtonElement>(root, "#sync-inc");
  const value = query<HTMLButtonElement>(root, "#sync-value");

  let theme = opts.theme;
  let syncMs = opts.syncMs;

  function paint(): void {
    for (const b of themeButtons) {
      b.setAttribute("aria-pressed", String(b.dataset.themeChoice === theme));
    }
    value.textContent = formatSync(syncMs);
    value.setAttribute(
      "aria-label",
      `Geser sinkron ${formatSync(syncMs)}. Klik dua kali atau tekan Enter untuk mengatur ulang ke nol`,
    );
    // aria-disabled (bukan disabled) agar fokus papan ketik tidak hilang di batas rentang.
    dec.setAttribute("aria-disabled", String(syncMs <= SYNC_MIN_MS));
    inc.setAttribute("aria-disabled", String(syncMs >= SYNC_MAX_MS));
  }

  function setSync(next: number): void {
    if (next === syncMs) return;
    syncMs = next;
    paint();
    opts.onSync(syncMs);
  }

  for (const b of themeButtons) {
    b.addEventListener("click", () => {
      theme = b.dataset.themeChoice as ThemePref;
      paint();
      opts.onTheme(theme);
    });
  }

  // Tombol +/−: satu langkah per tekan (Shift = 1 dtk); tahan untuk mengulang.
  // Klik papan ketik (detail 0) satu langkah; Shift+Enter/Spasi = langkah kasar.
  function bindStepper(button: HTMLButtonElement, dir: 1 | -1): void {
    let delay = 0;
    let repeat = 0;
    const stop = (): void => {
      window.clearTimeout(delay);
      window.clearInterval(repeat);
    };
    const step = (coarse: boolean): void => setSync(stepSync(syncMs, dir, coarse));

    button.addEventListener("pointerdown", (e) => {
      if (e.button !== 0) return;
      const coarse = e.shiftKey;
      step(coarse);
      stop();
      delay = window.setTimeout(() => {
        repeat = window.setInterval(() => step(coarse), HOLD_REPEAT_MS);
      }, HOLD_DELAY_MS);
    });
    for (const type of ["pointerup", "pointerleave", "pointercancel", "blur"]) {
      button.addEventListener(type, stop);
    }
    button.addEventListener("click", (e) => {
      if (e.detail === 0) step(e.shiftKey); // aktivasi papan ketik (Enter/Spasi)
    });
  }
  bindStepper(dec, -1);
  bindStepper(inc, 1);

  // Panah kiri/kanan pada tombol stepper terfokus: kiri = kurangi, kanan = tambah (Shift = 1 dtk).
  for (const button of [dec, inc]) {
    button.addEventListener("keydown", (e) => {
      const dir = e.key === "ArrowLeft" ? -1 : e.key === "ArrowRight" ? 1 : 0;
      if (dir === 0) return;
      e.preventDefault();
      setSync(stepSync(syncMs, dir, e.shiftKey));
    });
  }

  // `[` / `]` di mana saja: kurangi / tambah 0,1 dtk (Shift = 1 dtk). Tidak mengganggu kolom ketik.
  document.addEventListener("keydown", (e) => {
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    const t = e.target;
    if (t instanceof HTMLElement && (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName))) {
      return;
    }
    if (e.code === "BracketLeft") setSync(stepSync(syncMs, -1, e.shiftKey));
    else if (e.code === "BracketRight") setSync(stepSync(syncMs, 1, e.shiftKey));
  });

  // Reset ke 0: klik dua kali pada nilai, atau Enter saat nilai terfokus.
  value.addEventListener("dblclick", () => setSync(0));
  value.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      e.preventDefault();
      setSync(0);
    }
  });

  // Esc melepas fokus sehingga pil kembali tersembunyi.
  root.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && document.activeElement instanceof HTMLElement) {
      document.activeElement.blur();
    }
  });

  paint();
}
