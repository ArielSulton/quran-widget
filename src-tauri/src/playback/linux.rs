//! Sumber pemutaran Linux: membaca Spotify lewat MPRIS (D-Bus).

use super::{PlaybackSource, Snapshot, SourceError};
use mpris::{PlayerFinder, PlaybackStatus};

/// Awalan nama bus MPRIS Spotify (`org.mpris.MediaPlayer2.spotify`,
/// atau varian berinstance seperti `spotify.instance123`).
const PLAYER_PREFIX: &str = "spotify";

/// Batas waktu per panggilan D-Bus ke player (bawaan mpris 500 ms). Dipersingkat agar
/// Spotify yang menggantung tidak menahan satu `poll` terlalu lama.
const DBUS_TIMEOUT_MS: i32 = 250;

/// Pemetaan murni dari nilai mentah MPRIS (mikrodetik, string status) ke [`Snapshot`].
pub fn snapshot_from_mpris(
    title: &str,
    length_us: u64,
    position_us: u64,
    status: &str,
) -> Snapshot {
    Snapshot {
        title: title.to_string(),
        duration_ms: length_us / 1_000,
        position_ms: position_us / 1_000,
        is_playing: status == "Playing",
    }
}

/// Pilih item pertama yang sukses dan nama bus-nya (tanpa awalan MPRIS) berawalan `spotify`
/// (tanpa membedakan huruf besar/kecil). Item `Err` (player lain yang rusak/hang) dilewati
/// agar satu player bermasalah tidak menggagalkan seluruh `poll`.
fn select_spotify<P, E>(
    players: impl IntoIterator<Item = Result<P, E>>,
    bus_name: impl Fn(&P) -> &str,
) -> Option<P> {
    players
        .into_iter()
        .filter_map(Result::ok)
        .find(|p| bus_name(p).to_ascii_lowercase().starts_with(PLAYER_PREFIX))
}

fn status_str(status: PlaybackStatus) -> &'static str {
    match status {
        PlaybackStatus::Playing => "Playing",
        PlaybackStatus::Paused => "Paused",
        PlaybackStatus::Stopped => "Stopped",
    }
}

/// Implementasi MPRIS untuk player "spotify".
///
/// `PlayerFinder` (koneksi D-Bus) tidak `Send`, sehingga koneksi dibuat per `poll`
/// agar `LinuxSource` tetap memenuhi `PlaybackSource: Send`.
pub struct LinuxSource;

impl LinuxSource {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LinuxSource {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaybackSource for LinuxSource {
    /// `Ok(None)` jika Spotify tidak berjalan atau belum ada track (metadata tanpa judul).
    ///
    /// Jika Spotify melaporkan judul tetapi tidak ada panjang track (`mpris:length`),
    /// `Snapshot` tetap dikembalikan dengan `duration_ms == 0`; nilai 0 berarti durasi
    /// tidak diketahui. `position_ms` juga 0 bila posisi tidak terbaca.
    fn poll(&mut self) -> Result<Option<Snapshot>, SourceError> {
        let mut finder = PlayerFinder::new()
            .map_err(|e| SourceError::Unavailable(format!("gagal terhubung ke D-Bus: {e}")))?;
        finder.set_player_timeout_ms(DBUS_TIMEOUT_MS);

        let players = finder
            .iter_players()
            .map_err(|e| SourceError::Unavailable(format!("gagal mencari player MPRIS: {e}")))?;
        let Some(player) = select_spotify(players, |p| p.bus_name_trimmed()) else {
            return Ok(None);
        };

        let metadata = player
            .get_metadata()
            .map_err(|e| SourceError::Unavailable(format!("gagal membaca metadata: {e}")))?;
        let Some(title) = metadata.title().filter(|t| !t.is_empty()) else {
            return Ok(None);
        };
        let length_us = metadata.length_in_microseconds().unwrap_or(0);
        let position_us = player.get_position_in_microseconds().unwrap_or(0);
        let status = player
            .get_playback_status()
            .map(status_str)
            .unwrap_or("Stopped");

        Ok(Some(snapshot_from_mpris(title, length_us, position_us, status)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_from_mpris_maps_units_and_status() {
        let s = snapshot_from_mpris("Al-Fatihah", 1_743_000_000, 5_250_000, "Paused");
        assert_eq!(s.title, "Al-Fatihah");
        assert_eq!(s.duration_ms, 1_743_000);
        assert_eq!(s.position_ms, 5_250);
        assert!(!s.is_playing);
    }

    #[test]
    fn select_spotify_skips_err_items_before_spotify() {
        let items: Vec<Result<&str, &str>> =
            vec![Err("hung player"), Ok("firefox.instance1"), Ok("spotify")];
        assert_eq!(select_spotify(items, |n| n), Some("spotify"));
    }

    #[test]
    fn select_spotify_matches_prefix_case_insensitively() {
        let items: Vec<Result<&str, ()>> = vec![Ok("mpv"), Ok("Spotify.instance42")];
        assert_eq!(select_spotify(items, |n| n), Some("Spotify.instance42"));
    }

    #[test]
    fn select_spotify_returns_none_when_absent_or_only_errors() {
        let items: Vec<Result<&str, ()>> = vec![Err(()), Ok("mpv")];
        assert_eq!(select_spotify(items, |n| n), None);
        assert_eq!(select_spotify(Vec::<Result<&str, ()>>::new(), |n| n), None);
    }

    #[test]
    fn snapshot_from_mpris_playing_status() {
        let s = snapshot_from_mpris("x", 0, 0, "Playing");
        assert!(s.is_playing);
    }
}
