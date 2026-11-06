//! Linimasa ayat: memetakan posisi pemutaran Spotify ke bagian surah.

use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};

/// Toleransi selisih durasi (ms). Panjang MPRIS kadang dibulatkan ke detik.
const DURATION_TOLERANCE_MS: u64 = 1500;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Part {
    Intro,
    Ayah(u16),
}

impl Serialize for Part {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Part::Intro => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("kind", "intro")?;
                map.end()
            }
            Part::Ayah(n) => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("kind", "ayah")?;
                map.serialize_entry("ayah", n)?;
                map.end()
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Timeline {
    pub surah: u8,
    pub spotify_duration_ms: u64,
    /// offset_ms = waktu Spotify − waktu rekaman referensi untuk suara yang sama.
    pub offset_ms: i64,
    pub ayah_starts_ms: Vec<u64>,
}

impl Timeline {
    /// Bagian yang sedang dibacakan pada posisi Spotify tertentu.
    pub fn part_at(&self, spotify_position_ms: u64) -> Part {
        let reference_ms = spotify_position_ms as i64 - self.offset_ms;
        // Jumlah ayat yang sudah dimulai (start <= ref). Negatif => belum ada.
        let started = match u64::try_from(reference_ms) {
            Ok(r) => self.ayah_starts_ms.partition_point(|&start| start <= r),
            Err(_) => 0,
        };
        match started {
            0 => Part::Intro,
            n => Part::Ayah(n as u16),
        }
    }

    /// Apakah durasi track Spotify cocok dengan linimasa ini (±1500 ms).
    pub fn matches_duration(&self, spotify_duration_ms: u64) -> bool {
        self.spotify_duration_ms.abs_diff(spotify_duration_ms) <= DURATION_TOLERANCE_MS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tl() -> Timeline {
        Timeline {
            surah: 112,
            spotify_duration_ms: 30_000,
            offset_ms: 500,
            ayah_starts_ms: vec![2000, 10000, 20000],
        }
    }

    #[test]
    fn part_at_boundaries() {
        let t = tl();
        assert_eq!(t.part_at(0), Part::Intro);
        assert_eq!(t.part_at(2499), Part::Intro);
        assert_eq!(t.part_at(2500), Part::Ayah(1));
        assert_eq!(t.part_at(10499), Part::Ayah(1));
        assert_eq!(t.part_at(10500), Part::Ayah(2));
        assert_eq!(t.part_at(20500), Part::Ayah(3));
        assert_eq!(t.part_at(9_999_999), Part::Ayah(3));
    }

    #[test]
    fn negative_offset_shifts_the_other_way() {
        let mut t = tl();
        t.offset_ms = -500; // Spotify lebih awal dari referensi
        assert_eq!(t.part_at(1499), Part::Intro);
        assert_eq!(t.part_at(1500), Part::Ayah(1));
    }

    #[test]
    fn empty_ayah_list_is_always_intro() {
        let mut t = tl();
        t.ayah_starts_ms.clear();
        assert_eq!(t.part_at(9_999_999), Part::Intro);
    }

    #[test]
    fn matches_duration_has_1500ms_tolerance() {
        let t = tl();
        assert!(t.matches_duration(30_000));
        assert!(t.matches_duration(31_500));
        assert!(t.matches_duration(28_500));
        assert!(!t.matches_duration(31_501));
        assert!(!t.matches_duration(28_499));
        assert!(!t.matches_duration(200_000));
    }

    #[test]
    fn deserializes_timing_json() {
        let json = r#"{"surah":112,"spotify_duration_ms":42000,"offset_ms":2500,"ayah_starts_ms":[2100,6800,10400,13900]}"#;
        let t: Timeline = serde_json::from_str(json).unwrap();
        assert_eq!(t.surah, 112);
        assert_eq!(t.spotify_duration_ms, 42000);
        assert_eq!(t.offset_ms, 2500);
        assert_eq!(t.ayah_starts_ms, vec![2100, 6800, 10400, 13900]);
    }

    #[test]
    fn part_serializes_to_typescript_shape() {
        assert_eq!(
            serde_json::to_string(&Part::Intro).unwrap(),
            r#"{"kind":"intro"}"#
        );
        assert_eq!(
            serde_json::to_string(&Part::Ayah(2)).unwrap(),
            r#"{"kind":"ayah","ayah":2}"#
        );
    }
}
