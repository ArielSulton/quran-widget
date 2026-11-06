//! Logika murni sumber Windows (SMTC): pemetaan `TimeSpan`/`DateTime` 100-ns ke milidetik,
//! ekstrapolasi posisi, dan pencocokan AUMID Spotify.
//!
//! Modul ini dikompilasi di semua OS agar dapat diuji di Linux. Panggilan WinRT ada di
//! `windows.rs`.

use std::time::Duration;

/// 1 ms = 10.000 tick 100-ns.
pub const TICKS_PER_MS: i64 = 10_000;

/// Selisih detik antara epoch FILETIME (1601-01-01) dan epoch Unix (1970-01-01).
pub const UNIX_EPOCH_AS_FILETIME_SECS: i64 = 11_644_473_600;

/// Durasi track (ms) dari `EndTime − StartTime` (tick 100-ns). Hasil negatif menjadi 0.
pub fn duration_ms(start_100ns: i64, end_100ns: i64) -> u64 {
    ticks_to_ms(i128::from(end_100ns) - i128::from(start_100ns))
}

/// Tick 100-ns → ms; negatif menjadi 0, nilai terlalu besar tersaturasi.
fn ticks_to_ms(ticks: i128) -> u64 {
    u64::try_from(ticks.max(0) / i128::from(TICKS_PER_MS)).unwrap_or(u64::MAX)
}

/// Posisi saat ini (ms).
///
/// SMTC hanya memperbarui `Position` ketika aplikasi mendorong pembaruan; saat memutar,
/// posisi sekarang = `position + (now − last_updated)`. Aturan:
/// - jeda (`playing == false`): tanpa ekstrapolasi;
/// - `last_updated <= 0` (tidak diketahui): tanpa ekstrapolasi;
/// - `now < last_updated` (jam bergeser): selisih dianggap 0;
/// - `position` negatif dianggap 0;
/// - hasil dipotong ke `end_100ns` bila `end_100ns > 0`;
/// - aman dari overflow (tanpa panic untuk input apa pun).
pub fn position_ms(
    position_100ns: i64,
    last_updated_filetime: i64,
    now_filetime: i64,
    playing: bool,
    end_100ns: i64,
) -> u64 {
    let mut ticks = i128::from(position_100ns.max(0));
    if playing && last_updated_filetime > 0 {
        ticks += (i128::from(now_filetime) - i128::from(last_updated_filetime)).max(0);
    }
    // Tersaturasi ke rentang i64 seperti nilai tick aslinya.
    ticks = ticks.min(i128::from(i64::MAX));
    if end_100ns > 0 {
        ticks = ticks.min(i128::from(end_100ns));
    }
    ticks_to_ms(ticks)
}

/// Gabungan: `(duration_ms, position_ms)` dari properti timeline SMTC mentah. Posisi dan
/// `EndTime` dinyatakan relatif terhadap `StartTime`.
pub fn timeline_to_ms(
    start_100ns: i64,
    end_100ns: i64,
    position_100ns: i64,
    last_updated_filetime: i64,
    now_filetime: i64,
    playing: bool,
) -> (u64, u64) {
    let relative = |t: i64| {
        i64::try_from(i128::from(t) - i128::from(start_100ns)).unwrap_or(if t < start_100ns {
            i64::MIN
        } else {
            i64::MAX
        })
    };
    let end_rel = relative(end_100ns);
    (
        duration_ms(start_100ns, end_100ns),
        position_ms(
            relative(position_100ns),
            last_updated_filetime,
            now_filetime,
            playing,
            end_rel,
        ),
    )
}

/// Waktu sekarang sebagai FILETIME (tick 100-ns sejak 1601-01-01 UTC) dari durasi sejak epoch Unix.
pub fn filetime_from_unix(since_unix_epoch: Duration) -> i64 {
    let ticks = i128::from(UNIX_EPOCH_AS_FILETIME_SECS) * 10_000_000
        + (since_unix_epoch.as_nanos() / 100) as i128;
    i64::try_from(ticks).unwrap_or(i64::MAX)
}

/// Apakah `SourceAppUserModelId` milik aplikasi Spotify (mis. `Spotify.exe` atau
/// `SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify`). Tanpa membedakan huruf besar/kecil.
pub fn is_spotify_aumid(aumid: &str) -> bool {
    aumid.to_ascii_lowercase().contains("spotify")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEC: i64 = 10_000_000; // 1 detik dalam tick 100-ns

    // --- duration_ms -----------------------------------------------------------------

    #[test]
    fn duration_is_end_minus_start_in_ms() {
        assert_eq!(duration_ms(0, 1_743 * SEC), 1_743_000);
        assert_eq!(duration_ms(5 * SEC, 15 * SEC), 10_000);
        assert_eq!(duration_ms(0, 9_999), 0); // < 1 ms memotong ke bawah
        assert_eq!(duration_ms(0, 10_000), 1);
    }

    #[test]
    fn duration_negative_or_extreme_is_safe() {
        assert_eq!(duration_ms(10 * SEC, 5 * SEC), 0);
        assert_eq!(duration_ms(0, 0), 0);
        assert_eq!(duration_ms(i64::MIN, i64::MAX), 1_844_674_407_370_955);
        assert_eq!(duration_ms(i64::MAX, i64::MIN), 0);
    }

    // --- position_ms -----------------------------------------------------------------

    #[test]
    fn paused_has_no_extrapolation() {
        let pos = 30 * SEC;
        assert_eq!(
            position_ms(pos, 1_000 * SEC, 1_010 * SEC, false, 100 * SEC),
            30_000
        );
    }

    #[test]
    fn playing_adds_elapsed_since_last_update() {
        // posisi 30 s dilaporkan 2,5 s yang lalu → 32,5 s
        let last = 1_000 * SEC;
        let now = last + 25_000_000;
        assert_eq!(position_ms(30 * SEC, last, now, true, 100 * SEC), 32_500);
    }

    #[test]
    fn playing_with_zero_elapsed_is_the_reported_position() {
        assert_eq!(position_ms(30 * SEC, 5_000, 5_000, true, 100 * SEC), 30_000);
    }

    #[test]
    fn result_is_clamped_to_duration() {
        let last = 1_000 * SEC;
        assert_eq!(
            position_ms(95 * SEC, last, last + 20 * SEC, true, 100 * SEC),
            100_000
        );
        // posisi yang dilaporkan sudah melewati akhir, saat jeda pun
        assert_eq!(
            position_ms(120 * SEC, last, last, false, 100 * SEC),
            100_000
        );
    }

    #[test]
    fn unknown_end_means_no_clamp() {
        let last = 1_000 * SEC;
        assert_eq!(
            position_ms(95 * SEC, last, last + 20 * SEC, true, 0),
            115_000
        );
    }

    #[test]
    fn clock_skew_negative_elapsed_adds_nothing() {
        let last = 1_000 * SEC;
        let now = last - 5 * SEC;
        assert_eq!(position_ms(30 * SEC, last, now, true, 100 * SEC), 30_000);
    }

    #[test]
    fn unknown_last_updated_means_no_extrapolation() {
        let now = 132_000_000_000_000_000_i64; // ~ tahun 2019 dalam FILETIME
        assert_eq!(position_ms(30 * SEC, 0, now, true, 100 * SEC), 30_000);
        assert_eq!(position_ms(30 * SEC, -1, now, true, 100 * SEC), 30_000);
    }

    #[test]
    fn negative_position_is_zero() {
        assert_eq!(
            position_ms(-5 * SEC, 1_000 * SEC, 1_000 * SEC, true, 100 * SEC),
            0
        );
        assert_eq!(position_ms(i64::MIN, 1, i64::MAX, false, 100 * SEC), 0);
    }

    #[test]
    fn extreme_inputs_do_not_overflow_or_panic() {
        let _ = position_ms(i64::MAX, 1, i64::MAX, true, 0);
        let _ = position_ms(i64::MAX, 1, i64::MAX, true, i64::MAX);
        let _ = position_ms(i64::MIN, i64::MIN, i64::MAX, true, i64::MIN);
        let _ = position_ms(0, i64::MIN, i64::MAX, true, 0);
        // dengan end tak dikenal: tak terpotong, tetapi tetap tersaturasi
        assert_eq!(
            position_ms(i64::MAX, 1, i64::MAX, true, 0),
            (i64::MAX as u64) / 10_000
        );
        // dengan end: dipotong
        assert_eq!(position_ms(i64::MAX, 1, i64::MAX, true, 50 * SEC), 50_000);
    }

    // --- timeline_to_ms --------------------------------------------------------------

    #[test]
    fn timeline_combines_duration_and_position_relative_to_start() {
        // StartTime 0, track 1743 s, posisi 10 s dilaporkan 1 s lalu, sedang memutar → 11 s
        let last = 5_000 * SEC;
        let got = timeline_to_ms(0, 1_743 * SEC, 10 * SEC, last, last + SEC, true);
        assert_eq!(got, (1_743_000, 11_000));
    }

    #[test]
    fn timeline_with_nonzero_start_is_relative() {
        // Start 100 s, End 200 s, Position 130 s → durasi 100 s, posisi 30 s
        let got = timeline_to_ms(100 * SEC, 200 * SEC, 130 * SEC, 0, 0, false);
        assert_eq!(got, (100_000, 30_000));
    }

    #[test]
    fn timeline_paused_does_not_extrapolate() {
        let last = 5_000 * SEC;
        let got = timeline_to_ms(0, 100 * SEC, 10 * SEC, last, last + 50 * SEC, false);
        assert_eq!(got, (100_000, 10_000));
    }

    // --- filetime_from_unix ----------------------------------------------------------

    #[test]
    fn filetime_epoch_conversion() {
        assert_eq!(
            filetime_from_unix(Duration::ZERO),
            UNIX_EPOCH_AS_FILETIME_SECS * SEC
        );
        assert_eq!(
            filetime_from_unix(Duration::from_secs(1)),
            (UNIX_EPOCH_AS_FILETIME_SECS + 1) * SEC
        );
        // 2024-01-01T00:00:00Z = 1704067200 → FILETIME 133485408000000000
        assert_eq!(
            filetime_from_unix(Duration::from_secs(1_704_067_200)),
            133_485_408_000_000_000
        );
        // sub-detik: 250 ms = 2.500.000 tick
        assert_eq!(
            filetime_from_unix(Duration::from_millis(250)),
            UNIX_EPOCH_AS_FILETIME_SECS * SEC + 2_500_000
        );
    }

    #[test]
    fn filetime_saturates_instead_of_overflowing() {
        assert_eq!(filetime_from_unix(Duration::MAX), i64::MAX);
    }

    // --- is_spotify_aumid ------------------------------------------------------------

    #[test]
    fn spotify_aumids_match_case_insensitively() {
        for yes in [
            "Spotify.exe",
            "spotify.exe",
            "SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify",
            "SPOTIFY",
            "C:\\Users\\x\\AppData\\Roaming\\Spotify\\Spotify.exe",
        ] {
            assert!(is_spotify_aumid(yes), "harus cocok: {yes}");
        }
    }

    #[test]
    fn other_aumids_do_not_match() {
        for no in [
            "",
            "chrome.exe",
            "Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic",
            "Spotfy.exe",
            "VLC",
        ] {
            assert!(!is_spotify_aumid(no), "tidak boleh cocok: {no}");
        }
    }
}
