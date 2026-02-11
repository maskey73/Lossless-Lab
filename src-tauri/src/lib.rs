pub mod audio;
pub mod commands;
pub mod library;
pub mod metadata;
pub mod playlist;

use commands::AppState;
use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let engine = Arc::new(audio::engine::AudioEngine::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            engine: engine.clone(),
        })
        .invoke_handler(tauri::generate_handler![
            // Playback
            commands::play_file,
            commands::pause,
            commands::resume,
            commands::stop,
            commands::seek,
            commands::set_volume,
            commands::get_playback_state,
            commands::get_position,
            // EQ
            commands::set_eq_bands,
            commands::set_eq_enabled,
            commands::get_eq_preset,
            // Devices
            commands::get_audio_devices,
            // Metadata
            commands::read_file_metadata,
            commands::get_album_art_base64,
            // Dialogs
            commands::open_files_dialog,
            commands::open_folder_dialog,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
