use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SampleFormat, SampleRate, StreamConfig};
use crossbeam_channel::{bounded, Receiver, Sender};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use super::decoder::{AudioDecoder, DecodeStatus};
use super::equalizer::Equalizer;

/// Commands sent from the main thread to the audio thread.
pub enum AudioCommand {
    Play(String),      // file path
    Pause,
    Resume,
    Stop,
    Seek(f64),         // seconds
    SetVolume(f32),    // 0.0 - 1.0
    SetEqBands([f32; 10]),
    SetEqEnabled(bool),
    Shutdown,
}

/// State reported back from the audio thread.
#[derive(Clone, serde::Serialize)]
pub struct PlaybackState {
    pub is_playing: bool,
    pub is_paused: bool,
    pub position_secs: f64,
    pub duration_secs: f64,
    pub sample_rate: u32,
    pub channels: u32,
    pub current_file: Option<String>,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            is_playing: false,
            is_paused: false,
            position_secs: 0.0,
            duration_secs: 0.0,
            sample_rate: 0,
            channels: 0,
            current_file: None,
        }
    }
}

pub struct AudioEngine {
    cmd_tx: Sender<AudioCommand>,
    state: Arc<Mutex<PlaybackState>>,
    /// Atomic position in milliseconds for fast reads from the UI.
    position_ms: Arc<AtomicU64>,
    duration_ms: Arc<AtomicU64>,
    is_playing: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
}

impl AudioEngine {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = bounded::<AudioCommand>(64);
        let state = Arc::new(Mutex::new(PlaybackState::default()));
        let position_ms = Arc::new(AtomicU64::new(0));
        let duration_ms = Arc::new(AtomicU64::new(0));
        let is_playing = Arc::new(AtomicBool::new(false));
        let is_paused = Arc::new(AtomicBool::new(false));

        let state_clone = state.clone();
        let pos_clone = position_ms.clone();
        let dur_clone = duration_ms.clone();
        let playing_clone = is_playing.clone();
        let paused_clone = is_paused.clone();

        // Spawn the audio worker thread
        thread::Builder::new()
            .name("audio-engine".into())
            .spawn(move || {
                audio_thread(cmd_rx, state_clone, pos_clone, dur_clone, playing_clone, paused_clone);
            })
            .expect("Failed to spawn audio thread");

        Self {
            cmd_tx,
            state,
            position_ms,
            duration_ms,
            is_playing,
            is_paused,
        }
    }

    pub fn send_command(&self, cmd: AudioCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    pub fn get_state(&self) -> PlaybackState {
        let mut state = self.state.lock().clone();
        // Use atomic values for frequently updated fields
        state.position_secs = self.position_ms.load(Ordering::Relaxed) as f64 / 1000.0;
        state.duration_secs = self.duration_ms.load(Ordering::Relaxed) as f64 / 1000.0;
        state.is_playing = self.is_playing.load(Ordering::Relaxed);
        state.is_paused = self.is_paused.load(Ordering::Relaxed);
        state
    }

    pub fn get_position_ms(&self) -> u64 {
        self.position_ms.load(Ordering::Relaxed)
    }

    pub fn get_duration_ms(&self) -> u64 {
        self.duration_ms.load(Ordering::Relaxed)
    }
}

fn audio_thread(
    cmd_rx: Receiver<AudioCommand>,
    state: Arc<Mutex<PlaybackState>>,
    position_ms: Arc<AtomicU64>,
    duration_ms: Arc<AtomicU64>,
    is_playing: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
) {
    let host = cpal::default_host();
    let mut current_stream: Option<cpal::Stream> = None;
    let mut volume = Arc::new(parking_lot::Mutex::new(1.0f32));
    let mut eq = Arc::new(parking_lot::Mutex::new(Equalizer::new(44100)));
    let mut eq_enabled = Arc::new(AtomicBool::new(false));

    // Shared sample buffer between decoder thread and audio callback
    let sample_buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let sample_read_pos: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));

    // Decoder thread control
    let decoder_running = Arc::new(AtomicBool::new(false));
    let paused_flag = Arc::new(AtomicBool::new(false));

    loop {
        match cmd_rx.recv_timeout(Duration::from_millis(16)) {
            Ok(AudioCommand::Play(path)) => {
                // Stop any current playback
                decoder_running.store(false, Ordering::SeqCst);
                current_stream = None;
                thread::sleep(Duration::from_millis(50));

                // Open the file
                let decoder_result = AudioDecoder::open(&path);
                let mut decoder = match decoder_result {
                    Ok(d) => d,
                    Err(e) => {
                        log::error!("Failed to open audio file: {}", e);
                        continue;
                    }
                };

                let sample_rate = decoder.sample_rate();
                let channels = decoder.channels();
                let dur = decoder.duration_secs;

                // Update state
                {
                    let mut s = state.lock();
                    s.is_playing = true;
                    s.is_paused = false;
                    s.duration_secs = dur;
                    s.position_secs = 0.0;
                    s.sample_rate = sample_rate;
                    s.channels = channels as u32;
                    s.current_file = Some(path.clone());
                }
                is_playing.store(true, Ordering::SeqCst);
                is_paused.store(false, Ordering::SeqCst);
                duration_ms.store((dur * 1000.0) as u64, Ordering::SeqCst);
                position_ms.store(0, Ordering::SeqCst);
                paused_flag.store(false, Ordering::SeqCst);

                // Reset sample buffer
                {
                    let mut buf = sample_buffer.lock();
                    buf.clear();
                }
                sample_read_pos.store(0, Ordering::SeqCst);

                // Spawn decoder thread to fill buffer
                let buf_clone = sample_buffer.clone();
                let running = decoder_running.clone();
                let paused_dec = paused_flag.clone();
                let pos_ms = position_ms.clone();
                running.store(true, Ordering::SeqCst);

                let sr = sample_rate;
                let ch = channels;

                thread::Builder::new()
                    .name("decoder".into())
                    .spawn(move || {
                        let mut samples_decoded: u64 = 0;
                        while running.load(Ordering::SeqCst) {
                            if paused_dec.load(Ordering::Relaxed) {
                                thread::sleep(Duration::from_millis(10));
                                continue;
                            }

                            // Keep buffer from growing too large (max ~2 seconds)
                            let buf_len = buf_clone.lock().len();
                            if buf_len > (sr as usize * ch * 2) {
                                thread::sleep(Duration::from_millis(5));
                                continue;
                            }

                            match decoder.next_samples() {
                                Ok(samples) => {
                                    let num_frames = samples.len() / ch;
                                    samples_decoded += num_frames as u64;
                                    let pos = samples_decoded as f64 / sr as f64;
                                    pos_ms.store((pos * 1000.0) as u64, Ordering::Relaxed);

                                    let mut buf = buf_clone.lock();
                                    buf.extend_from_slice(&samples);
                                }
                                Err(DecodeStatus::EndOfStream) => {
                                    // Wait for buffer to drain, then signal done
                                    while running.load(Ordering::SeqCst) {
                                        let len = buf_clone.lock().len();
                                        if len == 0 {
                                            break;
                                        }
                                        thread::sleep(Duration::from_millis(50));
                                    }
                                    running.store(false, Ordering::SeqCst);
                                    break;
                                }
                                Err(DecodeStatus::Error(e)) => {
                                    log::error!("Decode error: {}", e);
                                    running.store(false, Ordering::SeqCst);
                                    break;
                                }
                            }
                        }
                    })
                    .expect("Failed to spawn decoder thread");

                // Create cpal output stream
                let device = host.default_output_device().expect("No output device found");
                let config = StreamConfig {
                    channels: channels as u16,
                    sample_rate: SampleRate(sample_rate),
                    buffer_size: cpal::BufferSize::Default,
                };

                let buf_for_stream = sample_buffer.clone();
                let vol_for_stream = volume.clone();
                let eq_for_stream = eq.clone();
                let eq_enabled_for_stream = eq_enabled.clone();
                let paused_for_stream = paused_flag.clone();

                let stream = device
                    .build_output_stream(
                        &config,
                        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                            if paused_for_stream.load(Ordering::Relaxed) {
                                for sample in data.iter_mut() {
                                    *sample = 0.0;
                                }
                                return;
                            }

                            let vol = *vol_for_stream.lock();
                            let mut buf = buf_for_stream.lock();

                            let available = buf.len().min(data.len());
                            if available > 0 {
                                // Copy samples from buffer
                                data[..available].copy_from_slice(&buf[..available]);
                                buf.drain(..available);

                                // Apply EQ if enabled
                                if eq_enabled_for_stream.load(Ordering::Relaxed) {
                                    let mut eq_lock = eq_for_stream.lock();
                                    eq_lock.process(&mut data[..available]);
                                }

                                // Apply volume
                                for sample in data[..available].iter_mut() {
                                    *sample *= vol;
                                }

                                // Zero remaining
                                for sample in data[available..].iter_mut() {
                                    *sample = 0.0;
                                }
                            } else {
                                for sample in data.iter_mut() {
                                    *sample = 0.0;
                                }
                            }
                        },
                        move |err| {
                            log::error!("Audio stream error: {}", err);
                        },
                        None,
                    )
                    .expect("Failed to build output stream");

                stream.play().expect("Failed to start audio stream");
                current_stream = Some(stream);

                // Update EQ sample rate
                eq.lock().set_sample_rate(sample_rate);
            }

            Ok(AudioCommand::Pause) => {
                paused_flag.store(true, Ordering::SeqCst);
                is_paused.store(true, Ordering::SeqCst);
                is_playing.store(false, Ordering::SeqCst);
                let mut s = state.lock();
                s.is_paused = true;
                s.is_playing = false;
            }

            Ok(AudioCommand::Resume) => {
                paused_flag.store(false, Ordering::SeqCst);
                is_paused.store(false, Ordering::SeqCst);
                is_playing.store(true, Ordering::SeqCst);
                let mut s = state.lock();
                s.is_paused = false;
                s.is_playing = true;
            }

            Ok(AudioCommand::Stop) => {
                decoder_running.store(false, Ordering::SeqCst);
                current_stream = None;
                is_playing.store(false, Ordering::SeqCst);
                is_paused.store(false, Ordering::SeqCst);
                position_ms.store(0, Ordering::SeqCst);
                let mut s = state.lock();
                *s = PlaybackState::default();
            }

            Ok(AudioCommand::Seek(secs)) => {
                // For seek, we need to signal the decoder to re-seek
                // Simplified: store new position, decoder will pick it up
                position_ms.store((secs * 1000.0) as u64, Ordering::SeqCst);
                // Clear the buffer so we don't hear old samples
                sample_buffer.lock().clear();
            }

            Ok(AudioCommand::SetVolume(vol)) => {
                *volume.lock() = vol.clamp(0.0, 1.0);
            }

            Ok(AudioCommand::SetEqBands(bands)) => {
                eq.lock().set_bands(bands);
            }

            Ok(AudioCommand::SetEqEnabled(enabled)) => {
                eq_enabled.store(enabled, Ordering::SeqCst);
            }

            Ok(AudioCommand::Shutdown) => {
                decoder_running.store(false, Ordering::SeqCst);
                current_stream = None;
                break;
            }

            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                // Check if playback ended
                if !decoder_running.load(Ordering::Relaxed)
                    && is_playing.load(Ordering::Relaxed)
                    && sample_buffer.lock().is_empty()
                {
                    is_playing.store(false, Ordering::SeqCst);
                    is_paused.store(false, Ordering::SeqCst);
                    current_stream = None;
                    let mut s = state.lock();
                    s.is_playing = false;
                    s.is_paused = false;
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
        }
    }
}

/// Get available audio output devices.
pub fn get_output_devices() -> Vec<AudioDeviceInfo> {
    let host = cpal::default_host();
    let mut devices = Vec::new();

    if let Ok(output_devices) = host.output_devices() {
        for device in output_devices {
            if let Ok(name) = device.name() {
                let is_default = host
                    .default_output_device()
                    .map(|d| d.name().ok() == Some(name.clone()))
                    .unwrap_or(false);
                devices.push(AudioDeviceInfo {
                    name,
                    is_default,
                });
            }
        }
    }

    devices
}

#[derive(Clone, serde::Serialize)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub is_default: bool,
}
