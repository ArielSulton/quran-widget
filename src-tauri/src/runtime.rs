//! Runtime: loop polling sumber pemutaran + interpolasi, dan pemuatan linimasa.

use crate::engine::{Engine, WidgetState};
use crate::playback::{PlaybackSource, SourceError};
use crate::timeline::Timeline;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Irama loop: `poll` = jarak antar pembacaan sumber (tick segar),
/// `tick` = jarak antar evaluasi ulang (refresh) di antaranya.
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

/// Satu langkah loop yang deterministik (waktu disuntikkan lewat `now`).
struct Stepper {
    source: Box<dyn PlaybackSource>,
    engine: Engine,
    poll_every: Duration,
    last_poll: Option<Instant>,
    last_emitted: Option<WidgetState>,
    last_error: Option<SourceError>,
}

impl Stepper {
    fn new(source: Box<dyn PlaybackSource>, engine: Engine, poll_every: Duration) -> Self {
        Self {
            source,
            engine,
            poll_every,
            last_poll: None,
            last_emitted: None,
            last_error: None,
        }
    }

    fn step(&mut self, now: Instant, emit: &mut impl FnMut(WidgetState)) {
        let due = self
            .last_poll
            .is_none_or(|t| now.saturating_duration_since(t) >= self.poll_every);
        let state = if due {
            self.last_poll = Some(now);
            // Galat sumber diperlakukan seperti Ok(None): widget menunggu.
            let snap = match self.source.poll() {
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
            self.engine.tick(snap, now)
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
pub fn run_loop(
    source: Box<dyn PlaybackSource>,
    engine: Engine,
    emit: impl FnMut(WidgetState),
    clock: impl Fn() -> Instant,
    stop: Arc<AtomicBool>,
) {
    run_loop_with(LoopTiming::default(), source, engine, emit, clock, stop)
}

/// Seperti `run_loop` dengan irama yang dapat diatur (dipakai tes).
pub fn run_loop_with(
    timing: LoopTiming,
    source: Box<dyn PlaybackSource>,
    engine: Engine,
    emit: impl FnMut(WidgetState),
    clock: impl Fn() -> Instant,
    stop: Arc<AtomicBool>,
) {
    let mut emit = emit;
    let mut stepper = Stepper::new(source, engine, timing.poll);
    while !stop.load(Ordering::SeqCst) {
        stepper.step(clock(), &mut emit);
        std::thread::sleep(timing.tick);
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

    fn drive(stepper: &mut Stepper, times: &[Instant]) -> Vec<WidgetState> {
        let mut emitted = Vec::new();
        for &t in times {
            stepper.step(t, &mut |s| emitted.push(s));
        }
        emitted
    }

    #[test]
    fn emits_intro_ayah_waiting_and_treats_error_as_none() {
        let (source, polls) = FakeSource::new(vec![
            Ok(Some(ikhlas(0, true))),
            Ok(Some(ikhlas(12_000, true))),
            Ok(None),
            Err(SourceError::Unavailable("dbus mati".into())),
        ]);
        let mut stepper = Stepper::new(source, engine(), ms(250));
        let t0 = Instant::now();
        let emitted = drive(
            &mut stepper,
            &[t0, t0 + ms(250), t0 + ms(500), t0 + ms(750)],
        );
        assert_eq!(polls.load(Ordering::SeqCst), 4);
        assert_eq!(
            emitted,
            vec![
                showing(Part::Intro, true),
                showing(Part::Ayah(2), true),
                WidgetState::Waiting,
            ]
        );
    }

    #[test]
    fn source_error_alone_never_panics_and_yields_waiting() {
        let (source, _) = FakeSource::new(vec![
            Err(SourceError::Unavailable("a".into())),
            Err(SourceError::Unavailable("a".into())),
            Err(SourceError::Unavailable("b".into())),
        ]);
        let mut stepper = Stepper::new(source, engine(), ms(250));
        let t0 = Instant::now();
        let emitted = drive(&mut stepper, &[t0, t0 + ms(250), t0 + ms(500)]);
        assert_eq!(emitted, vec![WidgetState::Waiting]);
    }

    #[test]
    fn no_duplicate_emits_for_unchanged_state() {
        let (source, _) = FakeSource::new(vec![
            Ok(Some(ikhlas(2_000, true))),
            Ok(Some(ikhlas(2_250, true))),
            Ok(Some(ikhlas(2_500, true))),
        ]);
        let mut stepper = Stepper::new(source, engine(), ms(250));
        let t0 = Instant::now();
        let times: Vec<Instant> = (0..15).map(|i| t0 + ms(i * 50)).collect();
        let emitted = drive(&mut stepper, &times);
        assert_eq!(emitted, vec![showing(Part::Ayah(1), true)]);
    }

    #[test]
    fn refresh_between_polls_emits_boundary_crossing_before_next_poll() {
        // Poll di 9_900 ms (ayat 1); batas ayat 2 di 10_000 ms terlewati 100 ms kemudian.
        let (source, polls) = FakeSource::new(vec![Ok(Some(ikhlas(9_900, true)))]);
        let mut stepper = Stepper::new(source, engine(), ms(250));
        let t0 = Instant::now();
        let mut emitted = Vec::new();
        stepper.step(t0, &mut |s| emitted.push(s));
        assert_eq!(emitted, vec![showing(Part::Ayah(1), true)]);
        stepper.step(t0 + ms(50), &mut |s| emitted.push(s));
        assert_eq!(emitted.len(), 1);
        stepper.step(t0 + ms(100), &mut |s| emitted.push(s));
        assert_eq!(
            emitted,
            vec![showing(Part::Ayah(1), true), showing(Part::Ayah(2), true)]
        );
        // Hanya satu poll sejauh ini: perubahan berasal dari interpolasi.
        assert_eq!(polls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn polls_only_every_poll_interval() {
        let (source, polls) = FakeSource::new(vec![]);
        let mut stepper = Stepper::new(source, engine(), ms(250));
        let t0 = Instant::now();
        let times: Vec<Instant> = (0..11).map(|i| t0 + ms(i * 50)).collect(); // 0..=500 ms
        drive(&mut stepper, &times);
        assert_eq!(polls.load(Ordering::SeqCst), 3); // 0, 250, 500
    }

    #[test]
    fn paused_stays_frozen_between_polls() {
        let (source, _) = FakeSource::new(vec![Ok(Some(ikhlas(9_950, false)))]);
        let mut stepper = Stepper::new(source, engine(), ms(250));
        let t0 = Instant::now();
        let times: Vec<Instant> = (0..5).map(|i| t0 + ms(i * 50)).collect();
        let emitted = drive(&mut stepper, &times);
        assert_eq!(emitted, vec![showing(Part::Ayah(1), false)]);
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
        );
        assert_eq!(emitted, vec![showing(Part::Intro, true)]);
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
