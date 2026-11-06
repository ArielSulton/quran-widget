//! Sumber pemutaran Windows: membaca sesi Spotify lewat SMTC
//! (`GlobalSystemMediaTransportControlsSessionManager`).
//!
//! Hanya panggilan OS yang ada di sini; seluruh aritmetika ada di `windows_timeline.rs`
//! (teruji di Linux). TIDAK DIVERIFIKASI di mesin Windows asli pada saat penulisan.
//!
//! Catatan SMTC: `TimelineProperties.Position` hanya diperbarui saat aplikasi mendorong
//! pembaruan (play/pause/seek/ganti track, dan berkala menurut kebijakan aplikasi). Posisi
//! sekarang diekstrapolasi `Position + (now − LastUpdatedTime)` saat memutar. Seberapa sering
//! Spotify mendorong pembaruan TIDAK diketahui; bisa terjadi lompatan posisi hingga ~1 detik.
//! Logika re-anchor runtime dan kontrol sinkronisasi pengguna menyerap sisa galat;
//! `accuracy_probe` adalah cara mengukurnya di mesin asli. Hanya aplikasi desktop Spotify yang
//! cocok (AUMID mengandung "spotify"); Spotify Web Player di browser memakai AUMID browser.

use super::windows_timeline::{filetime_from_unix, is_spotify_aumid, timeline_to_ms};
use super::{PlaybackSource, Snapshot, SourceError};
use std::time::{SystemTime, UNIX_EPOCH};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager as SessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
};
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

pub struct WindowsSource {
    /// `RoInitialize` dipanggil lazily di thread yang memanggil `poll` (runtime selalu
    /// memakai satu thread). Sumber dibuat di thread lain lalu dipindah (`Send`).
    winrt_ready: bool,
}

impl WindowsSource {
    pub fn new() -> Self {
        Self { winrt_ready: false }
    }
}

impl Default for WindowsSource {
    fn default() -> Self {
        Self::new()
    }
}

fn winrt(context: &str) -> impl Fn(windows::core::Error) -> SourceError + '_ {
    move |e| SourceError::Unavailable(format!("{context}: {e}"))
}

impl PlaybackSource for WindowsSource {
    /// `Ok(None)` jika tidak ada sesi Spotify atau sesi tanpa judul.
    fn poll(&mut self) -> Result<Option<Snapshot>, SourceError> {
        if !self.winrt_ready {
            // Galat diabaikan dengan sengaja: RPC_E_CHANGED_MODE berarti thread ini sudah
            // diinisialisasi (STA); WinRT tetap bisa dipakai. Kegagalan lain akan muncul
            // sebagai galat pada panggilan WinRT di bawah.
            // SAFETY: tanpa prasyarat selain dipanggil dari thread yang akan memakai WinRT.
            let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
            self.winrt_ready = true;
        }

        let manager = SessionManager::RequestAsync()
            .map_err(winrt("gagal meminta SMTC"))?
            .get()
            .map_err(winrt("gagal membuka SMTC"))?;
        let sessions = manager
            .GetSessions()
            .map_err(winrt("gagal membaca daftar sesi"))?;

        let Some(session) = sessions.into_iter().find(|s| {
            s.SourceAppUserModelId()
                .map(|id| is_spotify_aumid(&id.to_string()))
                .unwrap_or(false)
        }) else {
            return Ok(None);
        };

        let props = session
            .TryGetMediaPropertiesAsync()
            .map_err(winrt("gagal meminta properti media"))?
            .get()
            .map_err(winrt("gagal membaca properti media"))?;
        let title = props
            .Title()
            .map_err(winrt("gagal membaca judul"))?
            .to_string();
        if title.trim().is_empty() {
            return Ok(None);
        }

        let timeline = session
            .GetTimelineProperties()
            .map_err(winrt("gagal membaca timeline"))?;
        let start = timeline.StartTime().map_err(winrt("StartTime"))?.Duration;
        let end = timeline.EndTime().map_err(winrt("EndTime"))?.Duration;
        let position = timeline.Position().map_err(winrt("Position"))?.Duration;
        let last_updated = timeline
            .LastUpdatedTime()
            .map_err(winrt("LastUpdatedTime"))?
            .UniversalTime;
        let is_playing = session
            .GetPlaybackInfo()
            .and_then(|info| info.PlaybackStatus())
            .map_err(winrt("gagal membaca status pemutaran"))?
            == PlaybackStatus::Playing;

        let now = filetime_from_unix(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default(),
        );
        let (duration_ms, position_ms) =
            timeline_to_ms(start, end, position, last_updated, now, is_playing);

        Ok(Some(Snapshot {
            title,
            duration_ms,
            position_ms,
            is_playing,
        }))
    }
}
