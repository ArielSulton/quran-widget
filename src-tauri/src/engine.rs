//! Mesin keadaan widget: menggabungkan snapshot, jam posisi, dan linimasa.

use crate::clock::PositionClock;
use crate::playback::Snapshot;
use crate::surah::match_surah;
use crate::timeline::{Part, Timeline};
use serde::Serialize;
use std::time::Instant;

#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum WidgetState {
    Waiting,
    Showing {
        surah: u8,
        part: Part,
        playing: bool,
    },
}

/// Batas mutlak geser sinkron pengguna (ms).
pub const MAX_USER_SHIFT_MS: i64 = 2000;

pub struct Engine {
    timelines: Vec<Timeline>,
    clock: PositionClock,
    /// Geser sinkron pengguna (ms), sudah dijepit ke ±`MAX_USER_SHIFT_MS`.
    /// Positif => pergantian ayat muncul LEBIH LAMBAT (layar mendahului suara).
    user_shift_ms: i64,
    /// Identitas snapshot terakhir yang diterima `tick` (surah, sedang diputar);
    /// None bila tick terakhir menghasilkan Waiting atau belum pernah ada tick.
    accepted: Option<(u8, bool)>,
}

impl Engine {
    pub fn new(timelines: Vec<Timeline>) -> Self {
        Self {
            timelines,
            clock: PositionClock::new(),
            user_shift_ms: 0,
            accepted: None,
        }
    }

    /// Atur geser sinkron (ms), dijepit ke ±2000. Berlaku pada `tick`/`refresh` berikutnya.
    pub fn set_user_shift_ms(&mut self, ms: i64) {
        self.user_shift_ms = ms.clamp(-MAX_USER_SHIFT_MS, MAX_USER_SHIFT_MS);
    }

    /// Posisi jam pada `now` dikurangi geser; posisi efektif negatif dibulatkan ke 0 (Intro).
    fn effective_position_ms(&self, now: Instant) -> u64 {
        let position = i64::try_from(self.clock.position_ms(now)).unwrap_or(i64::MAX);
        u64::try_from(position.saturating_sub(self.user_shift_ms)).unwrap_or(0)
    }

    pub fn tick(&mut self, snap: Option<Snapshot>, now: Instant) -> WidgetState {
        match Self::resolve(&self.timelines, snap.as_ref()) {
            Some((snap, timeline)) => {
                self.clock.observe(snap, now);
                self.accepted = Some((timeline.surah, snap.is_playing));
                WidgetState::Showing {
                    surah: timeline.surah,
                    part: timeline.part_at(self.effective_position_ms(now)),
                    playing: snap.is_playing,
                }
            }
            None => {
                // Tidak ada track yang dikenali: buang jam agar tidak bocor ke track berikutnya.
                self.clock = PositionClock::new();
                self.accepted = None;
                WidgetState::Waiting
            }
        }
    }

    /// Evaluasi ulang keadaan dari snapshot terakhir yang diterima `tick` dan
    /// prediksi jam pada `now`, TANPA `observe`: dipakai di antara dua poll agar
    /// snapshot basi tidak me-re-anchor jam dan membekukan interpolasi.
    pub fn refresh(&mut self, now: Instant) -> WidgetState {
        let Some((surah, playing)) = self.accepted else {
            return WidgetState::Waiting;
        };
        match self.timelines.iter().find(|t| t.surah == surah) {
            Some(timeline) => WidgetState::Showing {
                surah,
                part: timeline.part_at(self.effective_position_ms(now)),
                playing,
            },
            None => WidgetState::Waiting,
        }
    }

    /// Snapshot + timeline yang cocok, atau None bila widget harus menunggu.
    fn resolve<'a>(
        timelines: &'a [Timeline],
        snap: Option<&'a Snapshot>,
    ) -> Option<(&'a Snapshot, &'a Timeline)> {
        let snap = snap?;
        let surah = match_surah(&snap.title)?;
        let timeline = timelines.iter().find(|t| t.surah == surah)?;
        timeline
            .matches_duration(snap.duration_ms)
            .then_some((snap, timeline))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn timeline(surah: u8, starts: Vec<u64>) -> Timeline {
        Timeline {
            surah,
            spotify_duration_ms: 30_000,
            offset_ms: 0,
            ayah_starts_ms: starts,
        }
    }

    fn engine() -> Engine {
        Engine::new(vec![
            timeline(112, vec![1000, 10_000, 20_000]),
            timeline(113, vec![5000, 15_000]),
        ])
    }

    fn snap(title: &str, duration_ms: u64, position_ms: u64, is_playing: bool) -> Snapshot {
        Snapshot {
            title: title.into(),
            duration_ms,
            position_ms,
            is_playing,
        }
    }

    fn showing(surah: u8, part: Part, playing: bool) -> WidgetState {
        WidgetState::Showing {
            surah,
            part,
            playing,
        }
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn no_snapshot_is_waiting() {
        let mut e = engine();
        assert_eq!(e.tick(None, Instant::now()), WidgetState::Waiting);
    }

    #[test]
    fn unrecognized_title_is_waiting() {
        let mut e = engine();
        let t0 = Instant::now();
        assert_eq!(
            e.tick(Some(snap("Bohemian Rhapsody", 30_000, 12_000, true)), t0),
            WidgetState::Waiting
        );
        assert_eq!(
            e.tick(Some(snap("", 30_000, 12_000, true)), t0),
            WidgetState::Waiting
        );
    }

    #[test]
    fn surah_without_timeline_is_waiting() {
        let mut e = engine();
        // An Nabaa = 78, tidak ada di timeline uji
        assert_eq!(
            e.tick(
                Some(snap("Surah An Nabaa", 30_000, 12_000, true)),
                Instant::now()
            ),
            WidgetState::Waiting
        );
    }

    #[test]
    fn duration_mismatch_is_waiting() {
        let mut e = engine();
        assert_eq!(
            e.tick(
                Some(snap("Surah Al Ikhlas", 200_000, 12_000, true)),
                Instant::now()
            ),
            WidgetState::Waiting
        );
    }

    #[test]
    fn shows_ayah_when_playing() {
        let mut e = engine();
        assert_eq!(
            e.tick(
                Some(snap("Surah Al Ikhlas", 30_000, 12_000, true)),
                Instant::now()
            ),
            showing(112, Part::Ayah(2), true)
        );
    }

    #[test]
    fn paused_keeps_part_with_playing_false() {
        let mut e = engine();
        let t0 = Instant::now();
        assert_eq!(
            e.tick(Some(snap("Surah Al Ikhlas", 30_000, 12_000, true)), t0),
            showing(112, Part::Ayah(2), true)
        );
        assert_eq!(
            e.tick(
                Some(snap("Surah Al Ikhlas", 30_000, 12_000, false)),
                t0 + ms(500)
            ),
            showing(112, Part::Ayah(2), false)
        );
    }

    #[test]
    fn clock_advances_between_ticks_without_new_position() {
        let mut e = engine();
        let t0 = Instant::now();
        e.tick(Some(snap("Surah Al Ikhlas", 30_000, 9_900, true)), t0);
        // posisi sumber kasar (belum berubah, selisih < 150 dari prediksi 10_050): jam yang menentukan
        assert_eq!(
            e.tick(
                Some(snap("Surah Al Ikhlas", 30_000, 9_900, true)),
                t0 + ms(150)
            ),
            showing(112, Part::Ayah(2), true)
        );
    }

    #[test]
    fn position_back_to_zero_is_intro() {
        let mut e = engine();
        let t0 = Instant::now();
        assert_eq!(
            e.tick(Some(snap("Surah Al Ikhlas", 30_000, 25_000, true)), t0),
            showing(112, Part::Ayah(3), true)
        );
        assert_eq!(
            e.tick(Some(snap("Surah Al Ikhlas", 30_000, 0, true)), t0 + ms(500)),
            showing(112, Part::Intro, true)
        );
    }

    #[test]
    fn position_beyond_last_start_is_last_ayah() {
        let mut e = engine();
        assert_eq!(
            e.tick(
                Some(snap("Surah Al Ikhlas", 30_000, 29_900, true)),
                Instant::now()
            ),
            showing(112, Part::Ayah(3), true)
        );
    }

    #[test]
    fn track_change_uses_new_timeline_and_position() {
        let mut e = engine();
        let t0 = Instant::now();
        assert_eq!(
            e.tick(Some(snap("Surah Al Ikhlas", 30_000, 25_000, true)), t0),
            showing(112, Part::Ayah(3), true)
        );
        // 50 ms kemudian: surah 113 pada 16_000 ms (ayat 2 menurut timeline 113)
        assert_eq!(
            e.tick(
                Some(snap("Surah Al Falaq", 30_000, 16_000, true)),
                t0 + ms(50)
            ),
            showing(113, Part::Ayah(2), true)
        );
    }

    #[test]
    fn waiting_then_track_does_not_reuse_stale_clock() {
        let mut e = engine();
        let t0 = Instant::now();
        e.tick(Some(snap("Surah Al Ikhlas", 30_000, 25_000, true)), t0);
        assert_eq!(e.tick(None, t0 + ms(100)), WidgetState::Waiting);
        // track yang sama dimulai dari awal; posisi lama tidak boleh bocor
        assert_eq!(
            e.tick(Some(snap("Surah Al Ikhlas", 30_000, 0, true)), t0 + ms(200)),
            showing(112, Part::Intro, true)
        );
    }

    #[test]
    fn refresh_advances_with_clock_across_a_boundary() {
        let mut e = engine();
        let t0 = Instant::now();
        // 9_950 ms = ayat 1; batas ayat 2 di 10_000 ms.
        assert_eq!(
            e.tick(Some(snap("Surah Al Ikhlas", 30_000, 9_950, true)), t0),
            showing(112, Part::Ayah(1), true)
        );
        assert_eq!(e.refresh(t0 + ms(100)), showing(112, Part::Ayah(2), true));
    }

    #[test]
    fn refresh_keeps_paused_state_frozen() {
        let mut e = engine();
        let t0 = Instant::now();
        assert_eq!(
            e.tick(Some(snap("Surah Al Ikhlas", 30_000, 9_950, false)), t0),
            showing(112, Part::Ayah(1), false)
        );
        assert_eq!(
            e.refresh(t0 + ms(5_000)),
            showing(112, Part::Ayah(1), false)
        );
    }

    #[test]
    fn refresh_after_waiting_tick_is_waiting() {
        let mut e = engine();
        let t0 = Instant::now();
        e.tick(Some(snap("Surah Al Ikhlas", 30_000, 9_950, true)), t0);
        assert_eq!(e.tick(None, t0 + ms(50)), WidgetState::Waiting);
        assert_eq!(e.refresh(t0 + ms(100)), WidgetState::Waiting);
        // Judul tak dikenal juga menghasilkan Waiting, dan refresh mengikutinya.
        e.tick(
            Some(snap("Bohemian Rhapsody", 30_000, 0, true)),
            t0 + ms(150),
        );
        assert_eq!(e.refresh(t0 + ms(200)), WidgetState::Waiting);
    }

    #[test]
    fn refresh_before_any_tick_is_waiting() {
        let mut e = engine();
        assert_eq!(e.refresh(Instant::now()), WidgetState::Waiting);
    }

    #[test]
    fn repeated_refresh_never_reanchors_to_stale_position() {
        let mut e = engine();
        let t0 = Instant::now();
        // Satu poll di 0 ms (Intro), lalu hanya refresh tiap 50 ms selama 10 s.
        e.tick(Some(snap("Surah Al Ikhlas", 30_000, 0, true)), t0);
        let mut last = e.refresh(t0 + ms(50));
        let mut ayah_changes = 0;
        for step in 2..=200u64 {
            let state = e.refresh(t0 + ms(step * 50));
            if state != last {
                ayah_changes += 1;
                last = state;
            }
        }
        // 10_000 ms: tepat di batas ayat 2 => Intro -> 1 (1000 ms) -> 2 (10_000 ms)
        assert_eq!(ayah_changes, 2);
        assert_eq!(last, showing(112, Part::Ayah(2), true));
    }

    /// Linimasa uji geser sinkron: mulai ayat [2000, 10000, 20000], offset 500 ms
    /// => ayat 2 dimulai pada posisi Spotify 10_500 ms tanpa geser.
    fn shifted_engine(shift_ms: i64) -> Engine {
        let mut e = Engine::new(vec![Timeline {
            surah: 112,
            spotify_duration_ms: 30_000,
            offset_ms: 500,
            ayah_starts_ms: vec![2000, 10_000, 20_000],
        }]);
        e.set_user_shift_ms(shift_ms);
        e
    }

    /// Ayat yang tampil untuk posisi Spotify `pos` (tick pertama, jam baru).
    fn part_via_tick(shift_ms: i64, pos: u64) -> Part {
        let mut e = shifted_engine(shift_ms);
        match e.tick(
            Some(snap("Surah Al Ikhlas", 30_000, pos, true)),
            Instant::now(),
        ) {
            WidgetState::Showing { part, .. } => part,
            other => panic!("diharapkan Showing, dapat {other:?}"),
        }
    }

    #[test]
    fn user_shift_zero_changes_ayah_exactly_at_10500() {
        assert_eq!(part_via_tick(0, 10_499), Part::Ayah(1));
        assert_eq!(part_via_tick(0, 10_500), Part::Ayah(2));
    }

    #[test]
    fn positive_shift_delays_the_transition() {
        assert_eq!(part_via_tick(300, 10_799), Part::Ayah(1));
        assert_eq!(part_via_tick(300, 10_800), Part::Ayah(2));
    }

    #[test]
    fn negative_shift_advances_the_transition() {
        assert_eq!(part_via_tick(-300, 10_199), Part::Ayah(1));
        assert_eq!(part_via_tick(-300, 10_200), Part::Ayah(2));
    }

    #[test]
    fn shift_is_clamped_to_plus_minus_2000() {
        assert_eq!(part_via_tick(5000, 12_499), Part::Ayah(1));
        assert_eq!(part_via_tick(5000, 12_500), Part::Ayah(2));
        assert_eq!(part_via_tick(-5000, 8_499), Part::Ayah(1));
        assert_eq!(part_via_tick(-5000, 8_500), Part::Ayah(2));
        assert_eq!(part_via_tick(i64::MIN, 8_500), Part::Ayah(2));
        assert_eq!(part_via_tick(i64::MAX, 12_500), Part::Ayah(2));
    }

    #[test]
    fn positive_shift_larger_than_position_is_intro_not_a_panic() {
        // posisi efektif negatif (100 - 2000) => seperti sebelum rekaman dimulai
        assert_eq!(part_via_tick(2000, 100), Part::Intro);
        assert_eq!(part_via_tick(2000, 0), Part::Intro);
    }

    #[test]
    fn refresh_agrees_with_tick_under_shift() {
        for shift in [0i64, 300, -300, 5000, -5000] {
            let mut e = shifted_engine(shift);
            let t0 = Instant::now();
            // 10_000 ms dilaporkan pada t0; 500 ms kemudian jam di 10_500 ms.
            e.tick(Some(snap("Surah Al Ikhlas", 30_000, 10_000, true)), t0);
            let via_refresh = e.refresh(t0 + ms(500));
            let expected = part_via_tick(shift, 10_500);
            assert_eq!(via_refresh, showing(112, expected, true), "shift {shift}");
        }
    }

    #[test]
    fn changing_the_shift_applies_to_the_next_refresh_without_a_new_tick() {
        let mut e = shifted_engine(0);
        let t0 = Instant::now();
        e.tick(Some(snap("Surah Al Ikhlas", 30_000, 10_600, true)), t0);
        assert_eq!(e.refresh(t0), showing(112, Part::Ayah(2), true));
        e.set_user_shift_ms(300); // efektif 10_300 < 10_500
        assert_eq!(e.refresh(t0), showing(112, Part::Ayah(1), true));
        e.set_user_shift_ms(-300); // efektif 10_900
        assert_eq!(e.refresh(t0), showing(112, Part::Ayah(2), true));
    }

    #[test]
    fn widget_state_serializes_to_typescript_shape() {
        assert_eq!(
            serde_json::to_string(&WidgetState::Waiting).unwrap(),
            r#"{"kind":"waiting"}"#
        );
        assert_eq!(
            serde_json::to_string(&showing(112, Part::Ayah(2), true)).unwrap(),
            r#"{"kind":"showing","surah":112,"part":{"kind":"ayah","ayah":2},"playing":true}"#
        );
        assert_eq!(
            serde_json::to_string(&showing(112, Part::Intro, false)).unwrap(),
            r#"{"kind":"showing","surah":112,"part":{"kind":"intro"},"playing":false}"#
        );
    }
}
