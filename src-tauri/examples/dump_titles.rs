//! Perekam judul track Spotify untuk membuat fixture `tests/fixtures/spotify_titles.json`.
//!
//! Pakai: `cargo run --example dump_titles -- [path-fixture]`, lalu putar track Juz 30 satu per satu
//! di Spotify. Setiap judul baru dicatat sebagai `{"title": ..., "duration_ms": ...}`.
//! Berhenti dengan Ctrl+C (file ditulis atomik setiap judul baru, jadi aman).

use quran_widget_lib::playback::{default_source, SourceError};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{fs, thread};

const POLL_INTERVAL: Duration = Duration::from_millis(500);

fn default_fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("fixtures")
        .join("spotify_titles.json")
}

/// Baca fixture; buat `[]` jika belum ada.
fn load_entries(path: &Path) -> Result<Vec<Value>, String> {
    if !path.exists() {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("gagal membuat {}: {e}", dir.display()))?;
        }
        write_atomic(path, &[])?;
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(path).map_err(|e| format!("gagal membaca {}: {e}", path.display()))?;
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::Array(items)) => Ok(items),
        Ok(_) => Err(format!("{} bukan array JSON", path.display())),
        Err(e) => Err(format!("{} bukan JSON valid: {e}", path.display())),
    }
}

/// Tulis atomik: file sementara di direktori yang sama lalu `rename`.
fn write_atomic(path: &Path, entries: &[Value]) -> Result<(), String> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    let body = serde_json::to_string_pretty(entries).map_err(|e| e.to_string())? + "\n";
    fs::write(&tmp, body).map_err(|e| format!("gagal menulis {}: {e}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|e| format!("gagal mengganti {}: {e}", path.display()))
}

/// Sisipkan/perbarui entri berdasarkan judul (unik). Mengembalikan true jika ada perubahan.
fn upsert(entries: &mut Vec<Value>, title: &str, duration_ms: u64) -> bool {
    let new_entry = json!({ "title": title, "duration_ms": duration_ms });
    match entries
        .iter_mut()
        .find(|e| e.get("title").and_then(Value::as_str) == Some(title))
    {
        Some(existing) if *existing == new_entry => false,
        Some(existing) => {
            *existing = new_entry;
            true
        }
        None => {
            entries.push(new_entry);
            true
        }
    }
}

#[derive(PartialEq, Clone)]
enum State {
    Waiting,
    Error(String),
    Running,
}

fn main() {
    let path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(default_fixture_path);
    let mut entries = match load_entries(&path) {
        Ok(e) => e,
        Err(msg) => {
            eprintln!("{msg}");
            std::process::exit(1);
        }
    };
    println!("Fixture: {} ({} entri). Ctrl+C untuk berhenti.", path.display(), entries.len());

    let mut source = default_source();
    let mut last_title: Option<String> = None;
    let mut state: Option<State> = None;

    loop {
        let next_state = match source.poll() {
            Ok(Some(snap)) => {
                if last_title.as_deref() != Some(snap.title.as_str()) {
                    if upsert(&mut entries, &snap.title, snap.duration_ms) {
                        match write_atomic(&path, &entries) {
                            Ok(()) => println!(
                                "[{}] {} ({} ms)",
                                entries.len(),
                                snap.title,
                                snap.duration_ms
                            ),
                            Err(msg) => eprintln!("{msg}"),
                        }
                    }
                    last_title = Some(snap.title);
                }
                State::Running
            }
            Ok(None) => State::Waiting,
            Err(SourceError::Unavailable(msg)) => State::Error(msg),
        };
        if state.as_ref() != Some(&next_state) {
            match &next_state {
                State::Waiting => println!("Spotify tidak berjalan, menunggu…"),
                State::Error(msg) => eprintln!("Sumber pemutaran tidak tersedia ({msg}), menunggu…"),
                State::Running => {}
            }
            state = Some(next_state);
        }
        thread::sleep(POLL_INTERVAL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_appends_new_titles_and_keeps_them_unique() {
        let mut v = Vec::new();
        assert!(upsert(&mut v, "An-Naba", 1000));
        assert!(upsert(&mut v, "An-Nazi'at", 2000));
        assert!(!upsert(&mut v, "An-Naba", 1000));
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn upsert_updates_duration_of_existing_title() {
        let mut v = Vec::new();
        upsert(&mut v, "An-Naba", 1000);
        assert!(upsert(&mut v, "An-Naba", 1500));
        assert_eq!(v, vec![json!({"title": "An-Naba", "duration_ms": 1500})]);
    }

    #[test]
    fn load_creates_missing_file_as_empty_array() {
        let dir = std::env::temp_dir().join(format!("dump_titles_test_{}", std::process::id()));
        let path = dir.join("fixtures").join("t.json");
        assert!(load_entries(&path).unwrap().is_empty());
        assert_eq!(fs::read_to_string(&path).unwrap().trim(), "[]");
        let _ = fs::remove_dir_all(&dir);
    }
}
