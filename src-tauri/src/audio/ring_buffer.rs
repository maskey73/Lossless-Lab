/// Lock-free single-producer single-consumer (SPSC) ring buffer for audio.
///
/// This is the core safety mechanism that prevents audio glitches:
///   - The decoder thread WRITES samples into the buffer (producer)
///   - The audio callback READS samples from the buffer (consumer)
///   - NO MUTEX is ever used — atomic read/write pointers only
///   - The audio callback NEVER blocks, even if the buffer is empty
///
/// Design based on the same principles used by foobar2000, JACK, and
/// professional audio software.

use std::sync::atomic::{AtomicUsize, Ordering};

pub struct RingBuffer {
    /// The sample data. Fixed-size, allocated once.
    buffer: Box<[f32]>,
    /// Write position (only modified by producer/decoder thread).
    write_pos: AtomicUsize,
    /// Read position (only modified by consumer/audio callback).
    read_pos: AtomicUsize,
    /// Capacity (always power of 2 for fast masking).
    capacity: usize,
    /// Bit mask for fast modulo: capacity - 1 (works because capacity is power of 2).
    mask: usize,
}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        // Ensure capacity is a power of 2
        assert!(capacity.is_power_of_two(), "Ring buffer capacity must be power of 2");

        Self {
            buffer: vec![0.0; capacity].into_boxed_slice(),
            write_pos: AtomicUsize::new(0),
            read_pos: AtomicUsize::new(0),
            capacity,
            mask: capacity - 1,
        }
    }

    /// Write samples into the ring buffer (called by decoder thread).
    /// Returns the number of samples actually written (may be less than input if buffer is full).
    pub fn write(&self, data: &[f32]) -> usize {
        let write = self.write_pos.load(Ordering::Relaxed);
        let read = self.read_pos.load(Ordering::Acquire);

        // Available space = capacity - 1 - used (keep 1 slot empty to distinguish full from empty)
        let used = write.wrapping_sub(read);
        let available = self.capacity - 1 - used;
        let to_write = data.len().min(available);

        if to_write == 0 {
            return 0;
        }

        // Write samples — safe because only ONE thread writes
        // We need unsafe to write into the boxed slice from the "wrong" thread,
        // but this is safe because:
        //   1. Only one producer thread
        //   2. We only write to positions between write_pos and write_pos + to_write
        //   3. The consumer only reads up to read_pos..write_pos
        //   4. The ordering ensures the consumer sees the data after we publish write_pos
        let buf_ptr = self.buffer.as_ptr() as *mut f32;
        for i in 0..to_write {
            let idx = (write + i) & self.mask;
            unsafe {
                buf_ptr.add(idx).write(data[i]);
            }
        }

        // Publish the new write position (Release ensures data is visible before pointer update)
        self.write_pos.store(write.wrapping_add(to_write), Ordering::Release);

        to_write
    }

    /// Read samples from the ring buffer (called by audio callback).
    /// Fills `output` with as many samples as available. Returns number of samples read.
    /// NEVER BLOCKS — returns 0 if buffer is empty.
    pub fn read(&self, output: &mut [f32]) -> usize {
        let read = self.read_pos.load(Ordering::Relaxed);
        let write = self.write_pos.load(Ordering::Acquire);

        let available = write.wrapping_sub(read);
        let to_read = output.len().min(available);

        if to_read == 0 {
            return 0;
        }

        // Read samples — safe because only ONE thread reads
        let buf_ptr = self.buffer.as_ptr();
        for i in 0..to_read {
            let idx = (read + i) & self.mask;
            output[i] = unsafe { buf_ptr.add(idx).read() };
        }

        // Publish the new read position
        self.read_pos.store(read.wrapping_add(to_read), Ordering::Release);

        to_read
    }

    /// Number of samples available to read.
    pub fn available_read(&self) -> usize {
        let write = self.write_pos.load(Ordering::Acquire);
        let read = self.read_pos.load(Ordering::Relaxed);
        write.wrapping_sub(read)
    }

    /// Number of samples that can be written.
    pub fn available_write(&self) -> usize {
        let write = self.write_pos.load(Ordering::Relaxed);
        let read = self.read_pos.load(Ordering::Acquire);
        let used = write.wrapping_sub(read);
        self.capacity - 1 - used
    }

    /// Clear the buffer (reset both pointers). Call from a single thread only,
    /// typically during stop/seek when the stream is not running.
    pub fn clear(&self) {
        self.write_pos.store(0, Ordering::SeqCst);
        self.read_pos.store(0, Ordering::SeqCst);
    }
}

// Safety: RingBuffer is safe to share between threads because:
// - The buffer data is only accessed through atomic-guarded positions
// - write_pos is only modified by the producer
// - read_pos is only modified by the consumer
// - Proper Acquire/Release ordering ensures data visibility
unsafe impl Send for RingBuffer {}
unsafe impl Sync for RingBuffer {}
