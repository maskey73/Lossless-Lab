use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleRate, StreamConfig};
use crossbeam_channel::{bounded, Receiver, Sender};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use super::decoder::{AudioDecoder, DecodeStatus};
use super::replaygain::ReplayGainState;
use super::ring_buffer::RingBuffer;

// ─── Safety Constants ───

/// Fade ramp in samples. 256 samples @ 44.1kHz ≈ 5.8ms. Eliminates all pops.
const FADE_RAMP_SAMPLES: usize = 256;

/// Hard limiter ceiling. Applied ONLY when volume < 1.0 or ReplayGain is active.
/// In bit-perfect mode (vol=1.0, RG=off), NO limiting is applied.
const HARD_LIMIT_CEILING: f32 = 0.99;

/// Ring buffer size. Power of 2 for lock-free masking.
/// 131072 samples ≈ 1.5s at 44.1kHz stereo, ~0.34s at 192kHz stereo.
/// Balance between latency and buffer safety.
const RING_BUFFER_SIZE: usize = 131072;

// ─── Commands ───

pub enum AudioCommand {
    Play(String),
    Pause,
    Resume,
    Stop,
    Seek(f64),
    SetVolume(f32),
    SetReplayGain(ReplayGainMode),
    SetClippingPrevention(bool),
    Shutdown,
}

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ReplayGainMode {
    Off,
    Track,
    Album,
}

// ─── Playback State ───

#[derive(Clone, serde::Serialize)]
pub struct PlaybackState {
    pub is_playing: bool,
    pub is_paused: bool,
    pub position_secs: f64,
    pub duration_secs: f64,
    pub sample_rate: u32,
    pub bit_depth: Option<u8>,
    pub channels: u32,
    pub current_file: Option<String>,
    /// True if the OS is resampling (device doesn't support file's native sample rate).
    pub resampled: bool,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            is_playing: false,
            is_paused: false,
            position_secs: 0.0,
            duration_secs: 0.0,
            sample_rate: 0,
            bit_depth: None,
            channels: 0,
            current_file: None,
            resampled: false,
        }
    }
}

// ─── Audio Diagnostics (Latency Analyzer) ───

#[derive(Clone, serde::Serialize)]
pub struct AudioDiagnostics {
    /// Ring buffer capacity in samples.
    pub buffer_capacity: usize,
    /// Ring buffer currently filled (samples).
    pub buffer_filled: usize,
    /// Buffer fill percentage (0–100).
    pub buffer_fill_pct: f32,
    /// Estimated output latency in milliseconds.
    pub latency_ms: f64,
    /// Total number of buffer underruns (dropouts) since playback started.
    pub dropout_count: u64,
    /// Current sample rate being output.
    pub output_sample_rate: u32,
    /// Number of output channels.
    pub output_channels: u32,
    /// True when signal path is fully bit-perfect (vol=1.0, RG=off, no resample).
    pub is_bit_perfect: bool,
    /// Always true for MVP — cpal uses WASAPI Shared mode.
    pub shared_mode: bool,
}

// ─── Fade State Machine ───
// Uses equal-power (cosine) curves for professional-grade transitions.

#[derive(Clone, Copy, PartialEq)]
enum FadeState {
    Playing,
    FadingOut,
    Silent,
    FadingIn,
}

// ─── Audio Engine ───

pub struct AudioEngine {
    cmd_tx: Sender<AudioCommand>,
    state: Arc<Mutex<PlaybackState>>,
    position_ms: Arc<AtomicU64>,
    duration_ms: Arc<AtomicU64>,
    is_playing: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
    ring_buffer: Arc<RingBuffer>,
    dropout_count: Arc<AtomicU64>,
    current_sample_rate: Arc<AtomicU32>,
    current_channels: Arc<AtomicU32>,
    /// True when the signal path is bit-perfect (vol=1.0, RG=off).
    is_bit_perfect: Arc<AtomicBool>,
}

impl AudioEngine {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = bounded::<AudioCommand>(64);
        let state = Arc::new(Mutex::new(PlaybackState::default()));
        let position_ms = Arc::new(AtomicU64::new(0));
        let duration_ms = Arc::new(AtomicU64::new(0));
        let is_playing = Arc::new(AtomicBool::new(false));
        let is_paused = Arc::new(AtomicBool::new(false));
        let ring_buffer = Arc::new(RingBuffer::new(RING_BUFFER_SIZE));
        let dropout_count = Arc::new(AtomicU64::new(0));
        let current_sample_rate = Arc::new(AtomicU32::new(0));
        let current_channels = Arc::new(AtomicU32::new(0));
        let is_bit_perfect = Arc::new(AtomicBool::new(true));

        let state_c = state.clone();
        let pos_c = position_ms.clone();
        let dur_c = duration_ms.clone();
        let play_c = is_playing.clone();
        let pause_c = is_paused.clone();
        let ring_c = ring_buffer.clone();
        let drop_c = dropout_count.clone();
        let sr_c = current_sample_rate.clone();
        let ch_c = current_channels.clone();
        let bp_c = is_bit_perfect.clone();

        thread::Builder::new()
            .name("audio-engine".into())
            .spawn(move || {
                audio_thread(
                    cmd_rx, state_c, pos_c, dur_c, play_c, pause_c,
                    ring_c, drop_c, sr_c, ch_c, bp_c,
                );
            })
            .expect("Failed to spawn audio thread");

        Self {
            cmd_tx,
            state,
            position_ms,
            duration_ms,
            is_playing,
            is_paused,
            ring_buffer,
            dropout_count,
            current_sample_rate,
            current_channels,
            is_bit_perfect,
        }
    }

    pub fn send_command(&self, cmd: AudioCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    pub fn get_state(&self) -> PlaybackState {
        let mut s = self.state.lock().clone();
        s.position_secs = self.position_ms.load(Ordering::Relaxed) as f64 / 1000.0;
        s.duration_secs = self.duration_ms.load(Ordering::Relaxed) as f64 / 1000.0;
        s.is_playing = self.is_playing.load(Ordering::Relaxed);
        s.is_paused = self.is_paused.load(Ordering::Relaxed);
        s
    }

    pub fn get_position_ms(&self) -> u64 {
        self.position_ms.load(Ordering::Relaxed)
    }

    pub fn get_duration_ms(&self) -> u64 {
        self.duration_ms.load(Ordering::Relaxed)
    }

    /// Returns live audio diagnostics for the latency analyzer UI.
    pub fn get_diagnostics(&self) -> AudioDiagnostics {
        let filled = self.ring_buffer.available_read();
        let capacity = RING_BUFFER_SIZE;
        let sr = self.current_sample_rate.load(Ordering::Relaxed);
        let ch = self.current_channels.load(Ordering::Relaxed).max(1);

        let latency_ms = if sr > 0 {
            (filled as f64 / ch as f64) / sr as f64 * 1000.0
        } else {
            0.0
        };

        AudioDiagnostics {
            buffer_capacity: capacity,
            buffer_filled: filled,
            buffer_fill_pct: (filled as f32 / capacity as f32) * 100.0,
            latency_ms,
            dropout_count: self.dropout_count.load(Ordering::Relaxed),
            output_sample_rate: sr,
            output_channels: ch,
            is_bit_perfect: self.is_bit_perfect.load(Ordering::Relaxed),
            shared_mode: true, // cpal always uses WASAPI Shared — MVP limitation
        }
    }
}

// ─── Atomic f32 helpers (lock-free volume) ───

#[inline]
fn f32_to_atomic(v: f32) -> u32 {
    v.to_bits()
}
#[inline]
fn atomic_to_f32(b: u32) -> f32 {
    f32::from_bits(b)
}

// ─── Equal-power fade curve ───
// Cosine curve eliminates the perceived 3dB dip of linear fades.
// Standard in professional audio (Pro Tools, Ableton, foobar2000).

#[inline]
fn equal_power_gain(progress: f32) -> f32 {
    // progress: 0.0 = silent, 1.0 = full volume
    // sin(progress * π/2) gives equal-power curve
    (progress * std::f32::consts::FRAC_PI_2).sin()
}

// ─── Audio Thread ───

fn audio_thread(
    cmd_rx: Receiver<AudioCommand>,
    state: Arc<Mutex<PlaybackState>>,
    position_ms: Arc<AtomicU64>,
    duration_ms: Arc<AtomicU64>,
    is_playing: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
    ring_buffer: Arc<RingBuffer>,
    dropout_count: Arc<AtomicU64>,
    current_sample_rate: Arc<AtomicU32>,
    current_channels: Arc<AtomicU32>,
    is_bit_perfect: Arc<AtomicBool>,
) {
    let host = cpal::default_host();
    let mut current_stream: Option<cpal::Stream> = None;

    // Lock-free volume (atomic f32 via bit cast)
    let volume = Arc::new(AtomicU32::new(f32_to_atomic(1.0)));

    // ReplayGain state — applied in the decoder thread, not the callback
    let rg_state = Arc::new(Mutex::new(ReplayGainState::new()));

    // Bit-perfect flag — shared with callback for zero-processing passthrough
    let bit_perfect_cb = Arc::new(AtomicBool::new(true));

    // Fade request flags (atomic — callback reads, engine thread writes)
    let fade_req_pause = Arc::new(AtomicBool::new(false));
    let fade_req_resume = Arc::new(AtomicBool::new(false));
    let fade_req_stop = Arc::new(AtomicBool::new(false));

    // Decoder thread control
    let decoder_running = Arc::new(AtomicBool::new(false));
    let decoder_paused = Arc::new(AtomicBool::new(false));
    let seek_request_ms = Arc::new(AtomicU64::new(u64::MAX));

    /// Recalculate whether the signal path is bit-perfect.
    /// Bit-perfect = volume is exactly 1.0 AND ReplayGain is OFF (gain_linear ≈ 1.0).
    fn update_bit_perfect(
        volume: &AtomicU32,
        rg_state: &Mutex<ReplayGainState>,
        is_bit_perfect: &AtomicBool,
        bit_perfect_cb: &AtomicBool,
    ) {
        let vol = atomic_to_f32(volume.load(Ordering::Relaxed));
        let rg = rg_state.lock();
        let bp = (vol - 1.0).abs() < f32::EPSILON && rg.get_mode() == ReplayGainMode::Off;
        is_bit_perfect.store(bp, Ordering::SeqCst);
        bit_perfect_cb.store(bp, Ordering::SeqCst);
    }

    loop {
        match cmd_rx.recv_timeout(Duration::from_millis(16)) {
            Ok(AudioCommand::Play(path)) => {
                // Stop current playback
                decoder_running.store(false, Ordering::SeqCst);
                current_stream = None;
                thread::sleep(Duration::from_millis(50));

                // Open file
                let mut decoder = match AudioDecoder::open(&path) {
                    Ok(d) => d,
                    Err(e) => {
                        log::error!("Failed to open: {}", e);
                        continue;
                    }
                };

                let sr = decoder.sample_rate();
                let ch = decoder.channels();
                let dur = decoder.duration_secs;
                let bit_depth = decoder.bit_depth();

                // Read ReplayGain tags from file
                {
                    let mut rg = rg_state.lock();
                    rg.load_from_file(&path);
                }

                // ── Sample rate validation (A2) ──
                // Check if the output device actually supports the file's sample rate.
                let device = host.default_output_device().expect("No output device");
                let mut resampled = false;
                let actual_sr = if let Ok(configs) = device.supported_output_configs() {
                    let supports_sr = configs.into_iter().any(|range| {
                        sr >= range.min_sample_rate().0 && sr <= range.max_sample_rate().0
                            && range.channels() as usize >= ch
                    });
                    if supports_sr {
                        sr
                    } else {
                        // Device doesn't support this sample rate — use closest supported
                        log::warn!(
                            "Device doesn't natively support {}Hz. OS will resample (not bit-perfect).",
                            sr
                        );
                        resampled = true;
                        sr // Still request it — let cpal/WASAPI handle the conversion
                    }
                } else {
                    sr // Can't query — hope for the best
                };

                // Update state
                {
                    let mut s = state.lock();
                    s.is_playing = true;
                    s.is_paused = false;
                    s.duration_secs = dur;
                    s.position_secs = 0.0;
                    s.sample_rate = sr;
                    s.bit_depth = bit_depth;
                    s.channels = ch as u32;
                    s.current_file = Some(path.clone());
                    s.resampled = resampled;
                }
                is_playing.store(true, Ordering::SeqCst);
                is_paused.store(false, Ordering::SeqCst);
                duration_ms.store((dur * 1000.0) as u64, Ordering::SeqCst);
                position_ms.store(0, Ordering::SeqCst);
                current_sample_rate.store(sr, Ordering::SeqCst);
                current_channels.store(ch as u32, Ordering::SeqCst);
                dropout_count.store(0, Ordering::SeqCst);

                // Update bit-perfect status
                update_bit_perfect(&volume, &rg_state, &is_bit_perfect, &bit_perfect_cb);
                // If resampled, it's never truly bit-perfect at the DAC level
                if resampled {
                    is_bit_perfect.store(false, Ordering::SeqCst);
                    bit_perfect_cb.store(false, Ordering::SeqCst);
                }

                // Reset ring buffer and flags
                ring_buffer.clear();
                fade_req_pause.store(false, Ordering::SeqCst);
                fade_req_resume.store(false, Ordering::SeqCst);
                fade_req_stop.store(false, Ordering::SeqCst);
                decoder_paused.store(false, Ordering::SeqCst);
                seek_request_ms.store(u64::MAX, Ordering::SeqCst);

                // ── Spawn decoder thread ──
                // Pure signal path: decode → (optional ReplayGain) → ring buffer
                // No EQ, no DSP — bit-perfect when ReplayGain is off.
                let ring_c = ring_buffer.clone();
                let running = decoder_running.clone();
                let paused_d = decoder_paused.clone();
                let pos_ms = position_ms.clone();
                let rg_c = rg_state.clone();
                let seek_r = seek_request_ms.clone();
                running.store(true, Ordering::SeqCst);

                thread::Builder::new()
                    .name("decoder".into())
                    .spawn(move || {
                        let mut samples_decoded: u64 = 0;

                        while running.load(Ordering::SeqCst) {
                            // Check seek request
                            let seek_val = seek_r.load(Ordering::SeqCst);
                            if seek_val != u64::MAX {
                                let secs = seek_val as f64 / 1000.0;
                                seek_r.store(u64::MAX, Ordering::SeqCst);
                                ring_c.clear();
                                if let Err(e) = decoder.seek(secs) {
                                    log::error!("Seek failed: {}", e);
                                }
                                samples_decoded = (secs * sr as f64) as u64;
                                continue;
                            }

                            // Pause check
                            if paused_d.load(Ordering::Relaxed) {
                                thread::sleep(Duration::from_millis(10));
                                continue;
                            }

                            // Backpressure — don't flood buffer (1 second threshold)
                            if ring_c.available_read() > (sr as usize * ch) {
                                thread::sleep(Duration::from_millis(5));
                                continue;
                            }

                            // Decode
                            match decoder.next_samples() {
                                Ok(mut samples) => {
                                    let frames = samples.len() / ch;
                                    samples_decoded += frames as u64;
                                    let pos = samples_decoded as f64 / sr as f64;
                                    pos_ms.store((pos * 1000.0) as u64, Ordering::Relaxed);

                                    // Apply ReplayGain if enabled (the ONLY processing in the path)
                                    {
                                        let rg = rg_c.lock();
                                        rg.apply(&mut samples);
                                    }

                                    // Write to lock-free ring buffer
                                    ring_c.write(&samples);
                                }
                                Err(DecodeStatus::EndOfStream) => {
                                    // Wait for ring buffer to drain before signaling done
                                    while running.load(Ordering::SeqCst) {
                                        if ring_c.available_read() == 0 {
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

                // ── Create cpal output stream ──
                let config = StreamConfig {
                    channels: ch as u16,
                    sample_rate: SampleRate(actual_sr),
                    buffer_size: cpal::BufferSize::Default,
                };

                let ring_cb = ring_buffer.clone();
                let vol_cb = volume.clone();
                let bp_cb = bit_perfect_cb.clone();
                let pause_cb = fade_req_pause.clone();
                let resume_cb = fade_req_resume.clone();
                let stop_cb = fade_req_stop.clone();
                let drop_cb = dropout_count.clone();

                // ── AUDIO CALLBACK ──
                // Rules: NO locks, NO allocs, NO blocking.
                // Only atomics + lock-free ring buffer.
                //
                // AUDIOPHILE SIGNAL PATH:
                //   Bit-perfect mode (vol=1.0, RG=off): raw samples → output (ZERO processing)
                //   Normal mode: samples × volume → hard limiter → output
                //
                // Equal-power cosine fades on all transitions (no pops, no perceived dips).
                let stream = device
                    .build_output_stream(
                        &config,
                        {
                            let mut fade = FadeState::Playing;
                            let mut fade_ctr: usize = FADE_RAMP_SAMPLES;
                            let ch_count = ch;

                            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                                // Check fade requests (atomic swap — one-shot triggers)
                                if stop_cb.swap(false, Ordering::Relaxed) {
                                    fade = FadeState::FadingOut;
                                    fade_ctr = FADE_RAMP_SAMPLES;
                                }
                                if pause_cb.swap(false, Ordering::Relaxed) {
                                    if fade == FadeState::Playing || fade == FadeState::FadingIn {
                                        fade = FadeState::FadingOut;
                                        fade_ctr = FADE_RAMP_SAMPLES;
                                    }
                                }
                                if resume_cb.swap(false, Ordering::Relaxed) {
                                    if fade == FadeState::Silent || fade == FadeState::FadingOut {
                                        fade = FadeState::FadingIn;
                                        fade_ctr = 0;
                                    }
                                }

                                let vol = atomic_to_f32(vol_cb.load(Ordering::Relaxed));
                                let bit_perfect = bp_cb.load(Ordering::Relaxed);

                                match fade {
                                    FadeState::Silent => {
                                        for s in data.iter_mut() {
                                            *s = 0.0;
                                        }
                                    }

                                    FadeState::Playing => {
                                        let read = ring_cb.read(data);

                                        if bit_perfect {
                                            // ── BIT-PERFECT PASSTHROUGH ──
                                            // Vol=1.0 and RG=off: NO multiply, NO clamp.
                                            // Every sample passes through untouched.
                                            // This is the foobar2000/Qobuz gold standard.
                                            // (samples already in data from ring_cb.read)
                                        } else {
                                            // Normal mode: apply volume + hard limiter
                                            for s in data[..read].iter_mut() {
                                                *s = hard_limit(*s * vol);
                                            }
                                        }

                                        // Buffer underrun — fade out gracefully + count dropout
                                        if read < data.len() {
                                            if read > 0 {
                                                drop_cb.fetch_add(1, Ordering::Relaxed);
                                            }
                                            // Fade out the tail of what we did get
                                            let ramp = read.min(FADE_RAMP_SAMPLES);
                                            for i in 0..ramp {
                                                let idx = read - ramp + i;
                                                let progress = 1.0 - (i as f32 / ramp as f32);
                                                let g = equal_power_gain(progress);
                                                data[idx] *= g;
                                            }
                                            // Zero-fill the rest
                                            for s in data[read..].iter_mut() {
                                                *s = 0.0;
                                            }
                                        }
                                    }

                                    FadeState::FadingOut => {
                                        let read = ring_cb.read(data);
                                        let frames = read / ch_count.max(1);
                                        let mut frame_idx = 0;

                                        for frame_start in (0..read).step_by(ch_count.max(1)) {
                                            if fade_ctr == 0 {
                                                // Fade complete — zero remaining
                                                for c in 0..ch_count {
                                                    if frame_start + c < read {
                                                        data[frame_start + c] = 0.0;
                                                    }
                                                }
                                            } else {
                                                let progress =
                                                    fade_ctr as f32 / FADE_RAMP_SAMPLES as f32;
                                                let g = equal_power_gain(progress);
                                                for c in 0..ch_count {
                                                    if frame_start + c < read {
                                                        let s = &mut data[frame_start + c];
                                                        *s = if bit_perfect {
                                                            *s * g
                                                        } else {
                                                            hard_limit(*s * vol * g)
                                                        };
                                                    }
                                                }
                                                fade_ctr = fade_ctr.saturating_sub(1);
                                            }
                                            frame_idx += 1;
                                        }
                                        for s in data[read..].iter_mut() {
                                            *s = 0.0;
                                        }
                                        if fade_ctr == 0 {
                                            fade = FadeState::Silent;
                                        }
                                    }

                                    FadeState::FadingIn => {
                                        let read = ring_cb.read(data);

                                        for frame_start in (0..read).step_by(ch_count.max(1)) {
                                            let progress = if fade_ctr >= FADE_RAMP_SAMPLES {
                                                1.0
                                            } else {
                                                fade_ctr as f32 / FADE_RAMP_SAMPLES as f32
                                            };
                                            let g = equal_power_gain(progress);
                                            for c in 0..ch_count {
                                                if frame_start + c < read {
                                                    let s = &mut data[frame_start + c];
                                                    *s = if bit_perfect && progress >= 1.0 {
                                                        *s // Full volume, bit-perfect
                                                    } else if bit_perfect {
                                                        *s * g // Fading in, apply gain only
                                                    } else {
                                                        hard_limit(*s * vol * g)
                                                    };
                                                }
                                            }
                                            fade_ctr = fade_ctr
                                                .saturating_add(1)
                                                .min(FADE_RAMP_SAMPLES);
                                        }
                                        for s in data[read..].iter_mut() {
                                            *s = 0.0;
                                        }
                                        if fade_ctr >= FADE_RAMP_SAMPLES {
                                            fade = FadeState::Playing;
                                        }
                                    }
                                }
                            }
                        },
                        move |err| {
                            log::error!("Stream error: {}", err);
                        },
                        None,
                    )
                    .expect("Failed to build output stream");

                stream.play().expect("Failed to start stream");
                current_stream = Some(stream);
            }

            Ok(AudioCommand::Pause) => {
                fade_req_pause.store(true, Ordering::SeqCst);
                decoder_paused.store(true, Ordering::SeqCst);
                is_paused.store(true, Ordering::SeqCst);
                is_playing.store(false, Ordering::SeqCst);
                state.lock().is_paused = true;
                state.lock().is_playing = false;
            }

            Ok(AudioCommand::Resume) => {
                decoder_paused.store(false, Ordering::SeqCst);
                fade_req_resume.store(true, Ordering::SeqCst);
                is_paused.store(false, Ordering::SeqCst);
                is_playing.store(true, Ordering::SeqCst);
                state.lock().is_paused = false;
                state.lock().is_playing = true;
            }

            Ok(AudioCommand::Stop) => {
                fade_req_stop.store(true, Ordering::SeqCst);
                // A6 fix: use actual sample rate, not hardcoded 44100
                let sr = current_sample_rate.load(Ordering::Relaxed).max(1) as u64;
                thread::sleep(Duration::from_millis(
                    (FADE_RAMP_SAMPLES as u64 * 1000) / sr + 5,
                ));
                decoder_running.store(false, Ordering::SeqCst);
                current_stream = None;
                ring_buffer.clear();
                is_playing.store(false, Ordering::SeqCst);
                is_paused.store(false, Ordering::SeqCst);
                position_ms.store(0, Ordering::SeqCst);
                *state.lock() = PlaybackState::default();
            }

            Ok(AudioCommand::Seek(secs)) => {
                let ms = (secs * 1000.0) as u64;
                seek_request_ms.store(ms, Ordering::SeqCst);
                position_ms.store(ms, Ordering::SeqCst);
            }

            Ok(AudioCommand::SetVolume(v)) => {
                volume.store(f32_to_atomic(v.clamp(0.0, 1.0)), Ordering::Relaxed);
                update_bit_perfect(&volume, &rg_state, &is_bit_perfect, &bit_perfect_cb);
            }

            Ok(AudioCommand::SetReplayGain(mode)) => {
                rg_state.lock().set_mode(mode);
                update_bit_perfect(&volume, &rg_state, &is_bit_perfect, &bit_perfect_cb);
            }

            Ok(AudioCommand::SetClippingPrevention(on)) => {
                rg_state.lock().set_clipping_prevention(on);
                update_bit_perfect(&volume, &rg_state, &is_bit_perfect, &bit_perfect_cb);
            }

            Ok(AudioCommand::Shutdown) => {
                fade_req_stop.store(true, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(15));
                decoder_running.store(false, Ordering::SeqCst);
                current_stream = None;
                break;
            }

            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                // Auto-detect end of track
                if !decoder_running.load(Ordering::Relaxed)
                    && is_playing.load(Ordering::Relaxed)
                    && ring_buffer.available_read() == 0
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

// ─── Audio Safety ───

/// Hard limiter — ONLY used when NOT in bit-perfect mode.
/// Catches NaN, Inf, and any samples exceeding ±0.99.
#[inline(always)]
fn hard_limit(s: f32) -> f32 {
    if s.is_finite() {
        s.clamp(-HARD_LIMIT_CEILING, HARD_LIMIT_CEILING)
    } else {
        0.0
    }
}

#[inline]
pub fn db_to_linear(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

// ─── Device Enumeration ───

pub fn get_output_devices() -> Vec<AudioDeviceInfo> {
    let host = cpal::default_host();
    let mut devices = Vec::new();
    if let Ok(out) = host.output_devices() {
        for dev in out {
            if let Ok(name) = dev.name() {
                let is_default = host
                    .default_output_device()
                    .map(|d| d.name().ok() == Some(name.clone()))
                    .unwrap_or(false);
                devices.push(AudioDeviceInfo { name, is_default });
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
