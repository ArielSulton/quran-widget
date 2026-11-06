//! Pengukur akurasi posisi sumber pemutaran di mesin asli (Linux, macOS, Windows).
//!
//! Membaca `default_source()` tiap 100 ms selama N detik (bawaan 60), mencetak tiap sampel
//! (waktu dinding, posisi yang dilaporkan, status) lalu ringkasan: deviasi maksimum posisi
//! terhadap jam dinding, jumlah langkah mundur/lompatan > 150 ms, dan granularitas pembaruan
//! posisi. Tidak pernah mengirim perintah pemutaran (hanya membaca).
//!
//! Cara pakai (di mesin yang diukur, repo ini sudah di-clone, Rust terpasang):
//!
//! 1. Buka Spotify dan putar track MANA SAJA (mis. satu surah Juz 30) secara kontinu.
//!    Selama pengukuran JANGAN seek, jeda, atau ganti track.
//! 2. `cd src-tauri && cargo run --release --bin accuracy_probe -- --seconds 60`
//!    (macOS: izinkan dialog "Simak/Terminal ingin mengontrol Spotify" saat pertama kali;
//!    jalankan ulang setelah mengizinkan.)
//! 3. Kirim balik SELURUH keluaran terminal (terutama blok "RINGKASAN") beserta nama dan
//!    versi OS. Untuk Windows, sebutkan juga versi aplikasi Spotify (Microsoft Store atau
//!    installer .exe).
//!
//! Ambang acuan: deviasi maksimum ≤ 300 ms = baik; ukuran langkah ~1000 ms dengan jeda ~1000 ms
//! berarti sumber hanya memperbarui posisi sekali per detik (galat hingga ~1 s).

use quran_widget_lib::playback::default_source;
use quran_widget_lib::playback::probe_analysis::{analyze, Sample, Spread, Summary};
use std::time::{Duration, Instant};

const SAMPLE_INTERVAL: Duration = Duration::from_millis(100);
const DEFAULT_SECONDS: u64 = 60;

fn usage() -> &'static str {
    "Pemakaian: accuracy_probe [--seconds N]\n  \
     --seconds N   lama pengukuran dalam detik (bawaan 60, 1..=3600)"
}

fn parse_args(args: &[String]) -> Result<u64, String> {
    let mut seconds = DEFAULT_SECONDS;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--seconds" => {
                let v = it.next().ok_or("--seconds butuh nilai")?;
                seconds = v
                    .parse()
                    .map_err(|_| format!("nilai --seconds tidak valid: {v}"))?;
                if !(1..=3600).contains(&seconds) {
                    return Err(format!("--seconds harus 1..=3600, dapat {seconds}"));
                }
            }
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("argumen tidak dikenal: {other}")),
        }
    }
    Ok(seconds)
}

fn fmt_spread(label: &str, unit: &str, s: Option<Spread>) -> String {
    match s {
        Some(s) => format!(
            "{label}: min {} / median {} / p90 {} / maks {} {unit}",
            s.min, s.p50, s.p90, s.max
        ),
        None => format!("{label}: (tidak ada data)"),
    }
}

fn print_summary(sum: &Summary, seconds: u64, errors: usize, absent: usize) {
    println!();
    println!("=== RINGKASAN ===");
    println!("os: {} {}", std::env::consts::OS, std::env::consts::ARCH);
    println!(
        "durasi: {seconds} detik; sampel: {} (memutar: {})",
        sum.samples, sum.playing_samples
    );
    println!("poll gagal: {errors}; poll tanpa Spotify/track: {absent}");
    if sum.playing_samples < 2 {
        println!("TIDAK CUKUP DATA: butuh minimal 2 sampel saat memutar. Pastikan Spotify sedang memutar.");
        return;
    }
    println!(
        "deviasi maksimum |posisi − (posisi_awal + waktu_dinding)|: {} ms",
        sum.max_abs_deviation_ms
    );
    println!(
        "rentang deviasi (maks − min, tak bergantung jangkar): {} ms",
        sum.deviation_span_ms
    );
    println!("langkah mundur: {}", sum.backward_steps);
    println!("lompatan > 150 ms (|Δposisi − Δdinding|): {}", sum.jumps);
    println!("perubahan nilai posisi: {}", sum.position_changes);
    println!(
        "{}",
        fmt_spread("jeda antar perubahan posisi", "ms", sum.change_interval_ms)
    );
    println!(
        "{}",
        fmt_spread("besar perubahan posisi maju", "ms", sum.change_step_ms)
    );
    let verdict = if sum.max_abs_deviation_ms <= 300 {
        "<= 300 ms (baik)"
    } else {
        "> 300 ms (akurasi rendah)"
    };
    println!("penilaian: {verdict}");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let seconds = match parse_args(&args) {
        Ok(s) => s,
        Err(msg) => {
            if !msg.is_empty() {
                eprintln!("{msg}");
            }
            eprintln!("{}", usage());
            std::process::exit(if msg.is_empty() { 0 } else { 2 });
        }
    };

    println!(
        "Mengukur {seconds} detik, sampel tiap {} ms. Jangan seek/jeda/ganti track.",
        SAMPLE_INTERVAL.as_millis()
    );
    println!("waktu_dinding_ms\tposisi_ms\tmemutar");

    let mut source = default_source();
    let started = Instant::now();
    let deadline = started + Duration::from_secs(seconds);
    let mut samples: Vec<Sample> = Vec::new();
    let (mut errors, mut absent) = (0usize, 0usize);
    let mut last_title: Option<String> = None;
    let mut last_err: Option<String> = None;
    let mut next_tick = started;

    while Instant::now() < deadline {
        let before = Instant::now();
        let result = source.poll();
        let after = Instant::now();
        // Sama seperti runtime: sampel diberi cap titik tengah rentang poll().
        let wall_ms = (before + (after - before) / 2)
            .duration_since(started)
            .as_millis() as u64;
        match result {
            Ok(Some(snap)) => {
                last_err = None;
                if last_title.as_deref() != Some(snap.title.as_str()) {
                    println!("# judul: {} (durasi {} ms)", snap.title, snap.duration_ms);
                    last_title = Some(snap.title.clone());
                }
                println!("{wall_ms}\t{}\t{}", snap.position_ms, snap.is_playing);
                samples.push(Sample {
                    wall_ms,
                    position_ms: snap.position_ms,
                    is_playing: snap.is_playing,
                });
            }
            Ok(None) => {
                absent += 1;
                if absent == 1 || absent % 50 == 0 {
                    println!("# {wall_ms} ms: Spotify tidak berjalan atau tidak ada track");
                }
            }
            Err(e) => {
                errors += 1;
                let text = e.to_string();
                if last_err.as_deref() != Some(text.as_str()) {
                    println!("# {wall_ms} ms: GALAT: {text}");
                    last_err = Some(text);
                }
            }
        }
        next_tick += SAMPLE_INTERVAL;
        if let Some(wait) = next_tick.checked_duration_since(Instant::now()) {
            std::thread::sleep(wait);
        } else {
            next_tick = Instant::now(); // poll lebih lambat dari interval: jangan mengejar
        }
    }

    print_summary(&analyze(&samples), seconds, errors, absent);
}
