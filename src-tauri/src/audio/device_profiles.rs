/// Per-device audio profiles.
///
/// Saves and loads user preferences for each output device:
///   - Exclusive/Shared mode
///   - Buffer size preference
///   - Volume level
///   - ReplayGain mode
///
/// Profiles are stored as JSON in the app data directory.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::engine::ReplayGainMode;

#[derive(Clone, Serialize, Deserialize)]
pub struct DeviceProfile {
    /// Device name (as reported by cpal).
    pub device_name: String,
    /// Whether to use WASAPI Exclusive mode (true) or Shared mode (false).
    pub exclusive_mode: bool,
    /// Preferred buffer size in frames (0 = system default).
    pub buffer_size: u32,
    /// Volume level (0.0 – 1.0).
    pub volume: f32,
    /// ReplayGain mode for this device.
    pub replaygain_mode: ReplayGainMode,
    /// Whether clipping prevention is active.
    pub clipping_prevention: bool,
}

impl Default for DeviceProfile {
    fn default() -> Self {
        Self {
            device_name: String::new(),
            exclusive_mode: false,
            buffer_size: 0,
            volume: 1.0,
            replaygain_mode: ReplayGainMode::Off,
            clipping_prevention: true,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct DeviceProfileStore {
    profiles: HashMap<String, DeviceProfile>,
}

impl DeviceProfileStore {
    /// Load profiles from disk. Returns empty store if file doesn't exist.
    pub fn load(app_data_dir: &PathBuf) -> Self {
        let path = app_data_dir.join("device_profiles.json");
        if let Ok(data) = std::fs::read_to_string(&path) {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// Save profiles to disk.
    pub fn save(&self, app_data_dir: &PathBuf) -> Result<(), String> {
        let path = app_data_dir.join("device_profiles.json");
        std::fs::create_dir_all(app_data_dir)
            .map_err(|e| format!("Failed to create dir: {}", e))?;
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Serialize failed: {}", e))?;
        std::fs::write(&path, json)
            .map_err(|e| format!("Write failed: {}", e))?;
        Ok(())
    }

    /// Get profile for a device (or default if none saved).
    pub fn get(&self, device_name: &str) -> DeviceProfile {
        self.profiles
            .get(device_name)
            .cloned()
            .unwrap_or_else(|| {
                let mut p = DeviceProfile::default();
                p.device_name = device_name.to_string();
                p
            })
    }

    /// Save/update profile for a device.
    pub fn set(&mut self, profile: DeviceProfile) {
        self.profiles.insert(profile.device_name.clone(), profile);
    }

    /// List all saved device profiles.
    pub fn list(&self) -> Vec<DeviceProfile> {
        self.profiles.values().cloned().collect()
    }

    /// Delete a profile.
    pub fn delete(&mut self, device_name: &str) {
        self.profiles.remove(device_name);
    }
}
