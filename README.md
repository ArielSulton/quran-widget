# Simak

[![CI](https://github.com/ArielSulton/quran-widget/actions/workflows/ci.yml/badge.svg)](https://github.com/ArielSulton/quran-widget/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/code%20license-MIT-blue.svg)](LICENSE)

**English** | [Bahasa Indonesia](#bahasa-indonesia)

## English

**Simak** (from the Indonesian word "simak": to listen with full attention) is a small desktop widget. It shows the Quranic ayah, with its Indonesian translation, that is being recited on a track in the Spotify app. Search name: *Simak Ayat*. Tagline: *Ayat yang sedang dibacakan* (the ayah being recited).

> **Status: pre-release.** Installers for Linux, macOS and Windows are on the [Releases](https://github.com/ArielSulton/quran-widget/releases) page. Only Linux is verified. The macOS and Windows builds are experimental and not code-signed, so your OS will show a warning the first time you open them (see below).

### Screenshots

| Light | Dark, with controls |
|---|---|
| ![Simak, light theme](docs/images/simak-terang.png) | ![Simak, dark theme with the theme and sync shift controls](docs/images/simak-gelap-kontrol.png) |

The images above come from the development preview mode in a browser (with a colorful background so the glass effect is visible). They are not screenshots of the real app. The images are kept in `docs/images/`.

### How it works

1. Simak reads the track that is playing in the Spotify desktop app through the operating system's built-in media controls (MPRIS on Linux, AppleScript/osascript on macOS, SMTC on Windows; see "Limitations" for the verification status).
2. The track title is matched to a surah name.
3. The playback position is looked up in a table of ayah times that was computed beforehand (`data/timing/`), so the ayah changes when the reciter moves to the next ayah. The table is included in the repository: 35 surahs of Juz 30.
4. The widget shows the Arabic text and the Indonesian translation by Kementerian Agama RI.

Simak does not use the Spotify API or SDK, does not ask you to log in, and does not play or store any audio.

### Install

Download the file for your system from [Releases](https://github.com/ArielSulton/quran-widget/releases). You need the **Spotify desktop app**, signed in on the same computer. Simak is read-only: it never logs in to Spotify.

| System | File | Notes |
|---|---|---|
| Linux | `.deb` (Debian, Ubuntu) or `.AppImage` | AppImage: `chmod +x Simak_*.AppImage` then run it. Verified. |
| macOS | `.dmg` (universal: Apple silicon and Intel) | Experimental. Drag Simak to Applications. Gatekeeper blocks unsigned apps: right-click Simak, choose Open, or allow it in System Settings > Privacy & Security ("Open Anyway"). macOS asks permission for Simak to control Spotify: allow it. |
| Windows | `-setup.exe` or `.msi` | Experimental. SmartScreen shows "Windows protected your PC": More info > Run anyway. |

Check a download with the `SHA256SUMS` file on the release page (`sha256sum -c SHA256SUMS` on Linux, `shasum -a 256 -c SHA256SUMS` on macOS).

### Usage

1. Start Simak.
2. In the **desktop** Spotify app, play a Juz 30 surah track by Yasser Al-Dosari.
3. The widget follows: the ayah appears and changes with the recitation. When paused, the widget stays on the last ayah. When nothing matches, it shows "Menunggu bacaan Al-Qur'an" (waiting for a recitation).

Point the cursor at the widget (or focus it with the keyboard) to show the controls:

- **Theme:** Otomatis (follows the system), Terang (light) or Gelap (dark). The choice is saved.
- **Geser sinkron (sync shift):** shifts the ayah timing relative to the sound, from -2.0 to +2.0 seconds. The `-` / `+` buttons step by 0.1 second; hold **Shift** for a 1 second step. From the keyboard: `[` decreases and `]` increases (also with Shift). Double-click the value (or press Enter) to go back to 0. Increase the value if the ayah appears earlier than the sound, decrease it if it is late. The choice is saved.
- **Resize:** a small handle (three diagonal lines) at the bottom right corner of the card appears together with the controls. Drag it to resize the window. From the keyboard: focus the handle with Tab, then the arrow keys resize by 10 px (with **Shift**: 50 px). The minimum size is 430 x 270 (enforced by the window); the default is 520 x 320. The upper limit of 2000 x 1200 applies only to keyboard resize and to the restored size, not to dragging with the mouse. Both are also limited to the monitor size (minus a margin of 80 x 120 px for panels).
- **Saved window size:** the window size is remembered and restored the next time Simak opens. A size larger than the current monitor is shrunk to fit. The window position is not saved.
- **Font scales with the window:** the larger the window, the larger the Arabic text (up to 1.6 times at about 960 x 560), the translation and the footer, in proportion. In small windows with long ayahs, the Arabic font shrinks gradually so the ayah still fits without being cut off.

The sync shift is needed because the playback position that Spotify reports can differ from the audio you hear (see below).

#### How to tune the sync

1. Play one surah and watch when the reciter moves to the next ayah.
2. If the next ayah appears **earlier** than the sound, press `]` (increase). If it is **late**, press `[` (decrease). Hold Shift for a 1 second step, release Shift for the fine 0.1 second step.
3. Repeat at a few ayah changes until it fits. The value is saved; double-click the value to go back to 0.

### Limitations (honest)

- **Juz 30 only in v1.** Other surahs are not available yet.
- **35 of 37 surahs of Juz 30.** Surah 101 (Al-Qari'ah) and 104 (Al-Humazah) are excluded because their Spotify tracks are different recordings from the reference recording, so the timing table does not apply to them.
- **Timing accuracy is not uniform.** `data/timing/_summary.json` records the confidence per surah (17 high, 10 medium, 8 low) and the ayah boundaries that are flagged to be listened to again. Aligning the Spotify track to the reference recording carries an uncertainty of about ±1 second on every default offset (all surahs, not only some), and one case (surah 78, start of the track) has not been confirmed by listening.
- **Sync is not perfect.** The position that Spotify reports can drift about ±1 second from the audio on some systems, and the ±1 second uncertainty applies to every default offset, not only some surahs. Use the sync shift to adjust. The sync shift value is global per device (one value for all surahs), so tuning it for one surah can shift another surah.
- **Known issue: four surahs (086, 093, 100, 107) are about 1 second away from their class peers.** The default offsets of these four surahs measure 0.8 to 1.0 second different from the mode of their class (086, 093, 100 on long tracks; 107 on short tracks) and the data is left as measured; it has not been confirmed by listening which is right (their clips are on the listening priority list in `tools/spotcheck.py --flagged`). If the ayahs of these surahs feel early or late, correct it with the sync shift.
- **Only Linux is verified.** The macOS (osascript) and Windows (SMTC) playback sources are implemented, but not verified on real machines: they are only proven to compile and pass the pure tests in CI. Linux was verified on one X11/GNOME machine only; Wayland is untested. So the macOS/Windows builds are experimental. They are in the release, but unsigned: macOS Gatekeeper and Windows SmartScreen warn on first launch.
- **Background blur is not verified.** System-level blur is designed for macOS only (untested). On Linux there is no blur behind the window, and on Windows it is not verified; without blur, the card uses a more solid fill so the text stays readable over any wallpaper, so it does not look as glass-like as in the images above.
- **Minimum window size 430 x 270.** Anything smaller is not supported: long ayahs no longer fit without a font that is too small, or the text touches the control pill. At the minimum, the default and two larger sizes, all 564 ayahs of Juz 30 were checked to fit without being cut off and without colliding with the control pill, the handle or the footer (`node scripts/layout-sweep.mjs`, see below). That check was done in headless Chrome only; it has not been done in WebKitGTK, WKWebView or WebView2 (the web engines the real app uses), which may measure Arabic text slightly differently.
- **Linux: WebKit compositing is disabled.** In a transparent window on WebKitGTK, leftover letters or boxes from the previous ayah can stay at the edge of the card after the ayah changes (confirmed on X11/GNOME, Intel/Mesa, WebKitGTK 2.50). So on Linux Simak sets `WEBKIT_DISABLE_COMPOSITING_MODE=1` before WebKit starts. The trade-off: GPU compositing for the web view is off, so animation can be less smooth and `backdrop-filter` cannot be relied on (Linux already uses a solid card without blur). If you already set the variable yourself, your value is respected. To opt out: `WEBKIT_DISABLE_COMPOSITING_MODE=0 simak` (the binary name of the installed package is set by `mainBinaryName` in `tauri.conf.json`; development builds use the cargo name `quran-widget`). The effect on other machines is not verified.
- Only Spotify on the same computer; not Spotify on a phone or another device.
- Simak is not affiliated with Spotify (see below).

### Build from source

Common requirements: Node.js 22 and a Rust toolchain >= 1.88 (`rustup`).

Linux (Debian/Ubuntu), required system packages:

```bash
sudo apt install build-essential pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev libdbus-1-dev patchelf
```

Then:

```bash
npm install
npm run tauri dev      # run in development mode
npm run tauri build    # build the installer for this OS
```

Tests: `npx vitest run` (UI) and `cd src-tauri && cargo test -p quran-widget` (Rust core). Developers who use Rust 1.88 and update `Cargo.lock` should set `CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=fallback` so the chosen dependencies match that Rust version. The full list of checks that CI runs is in [CONTRIBUTING.md](CONTRIBUTING.md).

UI preview without Tauri (in a browser): `npm run dev`, then open e.g. `http://localhost:1420/?preview=ayah&surah=112&ayah=2&theme=dark&controls=1&backdrop=1`. Parameters: `preview=ayah|waiting|intro`, `surah`, `ayah`, `theme=auto|light|dark`, `sync` (ms), `controls=1` (shows the control pill and the resize handle), `backdrop=1`, `material=native|css|solid`. In a browser, the resize handle is only displayed and does not react.

Layout sweep (optional, no npm dependencies; needs `google-chrome` and Node 22): `node scripts/layout-sweep.mjs` renders all 564 ayahs at the minimum size (read from `tauri.conf.json`), 520x320, 720x420 and 960x560 in headless Chrome with the controls forced visible. It prints the number of problems (ayahs cut off, or text lines that touch the control pill, the handle or the footer) and fails (exit code 1) if there is a problem at any of those sizes. The script cleans up its own vite server, Chrome and temporary profile.

Window path test without the real app (optional): `node scripts/e2e-window-mock.mjs` loads the page with a fake Tauri that records commands, then checks restore and clamping of the size to the monitor, drag resize, keyboard arrows and saving the size. It is not a replacement for testing on a real window. There are only two options: `--url` (use an already running server, e.g. `http://localhost:1420`; without it the script starts vite itself) and `--chrome` (name or path of the Chrome browser).

#### Icon

Icon master: `src-tauri/icons/source/simak-icon.svg` (the letter sin as a path, no font dependency; made with `scripts/gen-icon-svg.py`). The platform set is made with `npx tauri icon src-tauri/icons/source/simak-icon.png` from a 1024 px PNG rendered from the SVG; only the files used by `tauri.conf.json` are kept.

### Repository layout

```
src/            UI (TypeScript + Vite): view, theme, sync shift
src-tauri/      Tauri app (Rust): playback source, ayah engine, window
data/quran/     Arabic text and translation of Juz 30 (JSON) + license notes
data/timing/    ayah timing tables per surah (generated with local tools)
tools/          Python tools to build and check the timing data
scripts/        helper scripts (icon, dependency license list, layout sweep)
docs/           images, specs and plans
.github/        CI workflows, issue and pull request templates
```

### Test on macOS or Windows

If you have a Mac or a Windows PC and want to help verify: play a Juz 30 track in Spotify without seeking or pausing, then run

```bash
cd src-tauri && cargo run --release --example accuracy_probe -- --seconds 60
```

This tool only reads the playback position (it sends no commands) and prints a summary of the deviation. On macOS, allow the Spotify control permission dialog the first time and run it again. Send the whole output (especially the "RINGKASAN" block), with the OS name and version, through the [platform test report](https://github.com/ArielSulton/quran-widget/issues/new?template=platform_test_report.yml) issue form.

### Ayah timing data

The tables in `data/timing/` were made with local measurement and alignment tools. The measurement tools that capture playback audio are intentionally not published. So the shipped `data/timing` was generated with local tools, and **contributors cannot regenerate the offsets from this repository**. If you think a timing is wrong, propose a correction with evidence (for example listening notes with the surah, the ayah and the time) as described in [CONTRIBUTING.md](CONTRIBUTING.md).

The Python tools that are published in `tools/` (see `tools/requirements.txt`; they need models and network, so they are not run in CI) build and check the tables from those results, and `python tools/check_timing.py` validates the shipped files. The finished tables (`data/timing/*.json`, 35 surahs) and the summary of their creation (`_summary.json`, with the confidence per surah and the excluded surahs) are committed. The app only reads the finished tables; no audio is included.

### Contributing

Contributions are welcome through fork and pull request. Please read [CONTRIBUTING.md](CONTRIBUTING.md) first. This project follows the [Code of Conduct](CODE_OF_CONDUCT.md). Report security problems privately as described in [SECURITY.md](SECURITY.md).

#### Help wanted

- **Testing on macOS and Windows** with `cargo run --release --example accuracy_probe -- --seconds 60` (see "Test on macOS or Windows" above) and a platform test report.
- **Listening checks of flagged boundaries**: the clips listed by `tools/spotcheck.py --flagged`, the four outlier surahs 086/093/100/107 and surah 78 at the start of the track.
- **Wayland testing** on Linux (the only verified machine uses X11/GNOME).
- **A freely licensed Indonesian translation** to replace the Kemenag one (see [data/README.md](data/README.md)).
- **Other reciters** and other juz, with data that has a compatible license.

### License

- **Code: MIT.** See [LICENSE](LICENSE). The MIT license also covers the generated timing numbers in `data/timing/`.
- **Data: separate terms.** The Quran text and the translation in `data/quran/juz30.json` are **not** under the MIT license. The Arabic text is from the Tanzil Project (CC BY 3.0, attribution required, text unchanged). The Indonesian translation by Kementerian Agama RI (via Tanzil) is for **non-commercial purposes only**, so the bundled data prevents commercial redistribution of Simak as it is, and Simak must stay free, without ads and without monetization unless the translator or publisher gives permission. Details: [data/README.md](data/README.md).
- The bundled font Amiri Quran (SIL OFL 1.1, unchanged; license text in `src/assets/fonts/OFL.txt`), the reference audio source and third-party dependencies are described in [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md). The attribution for the Quran data is also in [data/quran/NOTICE.md](data/quran/NOTICE.md).

So this repository is open source **code** with **data that has its own terms**. It is not "fully open source data".

### Not affiliated

Simak is not affiliated with, sponsored by or endorsed by Spotify AB. Spotify is a trademark of Spotify AB. Simak does not use the Spotify API or SDK and does not store or share any audio.

### Feedback

Bug reports and suggestions: [GitHub Issues](https://github.com/ArielSulton/quran-widget/issues).

---

## Bahasa Indonesia

[English](#english) | **Bahasa Indonesia**

**Simak** (dari kata "simak": mendengarkan dengan penuh perhatian) adalah widget desktop kecil yang menampilkan ayat Al-Qur'an beserta terjemahan Indonesia yang sedang dibacakan pada trek di aplikasi Spotify. Nama pencarian: *Simak Ayat*. Tagline: *Ayat yang sedang dibacakan*.

> **Status: pra-rilis.** Installer Linux, macOS, dan Windows tersedia di halaman [Releases](https://github.com/ArielSulton/quran-widget/releases). Baru Linux yang terverifikasi. Build macOS dan Windows bersifat eksperimental dan belum code-signed, jadi OS Anda menampilkan peringatan saat pertama dibuka (lihat di bawah).

### Tampilan

| Terang | Gelap, dengan kontrol |
|---|---|
| ![Simak, tema terang](docs/images/simak-terang.png) | ![Simak, tema gelap dengan kontrol tema dan geser sinkron](docs/images/simak-gelap-kontrol.png) |

Gambar di atas dibuat dari mode pratinjau pengembangan di browser (dengan latar warna-warni agar efek kaca terlihat), bukan tangkapan layar aplikasi asli. Gambar disimpan di `docs/images/`.

### Cara kerja

1. Simak membaca trek yang sedang diputar di aplikasi desktop Spotify lewat kontrol media bawaan sistem operasi (MPRIS di Linux, AppleScript/osascript di macOS, SMTC di Windows; lihat "Batasan" untuk status verifikasinya).
2. Judul trek dicocokkan ke nama surah.
3. Posisi pemutaran dicari di tabel waktu ayat yang sudah dihitung sebelumnya (`data/timing/`), sehingga ayat berganti ketika qari pindah ayat. Tabel ini sudah disertakan di repositori: 35 surah Juz 30.
4. Widget menampilkan teks Arab dan terjemahan Kementerian Agama RI.

Simak tidak memakai API atau SDK Spotify, tidak meminta login, dan tidak memutar atau menyimpan audio apa pun.

### Pasang

Unduh file untuk sistem Anda dari [Releases](https://github.com/ArielSulton/quran-widget/releases). Anda butuh **aplikasi desktop Spotify** yang sudah login di komputer yang sama. Simak bersifat read-only: tidak pernah login ke Spotify.

| Sistem | File | Catatan |
|---|---|---|
| Linux | `.deb` (Debian, Ubuntu) atau `.AppImage` | AppImage: `chmod +x Simak_*.AppImage` lalu jalankan. Terverifikasi. |
| macOS | `.dmg` (universal: Apple silicon dan Intel) | Eksperimental. Seret Simak ke Applications. Gatekeeper memblokir aplikasi tak bertanda: klik kanan Simak lalu pilih Open, atau izinkan di System Settings > Privacy & Security ("Open Anyway"). macOS meminta izin agar Simak mengontrol Spotify: izinkan. |
| Windows | `-setup.exe` atau `.msi` | Eksperimental. SmartScreen menampilkan "Windows protected your PC": More info > Run anyway. |

Periksa unduhan dengan file `SHA256SUMS` di halaman rilis (`sha256sum -c SHA256SUMS` di Linux, `shasum -a 256 -c SHA256SUMS` di macOS).

### Cara pakai

1. Jalankan Simak.
2. Di aplikasi **desktop** Spotify, putar trek surah Juz 30 oleh Yasser Al-Dosari.
3. Widget mengikuti: ayat tampil dan berganti mengikuti bacaan. Saat dijeda, widget diam di ayat terakhir. Bila tidak ada yang cocok, widget menampilkan "Menunggu bacaan Al-Qur'an".

Arahkan kursor ke widget (atau fokuskan dengan keyboard) untuk memunculkan kontrol:

- **Tema:** Otomatis (mengikuti sistem), Terang, atau Gelap. Pilihan disimpan.
- **Geser sinkron:** menggeser waktu ayat relatif terhadap suara, dari -2,0 sampai +2,0 detik. Tombol `-` / `+` melangkah 0,1 detik; tahan **Shift** untuk langkah 1 detik. Dari keyboard: `[` mengurangi dan `]` menambah (juga dengan Shift). Klik dua kali pada nilai (atau Enter) untuk kembali ke 0. Tambah nilainya jika ayat muncul lebih cepat dari suara, kurangi jika terlambat. Pilihan disimpan.
- **Resize:** pegangan kecil (tiga garis miring) di pojok kanan bawah kartu muncul bersama kontrol. Drag untuk mengubah ukuran jendela. Dari keyboard: fokuskan pegangan dengan Tab, lalu panah mengubah ukuran 10 px (dengan **Shift**: 50 px). Ukuran minimum 430 x 270 (dijaga oleh jendela); bawaan 520 x 320. Batas atas 2000 x 1200 hanya berlaku untuk resize lewat keyboard dan untuk ukuran yang dipulihkan, tidak untuk drag dengan mouse. Keduanya juga dibatasi ke ukuran monitor (dikurangi sisa 80 x 120 px untuk panel).
- **Ukuran tersimpan:** ukuran jendela diingat dan dipulihkan saat Simak dibuka lagi; ukuran yang lebih besar dari monitor saat ini diperkecil agar muat. Posisi jendela tidak disimpan.
- **Font ikut membesar:** semakin besar jendela, teks Arab (sampai 1,6 kali pada jendela sekitar 960 x 560), terjemahan, dan footer ikut membesar secara proporsional. Pada jendela kecil dan ayat panjang, font Arab mengecil bertahap agar ayat tetap muat tanpa terpotong.

Geser sinkron perlu karena posisi pemutaran yang dilaporkan Spotify bisa berbeda dari audio yang terdengar (lihat di bawah).

#### Cara menyetel sinkron

1. Putar satu surah dan perhatikan saat qari berpindah ayat.
2. Jika ayat berikutnya muncul **lebih cepat** dari suara, tekan `]` (tambah). Jika **terlambat**, tekan `[` (kurangi). Tahan Shift untuk langkah 1 detik, lepas Shift untuk langkah halus 0,1 detik.
3. Ulangi di beberapa pergantian ayat sampai pas. Nilai disimpan; klik dua kali pada nilai untuk kembali ke 0.

### Batasan (jujur)

- **Hanya Juz 30 pada v1.** Surah lain belum ada.
- **35 dari 37 surah Juz 30.** Surah 101 (Al-Qari'ah) dan 104 (Al-Humazah) dikecualikan karena trek Spotify-nya adalah rekaman yang berbeda dari rekaman rujukan, sehingga tabel waktunya tidak berlaku.
- **Ketelitian tabel waktu tidak seragam.** `data/timing/_summary.json` mencatat tingkat keyakinan per surah (17 tinggi, 10 sedang, 8 rendah) dan batas ayat yang ditandai untuk didengarkan ulang. Penyelarasan trek Spotify terhadap rekaman rujukan membawa ketidakpastian sekitar ±1 detik pada setiap offset bawaan (semua surah, bukan hanya sebagian), dan satu kasus (surah 78, awal trek) belum dipastikan dengan didengarkan.
- **Sinkron tidak sempurna.** Posisi yang dilaporkan Spotify bisa bergeser sekitar ±1 detik dari audio pada sebagian sistem, dan ketidakpastian ±1 detik itu berlaku untuk setiap offset bawaan, bukan hanya sebagian surah. Gunakan geser sinkron untuk menyesuaikan. Nilai geser sinkron berlaku global per perangkat (satu nilai untuk semua surah), jadi menyetelnya untuk satu surah bisa menggeser surah lain.
- **Masalah yang diketahui: empat surah (086, 093, 100, 107) berjarak sekitar 1 detik dari surah sekelasnya.** Offset bawaan keempat surah itu terukur 0,8 sampai 1,0 detik berbeda dari modus kelasnya (086, 093, 100 pada trek panjang; 107 pada trek pendek) dan data dibiarkan apa adanya sesuai pengukuran; belum dipastikan dengan didengarkan mana yang tepat (klipnya masuk daftar prioritas dengar di `tools/spotcheck.py --flagged`). Bila ayat pada surah-surah ini terasa lebih cepat atau lambat, koreksi dengan geser sinkron.
- **Baru Linux yang terverifikasi.** Sumber pemutaran macOS (osascript) dan Windows (SMTC) sudah diimplementasikan, tetapi belum diverifikasi di mesin asli: baru terbukti kompilasi dan lulus test murni di CI. Linux diverifikasi di satu mesin X11/GNOME saja; Wayland belum diuji. Karena itu build macOS/Windows bersifat eksperimental. Keduanya ikut rilis, tetapi belum ditandatangani: macOS Gatekeeper dan Windows SmartScreen memberi peringatan saat pertama dibuka.
- **Blur latar belum diverifikasi.** Blur tingkat sistem hanya dirancang untuk macOS (belum diuji). Di Linux tidak ada blur di belakang jendela, dan di Windows belum diverifikasi; tanpa blur, kartu memakai isian yang lebih pekat agar teks tetap terbaca di atas wallpaper apa pun, jadi tampilannya tidak semirip kaca seperti pada gambar di atas.
- **Ukuran jendela minimum 430 x 270.** Lebih kecil dari itu tidak didukung: ayat panjang tidak lagi muat tanpa font yang terlalu kecil atau teksnya menyentuh pil kontrol. Pada ukuran minimum, bawaan, dan dua ukuran lebih besar, seluruh 564 ayat Juz 30 diperiksa muat tanpa terpotong dan tanpa bertabrakan dengan pil kontrol, pegangan, atau footer (`node scripts/layout-sweep.mjs`, lihat bawah). Pemeriksaan itu dilakukan di Chrome tanpa kepala (headless); belum di WebKitGTK, WKWebView, atau WebView2 (mesin web yang dipakai aplikasi sebenarnya), yang bisa mengukur teks Arab sedikit berbeda.
- **Linux: compositing WebKit dimatikan.** Di jendela transparan pada WebKitGTK, sisa huruf atau kotak dari ayat sebelumnya bisa tertinggal di tepi kartu setelah ayat berganti (dipastikan di X11/GNOME, Intel/Mesa, WebKitGTK 2.50). Karena itu di Linux Simak menyetel `WEBKIT_DISABLE_COMPOSITING_MODE=1` sebelum WebKit dimulai. Kompromi: compositing GPU untuk tampilan web dimatikan, sehingga animasi bisa kurang mulus dan `backdrop-filter` tidak bisa diandalkan (Linux memang sudah memakai kartu solid tanpa blur). Bila variabel itu sudah Anda setel sendiri, nilai Anda dihormati. Untuk memilih keluar: `WEBKIT_DISABLE_COMPOSITING_MODE=0 simak` (nama binary paket terpasang diatur lewat `mainBinaryName` di `tauri.conf.json`; build pengembangan memakai nama cargo `quran-widget`). Efek pada mesin lain belum diverifikasi.
- Hanya Spotify di komputer yang sama; tidak untuk Spotify di HP atau perangkat lain.
- Simak tidak berafiliasi dengan Spotify (lihat di bawah).

### Membangun dari sumber

Prasyarat umum: Node.js 22 dan toolchain Rust >= 1.88 (`rustup`).

Linux (Debian/Ubuntu), paket sistem yang dibutuhkan:

```bash
sudo apt install build-essential pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev libdbus-1-dev patchelf
```

Lalu:

```bash
npm install
npm run tauri dev      # jalankan dalam mode pengembangan
npm run tauri build    # bangun installer untuk OS ini
```

Test: `npx vitest run` (antarmuka) dan `cd src-tauri && cargo test -p quran-widget` (inti Rust). Pengembang yang memakai Rust 1.88 dan memperbarui `Cargo.lock` sebaiknya menyetel `CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=fallback` agar dependensi yang dipilih cocok dengan versi Rust tersebut. Daftar lengkap pemeriksaan yang dijalankan CI ada di [CONTRIBUTING.md](CONTRIBUTING.md).

Pratinjau antarmuka tanpa Tauri (di browser): `npm run dev`, lalu buka mis. `http://localhost:1420/?preview=ayah&surah=112&ayah=2&theme=dark&controls=1&backdrop=1`. Parameter: `preview=ayah|waiting|intro`, `surah`, `ayah`, `theme=auto|light|dark`, `sync` (ms), `controls=1` (memunculkan pil kontrol dan pegangan resize), `backdrop=1`, `material=native|css|solid`. Di browser, pegangan resize hanya tampil dan tidak bereaksi.

Sapuan tata letak (opsional, tanpa dependensi npm; butuh `google-chrome` dan Node 22): `node scripts/layout-sweep.mjs` merender ke-564 ayat pada ukuran minimum (dibaca dari `tauri.conf.json`), 520x320, 720x420, dan 960x560 di Chrome headless dengan kontrol dipaksa terlihat, mencetak jumlah masalah (ayat terpotong, atau baris teks yang menyentuh pil kontrol, pegangan, atau footer), dan gagal (exit code 1) bila ada masalah pada salah satu ukuran itu. Skrip membersihkan server vite, Chrome, dan profil sementaranya sendiri.

Uji jalur jendela tanpa aplikasi asli (opsional): `node scripts/e2e-window-mock.mjs` memuat halaman dengan Tauri palsu yang mencatat perintah, lalu memeriksa pemulihan dan pembatasan ukuran ke monitor, drag resize, panah keyboard, dan penyimpanan ukuran. Ini bukan pengganti test pada jendela sungguhan. Opsi hanya dua: `--url` (pakai server yang sudah berjalan, mis. `http://localhost:1420`; tanpa itu skrip menyalakan vite sendiri) dan `--chrome` (nama atau jalur browser Chrome).

#### Ikon

Master ikon: `src-tauri/icons/source/simak-icon.svg` (huruf sin dalam bentuk path, tanpa ketergantungan font; dibuat dengan `scripts/gen-icon-svg.py`). Set platform dibuat dengan `npx tauri icon src-tauri/icons/source/simak-icon.png` dari PNG 1024 px hasil render SVG; hanya berkas yang dipakai `tauri.conf.json` yang disimpan.

### Struktur repositori

```
src/            antarmuka (TypeScript + Vite): tampilan, tema, geser sinkron
src-tauri/      aplikasi Tauri (Rust): sumber pemutaran, mesin ayat, jendela
data/quran/     teks Arab dan terjemahan Juz 30 (JSON) + catatan lisensi
data/timing/    tabel waktu ayat per surah (dihasilkan dengan tools lokal)
tools/          alat Python untuk membangun dan memeriksa data waktu
scripts/        skrip bantu (ikon, daftar lisensi dependensi, sapuan tata letak)
docs/           gambar, spesifikasi, dan rencana
.github/        workflow CI, template issue dan pull request
```

### Menguji di macOS atau Windows

Jika Anda punya Mac atau PC Windows dan bersedia membantu memverifikasi: putar sebuah trek Juz 30 di Spotify tanpa seek atau jeda, lalu jalankan

```bash
cd src-tauri && cargo run --release --example accuracy_probe -- --seconds 60
```

Alat ini hanya membaca posisi pemutaran (tidak mengirim perintah apa pun) dan mencetak ringkasan deviasi. Di macOS, izinkan dialog izin kontrol Spotify saat pertama kali dan jalankan ulang. Kirim seluruh keluaran (terutama blok "RINGKASAN"), beserta nama dan versi OS, lewat form issue [platform test report](https://github.com/ArielSulton/quran-widget/issues/new?template=platform_test_report.yml).

### Data waktu ayat

Tabel di `data/timing/` dibuat dengan alat pengukuran dan penyelarasan lokal. Alat pengukuran yang merekam audio pemutaran sengaja tidak dipublikasikan. Jadi `data/timing` yang disertakan dibuat dengan tools lokal, dan **kontributor tidak bisa membuat ulang offset dari repositori ini**. Jika Anda menduga suatu waktu salah, ajukan koreksi beserta bukti (misalnya catatan hasil mendengarkan: surah, ayat, dan waktunya) seperti dijelaskan di [CONTRIBUTING.md](CONTRIBUTING.md).

Alat Python yang dipublikasikan di `tools/` (lihat `tools/requirements.txt`; perlu model dan jaringan, jadi tidak dijalankan di CI) membangun dan memeriksa tabel dari hasil itu, dan `python tools/check_timing.py` memvalidasi berkas yang disertakan. Tabel yang sudah jadi (`data/timing/*.json`, 35 surah) dan ringkasan pembuatannya (`_summary.json`, memuat keyakinan per surah dan surah yang dikecualikan) ikut di-commit. Aplikasi hanya membaca tabel jadi; tidak ada audio yang disertakan.

### Berkontribusi

Kontribusi diterima lewat fork dan pull request. Mohon baca [CONTRIBUTING.md](CONTRIBUTING.md) lebih dulu. Proyek ini mengikuti [Code of Conduct](CODE_OF_CONDUCT.md). Laporkan masalah keamanan secara privat seperti dijelaskan di [SECURITY.md](SECURITY.md).

#### Bantuan dibutuhkan

- **Testing di macOS dan Windows** dengan `cargo run --release --example accuracy_probe -- --seconds 60` (lihat "Menguji di macOS atau Windows" di atas) dan sebuah platform test report.
- **Pengecekan dengan mendengarkan pada batas ayat yang ditandai**: klip yang didaftar oleh `tools/spotcheck.py --flagged`, empat surah outlier 086/093/100/107, dan surah 78 di awal trek.
- **Testing di Wayland** pada Linux (satu-satunya mesin terverifikasi memakai X11/GNOME).
- **Terjemahan Indonesia berlisensi bebas** untuk menggantikan terjemahan Kemenag (lihat [data/README.md](data/README.md)).
- **Qari lain** dan juz lain, dengan data yang berlisensi kompatibel.

### Lisensi

- **Kode: MIT.** Lihat [LICENSE](LICENSE). Lisensi MIT juga mencakup angka waktu hasil proyek di `data/timing/`.
- **Data: syarat terpisah.** Teks Al-Qur'an dan terjemahan di `data/quran/juz30.json` **tidak** berada di bawah lisensi MIT. Teks Arab berasal dari Tanzil Project (CC BY 3.0, atribusi wajib, teks tidak diubah). Terjemahan Indonesia oleh Kementerian Agama RI (lewat Tanzil) hanya untuk tujuan **non-komersial**, jadi data yang disertakan menghalangi redistribusi komersial Simak apa adanya, dan Simak harus tetap gratis, tanpa iklan, dan tanpa monetisasi, kecuali penerjemah atau penerbit memberi izin. Rincian: [data/README.md](data/README.md).
- Font Amiri Quran yang disertakan (SIL OFL 1.1, tanpa perubahan; teks lisensi di `src/assets/fonts/OFL.txt`), sumber audio rujukan, dan dependensi pihak ketiga dijelaskan di [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md). Atribusi data Qur'an juga ada di [data/quran/NOTICE.md](data/quran/NOTICE.md).

Jadi repositori ini adalah **kode** open source dengan **data yang punya syarat sendiri**. Ini bukan "data yang sepenuhnya open source".

### Tidak berafiliasi

Simak tidak berafiliasi dengan, disponsori, atau didukung oleh Spotify AB. Spotify adalah merek dagang Spotify AB. Simak tidak memakai API atau SDK Spotify dan tidak menyimpan atau membagikan audio apa pun.

### Masukan

Laporan masalah dan saran: [GitHub Issues](https://github.com/ArielSulton/quran-widget/issues).
