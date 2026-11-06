//! Menjalankan proses eksternal dengan batas waktu (tanpa shell). Murni std, sehingga dapat
//! diuji di semua OS unix; dipakai oleh sumber macOS untuk `osascript`/`pgrep`.

use std::io::{self, Read};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub enum RunError {
    /// Proses tidak bisa dijalankan (mis. program tidak ada).
    Spawn(io::Error),
    /// Melewati batas waktu; proses sudah dibunuh.
    Timeout,
    /// Gagal menunggu/membaca proses.
    Io(io::Error),
}

/// Jalankan `cmd` (stdin ditutup, stdout/stderr ditangkap) dan tunggu paling lama `timeout`.
/// Bila terlewat, proses dibunuh dan `Err(Timeout)` dikembalikan. Pembaca stdout/stderr berjalan
/// di thread sendiri agar keluaran besar tidak membuat proses tersumbat.
pub fn run_with_timeout(cmd: &mut Command, timeout: Duration) -> Result<Output, RunError> {
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(RunError::Spawn)?;

    fn drain(pipe: Option<impl Read + Send + 'static>) -> thread::JoinHandle<Vec<u8>> {
        thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut p) = pipe {
                let _ = p.read_to_end(&mut buf);
            }
            buf
        })
    }
    let out_reader = drain(child.stdout.take());
    let err_reader = drain(child.stderr.take());

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                // Pembaca sengaja tidak di-join: cucu proses yang masih memegang pipe tidak
                // boleh menahan kita; thread berakhir sendiri saat pipe tertutup.
                let _ = child.kill();
                let _ = child.wait();
                return Err(RunError::Timeout);
            }
            Ok(None) => thread::sleep(Duration::from_millis(5)),
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(RunError::Io(e));
            }
        }
    };
    let stdout = out_reader.join().unwrap_or_default();
    let stderr = err_reader.join().unwrap_or_default();
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn sh(script: &str) -> Command {
        let mut c = Command::new("sh");
        c.args(["-c", script]);
        c
    }

    #[test]
    fn captures_output_and_exit_code() {
        let out = run_with_timeout(
            &mut sh("printf hi; printf err >&2; exit 3"),
            Duration::from_secs(5),
        )
        .unwrap();
        assert_eq!(out.status.code(), Some(3));
        assert_eq!(out.stdout, b"hi");
        assert_eq!(out.stderr, b"err");
    }

    #[test]
    fn kills_on_timeout() {
        let t0 = Instant::now();
        let r = run_with_timeout(&mut sh("exec sleep 10"), Duration::from_millis(200));
        assert!(matches!(r, Err(RunError::Timeout)), "{r:?}");
        assert!(
            t0.elapsed() < Duration::from_secs(3),
            "harus kembali segera: {:?}",
            t0.elapsed()
        );
    }

    #[test]
    fn missing_program_is_spawn_error() {
        let r = run_with_timeout(
            &mut Command::new("/nonexistent/definitely-not-here"),
            Duration::from_secs(1),
        );
        assert!(matches!(r, Err(RunError::Spawn(_))), "{r:?}");
    }

    #[test]
    fn large_output_does_not_deadlock() {
        let out =
            run_with_timeout(&mut sh("head -c 300000 /dev/zero"), Duration::from_secs(10)).unwrap();
        assert_eq!(out.stdout.len(), 300_000);
    }
}
