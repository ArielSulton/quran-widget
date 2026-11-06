//! Logika murni sumber macOS: skrip AppleScript, pemisah field, dan parser keluaran `osascript`.
//!
//! Modul ini dikompilasi di semua OS (bukan hanya macOS) supaya dapat diuji dengan
//! `cargo test` di Linux. Panggilan ke OS ada di `macos.rs`.

use super::{Snapshot, SourceError};

/// Pemisah field pada baris keluaran skrip: U+001F (Unit Separator). Karakter kontrol ini
/// praktis tidak pernah muncul dalam judul lagu.
pub const FIELD_SEP: char = '\u{1f}';

/// Bundle id aplikasi desktop Spotify di macOS.
pub const BUNDLE_ID: &str = "com.spotify.client";

/// Petunjuk (Indonesia) bila izin Automation ditolak.
pub const PERMISSION_HINT: &str =
    "Izinkan Simak mengontrol Spotify di Pengaturan Sistem > Privasi & Keamanan > Otomasi";

/// Batas kewajaran: durasi track (ms) dan posisi (detik). Nilai di atasnya dianggap sampah.
const MAX_DURATION_MS: f64 = 1.0e12;
const MAX_POSITION_S: f64 = 1.0e9;

/// Baris skrip untuk `osascript -e` (satu argumen `-e` per baris).
///
/// Skrip TIDAK PERNAH meluncurkan Spotify: pembacaan hanya terjadi di dalam cabang
/// `application id "com.spotify.client" is running`. Ekspresi `is running` ditangani runtime
/// AppleScript lewat LaunchServices tanpa mengirim Apple event, jadi tidak meluncurkan aplikasi
/// dan tidak memicu dialog izin Automation. (Alternatif `System Events > name of processes`
/// ditolak: butuh izin Automation tambahan untuk System Events.) Blok `tell` dikompilasi
/// berdasarkan bundle id sehingga tidak bergantung pada nama aplikasi/locale.
///
/// Keluaran selalu satu baris 4 field dipisah U+001F (`character id 31`): judul, durasi (ms),
/// posisi (detik), status. Jika Spotify tidak berjalan atau `stopped`, baris berstatus `stopped`
/// (membaca `current track` saat stopped melempar error -1728, jadi dijaga lebih dulu).
pub fn script_lines() -> Vec<String> {
    let idle = r#"return "" & sep & "0" & sep & "0" & sep & "stopped""#;
    [
        "set sep to character id 31".to_string(),
        format!(r#"if application id "{BUNDLE_ID}" is running then"#),
        format!(r#"tell application id "{BUNDLE_ID}""#),
        "set st to (player state as text)".to_string(),
        format!(r#"if st is "stopped" then {idle}"#),
        "return (name of current track) & sep & ((duration of current track) as text) & sep & ((player position) as text) & sep & st"
            .to_string(),
        "end tell".to_string(),
        "end if".to_string(),
        idle.to_string(),
    ]
    .into()
}

/// Parse angka desimal ala AppleScript. Menerima pemisah desimal `.` maupun `,`
/// (tergantung locale), notasi ilmiah (`1.743E+6`), dan menolak sisanya.
pub fn parse_decimal(s: &str) -> Result<f64, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("angka kosong".into());
    }
    // Daftar putih karakter: otomatis menolak "nan"/"inf"/"missing value" yang diterima `f64::from_str`.
    if !s
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | 'e' | 'E' | '+' | '-'))
    {
        return Err(format!("bukan angka: {s:?}"));
    }
    let commas = s.matches(',').count();
    if commas > 1 || (commas == 1 && s.contains('.')) {
        return Err(format!("pemisah desimal ambigu: {s:?}"));
    }
    let normalized = s.replace(',', ".");
    let value: f64 = normalized
        .parse()
        .map_err(|_| format!("bukan angka: {s:?}"))?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!("angka di luar jangkauan: {s:?}"))
    }
}

/// Angka tak negatif dan terbatas oleh `max`, dikali `scale` lalu dibulatkan ke u64.
fn parse_scaled(field: &str, name: &str, scale: f64, max: f64) -> Result<u64, SourceError> {
    let bad = |why: String| SourceError::Unavailable(format!("{name} tidak valid: {why}"));
    let v = parse_decimal(field).map_err(bad)?;
    if v < 0.0 {
        return Err(bad(format!("negatif: {field:?}")));
    }
    if v > max {
        return Err(bad(format!("di luar jangkauan: {field:?}")));
    }
    Ok((v * scale).round() as u64)
}

/// Parse satu baris keluaran skrip: `judul<SEP>durasi_ms<SEP>posisi_detik<SEP>status`.
///
/// Dipisah dari kanan: tiga field terakhir adalah numerik/status, sisanya (dengan atau tanpa
/// pemisah) adalah judul. Jadi judul yang kebetulan memuat U+001F tidak merusak parsing.
/// `Ok(None)` untuk status `stopped`, judul kosong, atau `missing value`.
pub fn parse_applescript(line: &str) -> Result<Option<Snapshot>, SourceError> {
    let line = line.strip_suffix('\n').unwrap_or(line);
    let line = line.strip_suffix('\r').unwrap_or(line);

    let mut fields = line.rsplitn(4, FIELD_SEP);
    let (Some(state), Some(position), Some(duration), Some(title)) =
        (fields.next(), fields.next(), fields.next(), fields.next())
    else {
        return Err(SourceError::Unavailable(format!(
            "keluaran AppleScript tidak lengkap (kurang dari 4 field): {line:?}"
        )));
    };

    let state = state.trim();
    let is_playing = if state.eq_ignore_ascii_case("playing") || state.contains("kPSP") {
        true
    } else if state.eq_ignore_ascii_case("paused") || state.contains("kPSp") {
        false
    } else if state.eq_ignore_ascii_case("stopped") || state.contains("kPSS") {
        return Ok(None);
    } else {
        return Err(SourceError::Unavailable(format!(
            "status pemutar tidak dikenal: {state:?}"
        )));
    };

    let title_trimmed = title.trim();
    if title_trimmed.is_empty() || title_trimmed == "missing value" {
        return Ok(None);
    }

    Ok(Some(Snapshot {
        title: title.to_string(),
        duration_ms: parse_scaled(duration, "durasi", 1.0, MAX_DURATION_MS)?,
        position_ms: parse_scaled(position, "posisi", 1_000.0, MAX_POSITION_S)?,
        is_playing,
    }))
}

/// Petakan kegagalan `osascript` (exit code != 0) dari teks stderr-nya.
///
/// - `-1743` (Automation ditolak) → `Unavailable` dengan petunjuk Indonesia;
/// - `-600` (aplikasi tidak berjalan; Spotify keluar di antara cek dan baca) dan `-1728`
///   (tidak ada `current track`) → `Ok(None)`;
/// - lainnya → `Unavailable` dengan pesan asli (dipotong).
pub fn classify_failure(stderr: &str) -> Result<Option<Snapshot>, SourceError> {
    let msg = stderr.trim();
    if msg.contains("(-1743)") || msg.contains("Not authorized to send Apple events") {
        return Err(SourceError::Unavailable(format!(
            "izin Automation ditolak untuk Spotify. {PERMISSION_HINT}"
        )));
    }
    if msg.contains("(-600)") || msg.contains("(-1728)") {
        return Ok(None);
    }
    let shown: String = msg.chars().take(400).collect();
    Err(SourceError::Unavailable(format!(
        "osascript gagal: {shown}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(title: &str, dur: &str, pos: &str, state: &str) -> String {
        format!("{title}{FIELD_SEP}{dur}{FIELD_SEP}{pos}{FIELD_SEP}{state}")
    }

    fn snap(title: &str, duration_ms: u64, position_ms: u64, is_playing: bool) -> Snapshot {
        Snapshot {
            title: title.into(),
            duration_ms,
            position_ms,
            is_playing,
        }
    }

    // --- parse_decimal ---------------------------------------------------------------

    #[test]
    fn decimal_accepts_dot_and_comma() {
        assert_eq!(parse_decimal("5.25"), Ok(5.25));
        assert_eq!(parse_decimal("5,25"), Ok(5.25));
        assert_eq!(parse_decimal("  12,5 "), Ok(12.5));
        assert_eq!(parse_decimal("0"), Ok(0.0));
        assert_eq!(parse_decimal("1743000"), Ok(1_743_000.0));
        assert_eq!(parse_decimal(".5"), Ok(0.5));
    }

    #[test]
    fn decimal_accepts_scientific_notation() {
        assert_eq!(parse_decimal("1.743E+6"), Ok(1_743_000.0));
        assert_eq!(parse_decimal("1,743e6"), Ok(1_743_000.0));
    }

    #[test]
    fn decimal_rejects_garbage() {
        for bad in [
            "",
            " ",
            "abc",
            "missing value",
            "NaN",
            "nan",
            "inf",
            "-inf",
            "Infinity",
            "1.2.3",
            "1,2,3",
            "1,234.5",
            "1.234,5",
            "--1",
            "1e",
            "e5",
            "0x10",
            "5 s",
            "1e999",
        ] {
            assert!(parse_decimal(bad).is_err(), "harus ditolak: {bad:?}");
        }
    }

    // --- parse_applescript -----------------------------------------------------------

    #[test]
    fn parses_plan_example() {
        let got = parse_applescript(&line("Surah X", "1743000", "5.25", "playing"));
        assert_eq!(got, Ok(Some(snap("Surah X", 1_743_000, 5_250, true))));
    }

    #[test]
    fn parses_comma_decimal_position() {
        let got = parse_applescript(&line("Al-Fatihah", "1743000", "12,5", "paused"));
        assert_eq!(got, Ok(Some(snap("Al-Fatihah", 1_743_000, 12_500, false))));
    }

    #[test]
    fn rounds_position_to_nearest_ms() {
        let got = parse_applescript(&line("t", "1000", "0.0004", "playing"))
            .unwrap()
            .unwrap();
        assert_eq!(got.position_ms, 0);
        let got = parse_applescript(&line("t", "1000", "0.0006", "playing"))
            .unwrap()
            .unwrap();
        assert_eq!(got.position_ms, 1);
        let got = parse_applescript(&line("t", "1743000.0", "1.0004", "playing"))
            .unwrap()
            .unwrap();
        assert_eq!((got.duration_ms, got.position_ms), (1_743_000, 1_000));
    }

    #[test]
    fn trailing_newline_from_osascript_is_ignored() {
        let l = line("Surah X", "1000", "1", "playing");
        let want = Ok(Some(snap("Surah X", 1_000, 1_000, true)));
        assert_eq!(parse_applescript(&format!("{l}\n")), want);
        assert_eq!(parse_applescript(&format!("{l}\r\n")), want);
    }

    #[test]
    fn title_with_pipe_quotes_emoji_and_arabic_is_preserved() {
        for t in [
            "A | B | C",
            r#"He said "hi" and 'bye'"#,
            "Surah \u{1F54B} Al-Ikhlas",
            "سورة الإخلاص",
            "  spasi di tepi  ",
            "back\\slash",
        ] {
            let got = parse_applescript(&line(t, "1000", "2", "playing"));
            assert_eq!(got, Ok(Some(snap(t, 1_000, 2_000, true))), "judul {t:?}");
        }
    }

    #[test]
    fn extra_separators_belong_to_the_title() {
        let l = format!("a{FIELD_SEP}b{FIELD_SEP}c{FIELD_SEP}1000{FIELD_SEP}3{FIELD_SEP}playing");
        let got = parse_applescript(&l);
        let title = format!("a{FIELD_SEP}b{FIELD_SEP}c");
        assert_eq!(got, Ok(Some(snap(&title, 1_000, 3_000, true))));
    }

    #[test]
    fn too_few_fields_is_an_error() {
        for bad in [
            "",
            "judul",
            &format!("judul{FIELD_SEP}1000{FIELD_SEP}3"),
            "a|1000|5.25|playing",
        ] {
            assert!(parse_applescript(bad).is_err(), "harus ditolak: {bad:?}");
        }
    }

    #[test]
    fn stopped_means_no_track() {
        assert_eq!(parse_applescript(&line("", "0", "0", "stopped")), Ok(None));
        // stopped menang atas isi lain, bahkan angka sampah
        assert_eq!(
            parse_applescript(&line("x", "zzz", "yyy", "stopped")),
            Ok(None)
        );
        assert_eq!(
            parse_applescript(&line("Surah", "1000", "1", "Stopped")),
            Ok(None)
        );
    }

    #[test]
    fn empty_or_missing_value_title_means_no_track() {
        assert_eq!(
            parse_applescript(&line("", "1000", "1", "playing")),
            Ok(None)
        );
        assert_eq!(
            parse_applescript(&line("   ", "1000", "1", "playing")),
            Ok(None)
        );
        assert_eq!(
            parse_applescript(&line("missing value", "1000", "1", "playing")),
            Ok(None)
        );
        assert_eq!(
            parse_applescript(&line(
                "missing value",
                "missing value",
                "missing value",
                "paused"
            )),
            Ok(None)
        );
    }

    #[test]
    fn unknown_state_is_an_error() {
        assert!(parse_applescript(&line("t", "1000", "1", "rewinding")).is_err());
        assert!(parse_applescript(&line("t", "1000", "1", "")).is_err());
    }

    #[test]
    fn state_is_case_and_space_insensitive_and_accepts_raw_codes() {
        assert_eq!(
            parse_applescript(&line("t", "1000", "1", " Playing ")),
            Ok(Some(snap("t", 1_000, 1_000, true)))
        );
        assert_eq!(
            parse_applescript(&line("t", "1000", "1", "«constant ****kPSp»")),
            Ok(Some(snap("t", 1_000, 1_000, false)))
        );
        assert_eq!(
            parse_applescript(&line("t", "1000", "1", "«constant ****kPSP»")),
            Ok(Some(snap("t", 1_000, 1_000, true)))
        );
        assert_eq!(
            parse_applescript(&line("t", "1000", "1", "«constant ****kPSS»")),
            Ok(None)
        );
    }

    #[test]
    fn garbage_numbers_are_errors() {
        assert!(parse_applescript(&line("t", "abc", "1", "playing")).is_err());
        assert!(parse_applescript(&line("t", "1000", "abc", "playing")).is_err());
        assert!(parse_applescript(&line("t", "missing value", "1", "playing")).is_err());
        assert!(parse_applescript(&line("t", "1000", "missing value", "playing")).is_err());
    }

    #[test]
    fn negative_nan_and_infinite_positions_are_errors() {
        for bad in ["-1", "-0.5", "NaN", "inf", "-inf", "1e999"] {
            assert!(
                parse_applescript(&line("t", "1000", bad, "playing")).is_err(),
                "posisi {bad:?} harus ditolak"
            );
        }
        // -0 bukan negatif
        assert_eq!(
            parse_applescript(&line("t", "1000", "-0", "playing")),
            Ok(Some(snap("t", 1_000, 0, true)))
        );
    }

    #[test]
    fn negative_nan_and_huge_durations_are_errors() {
        for bad in [
            "-1",
            "NaN",
            "inf",
            "1e999",
            "1e13",
            "99999999999999999999999",
        ] {
            assert!(
                parse_applescript(&line("t", bad, "1", "playing")).is_err(),
                "durasi {bad:?} harus ditolak"
            );
        }
    }

    #[test]
    fn large_but_plausible_values_are_kept() {
        // 24 jam
        assert_eq!(
            parse_applescript(&line("t", "86400000", "86399.999", "playing")),
            Ok(Some(snap("t", 86_400_000, 86_399_999, true)))
        );
    }

    #[test]
    fn position_beyond_duration_is_passed_through() {
        // Pemotongan ke durasi adalah urusan engine, bukan parser.
        assert_eq!(
            parse_applescript(&line("t", "1000", "5", "playing")),
            Ok(Some(snap("t", 1_000, 5_000, true)))
        );
    }

    // --- classify_failure ------------------------------------------------------------

    #[test]
    fn permission_denied_maps_to_indonesian_unavailable() {
        for stderr in [
            "execution error: Not authorized to send Apple events to Spotify. (-1743)",
            "0:36: execution error: Spotify got an error: ... (-1743)\n",
        ] {
            match classify_failure(stderr) {
                Err(SourceError::Unavailable(msg)) => {
                    assert!(msg.contains("izin Automation ditolak"), "{msg}");
                    assert!(msg.contains(PERMISSION_HINT), "{msg}");
                }
                other => panic!("diharapkan Unavailable, dapat {other:?}"),
            }
        }
    }

    #[test]
    fn not_running_or_no_track_maps_to_none() {
        assert_eq!(
            classify_failure("execution error: Application isn't running. (-600)"),
            Ok(None)
        );
        assert_eq!(
            classify_failure("Spotify got an error: Can't get current track. (-1728)"),
            Ok(None)
        );
    }

    #[test]
    fn other_failures_keep_the_message() {
        match classify_failure("  execution error: AppleEvent timed out. (-1712)\n") {
            Err(SourceError::Unavailable(msg)) => assert!(msg.contains("(-1712)"), "{msg}"),
            other => panic!("diharapkan Unavailable, dapat {other:?}"),
        }
        assert!(matches!(
            classify_failure(""),
            Err(SourceError::Unavailable(_))
        ));
    }

    #[test]
    fn failure_message_is_truncated() {
        let long = "x".repeat(5_000);
        match classify_failure(&long) {
            Err(SourceError::Unavailable(msg)) => assert!(msg.chars().count() < 600),
            other => panic!("{other:?}"),
        }
    }

    // --- script_lines ----------------------------------------------------------------

    #[test]
    fn script_guards_with_is_running_and_never_launches() {
        let script = script_lines().join("\n");
        let guard = script
            .find("is running")
            .expect("harus ada penjaga is running");
        let tell = script.find("tell application").expect("harus ada tell");
        assert!(guard < tell, "penjaga is running harus mendahului tell");
        assert!(script.contains(BUNDLE_ID));
        for forbidden in [
            "launch",
            "activate",
            "reopen",
            "playpause",
            "next track",
            "previous track",
            "set player",
        ] {
            assert!(
                !script.contains(forbidden),
                "skrip tidak boleh memuat {forbidden:?}"
            );
        }
        assert!(script.contains("character id 31"), "pemisah harus U+001F");
        for needle in [
            "name of current track",
            "duration of current track",
            "player position",
            "player state",
        ] {
            assert!(script.contains(needle), "kurang {needle:?}");
        }
    }
}
