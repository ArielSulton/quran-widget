//! Runtime: loop polling sumber pemutaran + interpolasi, dan pemuatan linimasa.

use crate::engine::{Engine, WidgetState};
use crate::playback::{PlaybackSource, Snapshot, SourceError};
use crate::timeline::Timeline;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

/// Irama loop: `poll` = jarak antar pembacaan sumber (di thread poller),
/// `tick` = jarak antar evaluasi ulang (refresh) di thread loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoopTiming {
    pub poll: Duration,
    pub tick: Duration,
}

impl Default for LoopTiming {
    fn default() -> Self {
        Self {
            poll: Duration::from_millis(250),
            tick: Duration::from_millis(50),
        }
    }
}

/// Bila tidak ada hasil poll segar lebih lama dari ini, widget kembali ke Waiting
/// (Spotify menggantung / D-Bus macet) alih-alih terus berinterpolasi pada data basi.
pub const STALE_AFTER: Duration = Duration::from_secs(2);

/// Waktu tunggu maksimum saat berhenti agar thread poller selesai sebelum dilepas.
const JOIN_GRACE: Duration = Duration::from_millis(200);

type SharedClock = Arc<dyn Fn() -> Instant + Send + Sync>;

/// Hasil satu poll beserta instan yang dikaitkan dengan posisinya.
struct PollResult {
    snap: Option<Snapshot>,
    /// Titik tengah antara sampel jam sebelum dan sesudah `poll()`: urutan baca
    /// internal sumber tidak diketahui, jadi titik tengah meminimalkan galat terburuk.
    at: Instant,
}

/// Membungkus sumber: sampel jam di sekitar `poll()` dan petakan galat ke Waiting.
struct Poller {
    source: Box<dyn PlaybackSource>,
    clock: SharedClock,
    last_error: Option<SourceError>,
}

impl Poller {
    fn new(source: Box<dyn PlaybackSource>, clock: SharedClock) -> Self {
        Self {
            source,
            clock,
            last_error: None,
        }
    }

    fn poll_once(&mut self) -> PollResult {
        let before = (self.clock)();
        let polled = self.source.poll();
        let after = (self.clock)();
        let at = before + after.saturating_duration_since(before) / 2;
        // Galat sumber diperlakukan seperti Ok(None): widget menunggu.
        let snap = match polled {
            Ok(snap) => {
                self.last_error = None;
                snap
            }
            Err(e) => {
                if self.last_error.as_ref() != Some(&e) {
                    eprintln!("[runtime] {e}");
                    self.last_error = Some(e);
                }
                None
            }
        };
        PollResult { snap, at }
    }
}

/// Satu langkah loop yang deterministik (waktu dan hasil poll disuntikkan).
struct Stepper {
    engine: Engine,
    /// Geser sinkron pengguna yang dibagikan dengan thread UI (perintah Tauri).
    shift: Arc<AtomicI64>,
    /// Nilai geser yang sudah diteruskan ke `engine`.
    applied_shift: i64,
    started: Option<Instant>,
    last_fresh: Option<Instant>,
    last_emitted: Option<WidgetState>,
}

impl Stepper {
    fn new(engine: Engine, shift: Arc<AtomicI64>) -> Self {
        Self {
            engine,
            shift,
            applied_shift: 0,
            started: None,
            last_fresh: None,
            // Keadaan awal widget adalah Waiting: tidak perlu dipancarkan ulang.
            last_emitted: Some(WidgetState::Waiting),
        }
    }

    /// `fresh` = hasil poll baru sejak langkah sebelumnya (bila ada).
    fn step(
        &mut self,
        now: Instant,
        fresh: Option<PollResult>,
        emit: &mut impl FnMut(WidgetState),
    ) {
        let started = *self.started.get_or_insert(now);
        // Geser baru langsung berlaku di langkah ini (juga tanpa poll segar): semua cabang
        // di bawah menghitung keadaan dengan geser terbaru, dan perubahan dipancarkan.
        let shift = self.shift.load(Ordering::SeqCst);
        if shift != self.applied_shift {
            self.applied_shift = shift;
            self.engine.set_user_shift_ms(shift);
        }
        let state = if let Some(result) = fresh {
            self.last_fresh = Some(now);
            // Jangkarkan di instan baca (titik tengah), lalu prediksi sampai `now`.
            self.engine.tick(result.snap, result.at);
            self.engine.refresh(now)
        } else if now.saturating_duration_since(self.last_fresh.unwrap_or(started)) > STALE_AFTER {
            self.engine.tick(None, now)
        } else {
            // Di antara dua poll: jangan masukkan snapshot basi, cukup interpolasi.
            self.engine.refresh(now)
        };
        if self.last_emitted.as_ref() != Some(&state) {
            self.last_emitted = Some(state.clone());
            emit(state);
        }
    }
}

/// Jalankan loop dengan irama bawaan (poll 250 ms, tick 50 ms) sampai `stop` diset.
///
/// `shift` = geser sinkron pengguna (ms, boleh negatif) yang dibaca tiap iterasi dan
/// diteruskan ke `Engine::set_user_shift_ms` bila berubah; perubahan langsung dievaluasi
/// ulang dan dipancarkan tanpa menunggu poll berikutnya.
pub fn run_loop(
    source: Box<dyn PlaybackSource>,
    engine: Engine,
    emit: impl FnMut(WidgetState),
    clock: impl Fn() -> Instant + Send + Sync + 'static,
    stop: Arc<AtomicBool>,
    shift: Arc<AtomicI64>,
) {
    run_loop_with(
        LoopTiming::default(),
        source,
        engine,
        emit,
        clock,
        stop,
        shift,
    )
}

/// Seperti `run_loop` dengan irama yang dapat diatur (dipakai tes).
///
/// `source.poll()` berjalan di thread sendiri dan mengirim hasilnya (dengan instan
/// titik tengah) ke thread loop, yang tetap `refresh` tiap `tick` meski poll lambat.
/// Saat `stop` diset, thread poller diberi `JOIN_GRACE` untuk selesai lalu di-join; bila
/// `poll()` sedang menggantung, thread itu DILEPAS (detach) agar loop tetap kembali cepat;
/// ia berakhir sendiri begitu `poll()` kembali dan melihat `stop`.
pub fn run_loop_with(
    timing: LoopTiming,
    source: Box<dyn PlaybackSource>,
    engine: Engine,
    emit: impl FnMut(WidgetState),
    clock: impl Fn() -> Instant + Send + Sync + 'static,
    stop: Arc<AtomicBool>,
    shift: Arc<AtomicI64>,
) {
    let mut emit = emit;
    let clock: SharedClock = Arc::new(clock);
    let (tx, rx) = mpsc::channel();
    let done = Arc::new(AtomicBool::new(false));

    let poller_handle = {
        let (clock, stop, done) = (clock.clone(), stop.clone(), done.clone());
        std::thread::spawn(move || {
            poller_loop(Poller::new(source, clock), tx, timing, &stop);
            done.store(true, Ordering::SeqCst);
        })
    };

    let mut stepper = Stepper::new(engine, shift);
    while !stop.load(Ordering::SeqCst) {
        let fresh = rx.try_iter().last();
        stepper.step(clock(), fresh, &mut emit);
        std::thread::sleep(timing.tick);
    }

    let deadline = Instant::now() + JOIN_GRACE;
    while !done.load(Ordering::SeqCst) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    if done.load(Ordering::SeqCst) {
        let _ = poller_handle.join();
    } // else: poll() menggantung; thread dilepas (lihat dokumentasi di atas).
}

fn poller_loop(
    mut poller: Poller,
    tx: mpsc::Sender<PollResult>,
    timing: LoopTiming,
    stop: &AtomicBool,
) {
    while !stop.load(Ordering::SeqCst) {
        let started = (poller.clock)();
        let result = poller.poll_once();
        if tx.send(result).is_err() {
            return; // thread loop sudah selesai
        }
        let next = started + timing.poll;
        while !stop.load(Ordering::SeqCst) {
            let now = (poller.clock)();
            if now >= next {
                break;
            }
            std::thread::sleep(timing.tick.min(next - now));
        }
    }
}

/// Periksa linimasa: tidak kosong, ayat naik ketat, surah 78..=114.
pub fn validate_timeline(t: &Timeline) -> Result<(), String> {
    if !(78..=114).contains(&t.surah) {
        return Err(format!("surah {} di luar rentang 78..=114", t.surah));
    }
    if t.ayah_starts_ms.is_empty() {
        return Err("ayah_starts_ms kosong".into());
    }
    if let Some(w) = t.ayah_starts_ms.windows(2).find(|w| w[0] >= w[1]) {
        return Err(format!(
            "ayah_starts_ms tidak naik ketat ({} lalu {})",
            w[0], w[1]
        ));
    }
    Ok(())
}

/// Muat setiap `NNN.json` di `dir`. Mengembalikan linimasa valid (urut surah) dan
/// daftar pesan galat per berkas. Direktori hilang/kosong => kosong tanpa galat.
pub fn load_timelines_collect(dir: &Path) -> (Vec<Timeline>, Vec<String>) {
    let mut timelines = Vec::new();
    let mut errors = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (timelines, errors),
        Err(e) => {
            errors.push(format!("{}: tidak dapat dibaca: {e}", dir.display()));
            return (timelines, errors);
        }
    };
    let mut paths: Vec<_> = entries
        .filter_map(|entry| match entry {
            Ok(entry) => Some(entry.path()),
            Err(e) => {
                errors.push(format!("{}: entri tidak terbaca: {e}", dir.display()));
                None
            }
        })
        .filter(|p| is_timing_file_name(p))
        .collect();
    paths.sort();
    for path in paths {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let parsed = std::fs::read_to_string(&path)
            .map_err(|e| format!("tidak dapat dibaca: {e}"))
            .and_then(|text| {
                serde_json::from_str::<Timeline>(&text)
                    .map_err(|e| format!("JSON tidak valid: {e}"))
            })
            .and_then(|t| {
                validate_timeline(&t)?;
                if format!("{:03}.json", t.surah) != name {
                    return Err(format!("surah {} tidak cocok dengan nama berkas", t.surah));
                }
                Ok(t)
            });
        match parsed {
            Ok(t) => timelines.push(t),
            Err(e) => errors.push(format!("{name}: {e}")),
        }
    }
    timelines.sort_by_key(|t| t.surah);
    (timelines, errors)
}

/// Nama berkas linimasa: tepat tiga digit + `.json`.
fn is_timing_file_name(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name.strip_suffix(".json")
        .is_some_and(|stem| stem.len() == 3 && stem.bytes().all(|b| b.is_ascii_digit()))
}

/// Seperti `load_timelines_collect`, galat dicatat ke stderr (tidak ada yang dilewati diam-diam).
pub fn load_timelines(dir: &Path) -> Vec<Timeline> {
    let (timelines, errors) = load_timelines_collect(dir);
    for e in &errors {
        eprintln!("[timing] {e}");
    }
    timelines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playback::Snapshot;
    use crate::timeline::Part;
    use std::collections::VecDeque;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Mutex;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn timeline(surah: u8, starts: Vec<u64>) -> Timeline {
        Timeline {
            surah,
            spotify_duration_ms: 30_000,
            offset_ms: 0,
            ayah_starts_ms: starts,
        }
    }

    fn engine() -> Engine {
        Engine::new(vec![timeline(112, vec![1000, 10_000, 20_000])])
    }

    fn ikhlas(position_ms: u64, playing: bool) -> Snapshot {
        Snapshot {
            title: "Surah Al Ikhlas".into(),
            duration_ms: 30_000,
            position_ms,
            is_playing: playing,
        }
    }

    fn showing(part: Part, playing: bool) -> WidgetState {
        WidgetState::Showing {
            surah: 112,
            part,
            playing,
        }
    }

    /// Sumber palsu: mengembalikan hasil berurutan; setelah habis => Ok(None).
    struct FakeSource {
        results: VecDeque<Result<Option<Snapshot>, SourceError>>,
        polls: Arc<AtomicUsize>,
    }

    impl FakeSource {
        fn new(
            results: Vec<Result<Option<Snapshot>, SourceError>>,
        ) -> (Box<Self>, Arc<AtomicUsize>) {
            let polls = Arc::new(AtomicUsize::new(0));
            (
                Box::new(Self {
                    results: results.into(),
                    polls: polls.clone(),
                }),
                polls,
            )
        }
    }

    impl PlaybackSource for FakeSource {
        fn poll(&mut self) -> Result<Option<Snapshot>, SourceError> {
            self.polls.fetch_add(1, Ordering::SeqCst);
            self.results.pop_front().unwrap_or(Ok(None))
        }
    }

    /// Harness deterministik yang meniru satu iterasi loop: jam palsu, `Poller`, `Stepper`.
    struct Sim {
        clock: Arc<Mutex<Instant>>,
        poller: Poller,
        stepper: Stepper,
        shift: Arc<AtomicI64>,
        emitted: Vec<WidgetState>,
    }

    impl Sim {
        fn new(source: Box<dyn PlaybackSource>, clock: Arc<Mutex<Instant>>) -> Self {
            let shared: SharedClock = {
                let clock = clock.clone();
                Arc::new(move || *clock.lock().unwrap())
            };
            let shift = Arc::new(AtomicI64::new(0));
            Self {
                clock,
                poller: Poller::new(source, shared),
                stepper: Stepper::new(engine(), shift.clone()),
                shift,
                emitted: Vec::new(),
            }
        }

        /// Satu langkah pada waktu `t`; `poll` = apakah ada hasil poll segar langkah ini.
        fn step(&mut self, t: Instant, poll: bool) {
            *self.clock.lock().unwrap() = t;
            let fresh = poll.then(|| self.poller.poll_once());
            let now = *self.clock.lock().unwrap(); // poll() bisa memajukan jam
            let emitted = &mut self.emitted;
            self.stepper.step(now, fresh, &mut |s| emitted.push(s));
        }
    }

    fn sim(
        results: Vec<Result<Option<Snapshot>, SourceError>>,
    ) -> (Sim, Instant, Arc<AtomicUsize>) {
        let t0 = Instant::now();
        let (source, polls) = FakeSource::new(results);
        (Sim::new(source, Arc::new(Mutex::new(t0))), t0, polls)
    }

    #[test]
    fn emits_intro_ayah_waiting_and_treats_error_as_none() {
        let (mut sim, t0, polls) = sim(vec![
            Ok(Some(ikhlas(0, true))),
            Ok(Some(ikhlas(12_000, true))),
            Ok(None),
            Err(SourceError::Unavailable("dbus mati".into())),
        ]);
        for i in 0..4 {
            sim.step(t0 + ms(i * 250), true);
        }
        assert_eq!(polls.load(Ordering::SeqCst), 4);
        assert_eq!(
            sim.emitted,
            vec![
                showing(Part::Intro, true),
                showing(Part::Ayah(2), true),
                WidgetState::Waiting,
            ]
        );
    }

    #[test]
    fn source_error_alone_never_panics_and_yields_waiting() {
        let (mut sim, t0, _) = sim(vec![
            Err(SourceError::Unavailable("a".into())),
            Err(SourceError::Unavailable("a".into())),
            Err(SourceError::Unavailable("b".into())),
        ]);
        for i in 0..3 {
            sim.step(t0 + ms(i * 250), true);
        }
        assert!(sim.emitted.is_empty(), "{:?}", sim.emitted);
    }

    #[test]
    fn no_duplicate_emits_for_unchanged_state() {
        let (mut sim, t0, _) = sim(vec![
            Ok(Some(ikhlas(2_000, true))),
            Ok(Some(ikhlas(2_250, true))),
            Ok(Some(ikhlas(2_500, true))),
        ]);
        for i in 0..15 {
            sim.step(t0 + ms(i * 50), i % 5 == 0);
        }
        assert_eq!(sim.emitted, vec![showing(Part::Ayah(1), true)]);
    }

    #[test]
    fn refresh_between_polls_emits_boundary_crossing_before_next_poll() {
        // Poll di 9_900 ms (ayat 1); batas ayat 2 di 10_000 ms terlewati 100 ms kemudian.
        let (mut sim, t0, polls) = sim(vec![Ok(Some(ikhlas(9_900, true)))]);
        sim.step(t0, true);
        assert_eq!(sim.emitted, vec![showing(Part::Ayah(1), true)]);
        sim.step(t0 + ms(50), false);
        assert_eq!(sim.emitted.len(), 1);
        sim.step(t0 + ms(100), false);
        assert_eq!(
            sim.emitted,
            vec![showing(Part::Ayah(1), true), showing(Part::Ayah(2), true)]
        );
        // Hanya satu poll sejauh ini: perubahan berasal dari interpolasi.
        assert_eq!(polls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn paused_stays_frozen_between_polls() {
        let (mut sim, t0, _) = sim(vec![Ok(Some(ikhlas(9_950, false)))]);
        for i in 0..5 {
            sim.step(t0 + ms(i * 50), i == 0);
        }
        assert_eq!(sim.emitted, vec![showing(Part::Ayah(1), false)]);
    }

    #[test]
    fn stale_after_two_seconds_without_fresh_poll_goes_waiting_then_recovers() {
        let (mut sim, t0, _) = sim(vec![
            Ok(Some(ikhlas(2_000, true))),
            Ok(Some(ikhlas(12_000, true))),
        ]);
        sim.step(t0, true);
        // Tanpa poll segar: tepat 2000 ms belum basi (batas ketat '>' STALE_AFTER).
        for i in 1..=40 {
            sim.step(t0 + ms(i * 50), false);
        }
        assert_eq!(STALE_AFTER, ms(2_000));
        assert_eq!(sim.emitted, vec![showing(Part::Ayah(1), true)]);
        // 2050 ms: basi => Waiting; tetap Waiting seterusnya (tidak ada emit ganda).
        sim.step(t0 + ms(2_050), false);
        sim.step(t0 + ms(2_100), false);
        assert_eq!(
            sim.emitted,
            vec![showing(Part::Ayah(1), true), WidgetState::Waiting]
        );
        // Hasil segar pulih ke keadaan yang benar.
        sim.step(t0 + ms(2_150), true);
        assert_eq!(
            sim.emitted,
            vec![
                showing(Part::Ayah(1), true),
                WidgetState::Waiting,
                showing(Part::Ayah(2), true)
            ]
        );
    }

    #[test]
    fn never_polled_source_is_waiting_not_a_panic() {
        // Tanpa hasil sama sekali selama > 2 s: tetap Waiting (keadaan awal, tanpa emit).
        let (mut sim, t0, _) = sim(vec![]);
        for i in 0..60 {
            sim.step(t0 + ms(i * 50), false);
        }
        assert!(sim.emitted.is_empty(), "{:?}", sim.emitted);
    }

    #[test]
    fn poll_instant_is_the_midpoint_of_the_clock_samples_around_poll() {
        let t0 = Instant::now();
        let fake = Arc::new(Mutex::new(t0));
        let source = Box::new(SlowSource {
            now: fake.clone(),
            delay: ms(120),
            position_at_midpoint_ms: 9_900,
            done: false,
        });
        let mut sim = Sim::new(source, fake);
        // t0 + 120 ms setelah poll; posisi 9_900 dikaitkan ke t0 + 60 ms.
        sim.step(t0, true);
        assert_eq!(sim.emitted, vec![showing(Part::Ayah(1), true)]);
        // 9_900 + (t - (t0 + 60)) mencapai 10_000 tepat di t0 + 160 ms.
        sim.step(t0 + ms(159), false);
        assert_eq!(sim.emitted.len(), 1);
        sim.step(t0 + ms(160), false);
        assert_eq!(
            sim.emitted,
            vec![showing(Part::Ayah(1), true), showing(Part::Ayah(2), true)]
        );
    }

    #[test]
    fn shift_change_between_polls_reemits_without_a_new_poll() {
        // Poll di 10_100 ms (ayat 2, timeline uji offset 0). Geser +300 => efektif 9_800 => ayat 1.
        let (mut sim, t0, polls) = sim(vec![Ok(Some(ikhlas(10_100, false)))]);
        sim.step(t0, true);
        assert_eq!(sim.emitted, vec![showing(Part::Ayah(2), false)]);
        sim.shift.store(300, Ordering::SeqCst);
        sim.step(t0 + ms(50), false);
        assert_eq!(
            sim.emitted,
            vec![showing(Part::Ayah(2), false), showing(Part::Ayah(1), false)]
        );
        // Geser negatif besar (dijepit -2000) => efektif 12_100 => tetap ayat 2.
        sim.shift.store(-9_999, Ordering::SeqCst);
        sim.step(t0 + ms(100), false);
        assert_eq!(sim.emitted.last(), Some(&showing(Part::Ayah(2), false)));
        assert_eq!(sim.emitted.len(), 3);
        // Nilai yang sama berulang tidak memancarkan ulang.
        sim.step(t0 + ms(150), false);
        assert_eq!(sim.emitted.len(), 3);
        assert_eq!(polls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn run_loop_applies_a_shift_stored_between_polls_without_waiting_for_one() {
        // Poll sangat jarang (10 s): emit kedua hanya mungkin dari re-evaluasi geser.
        let (source, polls) = FakeSource::new(vec![Ok(Some(ikhlas(10_100, false)))]);
        let stop = Arc::new(AtomicBool::new(false));
        let shift = Arc::new(AtomicI64::new(0));
        let (stop_in_emit, shift_in_emit) = (stop.clone(), shift.clone());
        let mut emitted = Vec::new();
        let timing = LoopTiming {
            poll: Duration::from_secs(10),
            tick: ms(1),
        };
        run_loop_with(
            timing,
            source,
            engine(),
            |s| {
                emitted.push(s);
                match emitted.len() {
                    1 => shift_in_emit.store(300, Ordering::SeqCst),
                    _ => stop_in_emit.store(true, Ordering::SeqCst),
                }
            },
            Instant::now,
            stop,
            shift,
        );
        assert_eq!(
            emitted,
            vec![showing(Part::Ayah(2), false), showing(Part::Ayah(1), false)]
        );
        assert_eq!(polls.load(Ordering::SeqCst), 1);
    }

    fn no_shift() -> Arc<AtomicI64> {
        Arc::new(AtomicI64::new(0))
    }

    fn fast() -> LoopTiming {
        LoopTiming {
            poll: ms(4),
            tick: ms(1),
        }
    }

    #[test]
    fn run_loop_returns_immediately_when_stop_already_set() {
        let (source, polls) = FakeSource::new(vec![Ok(Some(ikhlas(0, true)))]);
        let stop = Arc::new(AtomicBool::new(true));
        let mut emitted = Vec::new();
        run_loop_with(
            fast(),
            source,
            engine(),
            |s| emitted.push(s),
            Instant::now,
            stop,
            no_shift(),
        );
        assert!(emitted.is_empty());
        assert_eq!(polls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn run_loop_ends_when_stop_is_set_during_the_loop() {
        let (source, _) = FakeSource::new(vec![Ok(Some(ikhlas(0, true)))]);
        let stop = Arc::new(AtomicBool::new(false));
        let stop_in_emit = stop.clone();
        let mut emitted = Vec::new();
        run_loop_with(
            fast(),
            source,
            engine(),
            |s| {
                emitted.push(s);
                stop_in_emit.store(true, Ordering::SeqCst);
            },
            Instant::now,
            stop,
            no_shift(),
        );
        assert_eq!(emitted, vec![showing(Part::Intro, true)]);
    }

    #[test]
    fn default_run_loop_uses_250_50_timing_and_honours_stop() {
        assert_eq!(
            LoopTiming::default(),
            LoopTiming {
                poll: ms(250),
                tick: ms(50)
            }
        );
        let (source, _) = FakeSource::new(vec![Ok(Some(ikhlas(0, true)))]);
        let stop = Arc::new(AtomicBool::new(false));
        let stop_in_emit = stop.clone();
        let mut emitted = Vec::new();
        run_loop(
            source,
            engine(),
            |s| {
                emitted.push(s);
                stop_in_emit.store(true, Ordering::SeqCst);
            },
            Instant::now,
            stop,
            no_shift(),
        );
        assert_eq!(emitted, vec![showing(Part::Intro, true)]);
    }

    /// Sumber yang memajukan jam palsu sebesar `delay` di dalam `poll`, lalu melaporkan
    /// posisi yang dibaca di titik tengah poll (model titik tengah).
    struct SlowSource {
        now: Arc<Mutex<Instant>>,
        delay: Duration,
        position_at_midpoint_ms: u64,
        done: bool,
    }

    impl PlaybackSource for SlowSource {
        fn poll(&mut self) -> Result<Option<Snapshot>, SourceError> {
            if self.done {
                return Ok(None);
            }
            self.done = true;
            *self.now.lock().unwrap() += self.delay;
            Ok(Some(ikhlas(self.position_at_midpoint_ms, true)))
        }
    }

    #[test]
    fn position_is_attributed_to_the_poll_midpoint_not_the_pre_poll_instant() {
        // poll() makan 120 ms; posisi 9_900 dibaca di titik tengah (t0 + 60 ms).
        // Kebenaran di t0 + 120 ms = 9_960 => ayat 1. Bila posisi dikaitkan ke t0
        // (sebelum poll), widget memimpin: 9_900 + 120 = 10_020 => ayat 2 (salah).
        let t0 = Instant::now();
        let fake = Arc::new(Mutex::new(t0));
        let calls = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let clock = {
            let (fake, calls, stop) = (fake.clone(), calls.clone(), stop.clone());
            move || {
                if calls.fetch_add(1, Ordering::SeqCst) >= 12 {
                    stop.store(true, Ordering::SeqCst);
                }
                *fake.lock().unwrap()
            }
        };
        let source = Box::new(SlowSource {
            now: fake.clone(),
            delay: ms(120),
            position_at_midpoint_ms: 9_900,
            done: false,
        });
        let timing = LoopTiming {
            poll: Duration::from_secs(3600),
            tick: ms(2),
        };
        let mut emitted = Vec::new();
        run_loop_with(
            timing,
            source,
            engine(),
            |s| emitted.push(s),
            clock,
            stop,
            no_shift(),
        );
        assert_eq!(emitted, vec![showing(Part::Ayah(1), true)]);
    }

    /// Sumber yang poll pertamanya cepat lalu poll berikutnya menggantung sampai dilepas.
    struct HangingSource {
        first: Option<Snapshot>,
        release: std::sync::mpsc::Receiver<()>,
    }

    impl PlaybackSource for HangingSource {
        fn poll(&mut self) -> Result<Option<Snapshot>, SourceError> {
            match self.first.take() {
                Some(s) => Ok(Some(s)),
                None => {
                    let _ = self.release.recv_timeout(Duration::from_secs(5));
                    Ok(None)
                }
            }
        }
    }

    #[test]
    fn blocked_poll_does_not_stall_refresh_driven_emits() {
        // Poll pertama: posisi 9_900 (ayat 1). Poll kedua menggantung 5 s, tetapi batas
        // ayat 2 (10_000 ms) tercapai ~100 ms kemudian dan harus tampil tanpa menunggu poll.
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let source = Box::new(HangingSource {
            first: Some(ikhlas(9_900, true)),
            release: release_rx,
        });
        let stop = Arc::new(AtomicBool::new(false));
        let stop_in_emit = stop.clone();
        // Pengaman: tes tidak boleh menggantung selamanya bila loop tertahan poll.
        let watchdog = stop.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(4));
            watchdog.store(true, Ordering::SeqCst);
        });
        let timing = LoopTiming {
            poll: ms(10),
            tick: ms(5),
        };
        let started = Instant::now();
        let mut emitted = Vec::new();
        run_loop_with(
            timing,
            source,
            engine(),
            |s| {
                if s == showing(Part::Ayah(2), true) {
                    stop_in_emit.store(true, Ordering::SeqCst);
                }
                emitted.push(s);
            },
            Instant::now,
            stop,
            no_shift(),
        );
        let took = started.elapsed();
        drop(release_tx);
        assert_eq!(
            emitted,
            vec![showing(Part::Ayah(1), true), showing(Part::Ayah(2), true)]
        );
        assert!(took < ms(2_000), "loop tertahan poll: {took:?}");
    }

    /// Mencatat kapan sumber (dan thread pemiliknya) dibuang.
    struct DropProbe(Arc<AtomicBool>);

    impl Drop for DropProbe {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    struct ProbedSource {
        _probe: DropProbe,
        hang: Option<std::sync::mpsc::Receiver<()>>,
    }

    impl PlaybackSource for ProbedSource {
        fn poll(&mut self) -> Result<Option<Snapshot>, SourceError> {
            if let Some(release) = &self.hang {
                let _ = release.recv_timeout(Duration::from_secs(5));
            }
            Ok(Some(ikhlas(0, true)))
        }
    }

    #[test]
    fn stop_joins_the_poller_thread_so_nothing_leaks() {
        let dropped = Arc::new(AtomicBool::new(false));
        let source = Box::new(ProbedSource {
            _probe: DropProbe(dropped.clone()),
            hang: None,
        });
        let stop = Arc::new(AtomicBool::new(false));
        let stopper = stop.clone();
        std::thread::spawn(move || {
            std::thread::sleep(ms(60));
            stopper.store(true, Ordering::SeqCst);
        });
        run_loop_with(
            fast(),
            source,
            engine(),
            |_| {},
            Instant::now,
            stop,
            no_shift(),
        );
        // Sumber hidup di thread poller: sudah dibuang berarti thread itu sudah di-join.
        assert!(dropped.load(Ordering::SeqCst));
    }

    #[test]
    fn stop_returns_promptly_even_if_poll_is_blocked_and_poller_ends_afterwards() {
        let dropped = Arc::new(AtomicBool::new(false));
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let source = Box::new(ProbedSource {
            _probe: DropProbe(dropped.clone()),
            hang: Some(release_rx),
        });
        let stop = Arc::new(AtomicBool::new(false));
        let stopper = stop.clone();
        std::thread::spawn(move || {
            std::thread::sleep(ms(50));
            stopper.store(true, Ordering::SeqCst);
        });
        let started = Instant::now();
        run_loop_with(
            fast(),
            source,
            engine(),
            |_| {},
            Instant::now,
            stop,
            no_shift(),
        );
        let took = started.elapsed();
        assert!(
            took < ms(1_500),
            "loop menunggu poll yang menggantung: {took:?}"
        );
        assert!(!dropped.load(Ordering::SeqCst), "poll masih menggantung");
        // Setelah poll dilepas, thread poller yang dilepas berakhir sendiri (tidak bocor).
        drop(release_tx);
        let deadline = Instant::now() + Duration::from_secs(2);
        while !dropped.load(Ordering::SeqCst) && Instant::now() < deadline {
            std::thread::sleep(ms(5));
        }
        assert!(dropped.load(Ordering::SeqCst), "thread poller bocor");
    }

    #[test]
    fn poller_thread_polls_on_its_own_cadence() {
        let (source, polls) = FakeSource::new(vec![Ok(Some(ikhlas(0, true)))]);
        let stop = Arc::new(AtomicBool::new(false));
        let stopper = stop.clone();
        std::thread::spawn(move || {
            std::thread::sleep(ms(300));
            stopper.store(true, Ordering::SeqCst);
        });
        let timing = LoopTiming {
            poll: ms(50),
            tick: ms(5),
        };
        run_loop_with(
            timing,
            source,
            engine(),
            |_| {},
            Instant::now,
            stop,
            no_shift(),
        );
        let n = polls.load(Ordering::SeqCst);
        // ~300 ms / 50 ms = 6 poll (+1 awal); batas atas ketat, batas bawah longgar.
        assert!((2..=8).contains(&n), "jumlah poll {n}");
    }

    // ---- load_timelines ----

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            static COUNTER: AtomicUsize = AtomicUsize::new(0);
            let n = COUNTER.fetch_add(1, Ordering::SeqCst);
            let dir = std::env::temp_dir().join(format!(
                "quran-widget-test-{}-{}-{}",
                std::process::id(),
                tag,
                n
            ));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn write(&self, name: &str, body: &str) {
            fs::write(self.0.join(name), body).unwrap();
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn json(surah: u8, starts: &str) -> String {
        format!(
            r#"{{"surah":{surah},"spotify_duration_ms":42000,"offset_ms":100,"ayah_starts_ms":{starts}}}"#
        )
    }

    #[test]
    fn missing_directory_yields_empty_without_errors() {
        let dir = std::env::temp_dir().join("quran-widget-test-does-not-exist-xyz");
        let _ = fs::remove_dir_all(&dir);
        let (timelines, errors) = load_timelines_collect(&dir);
        assert!(timelines.is_empty());
        assert!(errors.is_empty());
        assert!(load_timelines(&dir).is_empty());
    }

    #[test]
    fn empty_directory_and_gitkeep_yield_empty_without_errors() {
        let tmp = TempDir::new("empty");
        tmp.write(".gitkeep", "");
        let (timelines, errors) = load_timelines_collect(&tmp.0);
        assert!(timelines.is_empty());
        assert!(errors.is_empty());
    }

    #[test]
    fn loads_valid_files_sorted_by_surah_and_ignores_other_files() {
        let tmp = TempDir::new("valid");
        tmp.write("112.json", &json(112, "[2100,6800,10400,13900]"));
        tmp.write("078.json", &json(78, "[500,1500]"));
        tmp.write("README.txt", "bukan linimasa");
        tmp.write("notes.json", "{}");
        let (timelines, errors) = load_timelines_collect(&tmp.0);
        assert!(errors.is_empty(), "{errors:?}");
        let surahs: Vec<u8> = timelines.iter().map(|t| t.surah).collect();
        assert_eq!(surahs, vec![78, 112]);
        assert_eq!(timelines[1].ayah_starts_ms, vec![2100, 6800, 10400, 13900]);
        assert_eq!(timelines[1].offset_ms, 100);
    }

    #[test]
    fn invalid_files_are_skipped_with_one_error_each() {
        let tmp = TempDir::new("invalid");
        tmp.write("112.json", &json(112, "[1000,2000]")); // valid
        tmp.write("111.json", "{ bukan json"); // JSON rusak
        tmp.write("110.json", &json(110, "[2000,1000]")); // tidak urut
        tmp.write("109.json", &json(109, "[1000,1000]")); // tidak naik ketat
        tmp.write("108.json", &json(108, "[]")); // kosong
        tmp.write("077.json", &json(77, "[1000]")); // surah di luar 78..=114
        tmp.write("107.json", &json(106, "[1000]")); // surah tidak cocok nama berkas
        let (timelines, errors) = load_timelines_collect(&tmp.0);
        let surahs: Vec<u8> = timelines.iter().map(|t| t.surah).collect();
        assert_eq!(surahs, vec![112]);
        assert_eq!(errors.len(), 6, "{errors:?}");
        for name in [
            "111.json", "110.json", "109.json", "108.json", "077.json", "107.json",
        ] {
            assert!(
                errors.iter().any(|e| e.contains(name)),
                "tidak ada galat untuk {name}: {errors:?}"
            );
        }
    }

    #[test]
    fn validate_timeline_rules() {
        assert!(validate_timeline(&timeline(112, vec![1, 2, 3])).is_ok());
        assert!(validate_timeline(&timeline(78, vec![5])).is_ok());
        assert!(validate_timeline(&timeline(114, vec![5])).is_ok());
        assert!(validate_timeline(&timeline(112, vec![])).is_err());
        assert!(validate_timeline(&timeline(112, vec![3, 2])).is_err());
        assert!(validate_timeline(&timeline(112, vec![2, 2])).is_err());
        assert!(validate_timeline(&timeline(77, vec![1])).is_err());
        assert!(validate_timeline(&timeline(115, vec![1])).is_err());
    }
}
