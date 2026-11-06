//! Analisis murni untuk `accuracy_probe`: mengukur seberapa baik posisi yang dilaporkan sumber
//! pemutaran mengikuti jam dinding saat memutar tanpa seek.

/// Satu sampel poll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    /// Waktu dinding sejak probe mulai (ms).
    pub wall_ms: u64,
    /// Posisi yang dilaporkan sumber (ms).
    pub position_ms: u64,
    pub is_playing: bool,
}

/// Ambang lompatan: sama dengan ambang re-anchor jam posisi (Task 5).
pub const JUMP_THRESHOLD_MS: i64 = 150;

/// Ringkasan sebaran (nearest-rank).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spread {
    pub min: u64,
    pub p50: u64,
    pub p90: u64,
    pub max: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub samples: usize,
    pub playing_samples: usize,
    /// Maks |posisi_dilaporkan − (posisi_pertama + waktu_dinding_berlalu)| selama memutar.
    pub max_abs_deviation_ms: u64,
    /// Maks − min deviasi bertanda (tak bergantung pada pilihan titik jangkar).
    pub deviation_span_ms: u64,
    /// Berapa kali posisi turun (Δposisi < 0) di antara dua sampel memutar berurutan.
    pub backward_steps: usize,
    /// Berapa kali |Δposisi − Δdinding| > 150 ms di antara dua sampel memutar berurutan.
    pub jumps: usize,
    /// Berapa kali nilai posisi berubah di antara dua sampel memutar berurutan.
    pub position_changes: usize,
    /// Jeda dinding (ms) antar dua perubahan posisi berurutan.
    pub change_interval_ms: Option<Spread>,
    /// Besar tiap perubahan posisi maju (ms).
    pub change_step_ms: Option<Spread>,
}

/// Nearest-rank pada data terurut (tidak kosong).
fn rank(sorted: &[u64], pct: usize) -> u64 {
    let idx = (sorted.len() * pct).div_ceil(100).max(1) - 1;
    sorted[idx.min(sorted.len() - 1)]
}

fn spread(mut values: Vec<u64>) -> Option<Spread> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    Some(Spread {
        min: values[0],
        p50: rank(&values, 50),
        p90: rank(&values, 90),
        max: values[values.len() - 1],
    })
}

/// Analisis urutan sampel. Hanya sampel `is_playing` yang dinilai; setiap run sampel memutar
/// yang berurutan (dibatasi jeda) adalah satu segmen dengan jangkar sampel pertamanya.
pub fn analyze(samples: &[Sample]) -> Summary {
    let mut sum = Summary {
        samples: samples.len(),
        playing_samples: 0,
        max_abs_deviation_ms: 0,
        deviation_span_ms: 0,
        backward_steps: 0,
        jumps: 0,
        position_changes: 0,
        change_interval_ms: None,
        change_step_ms: None,
    };
    let mut intervals: Vec<u64> = Vec::new();
    let mut steps: Vec<u64> = Vec::new();
    let (mut dev_min, mut dev_max) = (0i128, 0i128);

    let mut prev: Option<Sample> = None; // sampel memutar sebelumnya dalam segmen
    let mut anchor: Option<Sample> = None;
    let mut last_change_wall: Option<u64> = None;

    for &cur in samples {
        if !cur.is_playing {
            prev = None;
            anchor = None;
            last_change_wall = None;
            continue;
        }
        sum.playing_samples += 1;
        let a = *anchor.get_or_insert(cur);
        let dev = i128::from(cur.position_ms)
            - (i128::from(a.position_ms) + i128::from(cur.wall_ms) - i128::from(a.wall_ms));
        dev_min = dev_min.min(dev);
        dev_max = dev_max.max(dev);
        sum.max_abs_deviation_ms = sum
            .max_abs_deviation_ms
            .max(u64::try_from(dev.abs()).unwrap_or(u64::MAX));

        if let Some(p) = prev {
            let d_pos = i128::from(cur.position_ms) - i128::from(p.position_ms);
            let d_wall = i128::from(cur.wall_ms) - i128::from(p.wall_ms);
            if d_pos < 0 {
                sum.backward_steps += 1;
            }
            if (d_pos - d_wall).abs() > i128::from(JUMP_THRESHOLD_MS) {
                sum.jumps += 1;
            }
            if d_pos != 0 {
                sum.position_changes += 1;
                if d_pos > 0 {
                    steps.push(u64::try_from(d_pos).unwrap_or(u64::MAX));
                }
                if let Some(prev_change) = last_change_wall {
                    intervals.push(cur.wall_ms.saturating_sub(prev_change));
                }
                last_change_wall = Some(cur.wall_ms);
            }
        }
        prev = Some(cur);
    }
    sum.deviation_span_ms = u64::try_from(dev_max - dev_min).unwrap_or(u64::MAX);
    sum.change_interval_ms = spread(intervals);
    sum.change_step_ms = spread(steps);
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(wall_ms: u64, position_ms: u64, is_playing: bool) -> Sample {
        Sample {
            wall_ms,
            position_ms,
            is_playing,
        }
    }

    /// Posisi mulus: pos = start + wall, tiap 100 ms.
    fn smooth(n: u64, start: u64) -> Vec<Sample> {
        (0..n).map(|i| s(i * 100, start + i * 100, true)).collect()
    }

    #[test]
    fn empty_input() {
        let r = analyze(&[]);
        assert_eq!(r.samples, 0);
        assert_eq!(r.playing_samples, 0);
        assert_eq!(r.max_abs_deviation_ms, 0);
        assert_eq!(r.change_interval_ms, None);
        assert_eq!(r.change_step_ms, None);
    }

    #[test]
    fn perfectly_smooth_playback_has_zero_deviation() {
        let r = analyze(&smooth(50, 10_000));
        assert_eq!(r.samples, 50);
        assert_eq!(r.playing_samples, 50);
        assert_eq!(r.max_abs_deviation_ms, 0);
        assert_eq!(r.deviation_span_ms, 0);
        assert_eq!(r.backward_steps, 0);
        assert_eq!(r.jumps, 0);
        assert_eq!(r.position_changes, 49);
        let iv = r.change_interval_ms.unwrap();
        assert_eq!((iv.min, iv.p50, iv.p90, iv.max), (100, 100, 100, 100));
        let st = r.change_step_ms.unwrap();
        assert_eq!((st.min, st.max), (100, 100));
    }

    #[test]
    fn start_position_does_not_matter_only_drift_from_first_sample() {
        // posisi awal berapa pun: jangkarnya adalah sampel pertama
        let r = analyze(&smooth(20, 123_456));
        assert_eq!(r.max_abs_deviation_ms, 0);
    }

    #[test]
    fn one_second_steps_measure_granularity_and_deviation() {
        // posisi hanya berubah tiap 1000 ms (loncat 1000), poll tiap 100 ms, 30 sampel (3 s)
        // pos(i) = 10_000 + floor(i/10)*1000 ; dinding = i*100
        let samples: Vec<Sample> = (0..30u64)
            .map(|i| s(i * 100, 10_000 + (i / 10) * 1000, true))
            .collect();
        let r = analyze(&samples);
        // deviasi = pos − (10_000 + wall) = (i/10)*1000 − i*100 → 0 di i=0,10,20; −900 di i=9,19,29
        assert_eq!(r.max_abs_deviation_ms, 900);
        assert_eq!(r.deviation_span_ms, 900);
        assert_eq!(r.position_changes, 2);
        assert_eq!(r.jumps, 2); // dua loncatan +900 (Δpos 1000 − Δdinding 100)
        assert_eq!(r.backward_steps, 0);
        let iv = r.change_interval_ms.unwrap();
        assert_eq!((iv.min, iv.max), (1000, 1000));
        assert_eq!(r.change_step_ms.unwrap().p50, 1000);
    }

    #[test]
    fn backward_step_is_counted_and_is_a_jump() {
        let samples = vec![
            s(0, 10_000, true),
            s(100, 10_100, true),
            s(200, 9_000, true), // mundur
            s(300, 9_100, true),
        ];
        let r = analyze(&samples);
        assert_eq!(r.backward_steps, 1);
        assert_eq!(r.jumps, 1);
        assert_eq!(r.max_abs_deviation_ms, 1_200); // 9_000 − (10_000 + 200) = −1200
    }

    #[test]
    fn jump_threshold_is_strictly_greater_than_150() {
        // Δposisi − Δdinding = 150 → bukan lompatan; 151 → lompatan
        let a = analyze(&[s(0, 0, true), s(100, 250, true)]);
        assert_eq!(a.jumps, 0);
        let b = analyze(&[s(0, 0, true), s(100, 251, true)]);
        assert_eq!(b.jumps, 1);
        // arah mundur sama
        let c = analyze(&[s(0, 1_000, true), s(100, 850, true)]); // Δpos −150 − Δdinding 100 = −250
        assert_eq!(c.jumps, 1);
    }

    #[test]
    fn paused_samples_are_ignored_and_resume_reanchors() {
        let samples = vec![
            s(0, 10_000, true),
            s(100, 10_100, true),
            s(200, 10_100, false), // jeda
            s(5_000, 10_100, false),
            s(5_100, 10_100, true), // lanjut: jangkar baru
            s(5_200, 10_200, true),
        ];
        let r = analyze(&samples);
        assert_eq!(r.samples, 6);
        assert_eq!(r.playing_samples, 4);
        assert_eq!(r.max_abs_deviation_ms, 0);
        assert_eq!(r.jumps, 0);
        assert_eq!(r.backward_steps, 0);
    }

    #[test]
    fn single_playing_sample_has_no_pairs() {
        let r = analyze(&[s(0, 5, true)]);
        assert_eq!(r.playing_samples, 1);
        assert_eq!(r.max_abs_deviation_ms, 0);
        assert_eq!(r.position_changes, 0);
        assert_eq!(r.change_interval_ms, None);
    }

    #[test]
    fn spread_uses_nearest_rank() {
        // perubahan pada t = 100, 300, 600, 1000, 2000 → interval 200, 300, 400, 1000
        let mut v = vec![s(0, 0, true)];
        let mut t = 0;
        let mut p = 0;
        for d in [100u64, 200, 300, 400, 1000] {
            t += d;
            p += d;
            v.push(s(t, p, true));
        }
        let iv = analyze(&v).change_interval_ms.unwrap();
        assert_eq!((iv.min, iv.p50, iv.p90, iv.max), (200, 300, 1000, 1000));
    }

    #[test]
    fn unsorted_wall_clock_does_not_panic() {
        let _ = analyze(&[s(500, 0, true), s(100, 100, true), s(0, u64::MAX, true)]);
    }
}
