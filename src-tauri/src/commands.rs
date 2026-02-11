use crate::audio::device_profiles::{DeviceProfile, DeviceProfileStore};
use crate::audio::engine::{
    AudioCommand, AudioDeviceInfo, AudioDiagnostics, AudioEngine, PlaybackState, ReplayGainMode,
};
use crate::audio::null_test;
use crate::metadata::reader;
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

pub struct AppState {
    pub engine: Arc<AudioEngine>,
    pub device_profiles: Arc<Mutex<DeviceProfileStore>>,
    pub app_data_dir: PathBuf,
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

// ─── ReplayGain Commands ───

#[tauri::command]
pub fn set_replaygain_mode(mode: ReplayGainMode, state: State<'_, AppState>) -> Result<(), String> {
    state.engine.send_command(AudioCommand::SetReplayGain(mode));
    Ok(())
}

#[tauri::command]
pub fn set_clipping_prevention(enabled: bool, state: State<'_, AppState>) -> Result<(), String> {
    state
        .engine
        .send_command(AudioCommand::SetClippingPrevention(enabled));
    Ok(())
}

// ─── Audio Diagnostics (Latency Analyzer) ───

#[tauri::command]
pub fn get_audio_diagnostics(state: State<'_, AppState>) -> AudioDiagnostics {
    state.engine.get_diagnostics()
}

// ─── Bit-Perfect Null Test ───

#[tauri::command]
pub fn run_null_test(path: String) -> Result<null_test::NullTestResult, String> {
    null_test::run_null_test(&path)
}

// ─── Device Commands ───

#[tauri::command]
pub fn get_audio_devices() -> Vec<AudioDeviceInfo> {
    crate::audio::engine::get_output_devices()
}

// ─── Per-Device Audio Profiles ───

#[tauri::command]
pub fn get_device_profile(
    device_name: String,
    state: State<'_, AppState>,
) -> DeviceProfile {
    state.device_profiles.lock().get(&device_name)
}

#[tauri::command]
pub fn save_device_profile(
    profile: DeviceProfile,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut store = state.device_profiles.lock();
    store.set(profile);
    store.save(&state.app_data_dir)
}

#[tauri::command]
pub fn list_device_profiles(state: State<'_, AppState>) -> Vec<DeviceProfile> {
    state.device_profiles.lock().list()
}

#[tauri::command]
pub fn delete_device_profile(
    device_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut store = state.device_profiles.lock();
    store.delete(&device_name);
    store.save(&state.app_data_dir)
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

    let files = app
        .dialog()
        .file()
        .add_filter(
            "Audio Files",
            &["flac", "mp3", "wav", "ogg", "m4a", "aac", "wma"],
        )
        .add_filter("FLAC", &["flac"])
        .add_filter("All Files", &["*"])
        .blocking_pick_files();

    match files {
        Some(paths) => Ok(paths
            .iter()
            .map(|p| p.path.to_string_lossy().to_string())
            .collect()),
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
