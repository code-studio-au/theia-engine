//! Optional benchmark clock shared with the isolated Bevy reference.
//! Deadlines start at recorded frame zero; late frames are never skipped or rebased.
use serde_json::{Value, json};
use std::time::{Duration, Instant};

pub struct FramePacer {
    hz: u32,
    epoch: Option<Instant>,
}
impl FramePacer {
    pub fn new(hz: u32) -> Self {
        assert!((1..=240).contains(&hz));
        Self { hz, epoch: None }
    }
    fn offset(&self, frame: u64) -> Duration {
        // Integer arithmetic avoids accumulated floating-point/rounded-period drift.
        Duration::from_nanos(frame * 1_000_000_000 / u64::from(self.hz))
    }
    pub fn wait(&mut self, frame: u64) -> Value {
        let entered = Instant::now();
        let epoch = *self.epoch.get_or_insert(entered);
        let offset = self.offset(frame);
        let deadline = epoch + offset;
        // Sleep rather than spin. Repeat for an early wakeup; scheduler overshoot
        // is retained as lateness instead of silently changing the next deadline.
        while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
            std::thread::sleep(remaining);
        }
        let now = Instant::now();
        self.observation(
            frame,
            now.duration_since(epoch),
            now.duration_since(entered),
        )
    }
    fn observation(&self, frame: u64, elapsed: Duration, waited: Duration) -> Value {
        let scheduled = self.offset(frame);
        json!({"scheduled_ms":scheduled.as_secs_f64()*1000.,
            "elapsed_ms":elapsed.as_secs_f64()*1000.,
            "lateness_ms":elapsed.saturating_sub(scheduled).as_secs_f64()*1000.,
            "wait_ms":waited.as_secs_f64()*1000.})
    }
    pub fn description(&self) -> Value {
        json!({"target_hz":self.hz,"epoch":"recorded frame zero, after startup/pre-roll",
            "policy":"absolute monotonic deadlines; no skipped frames or rebasing; sleep excluded from CPU work metric; inspect per-frame lateness before interpreting paired demand"})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deadlines_do_not_accumulate_rounded_period_drift() {
        let clock = FramePacer::new(60);
        assert_eq!(clock.offset(0), Duration::ZERO);
        assert_eq!(clock.offset(60), Duration::from_secs(1));
        assert_eq!(clock.offset(120_000), Duration::from_secs(2000));
        assert_eq!(clock.offset(1).as_nanos(), 16_666_666);
    }
    #[test]
    fn late_frames_keep_the_original_schedule() {
        let clock = FramePacer::new(30);
        let late = clock.observation(3, Duration::from_millis(175), Duration::ZERO);
        assert_eq!(late["scheduled_ms"], 100.);
        assert_eq!(late["lateness_ms"], 75.);
        let next = clock.observation(4, Duration::from_millis(176), Duration::ZERO);
        assert!((next["scheduled_ms"].as_f64().unwrap() - 133.333333).abs() < 1e-6);
        assert!(next["lateness_ms"].as_f64().unwrap() > 42.);
    }
}
