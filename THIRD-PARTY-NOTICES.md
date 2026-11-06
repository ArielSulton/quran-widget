# Pemberitahuan pihak ketiga (Third-party notices)

Simak (https://github.com/ArielSulton/quran-widget) memuat atau memakai karya pihak ketiga berikut. Dokumen ini mengikuti persyaratan atribusi masing-masing sumber.

## 1. Teks Al-Qur'an (Arab): Tanzil Project

- Sumber: Tanzil Quran Text, Uthmani, Versi 1.1, https://tanzil.net
- Lisensi: Creative Commons Attribution 3.0 (CC BY 3.0). Pembaruan teks: http://tanzil.net/updates/
- Berkas: `data/quran/juz30.json` (kunci `ar`). Salinan atribusi ini juga ada di `data/quran/NOTICE.md`, di samping datanya.
- Satu-satunya perubahan: teks dipecah ke JSON per surah/ayat, dan basmalah yang Tanzil letakkan di awal ayat 1 setiap surah dihapus dari ayat 1 (agar aplikasi dapat menampilkannya terpisah). Tidak ada huruf atau tanda baca ayat yang diubah.

Blok hak cipta dan syarat penggunaan di bawah ini disalin apa adanya dari berkas sumber `quran-uthmani.txt` yang diunduh dari Tanzil:

```
# PLEASE DO NOT REMOVE OR CHANGE THIS COPYRIGHT BLOCK
#====================================================================
#
#  Tanzil Quran Text (Uthmani, Version 1.1)
#  Copyright (C) 2007-2026 Tanzil Project
#  License: Creative Commons Attribution 3.0
#
#  This copy of the Quran text is carefully produced, highly 
#  verified and continuously monitored by a group of specialists 
#  at Tanzil Project.
#
#  TERMS OF USE:
#
#  - Permission is granted to copy and distribute verbatim copies 
#    of this text, but CHANGING IT IS NOT ALLOWED.
#
#  - This Quran text can be used in any website or application, 
#    provided that its source (Tanzil Project) is clearly indicated, 
#    and a link is made to tanzil.net to enable users to keep
#    track of changes.
#
#  - This copyright notice shall be included in all verbatim copies 
#    of the text, and shall be reproduced appropriately in all files 
#    derived from or containing substantial portion of this text.
#
#  Please check updates at: http://tanzil.net/updates/
#
#====================================================================
```

Teks Qur'an: Tanzil Project (https://tanzil.net).

## 2. Terjemahan Indonesia: Kementerian Agama RI (via Tanzil)

- Sumber: "Bahasa Indonesia", penerjemah: Indonesian Ministry of Religious Affairs (Kementerian Agama RI), pembaruan terakhir 4 Juni 2010, melalui https://tanzil.net/trans/id.indonesian
- Berkas: `data/quran/juz30.json` (kunci `id`).
- Syarat dari Tanzil (https://tanzil.net/trans/): "The translations provided at this page are for non-commercial purposes only. If used otherwise, you need to obtain necessary permission from the translator or the publisher."
- Akibatnya **Simak harus tetap gratis dan tanpa monetisasi** (tanpa penjualan, iklan, atau bentuk komersial lain) kecuali ada izin dari Kementerian Agama RI. Tanzil tidak menyatakan lisensi terpisah dari Kemenag; status hukum terjemahan tersebut belum dikonfirmasi secara resmi.
- Terjemahan: Kementerian Agama RI (via Tanzil.net).

## 3. Audio rujukan: mp3quran.net (Yasser Al-Dosari)

Rekaman bacaan Yasser Al-Dosari dari mp3quran.net dipakai **hanya pada saat pengembangan**, di komputer pengembang, untuk menghitung angka waktu ayat. Tidak ada audio yang disertakan di repositori maupun di aplikasi; yang dikirim hanyalah tabel angka waktu (`data/timing/`). Hak atas rekaman tetap milik pemegangnya masing-masing.

Simak juga tidak berafiliasi dengan Spotify AB; lihat README.

## 4. Font

- **Amiri Quran (disertakan dalam aplikasi).** Aplikasi menyertakan berkas font `src/assets/fonts/AmiriQuran-Regular.woff2` untuk menampilkan teks Arab, karena font sistem umumnya tidak memuat tanda-tanda Qur'an (muncul sebagai kotak kosong).
  - Nama dan versi: Amiri Quran Regular, Version 1.001, perancang Khaled Hosny (Alif Type), https://github.com/aliftype/amiri
  - Hak cipta (persis seperti di tabel nama font): "Copyright 2010-2022 The Amiri Project Authors (https://github.com/aliftype/amiri)."
  - Lisensi: SIL Open Font License, Version 1.1 (https://openfontlicense.org/). Teks lengkap: `src/assets/fonts/OFL.txt`. Font tidak mencantumkan Reserved Font Name.
  - Perubahan: **tidak ada.** Berkas `AmiriQuran.ttf` asli (SHA-256 `a406aa685f307d96327deecaae6d69322d28e30cd7ec8929badbee19a7e6c585`) hanya dibungkus ulang sebagai WOFF2 tanpa mengubah glif, tabel, atau fitur apa pun dan tanpa subset (SHA-256 berkas yang dikirim: `d2300497794c78746ad11ce3ead5c2c246cc9b0aeaaa35308288a3e211853035`). Rincian ada di `src/assets/fonts/README.md`.
  - Font ini dikirim bersama aplikasi dan tidak dijual terpisah, sesuai syarat OFL. Pengecekan bahwa font memuat semua karakter yang ditampilkan: `tools/check_font_coverage.py`.
- Font **cadangan** yang hanya disebut namanya di `src/style.css` (tidak disertakan): "Scheherazade New", "Noto Naskh Arabic", "Geeza Pro", "Traditional Arabic", "Arabic Typesetting", lalu `serif`; untuk antarmuka font sistem (`-apple-system`, "SF Pro Text", "Segoe UI", `system-ui`, `sans-serif`).
- **Ikon aplikasi** (`src-tauri/icons/`) memuat kerangka vektor huruf sin (U+0633) dari font **Amiri Bold**, yang diubah menjadi path SVG (`src-tauri/icons/source/simak-icon.svg`, dibuat oleh `scripts/gen-icon-svg.py`). Amiri: "Copyright 2010-2022 The Amiri Project Authors (https://github.com/aliftype/amiri)", dilisensikan di bawah SIL Open Font License 1.1 (https://scripts.sil.org/OFL). Berkas font Amiri Bold sendiri tidak didistribusikan oleh Simak.

## 5. Dependensi perangkat lunak

Dependensi langsung (lisensi dibaca dari metadata `cargo metadata` dan `package-lock.json`):

| Komponen | Versi | Lisensi | Dipakai untuk |
|---|---|---|---|
| tauri | 2.11.6 | Apache-2.0 OR MIT | kerangka aplikasi desktop |
| tauri-build | 2.6.3 | Apache-2.0 OR MIT | build |
| serde | 1.0.229 | MIT OR Apache-2.0 | serialisasi |
| serde_json | 1.0.154 | MIT OR Apache-2.0 | JSON |
| mpris (Linux saja) | 2.1.0 | Apache-2.0 | membaca pemutaran lewat D-Bus MPRIS |
| @tauri-apps/api | 2.11.1 | Apache-2.0 OR MIT | jembatan antarmuka ke Tauri |
| @tauri-apps/cli (build) | 2.12.4 | Apache-2.0 OR MIT | pembuatan installer dan ikon |
| typescript (build) | 5.8.3 | Apache-2.0 | kompilasi antarmuka |
| vite (build) | 7.3.7 | MIT | bundler antarmuka |
| vitest (build) | 3.2.7 | MIT | tes antarmuka |

Dependensi transitif (sekitar 430 paket Rust, termasuk yang khusus platform lain, dan ratusan paket npm untuk pengembangan) **tidak didaftar satu per satu di sini**. Ringkasan nama, versi, dan lisensi semuanya dapat dihasilkan lokal tanpa jaringan:

```bash
scripts/gen-notices.sh > build/dependency-licenses.md
```

Pada daftar transitif tersebut mayoritas berlisensi MIT atau Apache-2.0 (sering ganda). Perlu diketahui: beberapa crate transitif berlisensi MPL-2.0 (`cssparser`, `cssparser-macros`, `dtoa-short`, `option-ext`, `selectors`), dan di Linux aplikasi terhubung secara dinamis ke GTK 3 dan WebKitGTK yang dipasang sistem (LGPL). Teks lisensi lengkap tiap paket tidak disalin ke dokumen ini; ambil dari repositori/registri masing-masing atau dari berkas lisensi di dalam paket sumbernya. Daftar tersebut belum diaudit satu per satu oleh manusia.
