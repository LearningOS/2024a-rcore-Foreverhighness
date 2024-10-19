//! Stride definition for Stride algorithm

use core::{
    num::Saturating,
    ops::{AddAssign, Div},
};

use super::priority::PriorityImpl;

type StrideInner = u32;

/// Stride algorithm
pub type Stride = StrideImpl<StrideInner>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StrideImpl<T> {
    stride: Saturating<T>,
}

pub trait BigStride<T> {
    const BIG_STRIDE: T;
}

impl BigStride<u32> for u32 {
    const BIG_STRIDE: u32 = u32::MAX / 100;
}

impl BigStride<u8> for u8 {
    const BIG_STRIDE: u8 = 255;
}

impl<T> StrideImpl<T>
where
    T: Div<T, Output = T> + Clone + Copy,
    Saturating<T>: AddAssign,
    T: BigStride<T>,
{
    pub fn step(&mut self, priority: PriorityImpl<T>) {
        self.stride += Saturating(T::BIG_STRIDE / priority.0);
    }
}

impl<T> Ord for StrideImpl<T>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.stride.0.cmp(&other.stride.0)
    }
}

impl<T> PartialOrd for StrideImpl<T>
where
    StrideImpl<T>: Ord,
{
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

mod tests {
    use alloc::vec::Vec;

    use super::*;

    #[derive(Debug, Default)]
    struct Task {
        pub id: u8,
        pub priority: u8,
        pub stride: StrideImpl<u8>,
        pub real_stride: u64,
    }

    impl Task {
        fn new(pid: u8, priority: u8) -> Self {
            Self {
                id: pid,
                priority,
                stride: StrideImpl::default(),
                real_stride: 0,
            }
        }
    }

    #[allow(unused)]
    fn test_stride() {
        let mut tasks = (0..=8).map(|i| Task::new(i, 2 + i)).collect::<Vec<_>>();

        while tasks.iter().map(|t| t.real_stride).max().unwrap() < u32::MAX.into() {
            let id = tasks.iter().max_by_key(|t| t.stride).map(|t| t.id).unwrap();
            let id_expect = tasks
                .iter()
                .max_by_key(|t| t.real_stride)
                .map(|t| t.id)
                .unwrap();

            assert_eq!(id, id_expect);

            let task = &mut tasks[id as usize];
            task.stride
                .step((task.priority as isize).try_into().unwrap());
            task.real_stride += (u8::BIG_STRIDE / task.priority) as u64;
        }
    }
}
