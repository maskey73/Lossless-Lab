/// 10-band graphic equalizer with biquad filters.
/// Bands: 31Hz, 62Hz, 125Hz, 250Hz, 500Hz, 1kHz, 2kHz, 4kHz, 8kHz, 16kHz

const NUM_BANDS: usize = 10;
const BAND_FREQUENCIES: [f32; NUM_BANDS] = [
    31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

#[derive(Clone)]
struct BiquadFilter {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    // State per channel (stereo = 2)
    x1: [f64; 2],
    x2: [f64; 2],
    y1: [f64; 2],
    y2: [f64; 2],
}

impl BiquadFilter {
    fn new() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1: [0.0; 2],
            x2: [0.0; 2],
            y1: [0.0; 2],
            y2: [0.0; 2],
        }
    }

    /// Design a peaking EQ filter.
    fn set_peaking_eq(&mut self, sample_rate: f32, freq: f32, gain_db: f32, q: f32) {
        let a = 10.0_f64.powf(gain_db as f64 / 40.0);
        let w0 = 2.0 * std::f64::consts::PI * freq as f64 / sample_rate as f64;
        let alpha = w0.sin() / (2.0 * q as f64);

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * w0.cos();
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * w0.cos();
        let a2 = 1.0 - alpha / a;

        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }

    fn process_sample(&mut self, input: f32, channel: usize) -> f32 {
        let x = input as f64;
        let y = self.b0 * x + self.b1 * self.x1[channel] + self.b2 * self.x2[channel]
            - self.a1 * self.y1[channel]
            - self.a2 * self.y2[channel];

        self.x2[channel] = self.x1[channel];
        self.x1[channel] = x;
        self.y2[channel] = self.y1[channel];
        self.y1[channel] = y;

        y as f32
    }

    fn reset(&mut self) {
        self.x1 = [0.0; 2];
        self.x2 = [0.0; 2];
        self.y1 = [0.0; 2];
        self.y2 = [0.0; 2];
    }
}

pub struct Equalizer {
    filters: [BiquadFilter; NUM_BANDS],
    gains: [f32; NUM_BANDS],
    sample_rate: u32,
}

impl Equalizer {
    pub fn new(sample_rate: u32) -> Self {
        let mut eq = Self {
            filters: std::array::from_fn(|_| BiquadFilter::new()),
            gains: [0.0; NUM_BANDS],
            sample_rate,
        };
        eq.update_filters();
        eq
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate;
        self.update_filters();
    }

    /// Set gain for all bands in dB (-12.0 to +12.0).
    pub fn set_bands(&mut self, gains: [f32; NUM_BANDS]) {
        self.gains = gains;
        self.update_filters();
    }

    fn update_filters(&mut self) {
        for (i, filter) in self.filters.iter_mut().enumerate() {
            filter.reset();
            filter.set_peaking_eq(
                self.sample_rate as f32,
                BAND_FREQUENCIES[i],
                self.gains[i],
                1.414, // Q factor — moderate bandwidth
            );
        }
    }

    /// Process interleaved stereo samples in-place.
    pub fn process(&mut self, samples: &mut [f32]) {
        let channels = 2; // Assume stereo
        for i in (0..samples.len()).step_by(channels) {
            for ch in 0..channels {
                if i + ch < samples.len() {
                    let mut sample = samples[i + ch];
                    for filter in self.filters.iter_mut() {
                        sample = filter.process_sample(sample, ch);
                    }
                    samples[i + ch] = sample;
                }
            }
        }
    }
}

/// Built-in EQ presets.
pub fn get_preset(name: &str) -> Option<[f32; NUM_BANDS]> {
    match name {
        "flat" => Some([0.0; NUM_BANDS]),
        "rock" => Some([5.0, 4.0, 2.0, 0.0, -1.0, 1.0, 3.0, 4.0, 5.0, 5.0]),
        "pop" => Some([-1.0, 2.0, 4.0, 5.0, 4.0, 2.0, 0.0, -1.0, -1.0, -1.0]),
        "jazz" => Some([3.0, 2.0, 0.0, 2.0, -2.0, -2.0, 0.0, 2.0, 3.0, 4.0]),
        "classical" => Some([4.0, 3.0, 2.0, 1.0, -1.0, -1.0, 0.0, 2.0, 3.0, 4.0]),
        "bass_boost" => Some([8.0, 6.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        "vocal" => Some([-2.0, -1.0, 0.0, 3.0, 5.0, 5.0, 3.0, 1.0, 0.0, -1.0]),
        "electronic" => Some([5.0, 4.0, 1.0, 0.0, -2.0, 2.0, 1.0, 3.0, 5.0, 4.0]),
        _ => None,
    }
}
