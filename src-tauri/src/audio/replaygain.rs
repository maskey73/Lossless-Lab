/// ReplayGain implementation.
///
/// Reads R128 / ReplayGain tags from audio files and applies gain adjustment
/// in the decoder thread. When mode is Off, the signal path is 100% untouched
/// (bit-perfect). Clipping prevention optionally limits gain to prevent
/// the adjusted signal from exceeding 0 dBFS.

use super::engine::{db_to_linear, ReplayGainMode};
use lofty::prelude::*;
use lofty::probe::Probe;

/// Per-track ReplayGain values read from metadata tags.
#[derive(Clone, serde::Serialize)]
pub struct ReplayGainInfo {
    /// Track gain in dB (e.g. -7.5).
    pub track_gain_db: Option<f32>,
    /// Track peak as linear value (e.g. 0.98).
    pub track_peak: Option<f32>,
    /// Album gain in dB.
    pub album_gain_db: Option<f32>,
    /// Album peak as linear value.
    pub album_peak: Option<f32>,
}

impl Default for ReplayGainInfo {
    fn default() -> Self {
        Self {
            track_gain_db: None,
            track_peak: None,
            album_gain_db: None,
            album_peak: None,
        }
    }
}

pub struct ReplayGainState {
    mode: ReplayGainMode,
    clipping_prevention: bool,
    info: ReplayGainInfo,
    /// Cached linear gain to apply. Recalculated when mode/info changes.
    gain_linear: f32,
}

impl ReplayGainState {
    pub fn new() -> Self {
        Self {
            mode: ReplayGainMode::Off,
            clipping_prevention: true,
            info: ReplayGainInfo::default(),
            gain_linear: 1.0,
        }
    }

    pub fn set_mode(&mut self, mode: ReplayGainMode) {
        self.mode = mode;
        self.recalculate_gain();
    }

    pub fn set_clipping_prevention(&mut self, on: bool) {
        self.clipping_prevention = on;
        self.recalculate_gain();
    }

    pub fn get_info(&self) -> &ReplayGainInfo {
        &self.info
    }

    pub fn get_mode(&self) -> ReplayGainMode {
        self.mode
    }

    /// Read ReplayGain tags from an audio file.
    pub fn load_from_file(&mut self, path: &str) {
        self.info = read_replaygain_tags(path).unwrap_or_default();
        self.recalculate_gain();
    }

    fn recalculate_gain(&mut self) {
        let gain_db = match self.mode {
            ReplayGainMode::Off => {
                self.gain_linear = 1.0;
                return;
            }
            ReplayGainMode::Track => self.info.track_gain_db,
            ReplayGainMode::Album => {
                // Fall back to track gain if album gain missing
                self.info.album_gain_db.or(self.info.track_gain_db)
            }
        };

        let Some(db) = gain_db else {
            // No gain tag found — passthrough
            self.gain_linear = 1.0;
            return;
        };

        let mut gain = db_to_linear(db);

        // Clipping prevention: limit gain so (gain * peak) <= 1.0
        if self.clipping_prevention {
            let peak = match self.mode {
                ReplayGainMode::Track => self.info.track_peak,
                ReplayGainMode::Album => self.info.album_peak.or(self.info.track_peak),
                ReplayGainMode::Off => None,
            };

            if let Some(peak) = peak {
                if peak > 0.0 {
                    let max_gain = 1.0 / peak;
                    if gain > max_gain {
                        gain = max_gain;
                    }
                }
            }
        }

        self.gain_linear = gain;
    }

    /// Apply ReplayGain to a buffer of interleaved samples.
    /// When mode is Off, this is a no-op (bit-perfect passthrough).
    #[inline]
    pub fn apply(&self, samples: &mut [f32]) {
        // Fast path: if gain is exactly 1.0, don't touch the data at all.
        // This ensures bit-perfect playback when ReplayGain is off or no tags found.
        if (self.gain_linear - 1.0).abs() < f32::EPSILON {
            return;
        }

        let g = self.gain_linear;
        for s in samples.iter_mut() {
            *s *= g;
        }
    }
}

/// Parse ReplayGain tags from an audio file using lofty.
fn read_replaygain_tags(path: &str) -> Result<ReplayGainInfo, String> {
    let tagged = Probe::open(path)
        .map_err(|e| format!("{}", e))?
        .read()
        .map_err(|e| format!("{}", e))?;

    let tag = match tagged.primary_tag().or_else(|| tagged.first_tag()) {
        Some(t) => t,
        None => return Ok(ReplayGainInfo::default()),
    };

    // Try standard ReplayGain tags (Vorbis Comments / ID3v2 TXXX / APE)
    let track_gain = find_tag_value(tag, &[
        "REPLAYGAIN_TRACK_GAIN",
        "replaygain_track_gain",
        "R128_TRACK_GAIN",
    ]);
    let track_peak = find_tag_value(tag, &[
        "REPLAYGAIN_TRACK_PEAK",
        "replaygain_track_peak",
    ]);
    let album_gain = find_tag_value(tag, &[
        "REPLAYGAIN_ALBUM_GAIN",
        "replaygain_album_gain",
        "R128_ALBUM_GAIN",
    ]);
    let album_peak = find_tag_value(tag, &[
        "REPLAYGAIN_ALBUM_PEAK",
        "replaygain_album_peak",
    ]);

    Ok(ReplayGainInfo {
        track_gain_db: parse_gain_value(&track_gain),
        track_peak: parse_peak_value(&track_peak),
        album_gain_db: parse_gain_value(&album_gain),
        album_peak: parse_peak_value(&album_peak),
    })
}

fn find_tag_value(tag: &lofty::tag::Tag, keys: &[&str]) -> Option<String> {
    for key in keys {
        // Try as ItemKey::Unknown (custom tags)
        if let Some(item) = tag.get_string(&lofty::tag::ItemKey::Unknown(key.to_string())) {
            return Some(item.to_string());
        }
    }
    None
}

/// Parse a gain value like "-7.5 dB" → -7.5
fn parse_gain_value(s: &Option<String>) -> Option<f32> {
    s.as_ref().and_then(|v| {
        v.trim()
            .trim_end_matches(" dB")
            .trim_end_matches(" db")
            .trim_end_matches("dB")
            .trim()
            .parse::<f32>()
            .ok()
    })
}

/// Parse a peak value like "0.988" → 0.988
fn parse_peak_value(s: &Option<String>) -> Option<f32> {
    s.as_ref().and_then(|v| v.trim().parse::<f32>().ok())
}
