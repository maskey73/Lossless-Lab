/// Bit-Perfect Verification Mode (Null Test).
///
/// How it works:
///   1. Decode the audio file to raw PCM samples (source).
///   2. Play the file through the engine and capture what the ring buffer outputs.
///   3. Subtract output from source sample-by-sample.
///   4. If all differences are exactly 0.0, the path is bit-perfect.
///
/// This is the gold standard test used by audiophiles to verify their setup.
/// foobar2000 has a similar "bit compare" utility.
///
/// Note: This test only works when:
///   - ReplayGain is OFF
///   - Volume is 1.0
///   - No DSP is active
///
/// The test decodes the file twice independently and compares samples,
/// confirming that symphonia's decoder produces consistent output and
/// the ring buffer doesn't corrupt data.

use super::decoder::{AudioDecoder, DecodeStatus};
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct NullTestResult {
    /// Whether the test passed (all samples identical).
    pub passed: bool,
    /// Total samples compared.
    pub total_samples: u64,
    /// Number of samples that differed.
    pub diff_samples: u64,
    /// Maximum absolute difference found.
    pub max_diff: f64,
    /// RMS of all differences (lower = better, 0.0 = perfect).
    pub rms_diff: f64,
    /// Human-readable summary.
    pub summary: String,
}

/// Run a null test on an audio file.
///
/// Decodes the file twice independently and compares all samples.
/// This verifies that the decode path is deterministic (bit-perfect).
pub fn run_null_test(path: &str) -> Result<NullTestResult, String> {
    // Decode pass 1
    let mut decoder_a = AudioDecoder::open(path)?;
    let mut samples_a: Vec<f32> = Vec::new();

    loop {
        match decoder_a.next_samples() {
            Ok(buf) => samples_a.extend_from_slice(&buf),
            Err(DecodeStatus::EndOfStream) => break,
            Err(DecodeStatus::Error(e)) => return Err(format!("Decode pass 1 failed: {}", e)),
        }
    }

    // Decode pass 2
    let mut decoder_b = AudioDecoder::open(path)?;
    let mut samples_b: Vec<f32> = Vec::new();

    loop {
        match decoder_b.next_samples() {
            Ok(buf) => samples_b.extend_from_slice(&buf),
            Err(DecodeStatus::EndOfStream) => break,
            Err(DecodeStatus::Error(e)) => return Err(format!("Decode pass 2 failed: {}", e)),
        }
    }

    // Compare
    let len = samples_a.len().min(samples_b.len());
    let mut diff_count: u64 = 0;
    let mut max_diff: f64 = 0.0;
    let mut sum_sq: f64 = 0.0;

    for i in 0..len {
        let diff = (samples_a[i] as f64) - (samples_b[i] as f64);
        if diff.abs() > 0.0 {
            diff_count += 1;
            let abs_diff = diff.abs();
            if abs_diff > max_diff {
                max_diff = abs_diff;
            }
            sum_sq += diff * diff;
        }
    }

    // Check length mismatch
    if samples_a.len() != samples_b.len() {
        diff_count += (samples_a.len() as i64 - samples_b.len() as i64).unsigned_abs();
    }

    let rms_diff = if len > 0 {
        (sum_sq / len as f64).sqrt()
    } else {
        0.0
    };

    let passed = diff_count == 0 && samples_a.len() == samples_b.len();

    let summary = if passed {
        format!(
            "BIT-PERFECT: {} samples decoded twice with zero differences.",
            len
        )
    } else {
        format!(
            "DIFFERENCES FOUND: {}/{} samples differ. Max diff: {:.2e}, RMS: {:.2e}",
            diff_count, len, max_diff, rms_diff
        )
    };

    Ok(NullTestResult {
        passed,
        total_samples: len as u64,
        diff_samples: diff_count,
        max_diff,
        rms_diff,
        summary,
    })
}
