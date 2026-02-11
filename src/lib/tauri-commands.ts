import { invoke } from "@tauri-apps/api/core";
import type {
  PlaybackState,
  AudioDiagnostics,
  NullTestResult,
  AudioDeviceInfo,
  DeviceProfile,
  ReplayGainMode,
  TrackMetadata,
} from "./types";

// ─── Playback ───

export const playFile = (path: string) =>
  invoke<void>("play_file", { path });

export const pause = () => invoke<void>("pause");

export const resume = () => invoke<void>("resume");

export const stop = () => invoke<void>("stop");

export const seek = (positionSecs: number) =>
  invoke<void>("seek", { position_secs: positionSecs });

export const setVolume = (volume: number) =>
  invoke<void>("set_volume", { volume });

export const getPlaybackState = () =>
  invoke<PlaybackState>("get_playback_state");

export const getPosition = () => invoke<number>("get_position");

// ─── ReplayGain ───

export const setReplaygainMode = (mode: ReplayGainMode) =>
  invoke<void>("set_replaygain_mode", { mode });

export const setClippingPrevention = (enabled: boolean) =>
  invoke<void>("set_clipping_prevention", { enabled });

// ─── Diagnostics ───

export const getAudioDiagnostics = () =>
  invoke<AudioDiagnostics>("get_audio_diagnostics");

// ─── Null Test ───

export const runNullTest = (path: string) =>
  invoke<NullTestResult>("run_null_test", { path });

// ─── Devices ───

export const getAudioDevices = () =>
  invoke<AudioDeviceInfo[]>("get_audio_devices");

// ─── Device Profiles ───

export const getDeviceProfile = (deviceName: string) =>
  invoke<DeviceProfile>("get_device_profile", { device_name: deviceName });

export const saveDeviceProfile = (profile: DeviceProfile) =>
  invoke<void>("save_device_profile", { profile });

export const listDeviceProfiles = () =>
  invoke<DeviceProfile[]>("list_device_profiles");

export const deleteDeviceProfile = (deviceName: string) =>
  invoke<void>("delete_device_profile", { device_name: deviceName });

// ─── Metadata ───

export const readFileMetadata = (path: string) =>
  invoke<TrackMetadata>("read_file_metadata", { path });

export const getAlbumArtBase64 = (path: string) =>
  invoke<string | null>("get_album_art_base64", { path });

// ─── Dialogs ───

export const openFilesDialog = () =>
  invoke<string[]>("open_files_dialog");

export const openFolderDialog = () =>
  invoke<string | null>("open_folder_dialog");
