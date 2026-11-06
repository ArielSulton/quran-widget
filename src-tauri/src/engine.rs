//! Mesin keadaan widget: menggabungkan snapshot, jam posisi, dan linimasa.

use crate::clock::PositionClock;
use crate::playback::Snapshot;
use crate::surah::match_surah;
use crate::timeline::{Part, Timeline};
use serde::Serialize;
use std::time::Instant;

#[derive(PartialEq, Eq, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum WidgetState {
    Waiting,
    Showing {
        surah: u8,
        part: Part,
        playing: bool,
    },
}

pub struct Engine {
    timelines: Vec<Timeline>,
    clock: PositionClock,
}

impl Engine {
    pub fn new(timelines: Vec<Timeline>) -> Self {
        Self {
            timelines,
            clock: PositionClock::new(),
        }
    }

    pub fn tick(&mut self, snap: Option<Snapshot>, now: Instant) -> WidgetState {
        match Self::resolve(&self.timelines, snap.as_ref()) {
            Some((snap, timeline)) => {
                self.clock.observe(snap, now);
                WidgetState::Showing {
                    surah: timeline.surah,
                    part: timeline.part_at(self.clock.position_ms(now)),
                    playing: snap.is_playing,
                }
            }
            None => {
                // Tidak ada track yang dikenali: buang jam agar tidak bocor ke track berikutnya.
                self.clock = PositionClock::new();
                WidgetState::Waiting
            }
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
