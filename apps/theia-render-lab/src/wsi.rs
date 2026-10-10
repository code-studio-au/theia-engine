use ash::vk;

/// Presentation errors differ in whether semaphore waits and the present fence were queued.
pub struct PresentOutcome {
    pub pending: bool,
    pub recreate: bool,
    pub presented: bool,
    pub error: Option<vk::Result>,
}
impl PresentOutcome {
    pub fn classify(result: std::result::Result<bool, vk::Result>) -> Self {
        match result {
            Ok(suboptimal) => Self {
                pending: true,
                recreate: suboptimal,
                presented: true,
                error: None,
            },
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => Self {
                pending: true,
                recreate: true,
                presented: false,
                error: None,
            },
            Err(error) => Self {
                pending: matches!(
                    error,
                    vk::Result::ERROR_SURFACE_LOST_KHR
                        | vk::Result::ERROR_FULL_SCREEN_EXCLUSIVE_MODE_LOST_EXT
                ),
                recreate: false,
                presented: false,
                error: Some(error),
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Probe {
    AcquireOutOfDate,
    AcquireSuboptimal,
    PresentOutOfDate,
    PresentSuboptimal,
}

/// Optional one-shot probes; no changes to the normal qualification/render path.
pub struct RecoveryProbes(u8);
impl RecoveryProbes {
    pub fn new(enabled: bool) -> Self {
        Self(if enabled { 0b1111 } else { 0 })
    }
    pub fn hit(&mut self, probe: Probe, presented: u64) -> bool {
        let (bit, frame) = match probe {
            Probe::AcquireOutOfDate => (1, 10),
            Probe::AcquireSuboptimal => (2, 30),
            Probe::PresentOutOfDate => (4, 50),
            Probe::PresentSuboptimal => (8, 80),
        };
        if self.0 & bit == 0 || presented < frame {
            return false;
        }
        self.0 &= !bit;
        println!("WSI INJECTED probe: {probe:?} at presented frame {presented}");
        true
    }
    pub fn remaining(&self) -> u8 {
        self.0.count_ones() as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recoverable_present_results_retain_fence_and_request_recreation() {
        for result in [Ok(true), Err(vk::Result::ERROR_OUT_OF_DATE_KHR)] {
            let outcome = PresentOutcome::classify(result);
            assert!(outcome.pending && outcome.recreate && outcome.error.is_none());
            assert_eq!(outcome.presented, result.is_ok());
        }
        let outcome = PresentOutcome::classify(Ok(false));
        assert!(outcome.pending && outcome.presented && !outcome.recreate);
    }

    #[test]
    fn fatal_present_results_do_not_wait_on_unqueued_fences() {
        for error in [
            vk::Result::ERROR_SURFACE_LOST_KHR,
            vk::Result::ERROR_FULL_SCREEN_EXCLUSIVE_MODE_LOST_EXT,
            vk::Result::ERROR_OUT_OF_HOST_MEMORY,
            vk::Result::ERROR_OUT_OF_DEVICE_MEMORY,
            vk::Result::ERROR_DEVICE_LOST,
        ] {
            let outcome = PresentOutcome::classify(Err(error));
            assert_eq!(outcome.error, Some(error));
            assert!(!outcome.presented && !outcome.recreate);
            assert_eq!(
                outcome.pending,
                matches!(
                    error,
                    vk::Result::ERROR_SURFACE_LOST_KHR
                        | vk::Result::ERROR_FULL_SCREEN_EXCLUSIVE_MODE_LOST_EXT
                )
            );
        }
    }

    #[test]
    fn probes_are_bounded_and_disabled_by_default() {
        let probes = [
            Probe::AcquireOutOfDate,
            Probe::AcquireSuboptimal,
            Probe::PresentOutOfDate,
            Probe::PresentSuboptimal,
        ];
        let mut disabled = RecoveryProbes::new(false);
        let mut enabled = RecoveryProbes::new(true);
        for probe in probes {
            assert!(!disabled.hit(probe, 120));
            assert!(!enabled.hit(probe, 0));
            assert!(enabled.hit(probe, 120));
            assert!(!enabled.hit(probe, 120));
        }
        assert_eq!(enabled.remaining(), 0);
    }
}
