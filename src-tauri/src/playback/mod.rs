//! Sumber pemutaran: membaca judul/posisi track Spotify yang sedang diputar.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::LinuxSource;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub title: String,
    pub duration_ms: u64,
    pub position_ms: u64,
    pub is_playing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceError {
    Unavailable(String),
}

impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceError::Unavailable(msg) => write!(f, "sumber pemutaran tidak tersedia: {msg}"),
        }
    }
}

impl std::error::Error for SourceError {}

pub trait PlaybackSource: Send {
    /// Ok(None) = Spotify tidak berjalan.
    fn poll(&mut self) -> Result<Option<Snapshot>, SourceError>;
}

/// Stub untuk OS yang belum didukung (macOS/Windows menyusul di Task 10).
#[cfg(not(target_os = "linux"))]
pub struct UnsupportedSource;

#[cfg(not(target_os = "linux"))]
impl PlaybackSource for UnsupportedSource {
    fn poll(&mut self) -> Result<Option<Snapshot>, SourceError> {
        Err(SourceError::Unavailable(
            "sumber pemutaran untuk OS ini belum diimplementasikan (not implemented yet)".into(),
        ))
    }
}

/// Pilih sumber pemutaran sesuai OS.
pub fn default_source() -> Box<dyn PlaybackSource> {
    #[cfg(target_os = "linux")]
    {
        Box::new(LinuxSource::new())
    }
    #[cfg(not(target_os = "linux"))]
    {
        Box::new(UnsupportedSource)
    }
}
