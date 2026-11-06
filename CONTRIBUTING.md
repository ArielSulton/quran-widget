# Contributing to Simak / Berkontribusi ke Simak

[English](#english) | [Bahasa Indonesia](#bahasa-indonesia)

## English

Thank you for your interest in Simak. Anyone can contribute through a fork and a pull request. This project follows the [Code of Conduct](CODE_OF_CONDUCT.md). Please read the [README](README.md) first, especially the "Limitations" section, so you know what is already known.

### How to contribute

1. **Fork** the repository https://github.com/ArielSulton/quran-widget on GitHub.
2. **Clone** your fork and add the original repository as `upstream`.
3. Create a **branch** from the latest `main`. Name it `feat/short-name` for a feature or `fix/short-name` for a bug fix (also `docs/...`, `chore/...`, `test/...` when it fits).
4. Make **small, focused** changes. One pull request should do one thing.
5. Write **conventional commit** messages, with the subject at 72 characters or fewer: `feat(ui): add keyboard shortcut for reset`, `fix(linux): handle missing MPRIS player`, `docs: fix typo in README`. Common types: `feat`, `fix`, `docs`, `test`, `refactor`, `chore`, `ci`.
6. Keep your branch **rebased on `main`** (`git fetch upstream && git rebase upstream/main`) so the history stays clean.
7. Run the checks below, then open a **pull request** to `main` and fill in the template.

Pull request checklist:

- [ ] The change is small and focused, and the description says what and why.
- [ ] All the checks in "Run the checks" pass locally.
- [ ] New behavior has a test, or the description explains why not.
- [ ] Docs are updated (README, comments) when behavior changes.
- [ ] Platforms you tested on are listed. Untested platforms are said to be untested.
- [ ] Data or licensing impact is stated (see "Rules for data changes").
- [ ] The change does not record, store or redistribute Spotify audio.

If you are not sure whether an idea fits, open an issue first and ask. Please do this before you spend a lot of time on a big change.

### Development setup

Requirements:

- Node.js 22.
- Rust toolchain 1.88 or newer (install with `rustup`). CI uses a pinned newer Rust; Rust 1.88 is the minimum the code supports. If you use 1.88 and update `Cargo.lock`, set `CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=fallback`.
- Linux system packages (Debian/Ubuntu), the same as in the CI workflows:

```bash
sudo apt install build-essential pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev libdbus-1-dev patchelf
```

Then:

```bash
npm install
npm run tauri dev      # run the app in development mode
```

You can also preview the UI in a browser without Tauri: `npm run dev` (see the README for the preview parameters).

### Run the checks

CI runs these on every pull request. Run them locally before you push:

```bash
# Frontend (from the repository root)
npm ci
npx vitest run
npx tsc --noEmit

# Rust (from src-tauri/; run `npm run build` once at the root first,
# because tauri-build checks the dist folder)
cd src-tauri
cargo test --locked -p quran-widget
cargo test --locked -p quran-widget --examples
cargo clippy --locked --all-targets -- -D warnings
```

CI also compiles the macOS and Windows playback sources on their own OS (`cross-os-check`). You do not need those machines to open a pull request.

Python tools (`tools/`) are **not** run in CI. They need extra packages, models and network:

- Install the requirements into a local virtualenv: see `tools/requirements.txt` (large: PyTorch CPU, faster-whisper, and so on; `ffmpeg` must be on your PATH).
- Run the unit tests with `python -m pytest tools -m 'not slow'`. Tests marked `slow` run a real Whisper model on a real recording and need a model download (network); they are skipped by default.
- `python tools/check_timing.py` validates the shipped timing files and only needs Python. Run it whenever you touch `data/timing/`.

### Project layout

```
src/                       UI (TypeScript + Vite): view, theme, sync shift, window controls
src-tauri/src/playback/    playback sources: MPRIS (Linux), osascript (macOS), SMTC (Windows)
src-tauri/src/runtime.rs   loop that polls the playback source, interpolates, and loads timelines
src-tauri/src/engine.rs    widget state machine: combines snapshot, position clock and timeline
src-tauri/src/timeline.rs  ayah timeline: maps a Spotify position to a part of the surah
src-tauri/src/surah.rs     track title to surah matching
src-tauri/src/clock.rs     position clock: smooths the coarse position from the source
src-tauri/examples/        developer tools (accuracy_probe, dump_titles), not shipped
src-tauri/tests/           Rust tests, including validation of shipped timing
data/quran/                Arabic text and translation of Juz 30 + license notes
data/timing/               ayah timing tables per surah and _summary.json
tools/                     Python tools to build and check the timing data
scripts/                   helper scripts (icon, dependency licenses, layout sweep)
docs/                      images, specs and plans
.github/                   CI workflows, issue and pull request templates
```

### Contributions we especially welcome

- Testing on macOS, Windows and Wayland, with a [platform test report](https://github.com/ArielSulton/quran-widget/issues/new?template=platform_test_report.yml) (see "Help wanted" in the README).
- Listening checks of flagged ayah boundaries, with notes.
- A freely licensed Indonesian translation (see [data/README.md](data/README.md)).
- Bug fixes, tests, accessibility and documentation improvements.
- Other reciters and other juz, only with data that has a compatible license and a plan for how timing is produced without recording Spotify audio.

### What will NOT be accepted

- Anything that **records, stores or redistributes Spotify audio**, or that circumvents Spotify DRM or other protection.
- **Copyrighted text or audio** without a compatible license, and without the license evidence in the pull request.
- Spotify **trademarks or logos** (Simak is not affiliated with Spotify).
- Code that uses the **Spotify Web API or SDK**, unless there was a design discussion in an issue first and it was agreed. Today Simak needs no login and no network, and that is on purpose.
- Changes that add tracking, telemetry or network calls without a prior discussion.

### Rules for data changes

- **Timing changes need evidence.** A change to `data/timing/*.json` must come with listening evidence in the pull request: the surah, the ayah, the Spotify time you heard, and how you checked. Please say which Spotify track and recording you listened to.
- **Run `python tools/check_timing.py`** and make sure it prints `OK`. The Rust test `cargo test -p quran-widget` also checks the shipped files.
- **Say so when you edit generated files by hand.** `data/timing/*.json` and `_summary.json` were generated with local tools that are not published. If you edit them by hand, write it in the pull request, and keep `_summary.json` consistent with the files.
- **Do not change the Arabic text** in `data/quran/juz30.json`. It must stay verbatim from the Tanzil Project.
- **New data must state its license and source** in the pull request and in a notice file next to the data. See [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md) and [data/quran/NOTICE.md](data/quran/NOTICE.md) for the format.

### Licensing of your contributions

This project uses "inbound = outbound". By submitting a pull request, you agree that your contribution to the **code** is licensed under the [MIT license](LICENSE) of this repository, and that you have the right to submit it. **Data** contributions (text, translation, timing, images, fonts) must state their own license and source; they are not automatically MIT. Read [data/README.md](data/README.md) for how data is licensed here.

### Security

Do not report security problems in a public issue. Follow [SECURITY.md](SECURITY.md).

### Code of Conduct

Everyone who takes part in this project must follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Bahasa Indonesia

Terima kasih sudah tertarik pada Simak. Siapa pun bisa berkontribusi lewat fork dan pull request. Proyek ini mengikuti [Code of Conduct](CODE_OF_CONDUCT.md). Mohon baca [README](README.md) lebih dulu, terutama bagian "Batasan", agar Anda tahu apa yang sudah diketahui.

### Cara berkontribusi

1. **Fork** repositori https://github.com/ArielSulton/quran-widget di GitHub.
2. **Clone** fork Anda dan tambahkan repositori asli sebagai `upstream`.
3. Buat **branch** dari `main` terbaru. Beri nama `feat/nama-singkat` untuk fitur atau `fix/nama-singkat` untuk perbaikan bug (juga `docs/...`, `chore/...`, `test/...` bila cocok).
4. Buat perubahan yang **kecil dan terfokus**. Satu pull request mengerjakan satu hal.
5. Tulis pesan **conventional commit**, dengan subjek paling banyak 72 karakter: `feat(ui): add keyboard shortcut for reset`, `fix(linux): handle missing MPRIS player`, `docs: fix typo in README`. Tipe umum: `feat`, `fix`, `docs`, `test`, `refactor`, `chore`, `ci`.
6. Jaga branch Anda tetap **di-rebase ke `main`** (`git fetch upstream && git rebase upstream/main`) agar riwayatnya rapi.
7. Jalankan pemeriksaan di bawah, lalu buka **pull request** ke `main` dan isi template-nya.

Checklist pull request:

- [ ] Perubahannya kecil dan terfokus, dan deskripsinya menjelaskan apa dan mengapa.
- [ ] Semua pemeriksaan di "Menjalankan pemeriksaan" lulus secara lokal.
- [ ] Perilaku baru punya test, atau deskripsi menjelaskan mengapa tidak.
- [ ] Dokumentasi diperbarui (README, komentar) bila perilaku berubah.
- [ ] Platform yang Anda uji dicantumkan. Platform yang belum diuji disebut belum diuji.
- [ ] Dampak pada data atau lisensi dinyatakan (lihat "Aturan mengubah data").
- [ ] Perubahan tidak merekam, menyimpan, atau membagikan audio Spotify.

Jika Anda ragu apakah sebuah ide cocok, buka issue dulu dan tanyakan. Lakukan ini sebelum Anda menghabiskan banyak waktu pada perubahan besar.

### Menyiapkan lingkungan pengembangan

Prasyarat:

- Node.js 22.
- Toolchain Rust 1.88 atau lebih baru (pasang dengan `rustup`). CI memakai Rust yang lebih baru dan dipatok; Rust 1.88 adalah minimum yang didukung kode. Jika Anda memakai 1.88 dan memperbarui `Cargo.lock`, setel `CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=fallback`.
- Paket sistem Linux (Debian/Ubuntu), sama seperti di workflow CI:

```bash
sudo apt install build-essential pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev libdbus-1-dev patchelf
```

Lalu:

```bash
npm install
npm run tauri dev      # jalankan aplikasi dalam mode pengembangan
```

Anda juga bisa melihat pratinjau antarmuka di browser tanpa Tauri: `npm run dev` (lihat README untuk parameter pratinjau).

### Menjalankan pemeriksaan

CI menjalankan ini pada setiap pull request. Jalankan secara lokal sebelum push:

```bash
# Frontend (dari akar repositori)
npm ci
npx vitest run
npx tsc --noEmit

# Rust (dari src-tauri/; jalankan `npm run build` sekali di akar lebih dulu,
# karena tauri-build memeriksa folder dist)
cd src-tauri
cargo test --locked -p quran-widget
cargo test --locked -p quran-widget --examples
cargo clippy --locked --all-targets -- -D warnings
```

CI juga mengompilasi sumber pemutaran macOS dan Windows di OS masing-masing (`cross-os-check`). Anda tidak butuh mesin itu untuk membuka pull request.

Alat Python (`tools/`) **tidak** dijalankan di CI. Alat itu butuh paket tambahan, model, dan jaringan:

- Pasang kebutuhannya ke virtualenv lokal: lihat `tools/requirements.txt` (besar: PyTorch CPU, faster-whisper, dan lain-lain; `ffmpeg` harus ada di PATH).
- Jalankan unit test dengan `python -m pytest tools -m 'not slow'`. Test bertanda `slow` menjalankan model Whisper sungguhan pada rekaman sungguhan dan butuh unduhan model (jaringan); test itu dilewati secara bawaan.
- `python tools/check_timing.py` memvalidasi berkas waktu yang disertakan dan hanya butuh Python. Jalankan setiap kali Anda menyentuh `data/timing/`.

### Struktur proyek

```
src/                       antarmuka (TypeScript + Vite): tampilan, tema, geser sinkron, kontrol jendela
src-tauri/src/playback/    sumber pemutaran: MPRIS (Linux), osascript (macOS), SMTC (Windows)
src-tauri/src/runtime.rs   loop yang mem-polling sumber pemutaran, interpolasi, dan memuat timeline
src-tauri/src/engine.rs    state machine widget: menggabungkan snapshot, jam posisi, dan timeline
src-tauri/src/timeline.rs  timeline ayat: memetakan posisi Spotify ke bagian surah
src-tauri/src/surah.rs     pencocokan judul trek ke surah
src-tauri/src/clock.rs     jam posisi: menghaluskan posisi kasar dari sumber pemutaran
src-tauri/examples/        alat pengembang (accuracy_probe, dump_titles), tidak dikirim
src-tauri/tests/           test Rust, termasuk validasi data waktu yang disertakan
data/quran/                teks Arab dan terjemahan Juz 30 + catatan lisensi
data/timing/               tabel waktu ayat per surah dan _summary.json
tools/                     alat Python untuk membangun dan memeriksa data waktu
scripts/                   skrip bantu (ikon, lisensi dependensi, sapuan tata letak)
docs/                      gambar, spesifikasi, dan rencana
.github/                   workflow CI, template issue dan pull request
```

### Kontribusi yang sangat kami sambut

- Testing di macOS, Windows, dan Wayland, dengan [platform test report](https://github.com/ArielSulton/quran-widget/issues/new?template=platform_test_report.yml) (lihat "Bantuan dibutuhkan" di README).
- Pengecekan dengan mendengarkan pada batas ayat yang ditandai, beserta catatannya.
- Terjemahan Indonesia berlisensi bebas (lihat [data/README.md](data/README.md)).
- Perbaikan bug, test, aksesibilitas, dan perbaikan dokumentasi.
- Qari lain dan juz lain, hanya dengan data berlisensi kompatibel dan rencana bagaimana waktu dihasilkan tanpa merekam audio Spotify.

### Yang TIDAK akan diterima

- Apa pun yang **merekam, menyimpan, atau membagikan audio Spotify**, atau yang menghindari DRM atau perlindungan lain milik Spotify.
- **Teks atau audio berhak cipta** tanpa lisensi yang kompatibel, dan tanpa bukti lisensi di pull request.
- **Merek dagang atau logo** Spotify (Simak tidak berafiliasi dengan Spotify).
- Kode yang memakai **Spotify Web API atau SDK**, kecuali sudah ada diskusi desain di sebuah issue lebih dulu dan disetujui. Saat ini Simak tidak butuh login dan tidak butuh jaringan, dan itu disengaja.
- Perubahan yang menambah tracking, telemetry, atau panggilan jaringan tanpa diskusi lebih dulu.

### Aturan mengubah data

- **Perubahan waktu butuh bukti.** Perubahan pada `data/timing/*.json` harus disertai bukti hasil mendengarkan di pull request: surah, ayat, waktu Spotify yang Anda dengar, dan cara Anda memeriksanya. Sebutkan trek dan rekaman Spotify mana yang Anda dengarkan.
- **Jalankan `python tools/check_timing.py`** dan pastikan hasilnya `OK`. Test Rust `cargo test -p quran-widget` juga memeriksa berkas yang disertakan.
- **Sebutkan jika Anda mengedit berkas hasil generate dengan tangan.** `data/timing/*.json` dan `_summary.json` dihasilkan dengan tools lokal yang tidak dipublikasikan. Jika Anda mengeditnya dengan tangan, tulis di pull request, dan jaga `_summary.json` tetap konsisten dengan berkasnya.
- **Jangan ubah teks Arab** di `data/quran/juz30.json`. Teks harus tetap persis dari Tanzil Project.
- **Data baru harus menyebut lisensi dan sumbernya** di pull request dan di berkas notice di samping datanya. Lihat [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md) dan [data/quran/NOTICE.md](data/quran/NOTICE.md) untuk formatnya.

### Lisensi kontribusi Anda

Proyek ini memakai "inbound = outbound". Dengan mengirim pull request, Anda setuju bahwa kontribusi Anda pada **kode** dilisensikan di bawah [lisensi MIT](LICENSE) repositori ini, dan bahwa Anda berhak mengirimkannya. Kontribusi **data** (teks, terjemahan, waktu, gambar, font) harus menyebut lisensi dan sumbernya sendiri; tidak otomatis MIT. Baca [data/README.md](data/README.md) untuk cara data dilisensikan di sini.

### Keamanan

Jangan laporkan masalah keamanan di issue publik. Ikuti [SECURITY.md](SECURITY.md).

### Code of Conduct

Semua yang ikut serta dalam proyek ini harus mengikuti [Code of Conduct](CODE_OF_CONDUCT.md).
