pub mod clock;
pub mod engine;
pub mod playback;
pub mod runtime;
pub mod surah;
pub mod timeline;

use engine::{Engine, WidgetState};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{Emitter, Manager, RunEvent, WindowEvent};

/// Bendera penghenti loop runtime; diset saat jendela utama ditutup / aplikasi keluar.
struct StopFlag(Arc<AtomicBool>);

/// Keadaan terakhir yang dipancarkan, agar front end yang baru siap bisa mengambilnya
/// (event pertama bisa terbit sebelum `listen` terdaftar).
struct LatestState(Mutex<WidgetState>);

/// Geser sinkron pengguna (ms) yang dibagikan antara perintah UI dan thread loop.
struct SyncShift(Arc<AtomicI64>);

/// Simpan geser sinkron (ms); loop runtime mengambilnya pada iterasi berikutnya (<= 50 ms)
/// dan `Engine` menjepitnya ke ±2000.
#[tauri::command]
fn set_sync_shift_ms(ms: i64, shift: tauri::State<'_, SyncShift>) {
    shift.0.store(ms, Ordering::SeqCst);
}

#[tauri::command]
fn current_state(latest: tauri::State<'_, LatestState>) -> WidgetState {
    latest.0.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Folder linimasa: resource Tauri `data/timing`; saat dev jatuh balik ke `<repo>/data/timing`.
fn timing_dir(app: &tauri::App) -> PathBuf {
    let bundled = app
        .path()
        .resource_dir()
        .ok()
        .map(|dir| dir.join("data").join("timing"));
    match bundled {
        Some(dir) if dir.is_dir() => dir,
        _ => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("data")
            .join("timing"),
    }
}

fn stop_loop(app: &tauri::AppHandle) {
    if let Some(stop) = app.try_state::<StopFlag>() {
        stop.0.store(true, Ordering::SeqCst);
    }
}

/// Variabel lingkungan yang perlu disetel di Linux sebelum WebKitGTK dimulai (kosong bila sudah
/// disetel pengguna; pilihan pengguna dihormati, mis. `WEBKIT_DISABLE_COMPOSITING_MODE=0 simak`).
///
/// Alasan: di jendela transparan pada WebKitGTK, sisa glif (potongan huruf/kotak) dari ayat
/// sebelumnya tertinggal di tepi kartu setelah ayat berganti. Gejala ini hilang bila compositing
/// GPU webview dimatikan; sudah dipastikan pengguna di Linux (X11/GNOME, Intel/Mesa, WebKitGTK
/// 2.50). Kompromi: animasi webview bisa kurang mulus dan `backdrop-filter` tidak dapat
/// diandalkan; Linux memang sudah memakai bahan solid/CSS tanpa blur.
///
/// `lookup` memetakan nama variabel ke nilainya (bawaan: lingkungan proses), agar bisa diuji.
pub fn linux_webkit_env(
    lookup: impl Fn(&str) -> Option<String>,
) -> Vec<(&'static str, &'static str)> {
    const NAME: &str = "WEBKIT_DISABLE_COMPOSITING_MODE";
    if lookup(NAME).is_some() {
        Vec::new()
    } else {
        vec![(NAME, "1")]
    }
}

pub fn run() {
    // Harus sebelum inisialisasi GTK/WebKit dan sebelum thread mana pun dibuat (set_var tidak
    // aman terhadap pembacaan lingkungan dari thread lain).
    #[cfg(target_os = "linux")]
    for (name, value) in
        linux_webkit_env(|key| std::env::var_os(key).map(|v| v.to_string_lossy().into_owned()))
    {
        std::env::set_var(name, value);
    }

    let stop = Arc::new(AtomicBool::new(false));
    let shift = Arc::new(AtomicI64::new(0));
    let loop_shift = shift.clone();

    let app = tauri::Builder::default()
        .manage(StopFlag(stop.clone()))
        .manage(SyncShift(shift))
        .manage(LatestState(Mutex::new(WidgetState::Waiting)))
        .invoke_handler(tauri::generate_handler![current_state, set_sync_shift_ms])
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, WindowEvent::Destroyed) {
                stop_loop(window.app_handle());
            }
        })
        .setup(move |app| {
            let timelines = runtime::load_timelines(&timing_dir(app));
            eprintln!("[timing] {} linimasa dimuat", timelines.len());
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                runtime::run_loop(
                    playback::default_source(),
                    Engine::new(timelines),
                    |state| {
                        if let Some(latest) = handle.try_state::<LatestState>() {
                            *latest.0.lock().unwrap_or_else(|e| e.into_inner()) = state.clone();
                        }
                        if let Err(e) = handle.emit_to("main", "state", state) {
                            eprintln!("[runtime] gagal memancarkan event state: {e}");
                        }
                    },
                    Instant::now,
                    stop,
                    loop_shift,
                );
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("gagal membangun aplikasi Tauri");

    app.run(|handle, event| {
        if matches!(event, RunEvent::ExitRequested { .. } | RunEvent::Exit) {
            stop_loop(handle);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::linux_webkit_env;

    #[test]
    fn sets_compositing_off_when_unset() {
        let pairs = linux_webkit_env(|_| None);
        assert_eq!(pairs, vec![("WEBKIT_DISABLE_COMPOSITING_MODE", "1")]);
    }

    #[test]
    fn respects_the_users_choice_when_already_set() {
        // Nilai apa pun (termasuk "0" untuk memilih keluar) dihormati.
        assert!(linux_webkit_env(|_| Some("0".into())).is_empty());
        assert!(linux_webkit_env(|_| Some("1".into())).is_empty());
        assert!(linux_webkit_env(|_| Some(String::new())).is_empty());
    }

    #[test]
    fn only_looks_at_its_own_variable() {
        let asked = std::cell::RefCell::new(Vec::new());
        let _ = linux_webkit_env(|k| {
            asked.borrow_mut().push(k.to_string());
            None
        });
        assert_eq!(*asked.borrow(), vec!["WEBKIT_DISABLE_COMPOSITING_MODE"]);
    }
}
