//! Jam posisi: menghaluskan posisi kasar dari sumber pemutaran.

use crate::playback::Snapshot;
use std::time::Instant;

/// Selisih (ms) antara posisi teramati dan prediksi yang memicu re-anchor.
/// Disetel ulang di Task 10.
const REANCHOR_THRESHOLD_MS: u64 = 150;

struct Anchor {
    title: String,
    position_ms: u64,
    at: Instant,
    playing: bool,
}

/// Memprediksi posisi pemutaran di antara pengamatan; re-anchor hanya saat
/// pengamatan menyimpang dari prediksi, status play berubah, atau track berganti.
#[derive(Default)]
pub struct PositionClock {
    anchor: Option<Anchor>,
}

impl PositionClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe(&mut self, s: &Snapshot, now: Instant) {
        let keep = match &self.anchor {
            Some(a) if a.title == s.title && a.playing == s.is_playing => {
                Self::predict(a, now).abs_diff(s.position_ms) <= REANCHOR_THRESHOLD_MS
            }
            _ => false,
        };
        if !keep {
            self.anchor = Some(Anchor {
                title: s.title.clone(),
                position_ms: s.position_ms,
                at: now,
                playing: s.is_playing,
            });
        }
    }

    /// Maju jika playing, beku jika dijeda. 0 sebelum pengamatan pertama.
    pub fn position_ms(&self, now: Instant) -> u64 {
        self.anchor.as_ref().map_or(0, |a| Self::predict(a, now))
    }

    fn predict(a: &Anchor, now: Instant) -> u64 {
        if a.playing {
            let elapsed = now.saturating_duration_since(a.at).as_millis() as u64;
            a.position_ms.saturating_add(elapsed)
        } else {
            a.position_ms
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn snap(title: &str, position_ms: u64, is_playing: bool) -> Snapshot {
        Snapshot {
            title: title.into(),
            duration_ms: 100_000,
            position_ms,
            is_playing,
        }
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn advances_while_playing() {
        let t0 = Instant::now();
        let mut c = PositionClock::new();
        c.observe(&snap("a", 1000, true), t0);
        assert_eq!(c.position_ms(t0), 1000);
        assert_eq!(c.position_ms(t0 + ms(400)), 1400);
    }

    #[test]
    fn frozen_while_paused() {
        let t0 = Instant::now();
        let mut c = PositionClock::new();
        c.observe(&snap("a", 1000, false), t0);
        assert_eq!(c.position_ms(t0 + ms(400)), 1000);
    }

    #[test]
    fn keeps_prediction_within_jitter() {
        let t0 = Instant::now();
        let mut c = PositionClock::new();
        c.observe(&snap("a", 1000, true), t0);
        // prediksi di t0+1000 = 2000; teramati 2100 (selisih 100 <= 150): pertahankan prediksi
        c.observe(&snap("a", 2100, true), t0 + ms(1000));
        assert_eq!(c.position_ms(t0 + ms(1000)), 2000);
        assert_eq!(c.position_ms(t0 + ms(1400)), 2400);
    }

    #[test]
    fn reanchors_on_seek() {
        let t0 = Instant::now();
        let mut c = PositionClock::new();
        c.observe(&snap("a", 1000, true), t0);
        // prediksi 2000; teramati 5000 (lompatan)
        c.observe(&snap("a", 5000, true), t0 + ms(1000));
        assert_eq!(c.position_ms(t0 + ms(1000)), 5000);
        assert_eq!(c.position_ms(t0 + ms(1200)), 5200);
        // mundur ke 0 (track diulang)
        c.observe(&snap("a", 0, true), t0 + ms(1500));
        assert_eq!(c.position_ms(t0 + ms(1500)), 0);
    }

    #[test]
    fn reanchors_just_over_threshold() {
        let t0 = Instant::now();
        let mut c = PositionClock::new();
        c.observe(&snap("a", 1000, true), t0);
        c.observe(&snap("a", 2151, true), t0 + ms(1000)); // selisih 151
        assert_eq!(c.position_ms(t0 + ms(1000)), 2151);
    }

    #[test]
    fn reanchors_on_play_state_change() {
        let t0 = Instant::now();
        let mut c = PositionClock::new();
        c.observe(&snap("a", 1000, true), t0);
        // dijeda pada 1100 (prediksi 2000 sedikit meleset tapi < 150 dari 1950)
        c.observe(&snap("a", 1950, false), t0 + ms(1000));
        assert_eq!(c.position_ms(t0 + ms(1000)), 1950);
        assert_eq!(c.position_ms(t0 + ms(3000)), 1950);
        // lanjut main
        c.observe(&snap("a", 1950, true), t0 + ms(3000));
        assert_eq!(c.position_ms(t0 + ms(3500)), 2450);
    }

    #[test]
    fn reanchors_on_track_change() {
        let t0 = Instant::now();
        let mut c = PositionClock::new();
        c.observe(&snap("a", 1000, true), t0);
        // track baru, posisi kebetulan dekat prediksi (selisih 50 ms): tetap re-anchor
        c.observe(&snap("b", 2050, true), t0 + ms(1000));
        assert_eq!(c.position_ms(t0 + ms(1000)), 2050);
    }

    #[test]
    fn position_before_any_observation_is_zero() {
        let c = PositionClock::new();
        assert_eq!(c.position_ms(Instant::now()), 0);
    }
}
