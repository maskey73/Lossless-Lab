use std::fs::File;
use std::path::Path;
use symphonia::core::audio::{AudioBufferRef, SampleBuffer, SignalSpec};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

pub struct AudioDecoder {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn symphonia::core::codecs::Decoder>,
    track_id: u32,
    pub spec: SignalSpec,
    pub duration_secs: f64,
}

impl AudioDecoder {
    pub fn open(path: &str) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = Path::new(path).extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let meta_opts = MetadataOptions::default();
        let fmt_opts = FormatOptions {
            enable_gapless: true,
            ..Default::default()
        };

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &fmt_opts, &meta_opts)
            .map_err(|e| format!("Failed to probe format: {}", e))?;

        let format = probed.format;

        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or("No audio tracks found")?;

        let track_id = track.id;

        let dec_opts = DecoderOptions::default();
        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &dec_opts)
            .map_err(|e| format!("Failed to create decoder: {}", e))?;

        let spec = SignalSpec::new(
            track.codec_params.sample_rate.unwrap_or(44100),
            track
                .codec_params
                .channels
                .unwrap_or(symphonia::core::audio::Channels::FRONT_LEFT | symphonia::core::audio::Channels::FRONT_RIGHT),
        );

        let duration_secs = if let Some(n_frames) = track.codec_params.n_frames {
            let sample_rate = track.codec_params.sample_rate.unwrap_or(44100) as f64;
            n_frames as f64 / sample_rate
        } else {
            0.0
        };

        Ok(Self {
            format,
            decoder,
            track_id,
            spec,
            duration_secs,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.spec.rate
    }

    pub fn channels(&self) -> usize {
        self.spec.channels.count()
    }

    /// Decode the next packet, returning interleaved f32 samples.
    pub fn next_samples(&mut self) -> Result<Vec<f32>, DecodeStatus> {
        loop {
            let packet = match self.format.next_packet() {
                Ok(p) => p,
                Err(SymphoniaError::IoError(ref e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    return Err(DecodeStatus::EndOfStream);
                }
                Err(e) => return Err(DecodeStatus::Error(format!("{}", e))),
            };

            if packet.track_id() != self.track_id {
                continue;
            }

            let decoded = match self.decoder.decode(&packet) {
                Ok(d) => d,
                Err(SymphoniaError::DecodeError(_)) => continue,
                Err(e) => return Err(DecodeStatus::Error(format!("{}", e))),
            };

            let spec = *decoded.spec();
            let num_frames = decoded.frames();
            let mut sample_buf = SampleBuffer::<f32>::new(num_frames as u64, spec);
            sample_buf.copy_interleaved_ref(decoded);

            return Ok(sample_buf.samples().to_vec());
        }
    }

    /// Seek to a position in seconds.
    pub fn seek(&mut self, position_secs: f64) -> Result<(), String> {
        let seek_to = SeekTo::Time {
            time: Time::new(position_secs as u64, (position_secs.fract() * 1_000_000_000.0) as u32),
            track_id: Some(self.track_id),
        };
        self.format
            .seek(SeekMode::Accurate, seek_to)
            .map_err(|e| format!("Seek failed: {}", e))?;
        self.decoder.reset();
        Ok(())
    }
}

pub enum DecodeStatus {
    EndOfStream,
    Error(String),
}
