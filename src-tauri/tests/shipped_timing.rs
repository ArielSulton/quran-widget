//! Integration test over the REAL shipped `data/timing/` directory (not fixtures).

use std::collections::BTreeMap;
use std::path::PathBuf;

use quran_widget_lib::runtime::load_timelines_collect;
use quran_widget_lib::timeline::Part;

/// Surahs deliberately absent in v1 (different Spotify recording). Keep in sync with
/// `tools/check_timing.py:EXCLUDED`.
const EXCLUDED: [u8; 2] = [101, 104];

fn repo_file(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join(rel)
}

fn ayat_counts() -> BTreeMap<u8, usize> {
    let text = std::fs::read_to_string(repo_file("data/quran/juz30.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    v.as_object()
        .unwrap()
        .iter()
        .map(|(k, s)| (k.parse().unwrap(), s["ayat"].as_array().unwrap().len()))
        .collect()
}

#[test]
fn shipped_timing_dir_loads_cleanly() {
    let (timelines, errors) = load_timelines_collect(&repo_file("data/timing"));
    assert!(errors.is_empty(), "skipped/invalid timing files: {errors:?}");
    assert_eq!(timelines.len(), 35, "expected 35 timelines (Juz 30 minus 101, 104)");

    let surahs: Vec<u8> = timelines.iter().map(|t| t.surah).collect();
    let expected: Vec<u8> = (78..=114).filter(|n| !EXCLUDED.contains(n)).collect();
    assert_eq!(surahs, expected);

    let counts = ayat_counts();
    for t in &timelines {
        assert_eq!(
            t.ayah_starts_ms.len(),
            counts[&t.surah],
            "surah {}: ayah count differs from juz30.json",
            t.surah
        );
    }
}

#[test]
fn shipped_timelines_start_sanely() {
    let (timelines, _) = load_timelines_collect(&repo_file("data/timing"));
    assert!(!timelines.is_empty());
    for t in &timelines {
        // Position 0 must not panic (negative reference time => intro).
        let _ = t.part_at(0);
        assert!(
            t.spotify_duration_ms > 0 && t.offset_ms.abs() <= 10_000,
            "surah {}: implausible duration/offset",
            t.surah
        );
        // Sanity: the first ayah is reached within 30 s after the offset.
        let first = t.ayah_starts_ms[0] as i64;
        assert!(first < 30_000, "surah {}: first ayah starts at {first} ms", t.surah);
        let at_first = (first + t.offset_ms).max(0) as u64;
        assert_eq!(
            t.part_at(at_first),
            Part::Ayah(1),
            "surah {}: first ayah not reached at spotify {at_first} ms",
            t.surah
        );
        let before = (first + t.offset_ms - 1).max(0) as u64;
        if first + t.offset_ms >= 1 {
            assert_eq!(t.part_at(before), Part::Intro, "surah {}", t.surah);
        }
    }
}
