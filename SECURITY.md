# Security Policy / Kebijakan Keamanan

[English](#english) | [Bahasa Indonesia](#bahasa-indonesia)

## English

### Supported versions

Simak is pre-release. There is no stable release yet. Only the latest commit on the `main` branch is supported. Please check that a problem still exists on the latest `main` before you report it.

### How to report a vulnerability

Please **do not open a public issue** for a security problem. Report it privately through GitHub:

1. Go to the repository on GitHub: https://github.com/ArielSulton/quran-widget
2. Open the **Security** tab, then click **Report a vulnerability** (private vulnerability reporting).
3. Fill in the form.

What to include:

- A clear description of the problem and its impact.
- Steps to reproduce, or a small proof of concept.
- The affected version or commit, your operating system and its version.
- Any suggestion for a fix, if you have one.

Do not include personal data you do not want to share.

### Scope

In scope:

- The Simak app code in this repository (`src/`, `src-tauri/`).
- The scripts and tools in this repository (`tools/`, `scripts/`).
- The CI and release workflows in `.github/workflows/`, for example a way to run untrusted code with a write token or to leak a secret.
- The installers that are built from this repository, once they exist.

Notes on what Simak does: the app only reads local playback metadata (track title, position and status) from the operating system. It does not use the Spotify API or SDK, does not ask you to log in, does not play or store audio, and does not make network calls.

Out of scope:

- Spotify itself (the Spotify app, service or account). Please report those to Spotify.
- The operating system, WebKitGTK, WKWebView, WebView2, Tauri or other dependencies. Report those upstream. If a dependency problem affects Simak in a way we can fix or work around, we are happy to hear about it.
- Problems that need a machine that is already fully compromised.
- Reports that are only about the licensing of the Quran data (for that, open a normal issue, see `data/README.md`).

### What to expect

This is a small project run by volunteers. We handle reports on a **best-effort** basis. We will try to acknowledge a report and to say what we plan to do, but we cannot promise a response time or a fix date. Please give us reasonable time before you disclose a problem publicly. We will credit you in the fix notes if you want to be credited.

## Bahasa Indonesia

### Versi yang didukung

Simak masih pra-rilis. Belum ada rilis stabil. Hanya commit terbaru di branch `main` yang didukung. Mohon pastikan masalahnya masih ada di `main` terbaru sebelum Anda melapor.

### Cara melaporkan kerentanan

Mohon **jangan membuka issue publik** untuk masalah keamanan. Laporkan secara privat lewat GitHub:

1. Buka repositori di GitHub: https://github.com/ArielSulton/quran-widget
2. Buka tab **Security**, lalu klik **Report a vulnerability** (private vulnerability reporting).
3. Isi form-nya.

Yang perlu disertakan:

- Deskripsi masalah dan dampaknya yang jelas.
- Langkah mereproduksi, atau proof of concept kecil.
- Versi atau commit yang terdampak, sistem operasi Anda beserta versinya.
- Saran perbaikan, jika ada.

Jangan menyertakan data pribadi yang tidak ingin Anda bagikan.

### Cakupan

Dalam cakupan:

- Kode aplikasi Simak di repositori ini (`src/`, `src-tauri/`).
- Skrip dan tools di repositori ini (`tools/`, `scripts/`).
- Workflow CI dan release di `.github/workflows/`, misalnya cara menjalankan kode tidak tepercaya dengan token tulis atau membocorkan secret.
- Installer yang dibangun dari repositori ini, setelah ada.

Catatan tentang apa yang dilakukan Simak: aplikasi hanya membaca metadata pemutaran lokal (judul trek, posisi, dan status) dari sistem operasi. Simak tidak memakai API atau SDK Spotify, tidak meminta login, tidak memutar atau menyimpan audio, dan tidak melakukan panggilan jaringan.

Di luar cakupan:

- Spotify itu sendiri (aplikasi, layanan, atau akun Spotify). Mohon laporkan ke Spotify.
- Sistem operasi, WebKitGTK, WKWebView, WebView2, Tauri, atau dependensi lain. Laporkan ke upstream-nya. Jika masalah dependensi memengaruhi Simak dengan cara yang bisa kami perbaiki atau kami akali, kami senang mendengarnya.
- Masalah yang butuh mesin yang sudah sepenuhnya dikuasai penyerang.
- Laporan yang hanya soal lisensi data Al-Qur'an (untuk itu, buka issue biasa, lihat `data/README.md`).

### Yang bisa Anda harapkan

Ini proyek kecil yang dijalankan oleh relawan. Kami menangani laporan secara **best effort**. Kami akan berusaha mengonfirmasi laporan dan menyampaikan rencana kami, tetapi kami tidak bisa menjanjikan waktu respons atau tanggal perbaikan. Mohon beri kami waktu yang wajar sebelum Anda mengungkap masalah ke publik. Kami akan mencantumkan nama Anda di catatan perbaikan bila Anda mau.
