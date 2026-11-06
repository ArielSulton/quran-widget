pub mod clock;
pub mod engine;
pub mod playback;
pub mod surah;
pub mod timeline;

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("gagal menjalankan aplikasi Tauri");
}
