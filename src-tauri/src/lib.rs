pub mod audio;
pub mod commands;
pub mod library;
pub mod metadata;
pub mod playlist;

use audio::device_profiles::DeviceProfileStore;
use commands::AppState;
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let engine = Arc::new(audio::engine::AudioEngine::new());

    // App data directory for storing profiles, library DB, etc.
    let app_data_dir = dirs_next::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("masukii");

    let device_profiles = Arc::new(Mutex::new(DeviceProfileStore::load(&app_data_dir)));

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            engine: engine.clone(),
            device_profiles,
            app_data_dir,
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
            // ReplayGain
            commands::set_replaygain_mode,
            commands::set_clipping_prevention,
            // Diagnostics
            commands::get_audio_diagnostics,
            // Bit-Perfect Null Test
            commands::run_null_test,
            // Devices
            commands::get_audio_devices,
            // Device Profiles
            commands::get_device_profile,
            commands::save_device_profile,
            commands::list_device_profiles,
            commands::delete_device_profile,
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
