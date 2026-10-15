//! Aggregate-only native audio telemetry. No sample buffers cross this boundary.
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering::SeqCst};
use std::time::{Duration, Instant};

/// Shared with native/meter.inc. The C writer uses only lock-free atomics.
/// A retained Arc outlives the filter callback, including partial-start cleanup.
#[repr(C)]
#[derive(Debug, Default)]
pub struct Mailbox {
    valid: AtomicU32,
    sequence: AtomicU32,
    peak: [AtomicU32; 2],
    power: [AtomicU32; 2],
    frames: AtomicU64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reading {
    pub sequence: u32,
    pub peak: [f32; 2],
    pub rms: [f32; 2],
    pub frames: u64,
}

/// Rejects readings whose producer has stopped advancing its sequence.
/// Reset this guard when the active lease or pair changes.
#[derive(Debug, Default)]
pub struct Freshness {
    last_sequence: Option<u32>,
    changed_at: Option<Instant>,
}

impl Freshness {
    const MAX_AGE: Duration = Duration::from_millis(250);

    /// Return a reading only while its sequence continues to advance.
    /// Missing input clears the prior sequence and freshness timestamp.
    pub fn accept(&mut self, reading: Option<Reading>, now: Instant) -> Option<Reading> {
        let Some(reading) = reading else {
            self.last_sequence = None;
            self.changed_at = None;
            return None;
        };
        if self.last_sequence != Some(reading.sequence) {
            self.last_sequence = Some(reading.sequence);
            self.changed_at = Some(now);
            return Some(reading);
        }

        self.changed_at
            .filter(|changed_at| now.saturating_duration_since(*changed_at) <= Self::MAX_AGE)
            .map(|_| reading)
    }
}

impl Mailbox {
    pub fn clear(&self) {
        self.valid.store(0, SeqCst);
    }

    /// Bounded coherent read. The UI must separately expire a sequence that
    /// stops advancing; a previous valid sample is never evidence of live audio.
    pub fn read(&self) -> Option<Reading> {
        for _ in 0..4 {
            if self.valid.load(SeqCst) != 1 {
                return None;
            }
            let sequence = self.sequence.load(SeqCst);
            if sequence & 1 != 0 {
                continue;
            }
            let peak = self.peak.each_ref().map(|v| f32::from_bits(v.load(SeqCst)));
            let power = self
                .power
                .each_ref()
                .map(|v| f32::from_bits(v.load(SeqCst)));
            let frames = self.frames.load(SeqCst);
            if self.sequence.load(SeqCst) != sequence || self.valid.load(SeqCst) != 1 {
                continue;
            }
            if frames == 0 || peak.iter().chain(&power).any(|v| !v.is_finite() || *v < 0.) {
                return None;
            }
            return Some(Reading {
                sequence,
                peak,
                rms: power.map(f32::sqrt),
                frames,
            });
        }
        None
    }
}

const _: () = {
    assert!(std::mem::size_of::<Mailbox>() == 32);
    assert!(std::mem::offset_of!(Mailbox, frames) == 24);
    assert!(std::mem::align_of::<AtomicU32>() == 4);
};

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(sequence: u32) -> Reading {
        Reading {
            sequence,
            peak: [0.5, 0.25],
            rms: [0.25, 0.125],
            frames: u64::from(sequence),
        }
    }

    #[test]
    fn freshness_accepts_advancing_sequences_and_expires_unchanged_values() {
        let start = Instant::now();
        let mut guard = Freshness::default();

        assert_eq!(guard.accept(Some(reading(2)), start), Some(reading(2)));
        assert_eq!(
            guard.accept(Some(reading(2)), start + Duration::from_millis(250)),
            Some(reading(2))
        );
        assert_eq!(
            guard.accept(Some(reading(2)), start + Duration::from_millis(251)),
            None
        );
        assert_eq!(
            guard.accept(Some(reading(4)), start + Duration::from_millis(500)),
            Some(reading(4))
        );
    }

    #[test]
    fn missing_reading_clears_freshness_state() {
        let start = Instant::now();
        let mut guard = Freshness::default();

        assert_eq!(guard.accept(Some(reading(8)), start), Some(reading(8)));
        assert_eq!(guard.accept(None, start + Duration::from_secs(1)), None);
        assert_eq!(
            guard.accept(Some(reading(8)), start + Duration::from_secs(1)),
            Some(reading(8))
        );
    }

    #[test]
    fn coherent_stereo_power_read_and_clear() {
        let slot = Mailbox::default();
        assert_eq!(slot.read(), None);
        slot.sequence.store(1, SeqCst);
        slot.peak[0].store(0.5f32.to_bits(), SeqCst);
        slot.peak[1].store(1f32.to_bits(), SeqCst);
        slot.power[0].store(0.0625f32.to_bits(), SeqCst);
        slot.power[1].store(0.25f32.to_bits(), SeqCst);
        slot.frames.store(512, SeqCst);
        slot.valid.store(1, SeqCst);
        assert_eq!(slot.read(), None);
        slot.sequence.store(2, SeqCst);
        let reading = slot.read().unwrap();
        assert_eq!(reading.peak, [0.5, 1.]);
        assert_eq!(reading.rms, [0.25, 0.5]);
        slot.clear();
        assert_eq!(slot.read(), None);
    }

    #[test]
    fn invalid_aggregate_is_unavailable() {
        let slot = Mailbox::default();
        slot.valid.store(1, SeqCst);
        slot.frames.store(1, SeqCst);
        slot.power[0].store(f32::NAN.to_bits(), SeqCst);
        assert_eq!(slot.read(), None);
    }
}
