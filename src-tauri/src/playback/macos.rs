//! Sumber pemutaran macOS: membaca Spotify lewat AppleScript (`osascript`).
//!
//! Hanya panggilan OS yang ada di sini; seluruh parsing ada di `macos_parse.rs` (teruji di
//! Linux). TIDAK DIVERIFIKASI di mesin macOS asli pada saat penulisan.
//!
//! Aturan keselamatan: Spotify tidak boleh diluncurkan oleh Simak. Penjagaan berlapis:
//! 1. `pgrep -x Spotify` (tanpa Apple event, tanpa izin) mencegah `osascript` dijalankan sama
//!    sekali bila Spotify tidak berjalan (juga menghemat CPU: poll terjadi tiap 250 ms);
//! 2. di dalam skrip, pembacaan hanya di cabang `application id ... is running`.
//!
//! Izin: pembacaan pertama memicu dialog macOS "Simak ingin mengontrol Spotify". Selama dialog
//! menunggu jawaban, `osascript` tertahan dan kena batas waktu (1,5 s) lalu dibunuh; poll
//! berikutnya mengulang. Bila ditolak, `osascript` gagal dengan kode -1743, dipetakan ke
//! `SourceError::Unavailable` berpetunjuk Indonesia.
//!
//! Latensi: tiap poll memakan puluhan hingga ratusan ms (spawn `osascript`). Runtime menandai
//! poll dengan titik tengah jam di sekitar `poll()`, tetapi pembacaan Apple event terjadi di
//! suatu titik di dalam rentang itu, sehingga ada galat posisi beberapa puluh ms yang diukur
//! lewat `accuracy_probe`.

use super::macos_parse::{classify_failure, parse_applescript, script_lines};
use super::process::{run_with_timeout, RunError};
use super::{PlaybackSource, Snapshot, SourceError};
use std::process::Command;
use std::time::Duration;

/// Batas waktu satu panggilan `osascript` (dibunuh bila terlewat).
const OSASCRIPT_TIMEOUT: Duration = Duration::from_millis(1500);
/// Batas waktu `pgrep` (normalnya beberapa ms).
const PGREP_TIMEOUT: Duration = Duration::from_millis(500);

pub struct MacSource;

impl MacSource {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MacSource {
    fn default() -> Self {
        Self::new()
    }
}

/// `Some(true/false)` bila `pgrep` memberi jawaban pasti; `None` bila tidak bisa dipastikan
/// (lalu penjagaan di dalam skrip yang menentukan).
fn spotify_process_running() -> Option<bool> {
    let out = run_with_timeout(
        Command::new("/usr/bin/pgrep").args(["-x", "Spotify"]),
        PGREP_TIMEOUT,
    )
    .ok()?;
    match out.status.code() {
        Some(0) => Some(true),
        Some(1) => Some(false),
        _ => None,
    }
}

impl PlaybackSource for MacSource {
    fn poll(&mut self) -> Result<Option<Snapshot>, SourceError> {
        if spotify_process_running() == Some(false) {
            return Ok(None);
        }

        let mut cmd = Command::new("/usr/bin/osascript");
        for line in script_lines() {
            cmd.arg("-e").arg(line);
        }
        let out =
            match run_with_timeout(&mut cmd, OSASCRIPT_TIMEOUT) {
                Ok(out) => out,
                Err(RunError::Timeout) => return Err(SourceError::Unavailable(
                    "osascript tidak merespons dalam 1,5 detik (mungkin dialog izin Automation \
                     masih menunggu jawaban)"
                        .into(),
                )),
                Err(RunError::Spawn(e)) => {
                    return Err(SourceError::Unavailable(format!(
                        "gagal menjalankan osascript: {e}"
                    )))
                }
                Err(RunError::Io(e)) => {
                    return Err(SourceError::Unavailable(format!(
                        "gagal membaca keluaran osascript: {e}"
                    )))
                }
            };

        if out.status.success() {
            parse_applescript(&String::from_utf8_lossy(&out.stdout))
        } else {
            classify_failure(&String::from_utf8_lossy(&out.stderr))
        }
    }
}
