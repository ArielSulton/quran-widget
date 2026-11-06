<!-- Thank you for contributing! / Terima kasih sudah berkontribusi! -->
<!-- Read CONTRIBUTING.md first. / Baca CONTRIBUTING.md lebih dulu. -->

## Description / Deskripsi

<!-- What does this change do and why? Link the issue, e.g. "Closes #123". -->
<!-- Apa yang diubah dan mengapa? Tautkan issue, mis. "Closes #123". -->

## Type of change / Jenis perubahan

- [ ] Bug fix / Perbaikan bug (`fix:`)
- [ ] New feature / Fitur baru (`feat:`)
- [ ] Documentation / Dokumentasi (`docs:`)
- [ ] Data (timing, text, translation) / Data (waktu, teks, terjemahan)
- [ ] CI, build or chore / CI, build, atau chore
- [ ] Other / Lainnya:

## Checklist

- [ ] Small and focused; commits follow Conventional Commits. / Kecil dan terfokus; commit mengikuti Conventional Commits.
- [ ] Rebased on the latest `main`. / Sudah di-rebase ke `main` terbaru.
- [ ] Docs updated if behavior changed. / Dokumentasi diperbarui bila perilaku berubah.

### Tests run / Test yang dijalankan

- [ ] `npx vitest run`
- [ ] `npx tsc --noEmit`
- [ ] `cargo test --locked -p quran-widget` and `--examples`
- [ ] `cargo clippy --locked --all-targets -- -D warnings`
- [ ] `python tools/check_timing.py` (if `data/timing/` changed / jika `data/timing/` berubah)
- [ ] Not applicable (docs only) / Tidak berlaku (hanya dokumentasi)

### Platforms tested / Platform yang diuji

<!-- List OS and display server (X11/Wayland). Say "not tested" for the others. -->
<!-- Sebutkan OS dan display server (X11/Wayland). Tulis "belum diuji" untuk yang lain. -->

### Data and licensing impact / Dampak pada data dan lisensi

- [ ] No data or license impact. / Tidak ada dampak pada data atau lisensi.
- [ ] Data changed. Source, license and evidence are described below (listening notes for timing changes). / Data berubah. Sumber, lisensi, dan bukti dijelaskan di bawah (catatan mendengarkan untuk perubahan waktu).
- [ ] Generated files were edited by hand, and I said so below. / Berkas hasil generate diedit dengan tangan, dan saya menyebutkannya di bawah.

<!-- Details / Rincian: -->

### Safety / Keamanan

- [ ] This change does not record, store or redistribute Spotify audio, and does not circumvent Spotify protection. / Perubahan ini tidak merekam, menyimpan, atau membagikan audio Spotify, dan tidak menghindari perlindungan Spotify.
- [ ] No Spotify logo or trademark, no Spotify Web API or SDK, no new network calls. / Tidak ada logo atau merek dagang Spotify, tidak ada Spotify Web API atau SDK, tidak ada panggilan jaringan baru.

### License of my contribution / Lisensi kontribusi saya

- [ ] I agree that my code contribution is licensed under the MIT license of this repository. / Saya setuju kontribusi kode saya dilisensikan di bawah lisensi MIT repositori ini.
