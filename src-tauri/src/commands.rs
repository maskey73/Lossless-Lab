use crate::audio::engine::{AudioCommand, AudioDeviceInfo, AudioEngine, PlaybackState};
use crate::audio::equalizer;
use crate::metadata::reader;
use std::sync::Arc;
use tauri::State;

pub struct AppState {
    pub engine: Arc<AudioEngine>,
}

// ─── Playback Commands ───

#[tauri::command]
pub fn play_file(path: String, state: State<'_, AppState>) -> Result<(), String> {
    state.engine.send_command(AudioCommand::Play(path));
    Ok(())
}

#[tauri::command]
pub fn pause(state: State<'_, AppState>) -> Result<(), String> {
    state.engine.send_command(AudioCommand::Pause);
    Ok(())
}

#[tauri::command]
pub fn resume(state: State<'_, AppState>) -> Result<(), String> {
    state.engine.send_command(AudioCommand::Resume);
    Ok(())
}

#[tauri::command]
pub fn stop(state: State<'_, AppState>) -> Result<(), String> {
    state.engine.send_command(AudioCommand::Stop);
    Ok(())
}

#[tauri::command]
pub fn seek(position_secs: f64, state: State<'_, AppState>) -> Result<(), String> {
    state.engine.send_command(AudioCommand::Seek(position_secs));
    Ok(())
}

#[tauri::command]
pub fn set_volume(volume: f32, state: State<'_, AppState>) -> Result<(), String> {
    state.engine.send_command(AudioCommand::SetVolume(volume));
    Ok(())
}

#[tauri::command]
pub fn get_playback_state(state: State<'_, AppState>) -> PlaybackState {
    state.engine.get_state()
}

#[tauri::command]
pub fn get_position(state: State<'_, AppState>) -> u64 {
    state.engine.get_position_ms()
}

// ─── EQ Commands ───

#[tauri::command]
pub fn set_eq_bands(bands: [f32; 10], state: State<'_, AppState>) -> Result<(), String> {
    state.engine.send_command(AudioCommand::SetEqBands(bands));
    Ok(())
}

#[tauri::command]
pub fn set_eq_enabled(enabled: bool, state: State<'_, AppState>) -> Result<(), String> {
    state.engine.send_command(AudioCommand::SetEqEnabled(enabled));
    Ok(())
}

#[tauri::command]
pub fn get_eq_preset(name: String) -> Result<[f32; 10], String> {
    equalizer::get_preset(&name).ok_or_else(|| format!("Unknown preset: {}", name))
}

// ─── Device Commands ───

#[tauri::command]
pub fn get_audio_devices() -> Vec<AudioDeviceInfo> {
    crate::audio::engine::get_output_devices()
}

// ─── Metadata Commands ───

#[tauri::command]
pub fn read_file_metadata(path: String) -> Result<reader::TrackMetadata, String> {
    reader::read_metadata(&path)
}

#[tauri::command]
pub fn get_album_art_base64(path: String) -> Result<Option<String>, String> {
    reader::get_album_art_base64(&path)
}

// ─── File Dialog Commands ───

#[tauri::command]
pub async fn open_files_dialog(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let files = app.dialog().file()
        .add_filter("Audio Files", &["flac", "mp3", "wav", "ogg", "m4a", "aac", "wma"])
        .add_filter("FLAC", &["flac"])
        .add_filter("All Files", &["*"])
        .blocking_pick_files();

    match files {
        Some(paths) => Ok(paths.iter().map(|p| p.path.to_string_lossy().to_string()).collect()),
        None => Ok(vec![]),
    }
}

#[tauri::command]
pub async fn open_folder_dialog(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let folder = app.dialog().file().blocking_pick_folder();

    match folder {
        Some(path) => Ok(Some(path.path.to_string_lossy().to_string())),
        None => Ok(None),
    }
}
