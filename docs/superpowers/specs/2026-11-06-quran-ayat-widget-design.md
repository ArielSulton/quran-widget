# Widget Ayat Al-Qur'an untuk Playlist Spotify (Yasser Al-Dosari)

Tanggal: 2026-11-06
Status: draf untuk disetujui

Catatan sumber: kalimat bertanda **[Keputusan pengguna]** adalah hal yang pengguna putuskan sendiri. **[Pilihan saya]** adalah keputusan yang saya ambil dan belum disetujui. **[Belum terbukti]** adalah dugaan yang harus dibuktikan lebih dulu.

## 1. Tujuan dan alasan

Pengguna memutar playlist Al-Qur'an (satu track per surah, qari Yasser Al-Dosari) di aplikasi Spotify. Mereka ingin aplikasi terpisah berupa widget kecil yang, saat bacaan berjalan, otomatis menampilkan **satu ayat Arab beserta terjemah Indonesia, dan berganti ke ayat berikutnya tepat ketika qari berpindah ayat**. Widget akan dibagikan ke orang lain, tidak hanya dipakai sendiri.

Alasannya: pendengar bisa membaca dan memahami ayat yang sedang dibacakan tanpa membuka mushaf.

## 2. Tujuan, bukan tujuan, dan hal yang membuat gagal

**Tujuan**
- Ayat yang tampil selalu sesuai dengan yang sedang dibacakan. **[Keputusan pengguna: "harus tepat sekali"]**
- Pengguna lain bisa memakainya tanpa login ke Spotify dan tanpa menyiapkan data apa pun.
- Jalan di Windows, macOS, dan Linux. **[Keputusan pengguna]**

**Bukan tujuan (untuk versi pertama)**
- Surah di luar Juz 30 pada versi pertama. Versi pertama mencakup Juz 30 saja, yaitu surah 78 (An-Naba') sampai 114 (An-Nas), 37 surah. Surah lain ditambahkan bertahap sesudahnya sampai 114 surah. **[Keputusan pengguna: "juz 30 dulu"]** Catatan: tiga surah yang sudah saya ukur durasinya (Al-Fatihah, Al-Kahf, Al-Baqarah) berada di luar Juz 30, jadi untuk versi pertama durasi Spotify dan rekaman publik Juz 30 masih harus dibandingkan.
- Qari selain Yasser Al-Dosari. **[Keputusan pengguna: "sementara untuk Yasser dulu"]**
- Pemutaran Spotify dari HP atau perangkat lain. **[Keputusan pengguna: komputer yang sama]**
- Pemutar audio sendiri, pencarian ayat, bookmark, transliterasi, tafsir. **[Pilihan saya]**

**Hal yang membuat proyek ini gagal walau terlihat jalan**
- Ayat yang tampil salah atau terlambat terasa.
- Pengguna diminta login Spotify, membuat akun pengembang, atau mengunduh data tambahan.
- Aplikasi menyimpan atau membagikan audio Spotify atau audio rekaman qari.

## 3. Perilaku widget

Keputusan pengguna kecuali bertanda lain.

- **Tampilan:** "Fokus besar". Jendela lebih besar, ayat Arab sangat besar di tengah, terjemah Indonesia di bawahnya, dan nama surah serta nomor ayat kecil di bagian bawah. Contoh tampilannya ada di mockup pilihan C di `.superpowers/brainstorm/`.
- **Terjemah:** Kementerian Agama RI. Pengguna menyetujui ini.
- **Saat bacaan berjalan:** satu ayat tampil dan berganti tepat saat qari pindah ayat.
- **Saat dijeda:** widget diam di ayat terakhir yang tampil.
- **Saat pengguna ganti surah:** widget langsung ganti ke surah baru.
- **Saat posisi putar digeser (seek):** widget loncat ke ayat yang sesuai. **[Pilihan saya]**
- **Saat yang diputar bukan track Qur'an Yasser Al-Dosari, atau Spotify tidak berjalan:** widget menampilkan tulisan singkat seperti "Menunggu bacaan Al-Qur'an". **[Pilihan saya]**
- **Bagian sebelum ayat 1** (taawudz atau basmalah yang dibacakan qari sebelum ayat pertama): ditampilkan sebagai keadaan tersendiri. **[Pilihan saya]**

## 4. Hal teknis yang sudah diputuskan

- **Kerangka aplikasi:** Tauri (Rust), dengan tampilan HTML dan JavaScript. **[Keputusan pengguna]**
- **Tanpa login Spotify. [Pilihan saya, pengguna menyetujui arahnya]** Alasannya, menurut sumber yang saya temukan: aplikasi Spotify dalam mode pengembang dibatasi sekitar 5 pengguna dan butuh Premium milik pengembang, sedangkan mode tanpa batas hanya untuk badan usaha dengan sekitar 250.000 pengguna aktif bulanan. Sumber-sumbernya tidak sepenuhnya sepakat dan belum dicek ke dokumentasi resmi Spotify. **[Belum terbukti]**
- **Membaca pemutaran dari aplikasi Spotify di komputer yang sama. [Pilihan saya]**
  - Linux: MPRIS.
  - macOS: AppleScript ke aplikasi Spotify.
  - Windows: kontrol media sistem Windows (SMTC).
  - **[Belum terbukti]** Ketiganya diharapkan memberi judul track dan posisi detik, tetapi belum diuji, terutama Windows.

## 5. Data waktu ayat (bagian paling berisiko)

Spotify tidak tahu bacaan sudah sampai ayat berapa. Waktu mulai tiap ayat harus dibuat dari rekaman yang persis sama dengan yang ada di Spotify.

Yang sudah saya ukur:

| Surah | Durasi Spotify | Durasi rekaman publik (mp3quran, `server11.mp3quran.net/yasser/NNN.mp3`) | Selisih |
|---|---|---|---|
| Al-Fatihah | 0:54 | 0:48 | +6 detik |
| Al-Kahf | 29:06 | 29:03,5 | +2,5 detik |
| Al-Baqarah | 2:00:50 | 2:00:47,4 | +2,6 detik |

Kesimpulan sementara **[Belum terbukti]**: rekaman Spotify kemungkinan besar berasal dari rekaman publik yang sama, dengan hening tambahan di awal atau akhir. Selisih Al-Fatihah berbeda dari dua lainnya, jadi geserannya tidak bisa dianggap sama untuk semua surah.

Rencana **[Pilihan saya]**:
1. Cocokkan teks ayat dengan rekaman publik untuk tiap surah (*forced alignment*), hasilnya waktu mulai tiap ayat, termasuk bagian sebelum ayat 1.
2. Ukur geseran tiap track Spotify terhadap rekaman publik secara **otomatis oleh program, bukan dengan didengarkan manusia**. Program menangkap suara yang keluar dari komputer saat Spotify memutar potongan pendek di awal, akhir, dan beberapa titik tengah track, lalu membandingkannya dengan rekaman publik (misalnya korelasi silang) untuk mendapat geseran dalam milidetik. Potongan pendek dipakai supaya surah panjang seperti Al-Baqarah tidak perlu diputar dua jam penuh. Ini dijalankan sekali oleh pembuat aplikasi, bukan oleh pengguna. **[Pilihan saya, memperjelas kalimat sebelumnya]**
3. Simpan hanya angka-angka itu (waktu ayat dan geseran per surah) di dalam aplikasi. Audio tidak disimpan dan tidak dibagikan.
4. Data waktu dari Quran.com tidak dipakai, karena ada laporan audio Yasser Al-Dosari di sana tidak sinkron dengan teksnya.

Ketepatan: pergantian ayat harus terasa tepat. Target kerja: selisih tidak lebih dari 300 ms dari batas ayat sebenarnya. **[Keputusan pengguna: angka ini disetujui]**

Cakupan data versi pertama: 37 surah Juz 30 (surah 78 sampai 114). **[Keputusan pengguna]**

## 6. Risiko dan hal yang belum diketahui

- Apakah ketiga sistem operasi benar-benar memberi posisi detik yang cukup akurat dan cukup sering dari aplikasi Spotify. Posisi di antara pembaruan harus dihitung sendiri oleh widget. Kalau salah satu sistem operasi (paling mungkin Windows) ternyata tidak bisa mencapai target 300 ms, versi pertama tetap dirilis untuk sistem operasi itu dengan keterangan jelas bahwa ketepatannya lebih rendah, supaya ketiga sistem operasi tetap jalan. **[Keputusan pengguna]**
- Bentuk judul track di Spotify (bahasa dan penulisan) untuk 114 surah belum dilihat, jadi pencocokan judul ke nomor surah belum teruji.
- Apakah geseran Spotify terhadap rekaman publik konstan sepanjang satu track.
- Spotify bisa mengganti rekaman atau durasi track di masa depan, sehingga tabel waktu menjadi tidak cocok. Widget sebaiknya mengenali ketidakcocokan durasi dan tidak menampilkan ayat yang salah.
- Lisensi teks Arab, terjemah Kemenag, dan huruf Al-Qur'an untuk dibagikan dalam aplikasi belum diperiksa.
- Mengambil audio yang keluar dari Spotify untuk mengukur geseran harus dilakukan hanya untuk mengambil angka, tidak menyimpan atau membagikan audio.

## 7. Diserahkan kepada pembuat

- Sumber dan format teks Arab serta terjemah Kemenag, selama lisensinya mengizinkan dibagikan.
- Jenis huruf Al-Qur'an, ukuran jendela, warna, dan animasi pergantian ayat, selama mengikuti pilihan C.
- Pilihan alat untuk *forced alignment* dan pengambilan audio saat pengukuran geseran.
- Struktur kode, pengemasan, dan pemasangan di tiga sistem operasi.
- Apakah jendela selalu di atas jendela lain, bisa digeser, dan bisa disimpan posisinya.
