//! Page rotation helpers (index selection).
use crate::config::RotationMode;

#[allow(dead_code)]
pub fn next_index(mode: &RotationMode, current: usize, len: usize, step: bool) -> usize {
    if len == 0 {
        return 0;
    }
    match mode {
        RotationMode::Static => 0,
        RotationMode::RoundRobin | RotationMode::Smart => {
            if step {
                current.wrapping_add(1) % len
            } else {
                current % len
            }
        }
        RotationMode::Priority => 0, // priority mode shows combined, index unused
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_robin_wraps() {
        assert_eq!(next_index(&RotationMode::RoundRobin, 2, 3, true), 0);
        assert_eq!(next_index(&RotationMode::Static, 5, 3, true), 0);
    }
}
