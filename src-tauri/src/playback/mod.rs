//! Sumber pemutaran: membaca judul/posisi track Spotify yang sedang diputar.

// Logika murni (parser/pemetaan) dikompilasi di semua OS agar teruji di Linux;
// hanya pemanggilan OS yang dibatasi `cfg`.
pub mod macos_parse;
pub mod probe_analysis;
pub mod process;
pub mod windows_timeline;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::LinuxSource;

// `self::` karena nama modul `windows` bentrok dengan crate `windows` di prelude ekstern.
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::MacSource;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use self::windows::WindowsSource;

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

/// Stub untuk OS yang tidak didukung (bukan Linux/macOS/Windows, mis. FreeBSD).
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub struct UnsupportedSource;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
impl PlaybackSource for UnsupportedSource {
    fn poll(&mut self) -> Result<Option<Snapshot>, SourceError> {
        Err(SourceError::Unavailable(
            "sumber pemutaran untuk OS ini tidak didukung (hanya Linux, macOS, Windows)".into(),
        ))
    }
}

/// Pilih sumber pemutaran sesuai OS.
pub fn default_source() -> Box<dyn PlaybackSource> {
    #[cfg(target_os = "linux")]
    {
        Box::new(LinuxSource::new())
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(MacSource::new())
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(WindowsSource::new())
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Box::new(UnsupportedSource)
    }
}
