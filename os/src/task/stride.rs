//! Stride definition for Stride algorithm

use super::Priority;

type StrideInner = u32;

/// Stride algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stride(StrideInner);

impl Stride {
    const BIG_STRIDE: StrideInner = StrideInner::MAX / 10;

    pub fn step(&mut self, priority: Priority) {
        self.0 += Self::BIG_STRIDE / priority.0
    }
}

impl Ord for Stride {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
