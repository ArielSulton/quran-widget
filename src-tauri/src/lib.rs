pub mod playback;
pub mod surah;

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("gagal menjalankan aplikasi Tauri");
}
