use std::time::{Duration, Instant};

const FLOOR_DBFS: f32 = -60.0;
const RMS_DECAY_SECONDS: f32 = 0.300;
const PEAK_DECAY_SECONDS: f32 = 0.450;
const PEAK_HOLD_SECONDS: f32 = 1.000;
const HOLD_FALL_DB_PER_SECOND: f32 = 24.0;
const DECAY_SNAP_DB: f32 = 0.05;

/// Smoothed levels for the console renderer. Values use the visible -60..0 dBFS range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Display {
    pub rms_db: [f32; 2],
    pub peak_db: [f32; 2],
    pub hold_db: [f32; 2],
}

/// Converts stereo linear measurements into meter levels with quick attack and
/// slower decay. Missing or invalid measurements clear the display.
#[derive(Debug, Default)]
pub struct Ballistics {
    rms_db: Option<[f32; 2]>,
    peak_db: Option<[f32; 2]>,
    hold_db: Option<[f32; 2]>,
    hold_peak_at: Option<[Instant; 2]>,
    last_update: Option<Instant>,
}

impl Ballistics {
    /// Update from `(peak_linear_amplitudes, rms_linear_amplitudes)`.
    ///
    /// Any missing or invalid frame clears the prior display and returns `None`.
    /// Finite amplitudes above unity are clipped to 0 dBFS for visualization.
    pub fn update(
        &mut self,
        measurement: Option<([f32; 2], [f32; 2])>,
        now: Instant,
    ) -> Option<Display> {
        let Some((peak, rms)) = measurement else {
            self.reset();
            return None;
        };
        if peak
            .iter()
            .chain(rms.iter())
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            self.reset();
            return None;
        }

        let target_peak = peak.map(linear_to_dbfs);
        let target_rms = rms.map(linear_to_dbfs);
        let Some(previous_time) = self.last_update else {
            self.rms_db = Some(target_rms);
            self.peak_db = Some(target_peak);
            self.hold_db = Some(target_peak);
            self.hold_peak_at = Some([now; 2]);
            self.last_update = Some(now);
            return self.display();
        };

        let elapsed = now.saturating_duration_since(previous_time).as_secs_f32();
        let old_rms = self.rms_db.unwrap_or(target_rms);
        let old_peak = self.peak_db.unwrap_or(target_peak);
        let old_hold = self.hold_db.unwrap_or(target_peak);
        let old_hold_peak_at = self.hold_peak_at.unwrap_or([now; 2]);
        let mut next_rms = [FLOOR_DBFS; 2];
        let mut next_peak = [FLOOR_DBFS; 2];
        let mut next_hold = old_hold;
        let mut next_hold_peak_at = old_hold_peak_at;

        for channel in 0..2 {
            next_rms[channel] = follow_db(
                old_rms[channel],
                target_rms[channel],
                elapsed,
                RMS_DECAY_SECONDS,
            );
            next_peak[channel] = follow_db(
                old_peak[channel],
                target_peak[channel],
                elapsed,
                PEAK_DECAY_SECONDS,
            );

            if target_peak[channel] >= old_hold[channel] {
                next_hold[channel] = target_peak[channel];
                next_hold_peak_at[channel] = now;
            } else {
                let hold_ends =
                    old_hold_peak_at[channel] + Duration::from_secs_f32(PEAK_HOLD_SECONDS);
                let decay_from = previous_time.max(hold_ends);
                let decay_elapsed = now.saturating_duration_since(decay_from).as_secs_f32();
                if decay_elapsed > 0.0 {
                    next_hold[channel] = (old_hold[channel]
                        - decay_elapsed * HOLD_FALL_DB_PER_SECOND)
                        .max(target_peak[channel])
                        .max(FLOOR_DBFS);
                }
            }
        }

        self.rms_db = Some(next_rms);
        self.peak_db = Some(next_peak);
        self.hold_db = Some(next_hold);
        self.hold_peak_at = Some(next_hold_peak_at);
        self.last_update = Some(now);
        self.display()
    }

    fn display(&self) -> Option<Display> {
        Some(Display {
            rms_db: self.rms_db?,
            peak_db: self.peak_db?,
            hold_db: self.hold_db?,
        })
    }

    fn reset(&mut self) {
        self.rms_db = None;
        self.peak_db = None;
        self.hold_db = None;
        self.hold_peak_at = None;
        self.last_update = None;
    }
}

fn linear_to_dbfs(amplitude: f32) -> f32 {
    if amplitude == 0.0 {
        FLOOR_DBFS
    } else {
        (20.0 * amplitude.min(1.0).log10()).clamp(FLOOR_DBFS, 0.0)
    }
}

fn follow_db(current: f32, target: f32, elapsed: f32, decay_seconds: f32) -> f32 {
    let next = if target >= current {
        return target;
    } else if elapsed <= 0.0 {
        current
    } else {
        target + (current - target) * (-elapsed / decay_seconds).exp()
    };
    if next - target <= DECAY_SNAP_DB {
        target
    } else {
        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_stereo_linear_levels_and_clips_above_unity() {
        let now = Instant::now();
        let mut ballistics = Ballistics::default();
        let display = ballistics
            .update(Some(([1.0, 0.5], [0.25, 2.0])), now)
            .unwrap();

        assert!((display.peak_db[0] - 0.0).abs() < 0.001);
        assert!((display.peak_db[1] - -6.0206).abs() < 0.01);
        assert!((display.rms_db[0] - -12.0412).abs() < 0.01);
        assert_eq!(display.rms_db[1], 0.0);
    }

    #[test]
    fn silence_is_at_floor_and_no_measurement_clears_the_display() {
        let now = Instant::now();
        let mut ballistics = Ballistics::default();
        let display = ballistics.update(Some(([0.0; 2], [0.0; 2])), now).unwrap();
        assert_eq!(display.rms_db, [FLOOR_DBFS; 2]);
        assert_eq!(display.peak_db, [FLOOR_DBFS; 2]);
        assert_eq!(display.hold_db, [FLOOR_DBFS; 2]);
        assert_eq!(
            ballistics.update(None, now + Duration::from_millis(20)),
            None
        );
        assert_eq!(ballistics.rms_db, None);
    }

    #[test]
    fn loud_levels_decay_to_exact_floor_after_silence() {
        let start = Instant::now();
        let mut ballistics = Ballistics::default();
        ballistics
            .update(Some(([1.0; 2], [1.0; 2])), start)
            .unwrap();

        let silence = ballistics
            .update(Some(([0.0; 2], [0.0; 2])), start + Duration::from_secs(5))
            .unwrap();
        assert_eq!(silence.rms_db, [FLOOR_DBFS; 2]);
        assert_eq!(silence.peak_db, [FLOOR_DBFS; 2]);
        assert_eq!(silence.hold_db, [FLOOR_DBFS; 2]);
    }

    #[test]
    fn rms_and_peak_decay_while_peak_hold_waits_then_falls() {
        let start = Instant::now();
        let mut ballistics = Ballistics::default();
        let loud = Some(([1.0; 2], [1.0; 2]));
        let quiet = Some(([0.5; 2], [0.5; 2]));
        ballistics.update(loud, start).unwrap();

        let after_half_second = ballistics
            .update(quiet, start + Duration::from_millis(500))
            .unwrap();
        assert!(after_half_second.rms_db[0] < 0.0);
        assert!(after_half_second.rms_db[0] > -6.1);
        assert!(after_half_second.peak_db[0] < 0.0);
        assert_eq!(after_half_second.hold_db[0], 0.0);

        let after_hold = ballistics
            .update(quiet, start + Duration::from_millis(1_500))
            .unwrap();
        assert!(after_hold.hold_db[0] < 0.0);
        assert!(after_hold.hold_db[0] > -6.1);
    }

    #[test]
    fn peak_hold_decay_is_independent_of_update_cadence() {
        let start = Instant::now();
        let pulse = Some(([1.0; 2], [1.0; 2]));
        let silence = Some(([0.0; 2], [0.0; 2]));

        let mut sparse = Ballistics::default();
        sparse.update(pulse, start).unwrap();
        let sparse_display = sparse
            .update(silence, start + Duration::from_secs(2))
            .unwrap();

        let mut frequent = Ballistics::default();
        frequent.update(pulse, start).unwrap();
        frequent
            .update(silence, start + Duration::from_millis(1_250))
            .unwrap();
        let frequent_display = frequent
            .update(silence, start + Duration::from_secs(2))
            .unwrap();

        assert!((sparse_display.hold_db[0] - -24.0).abs() < 0.001);
        assert!((frequent_display.hold_db[0] - sparse_display.hold_db[0]).abs() < 0.001);
    }

    #[test]
    fn invalid_values_are_rejected_and_clear_old_levels() {
        let start = Instant::now();
        let mut ballistics = Ballistics::default();
        ballistics
            .update(Some(([1.0; 2], [0.5; 2])), start)
            .unwrap();

        assert_eq!(
            ballistics.update(Some(([1.0, f32::NAN], [0.5; 2])), start),
            None
        );
        assert_eq!(ballistics.peak_db, None);
        assert_eq!(
            ballistics.update(Some(([1.0; 2], [0.5, -0.1])), start),
            None
        );
        assert_eq!(
            ballistics.update(Some(([f32::INFINITY; 2], [0.5; 2])), start),
            None
        );
    }
}
