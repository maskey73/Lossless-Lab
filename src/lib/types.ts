// ─── Types matching Rust backend structs (serde snake_case) ───

export interface PlaybackState {
  is_playing: boolean;
  is_paused: boolean;
  position_secs: number;
  duration_secs: number;
  sample_rate: number;
  bit_depth: number | null;
  channels: number;
  current_file: string | null;
  resampled: boolean;
}

export interface AudioDiagnostics {
  buffer_capacity: number;
  buffer_filled: number;
  buffer_fill_pct: number;
  latency_ms: number;
  dropout_count: number;
  output_sample_rate: number;
  output_channels: number;
  is_bit_perfect: boolean;
  shared_mode: boolean;
}

export interface NullTestResult {
  passed: boolean;
  total_samples: number;
  diff_samples: number;
  max_diff: number;
  rms_diff: number;
  summary: string;
}

export interface AudioDeviceInfo {
  name: string;
  is_default: boolean;
}

export type ReplayGainMode = "Off" | "Track" | "Album";

export interface DeviceProfile {
  device_name: string;
  exclusive_mode: boolean;
  buffer_size: number;
  volume: number;
  replaygain_mode: ReplayGainMode;
  clipping_prevention: boolean;
}

export interface TrackMetadata {
  title: string | null;
  artist: string | null;
  album: string | null;
  album_artist: string | null;
  year: number | null;
  genre: string | null;
  track_number: number | null;
  disc_number: number | null;
  duration_secs: number;
  sample_rate: number | null;
  bit_depth: number | null;
  channels: number | null;
  file_path: string;
  file_name: string;
  format: string;
  has_album_art: boolean;
}

// ─── Frontend-only types ───

export type View = "now-playing" | "library" | "settings";

export type RepeatMode = "off" | "all" | "one";
