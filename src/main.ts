import "./style.css";
import { viewModel, type QuranData, type WidgetState } from "./viewModel";

// Data Al-Qur'an dimuat pada task berikutnya; sementara kosong.
const quran: QuranData = {};

function el(id: string): HTMLElement {
  const node = document.getElementById(id);
  if (!node) throw new Error(`Elemen #${id} tidak ditemukan`);
  return node;
}

const arabicEl = el("arabic");
const translationEl = el("translation");
const footerEl = el("footer");

export function render(state: WidgetState): void {
  const vm = viewModel(state, quran);
  arabicEl.textContent = vm.arabic;
  translationEl.textContent = vm.translation;
  footerEl.textContent = vm.footer;
}

// Keadaan awal saat aplikasi dibuka.
render({ kind: "waiting" });
