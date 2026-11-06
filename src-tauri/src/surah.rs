//! Pencocok judul trek Spotify -> nomor surah (Juz 30, surah 78..=114).
//!
//! Aturan pencocokan (hanya cocok-persis pada batang nama setelah normalisasi,
//! tanpa fuzzy/edit-distance):
//! 1. huruf kecil; semua karakter non-alfanumerik (spasi, `-`, `_`, titik,
//!    NBSP, dst.) dianggap pemisah kata;
//! 2. kata pertama `surah`/`surat`/`sura` dibuang bila ada;
//! 3. sisa kata digabung tanpa spasi, sehingga `Al Sharh`, `Al-Sharh` dan
//!    `AlSharh` setara;
//! 4. hasilnya harus sama persis dengan salah satu nama pada [`NAMES`].
//!
//! Konsekuensi: teks tambahan setelah nama (mis. `Surah Al Ikhlas (Remastered)`)
//! TIDAK dikenali dan menghasilkan `None`, karena durasi/versi lain belum
//! tervalidasi terhadap data waktu ayat.

/// Nama batang (tanpa awalan "Surah") sesuai ejaan judul Spotify asli di
/// `tests/fixtures/spotify_titles.json`, dipasangkan dengan nomor surah.
const NAMES: [(u8, &str); 37] = [
    (78, "An Nabaa"),
    (79, "An Naziat"),
    (80, "Abasa"),
    (81, "At Takwir"),
    (82, "Al Infitar"),
    (83, "Al Mutaffifeen"),
    (84, "Al Inshiqaq"),
    (85, "Al Buruj"),
    (86, "At Tariq"),
    (87, "Al Ala"),
    (88, "Al Gashiya"),
    (89, "Al Fajr"),
    (90, "Al Balad"),
    (91, "Ash Shams"),
    (92, "Al Lail"),
    (93, "Ad Dhuha"),
    (94, "Al Sharh"),
    (95, "At Tin"),
    (96, "Al Alaq"),
    (97, "Al Qadr"),
    (98, "Al Baiyina"),
    (99, "Al Zalzalah"),
    (100, "Al Adiyat"),
    (101, "Al Qaria"),
    (102, "At Takathur"),
    (103, "Al Asr"),
    (104, "Al Humazah"),
    (105, "Al Fil"),
    (106, "Quraish"),
    (107, "Al Maun"),
    (108, "Al Kauthar"),
    (109, "Al Kafirun"),
    (110, "An Nasr"),
    (111, "Al Masad"),
    (112, "Al Ikhlas"),
    (113, "Al Falaq"),
    (114, "An Nas"),
];

const PREFIXES: [&str; 3] = ["surah", "surat", "sura"];

/// Kunci kanonik: huruf kecil, tanpa awalan "surah"/"surat"/"sura", tanpa
/// pemisah. Kosong bila tidak ada nama tersisa.
fn canonical(s: &str) -> String {
    let lowered = s.to_lowercase();
    let mut words = lowered
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .peekable();
    if words.peek().is_some_and(|w| PREFIXES.contains(w)) {
        words.next();
    }
    words.collect()
}

/// Mengembalikan nomor surah (78..=114) untuk judul trek, atau `None`.
pub fn match_surah(title: &str) -> Option<u8> {
    let key = canonical(title);
    if key.is_empty() {
        return None;
    }
    NAMES
        .iter()
        .find(|(_, name)| canonical(name) == key)
        .map(|(n, _)| *n)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../tests/fixtures/spotify_titles.json");

    fn fixture() -> Vec<(u8, String)> {
        let v: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture JSON valid");
        v.as_array()
            .expect("fixture adalah array")
            .iter()
            .map(|e| {
                (
                    e["surah"].as_u64().expect("field surah") as u8,
                    e["title"].as_str().expect("field title").to_string(),
                )
            })
            .collect()
    }

    fn title_of(surah: u8) -> String {
        fixture().into_iter().find(|(s, _)| *s == surah).unwrap().1
    }

    #[test]
    fn fixture_covers_all_37_surahs() {
        let f = fixture();
        assert_eq!(f.len(), 37);
        for (i, (s, _)) in f.iter().enumerate() {
            assert_eq!(*s as usize, 78 + i);
        }
    }

    #[test]
    fn every_fixture_title_matches_its_surah() {
        for (surah, title) in fixture() {
            assert_eq!(match_surah(&title), Some(surah), "judul: {title:?}");
        }
    }

    #[test]
    fn unrelated_title_is_none() {
        assert_eq!(match_surah("Bohemian Rhapsody"), None);
    }

    #[test]
    fn empty_and_blank_are_none() {
        assert_eq!(match_surah(""), None);
        assert_eq!(match_surah("   "), None);
        assert_eq!(match_surah("Surah"), None);
        assert_eq!(match_surah("---"), None);
    }

    #[test]
    fn surahs_outside_juz_30_are_none() {
        for t in [
            "Surah Al Baqarah",
            "Al-Baqarah",
            "Surah Al Kahf",
            "Surah Yasin",
            "Surah Al Fatiha",
            "Surah An Nahl",
            "Surah An Nur",
            "Surah Al Mulk",
        ] {
            assert_eq!(match_surah(t), None, "judul: {t:?}");
        }
    }

    #[test]
    fn case_variants() {
        assert_eq!(match_surah("surah al ikhlas"), Some(112));
        assert_eq!(match_surah("SURAH AL IKHLAS"), Some(112));
        assert_eq!(match_surah("SURAT AL-MASAD"), Some(111));
        assert_eq!(match_surah("surat al-masad"), Some(111));
        assert_eq!(match_surah("Surah AN NABAA"), Some(78));
    }

    #[test]
    fn hyphen_space_and_article_variants() {
        // Judul asli: "Surah Al Sharh" / "Surah Al-Humazah" / "Surah An-Nasr".
        assert_eq!(match_surah("Surah Al-Sharh"), Some(94));
        assert_eq!(match_surah("Surah Al Humazah"), Some(104));
        assert_eq!(match_surah("Surah AlHumazah"), Some(104));
        assert_eq!(match_surah("Surah An Nasr"), Some(110));
        assert_eq!(match_surah("Surah An-Nas"), Some(114));
        assert_eq!(match_surah("Surah At-Takwir"), Some(81));
        assert_eq!(match_surah("Surat Al Masad"), Some(111));
    }

    #[test]
    fn surah_prefix_variants() {
        assert_eq!(match_surah("Surah Al Ikhlas"), Some(112));
        assert_eq!(match_surah("Surat Al Ikhlas"), Some(112));
        assert_eq!(match_surah("Sura Al Ikhlas"), Some(112));
        assert_eq!(match_surah("Al Ikhlas"), Some(112));
        assert_eq!(match_surah("Surah Quraish"), Some(106));
        assert_eq!(match_surah("Quraish"), Some(106));
    }

    #[test]
    fn whitespace_and_punctuation_variants() {
        assert_eq!(match_surah("  Surah   Al   Ikhlas  "), Some(112));
        assert_eq!(match_surah("Surah\tAl\u{a0}Ikhlas"), Some(112));
        assert_eq!(match_surah("Surah Al-Ikhlas."), Some(112));
        assert_eq!(match_surah("Surah Al_Ikhlas"), Some(112));
    }

    #[test]
    fn near_collisions_stay_distinct() {
        assert_eq!(match_surah("Surah Al Ala"), Some(87));
        assert_eq!(match_surah("Surah Al Alaq"), Some(96));
        assert_eq!(match_surah("Surah An Nas"), Some(114));
        assert_eq!(match_surah("Surah An Nasr"), Some(110));
        assert_eq!(match_surah("Surah Al Fil"), Some(105));
        assert_eq!(match_surah("Surah Al Falaq"), Some(113));
        assert_eq!(match_surah("Surah Al Fajr"), Some(89));
        assert_eq!(match_surah("Surah Al Lail"), Some(92));
        assert_eq!(match_surah("Surah Al Asr"), Some(103));
        assert_eq!(match_surah("Surah Al Adiyat"), Some(100));
        // Potongan/imbuhan nama bukan padanan.
        assert_eq!(match_surah("Surah Al A"), None);
        assert_eq!(match_surah("Surah Al Alaqq"), None);
        assert_eq!(match_surah("Surah An Na"), None);
        assert_eq!(match_surah("Surah Al Fi"), None);
        assert_eq!(match_surah("Surah Al"), None);
    }

    #[test]
    fn trailing_extra_text_is_rejected() {
        // Hanya cocok-persis setelah normalisasi; teks tambahan = tidak dikenal
        // (durasi/versi lain belum divalidasi).
        assert_eq!(match_surah("Surah Al Ikhlas (Remastered)"), None);
        assert_eq!(match_surah("Surah Al Ikhlas - Yasser Al-Dosari"), None);
        assert_eq!(match_surah("Surah Al Ikhlas 2"), None);
        assert_eq!(match_surah("Surah Al Ikhlas Al Falaq"), None);
    }

    #[test]
    fn canonical_names_are_unique_and_in_range() {
        let mut keys: Vec<String> = NAMES.iter().map(|(_, n)| canonical(n)).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), NAMES.len());
        for (i, (n, _)) in NAMES.iter().enumerate() {
            assert_eq!(*n as usize, 78 + i);
        }
    }

    #[test]
    fn title_of_helper_sanity() {
        assert_eq!(match_surah(&title_of(111)), Some(111));
    }
}
