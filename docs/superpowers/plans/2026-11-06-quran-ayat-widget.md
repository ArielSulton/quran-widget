# Widget Ayat Al-Qur'an (Juz 30, Yasser Al-Dosari) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Widget desktop Tauri yang membaca track Qur'an Yasser Al-Dosari yang sedang diputar di aplikasi Spotify lokal dan menampilkan ayat Arab + terjemah Kemenag yang berganti tepat (selisih ≤ 300 ms) mengikuti bacaan, untuk 37 surah Juz 30.

**Architecture:** Inti Rust murni dan bisa diuji (`Timeline` → `PositionClock` → `Engine`) menerima `Snapshot` dari sumber pemutaran per OS (`PlaybackSource`) dan menghasilkan `WidgetState`. Backend Tauri mengirim `WidgetState` ke frontend lewat event `state`; frontend (TypeScript tanpa framework) mengubahnya jadi tampilan "Fokus besar". Data waktu ayat dan geseran per track dibuat sekali oleh alat Python di `tools/` dan dibundel sebagai JSON; audio tidak disimpan.

**Tech Stack:** Tauri 2 (Rust), TypeScript + Vite + vitest, crate `mpris` (Linux), `windows` crate (SMTC), `osascript` via `std::process::Command` (macOS), Python 3 untuk alat kalibrasi.

**Spec:** `docs/superpowers/specs/2026-11-06-quran-ayat-widget-design.md`

**Catatan repo:** folder ini belum repositori git dan aturan pengguna melarang commit otomatis, jadi rencana ini tidak punya langkah commit. Commit hanya jika pengguna meminta.

## Global Constraints

- Kerangka: Tauri (Rust); tampilan HTML/JavaScript.
- Tanpa login Spotify dan tanpa Spotify Web API; baca pemutaran dari aplikasi Spotify di komputer yang sama.
- Platform: Windows, macOS, Linux.
- Versi pertama: Juz 30 saja, surah 78 (An-Naba') sampai 114 (An-Nas), 37 surah.
- Qari: Yasser Al-Dosari saja.
- Terjemah: Kementerian Agama RI.
- Tampilan: "Fokus besar" (mockup pilihan C): ayat Arab sangat besar di tengah, terjemah di bawahnya, nama surah dan nomor ayat kecil di bagian bawah.
- Ketepatan: selisih pergantian ayat ≤ 300 ms. Jika sebuah OS tidak mencapainya, tetap dirilis dengan keterangan "akurasi lebih rendah".
- Perilaku: dijeda → diam di ayat terakhir; ganti surah → langsung ganti; geser posisi → loncat ke ayat yang sesuai; bukan track Qur'an Juz 30 atau Spotify tidak berjalan → tulisan "Menunggu bacaan Al-Qur'an"; bagian sebelum ayat 1 → keadaan `Intro`.
- Hanya angka (waktu ayat, geseran) yang dibundel; audio Spotify dan audio qari tidak disimpan atau dibagikan.
- Rekaman referensi: `https://server11.mp3quran.net/yasser/NNN.mp3` (NNN = nomor surah 3 digit).

## Review Focus

1. Durasi track di Spotify tidak cocok dengan yang tercatat (rekaman diganti) → `Waiting`, bukan ayat yang mungkin salah. (Task 5)
2. Judul tak dikenali, iklan, atau lagu non-Qur'an → `Waiting`. (Task 4, 5)
3. Spotify ditutup atau tidak berjalan saat widget terbuka → `Waiting`, tanpa crash. (Task 5, 9)
4. Posisi di luar jangkauan: kembali ke 0 (track diulang) → `Intro`; melewati ayat terakhir → tetap ayat terakhir. (Task 5)
5. Ganti track berturut-turut dengan cepat → state mengikuti track terakhir, tanpa sisa ayat surah sebelumnya. (Task 5)

---

## File Structure

- `src/` frontend: `main.ts` (pasang event), `viewModel.ts` (state → teks), `style.css`, `index.html`
- `src-tauri/src/lib.rs` Tauri setup + loop; `surah.rs` pencocok judul; `timeline.rs`; `clock.rs`; `engine.rs`; `playback/{mod,linux,macos,windows}.rs`
- `data/quran/juz30.json` teks Arab + terjemah; `data/timing/NNN.json` waktu ayat per surah
- `tools/` Python: `align.py`, `offset.py`, `build_timing.py`, `spotcheck.py`
- `tests/fixtures/spotify_titles.json` judul asli track Juz 30 dari Spotify

---

### Task 1: Kerangka Tauri dan tampilan "Fokus besar"

**Files:**
- Create: proyek Tauri 2 + Vite TypeScript di root; `src/viewModel.ts`, `src/viewModel.test.ts`, `src/style.css`
- Test: `src/viewModel.test.ts`

**Interfaces:**
- Produces:
  ```ts
  type Part = { kind: "intro" } | { kind: "ayah"; ayah: number };
  type WidgetState =
    | { kind: "waiting" }
    | { kind: "showing"; surah: number; part: Part; playing: boolean };
  type QuranData = Record<number, { name: string; ayat: { ar: string; id: string }[] }>;
  const BASMALAH: { ar: string; id: string };
  function viewModel(s: WidgetState, q: QuranData): { arabic: string; translation: string; footer: string };
  ```

- [ ] **Step 1: Tulis test gagal** di `src/viewModel.test.ts`: `waiting` → `{arabic:"", translation:"Menunggu bacaan Al-Qur'an", footer:""}`; `intro` surah 112 → `arabic` = `BASMALAH.ar`, `footer` = `"Al-Ikhlas · mukadimah"`; `ayah` 1 surah 112 → teks dari `q[112].ayat[0]`, `footer` = `"Al-Ikhlas · 112:1"`.
- [ ] **Step 2: Jalankan** `npx vitest run src/viewModel.test.ts`; hasil yang diharapkan: FAIL (modul belum ada).
- [ ] **Step 3: Implementasikan `viewModel` dan `BASMALAH`** (terjemah Kemenag: "Dengan nama Allah Yang Maha Pengasih, Maha Penyayang."). `nama` surah ada di `QuranData`.
- [ ] **Step 4: Jalankan test**; hasil: PASS.
- [ ] **Step 5: Tampilan.** `index.html` + `style.css` + `main.ts` merender `viewModel` ke tiga elemen; huruf Arab besar di tengah (mengikuti mockup C di `.superpowers/brainstorm/`). Jendela: tanpa dekorasi, selalu di atas, bisa digeser (`tauri.conf.json`). Verifikasi: `npm run tauri dev` menampilkan jendela dengan `waiting`.

### Task 2: Teks Qur'an dan terjemah Kemenag Juz 30

**Files:**
- Create: `data/quran/juz30.json`, `data/quran/LICENSE-NOTES.md`, `tools/check_quran_data.py`
- Test: `tools/check_quran_data.py`

**Interfaces:**
- Produces: `data/quran/juz30.json` berbentuk `QuranData` (Task 1), kunci nomor surah 78..114.

- [ ] **Step 1: Periksa lisensi** sumber teks Arab, terjemah Kemenag, dan huruf Al-Qur'an untuk didistribusikan dalam aplikasi; tulis hasil dan sumber di `LICENSE-NOTES.md`. Jika tidak ada sumber yang mengizinkan, **berhenti dan laporkan ke pengguna** sebelum lanjut.
- [ ] **Step 2: Tulis `check_quran_data.py`** dengan tabel jumlah ayat: `{78:40,79:46,80:42,81:29,82:19,83:36,84:25,85:22,86:17,87:19,88:26,89:30,90:20,91:15,92:21,93:11,94:8,95:8,96:19,97:5,98:8,99:8,100:11,101:11,102:8,103:3,104:9,105:5,106:4,107:7,108:3,109:6,110:3,111:5,112:4,113:5,114:6}` (total 564). Asersi: tepat 37 surah, jumlah ayat tiap surah sesuai tabel, tak ada `ar` atau `id` kosong.
- [ ] **Step 3: Jalankan** `python3 -I tools/check_quran_data.py`; hasil: FAIL (data belum ada).
- [ ] **Step 4: Susun `juz30.json`** dari sumber berlisensi di Step 1.
- [ ] **Step 5: Jalankan ulang**; hasil: `OK 37 surah, 564 ayat`.

### Task 3: Sumber pemutaran Linux dan pengambilan judul track asli

**Files:**
- Create: `src-tauri/src/playback/mod.rs`, `src-tauri/src/playback/linux.rs`, `src-tauri/src/bin/dump_titles.rs`, `tests/fixtures/spotify_titles.json`
- Test: `src-tauri/src/playback/linux.rs` (unit untuk pemetaan metadata)

**Interfaces:**
- Produces:
  ```rust
  pub struct Snapshot { pub title: String, pub duration_ms: u64, pub position_ms: u64, pub is_playing: bool }
  pub enum SourceError { Unavailable(String) }
  pub trait PlaybackSource: Send {
      /// Ok(None) = Spotify tidak berjalan.
      fn poll(&mut self) -> Result<Option<Snapshot>, SourceError>;
  }
  pub fn default_source() -> Box<dyn PlaybackSource>; // pilih per OS dengan cfg
  pub struct LinuxSource; // implementasi MPRIS untuk player "spotify"
  ```

- [ ] **Step 1: Test gagal:** fungsi murni `snapshot_from_mpris(title:&str, length_us:u64, position_us:u64, status:&str) -> Snapshot` memetakan mikrodetik→ms dan `"Playing"`→`is_playing=true`; uji dengan nilai `(…, 1_743_000_000, 5_250_000, "Paused")` → `duration_ms=1_743_000`, `position_ms=5_250`, `is_playing=false`.
- [ ] **Step 2: Jalankan** `cargo test -p quran-widget snapshot_from_mpris`; hasil: FAIL.
- [ ] **Step 3: Implementasikan** fungsi itu, `LinuxSource` (crate `mpris`, cari player berawalan `spotify`), dan `default_source`.
- [ ] **Step 4: Jalankan test**; hasil: PASS.
- [ ] **Step 5: Ambil judul asli.** `dump_titles` memutar-baca: pengguna memutar 37 track Juz 30 satu per satu di Spotify; binary mencatat `title` dan `duration_ms` setiap track baru ke `tests/fixtures/spotify_titles.json` (`[{"title":…,"duration_ms":…}]`). Hasil: 37 entri. **Tanpa fixture ini Task 4 tidak bisa dikerjakan.**

### Task 4: Pencocok judul → nomor surah

**Files:**
- Create: `src-tauri/src/surah.rs`
- Test: `src-tauri/src/surah.rs` (membaca `tests/fixtures/spotify_titles.json`)

**Interfaces:**
- Consumes: fixture Task 3.
- Produces: `pub fn match_surah(title: &str) -> Option<u8>` — nomor 78..=114 atau `None`.

- [ ] **Step 1: Test gagal:** (a) untuk setiap entri fixture, urutan entri ke-i harus menghasilkan surah `78 + i` (asumsikan fixture diurut sesuai mushaf; jika tidak, pasangkan manual di fixture dengan field `surah`); (b) `match_surah("Bohemian Rhapsody") == None`; (c) `match_surah("") == None`; (d) judul surah di luar Juz 30 (mis. "Al-Baqarah") → `None`.
- [ ] **Step 2: Jalankan** `cargo test -p quran-widget surah`; hasil: FAIL.
- [ ] **Step 3: Implementasikan** normalisasi (huruf kecil, buang tanda baca/diakritik, variasi "Al"/"Al-"/"Surah") lalu cocokkan ke tabel nama yang dibangun dari fixture asli (bukan tebakan ejaan).
- [ ] **Step 4: Jalankan test**; hasil: PASS.

### Task 5: Inti waktu: Timeline, PositionClock, Engine

**Files:**
- Create: `src-tauri/src/timeline.rs`, `clock.rs`, `engine.rs`
- Test: modul `#[cfg(test)]` di tiap file

**Interfaces:**
- Consumes: `Snapshot`, `match_surah`.
- Produces:
  ```rust
  // timeline.rs
  #[derive(Clone, Copy, PartialEq, Debug)] pub enum Part { Intro, Ayah(u16) }
  #[derive(Deserialize)] pub struct Timeline { pub surah: u8, pub spotify_duration_ms: u64, pub offset_ms: i64, pub ayah_starts_ms: Vec<u64> }
  impl Timeline {
      /// offset_ms = waktu Spotify − waktu rekaman referensi untuk suara yang sama.
      pub fn part_at(&self, spotify_position_ms: u64) -> Part;
      pub fn matches_duration(&self, spotify_duration_ms: u64) -> bool; // toleransi ±1500 ms
  }
  // clock.rs
  pub struct PositionClock;
  impl PositionClock {
      pub fn new() -> Self;
      pub fn observe(&mut self, s: &Snapshot, now: Instant);
      pub fn position_ms(&self, now: Instant) -> u64; // maju jika playing, beku jika dijeda
  }
  // engine.rs
  #[derive(PartialEq, Debug, Serialize)] pub enum WidgetState { Waiting, Showing { surah: u8, part: Part, playing: bool } }
  pub struct Engine;
  impl Engine {
      pub fn new(timelines: Vec<Timeline>) -> Self;
      pub fn tick(&mut self, snap: Option<Snapshot>, now: Instant) -> WidgetState;
  }
  ```
  `Part` diserialisasi ke bentuk TypeScript Task 1 (`{kind:"intro"}` / `{kind:"ayah",ayah}`) dengan `#[serde(tag="kind", rename_all="lowercase")]`.

- [ ] **Step 1: Test gagal `timeline`:** dengan `ayah_starts_ms=[2000,10000,20000]`, `offset_ms=500`: `part_at(2499)==Intro`, `part_at(2500)==Ayah(1)`, `part_at(10499)==Ayah(1)`, `part_at(10500)==Ayah(2)`, `part_at(9_999_999)==Ayah(3)` (lewat akhir tetap ayat terakhir), `part_at(0)==Intro`.
- [ ] **Step 2: Test gagal `clock`:** setelah `observe(playing, 1000ms, t0)`, `position_ms(t0+400ms)==1400`; dengan `is_playing=false`, `position_ms(t0+400ms)==1000`.
- [ ] **Step 3: Test gagal `engine`** (pakai Timeline sintetis surah 112, `spotify_duration_ms=30000`):
  - snapshot `None` → `Waiting` (Review Focus 3)
  - judul tak dikenali / "Bohemian Rhapsody" / durasi 200 detik → `Waiting` (Review Focus 1, 2)
  - judul surah 112, durasi cocok, posisi sesuai ayat 2, playing → `Showing{112, Ayah(2), true}`; dijeda → sama, `playing=false`
  - posisi turun ke 0 (track diulang) → `Showing{…, Intro, …}` (Review Focus 4)
  - dua `tick` berturut-turut dengan surah 112 lalu surah 113 → hasil kedua `surah==113`, ayat dihitung dari timeline 113 (Review Focus 5)
- [ ] **Step 4: Jalankan** `cargo test -p quran-widget` (modul timeline, clock, engine); hasil: FAIL.
- [ ] **Step 5: Implementasikan** `Timeline::part_at` (`ref = pos − offset_ms`; `ref < ayah_starts_ms[0]` → `Intro`, selain itu ayat terakhir dengan `start ≤ ref`), `PositionClock`, dan `Engine::tick` (tolak jika `match_surah` kosong, timeline tak ada, atau `!matches_duration`). Aturan re-anchor jam (hindari lompatan karena posisi sumber kasar): re-anchor hanya jika posisi teramati berbeda dari prediksi > 150 ms atau status play berubah; nilai ini disetel di Task 10.
- [ ] **Step 6: Jalankan test**; hasil: PASS semua.

### Task 6: Alat penyelarasan: waktu mulai ayat dari rekaman referensi

**Files:**
- Create: `tools/align.py`, `tools/spotcheck.py`
- Test: `tools/test_align.py`

**Interfaces:**
- Produces: `align_surah(mp3_path: str, ayat_ar: list[str]) -> list[int]` — waktu mulai (ms, waktu rekaman referensi) tiap ayat; panjang = jumlah ayat.

- [ ] **Step 1: Test gagal:** unduh `https://server11.mp3quran.net/yasser/112.mp3` ke direktori baru (data tak tepercaya; jalankan Python dengan `-I`); `align_surah` pada Al-Ikhlas mengembalikan 4 nilai, naik ketat, nilai pertama ≥ 0 dan terakhir < durasi file (`ffprobe`).
- [ ] **Step 2: Jalankan** `python3 -I -m pytest tools/test_align.py`; hasil: FAIL.
- [ ] **Step 3: Implementasikan** dengan alat *forced alignment* pilihan Anda yang mendukung bahasa Arab (mis. CTC forced aligner berbasis MMS atau Whisper dengan timestamp kata); jika dua alat perlu dibandingkan, pilih yang lebih konsisten pada tiga surah pertama.
- [ ] **Step 4: Jalankan test**; hasil: PASS.
- [ ] **Step 5: `spotcheck.py <surah>`** mengekspor klip 1 detik sebelum dan sesudah tiga batas ayat acak. Gerbang: pengguna mendengarkan klip surah 112, 108, 114; batas harus terdengar tepat. Jika meleset, perbaiki `align.py` sebelum Task 8. Untuk surah lain cukup pemeriksaan otomatis di Task 8.

### Task 7: Alat pengukur geseran Spotify

**Files:**
- Create: `tools/offset.py`
- Test: `tools/test_offset.py`

**Interfaces:**
- Produces:
  - `estimate_offset(reference: np.ndarray, captured: np.ndarray, sr: int) -> float` — geseran dalam ms (positif = `captured` terlambat), via korelasi silang.
  - CLI `python3 -I tools/offset.py <surah>` — memutar potongan ~20 detik di awal, tiga titik tengah, dan akhir track di Spotify (seek lewat MPRIS), menangkap keluaran suara (monitor PulseAudio/PipeWire), dan menulis `offsets_ms: [...]` ke `build/offsets/NNN.json`.

- [ ] **Step 1: Test gagal:** sinyal sintetis noise 10 detik @16 kHz, `captured` = `reference` digeser 2500 ms (diawali hening); `estimate_offset` harus mengembalikan 2500 ± 10 ms; geser negatif juga diuji (−300 ms).
- [ ] **Step 2: Jalankan** `python3 -I -m pytest tools/test_offset.py`; hasil: FAIL.
- [ ] **Step 3: Implementasikan** `estimate_offset` dan CLI. Aturan: median dari titik-titik ukur menjadi `offset_ms`; jika sebaran (maks−min) > 100 ms, track ditandai `"constant": false` dan **dikecualikan** dari versi pertama (widget menampilkan `Waiting`). Hanya angka yang disimpan, bukan audio tangkapan.
- [ ] **Step 4: Jalankan test**; hasil: PASS.

### Task 8: Bangun dan validasi data waktu 37 surah

**Files:**
- Create: `tools/build_timing.py`, `data/timing/078.json … 114.json`, `tools/check_timing.py`
- Test: `tools/check_timing.py`

**Interfaces:**
- Consumes: `align_surah`, `build/offsets/NNN.json`, `tests/fixtures/spotify_titles.json` (durasi Spotify), `juz30.json`.
- Produces: tiap `data/timing/NNN.json` cocok dengan `Timeline` (Task 5): `{"surah":112,"spotify_duration_ms":…,"offset_ms":…,"ayah_starts_ms":[…]}`.

- [ ] **Step 1: Tulis `check_timing.py`** dengan asersi per file: jumlah `ayah_starts_ms` = jumlah ayat di `juz30.json`; naik ketat; jarak antar ayat ≥ 500 ms; nilai terakhir + `offset_ms` < `spotify_duration_ms`; ada 37 file kecuali surah yang dikecualikan (daftar `EXCLUDED` ditulis eksplisit beserta alasannya).
- [ ] **Step 2: Jalankan** `python3 -I tools/check_timing.py`; hasil: FAIL (file belum ada).
- [ ] **Step 3: Implementasikan `build_timing.py`** (jalankan Task 6 dan 7 per surah, tulis JSON).
- [ ] **Step 4: Jalankan ulang**; hasil: `OK`, dan cetak daftar surah yang dikecualikan (jika ada) untuk dilaporkan ke pengguna.

### Task 9: Rangkai runtime Tauri

**Files:**
- Modify: `src-tauri/src/lib.rs`, `src/main.ts`
- Create: `src-tauri/src/runtime.rs`
- Test: `src-tauri/src/runtime.rs` (dengan `PlaybackSource` palsu)

**Interfaces:**
- Consumes: `default_source`, `Engine`, `data/timing/*.json` (dibundel via `include_dir!` atau resource Tauri).
- Produces: `pub fn run_loop(source: Box<dyn PlaybackSource>, engine: Engine, mut emit: impl FnMut(WidgetState), clock: impl Fn() -> Instant, stop: Arc<AtomicBool>)` — poll sumber tiap 250 ms, `tick` tiap 50 ms, `emit` hanya saat state berubah. Backend memanggil `window.emit("state", state)`; `main.ts` memanggil `listen("state", …)` dan `viewModel`.

- [ ] **Step 1: Test gagal:** `PlaybackSource` palsu yang mengembalikan urutan: surah 112 posisi 0 → posisi di ayat 2 → `Ok(None)` → `Err(Unavailable)`; hasil `emit` harus `[Showing Intro, Showing Ayah(2), Waiting]` (error sumber diperlakukan seperti `None`, loop tidak panic, tidak ada emit duplikat).
- [ ] **Step 2: Jalankan** `cargo test -p quran-widget runtime`; hasil: FAIL.
- [ ] **Step 3: Implementasikan `run_loop`** dan sambungkan di `lib.rs` (thread terpisah, hentikan saat jendela ditutup).
- [ ] **Step 4: Jalankan test**; hasil: PASS.
- [ ] **Step 5: Verifikasi manual di Linux:** `npm run tauri dev`, putar Al-Ikhlas di Spotify; ayat berganti, jeda membekukan, ganti surah mengganti tampilan, geser posisi meloncat.

### Task 10: Sumber macOS dan Windows serta pengukuran akurasi

**Files:**
- Create: `src-tauri/src/playback/macos.rs`, `windows.rs`, `src-tauri/src/bin/accuracy_probe.rs`
- Modify: `src-tauri/src/playback/mod.rs` (cfg `default_source`)
- Test: pemetaan murni di tiap file; `accuracy_probe` untuk pengukuran di mesin asli

**Interfaces:**
- Produces: `MacSource` (AppleScript: `tell application "Spotify"` → `name of current track`, `duration of current track`, `player position`, `player state`) dan `WindowsSource` (SMTC `GlobalSystemMediaTransportControlsSessionManager`, sesi Spotify), keduanya `PlaybackSource`. `accuracy_probe` memutar-baca selama 60 detik, membandingkan posisi interpolasi `PositionClock` dengan posisi teramati, dan mencetak penyimpangan maksimum (ms). Konstanta `LOW_ACCURACY: bool` per OS di `playback/mod.rs`.

- [ ] **Step 1: Test gagal** untuk pemetaan murni: macOS `parse_applescript("Surah X|1743000|5.25|playing")` → `Snapshot{duration_ms=1_743_000, position_ms=5_250, is_playing=true}` (AppleScript memberi durasi dalam ms dan posisi dalam detik); Windows memetakan `TimeSpan` 100-ns ke ms.
- [ ] **Step 2: Jalankan** `cargo test -p quran-widget parse_` di tiap OS (macOS/Windows memerlukan mesin tersebut atau CI); hasil: FAIL.
- [ ] **Step 3: Implementasikan** kedua sumber.
- [ ] **Step 4: Jalankan test**; hasil: PASS.
- [ ] **Step 5: Ukur akurasi** `cargo run --bin accuracy_probe` di Windows, macOS, dan Linux saat Spotify memutar. Jika penyimpangan maksimum > 300 ms di sebuah OS: setel `LOW_ACCURACY=true` untuk OS itu (penyetelan aturan re-anchor Task 5 dulu), dan tampilkan keterangan "akurasi lebih rendah" kecil di footer lewat field `low_accuracy` pada event `state`. Catat angka hasil ukur di `docs/accuracy.md`.

### Task 11: Pengemasan dan lisensi

**Files:**
- Create: `.github/workflows/release.yml`, `THIRD-PARTY-NOTICES.md`
- Modify: `src-tauri/tauri.conf.json` (identitas aplikasi, ikon)

- [ ] **Step 1: Susun `THIRD-PARTY-NOTICES.md`** dari `LICENSE-NOTES.md` (Task 2) dan lisensi huruf serta crate.
- [ ] **Step 2: Workflow rilis** `tauri-apps/tauri-action` membangun Windows, macOS, dan Linux.
- [ ] **Step 3: Verifikasi:** `npm run tauri build` di Linux menghasilkan paket yang bisa dipasang dan dijalankan; paket Windows/macOS dari CI berjalan di mesin masing-masing (cek sekali secara manual).

---

## Self-Review

- **Cakupan spec:** tampilan C, terjemah, perilaku (jeda/ganti surah/seek/waiting/intro), tanpa login, tiga OS, Juz 30, akurasi 300 ms + fallback "akurasi lebih rendah", audio tak disimpan, pembuatan tabel waktu + geseran, lisensi → Task 1, 2, 5, 9, 3, 10, 6–8, 11. Risiko "judul track belum dilihat" ditangani Task 3 → 4; "geseran konstan?" Task 7; "durasi berubah" Task 5.
- **Konsistensi tipe:** `Snapshot`, `Part`, `WidgetState`, `Timeline` konsisten antar Task 3, 5, 9; bentuk JSON `Part` sama dengan tipe TypeScript Task 1. Field `low_accuracy` (Task 10) ditambahkan ke event `state` dan ke tipe `WidgetState` TypeScript pada saat itu.
- **Proporsi:** plan sekitar 3× panjang spec; sebagian besar isinya tanda tangan, nama test, dan nilai dari spec; hanya pengurutan dan aturan yang tidak ditentukan tanda tangan yang ditulis.
