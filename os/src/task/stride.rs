//! Stride definition for Stride algorithm

use core::{
    num::Wrapping,
    ops::{AddAssign, Div, Sub},
};

use super::priority::PriorityImpl;

type StrideInner = usize;

/// Stride algorithm
pub type Stride = StrideImpl<StrideInner>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StrideImpl<T> {
    stride: Wrapping<T>,
}

impl<T> StrideImpl<T> {
    fn new(stride: T) -> Self {
        Self {
            stride: Wrapping(stride),
        }
    }
}

pub trait BigStride<T>
where
    T: Div<T, Output = T>,
{
    const BIG_STRIDE: T;
    const BIG_STRIDE_DIV_2: T;
}

impl BigStride<u32> for u32 {
    const BIG_STRIDE: u32 = u32::MAX / 100;
    const BIG_STRIDE_DIV_2: u32 = Self::BIG_STRIDE / 2;
}

impl BigStride<u8> for u8 {
    const BIG_STRIDE: u8 = 255;
    const BIG_STRIDE_DIV_2: u8 = Self::BIG_STRIDE / 2;
}

impl BigStride<usize> for usize {
    const BIG_STRIDE: usize = isize::MAX as usize;
    const BIG_STRIDE_DIV_2: usize = Self::BIG_STRIDE / 2;
}

impl<T> StrideImpl<T>
where
    T: Div<T, Output = T> + BigStride<T> + Clone + Copy,
    Wrapping<T>: AddAssign,
{
    pub fn step<P, E>(&mut self, priority: PriorityImpl<P>)
    where
        T: TryFrom<P, Error = E>,
        E: core::fmt::Debug,
    {
        self.stride += Wrapping(T::BIG_STRIDE / priority.0.try_into().unwrap());
    }
}

impl<T> Ord for StrideImpl<T>
where
    T: Ord + BigStride<T> + Sub<T, Output = T> + Div<T, Output = T> + Copy,
{
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        let cmp = |min, max| {
            assert!(min < max);
            if max - min <= T::BIG_STRIDE_DIV_2 {
                core::cmp::Ordering::Less
            } else {
                core::cmp::Ordering::Greater
            }
        };
        match self.stride.cmp(&other.stride) {
            core::cmp::Ordering::Less => cmp(self.stride.0, other.stride.0),
            core::cmp::Ordering::Equal => core::cmp::Ordering::Equal,
            core::cmp::Ordering::Greater => cmp(other.stride.0, self.stride.0).reverse(),
        }
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

    #[derive(Default)]
    struct Task {
        pub id: usize,
        pub priority: u8,
        pub stride: StrideImpl<u8>,
        pub real_stride: u64,
    }

    impl core::fmt::Debug for Task {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_fmt(format_args!(
                "[{}, {}, {}]",
                self.stride.stride.0,
                self.real_stride,
                self.real_stride % 256
            ))
        }
    }

    impl Task {
        fn new(id: usize, priority: u8) -> Self {
            Self {
                id,
                priority,
                stride: StrideImpl::default(),
                real_stride: 0,
            }
        }
    }

    #[allow(unused)]
    fn test_stride() {
        assert!(!(StrideImpl::<u8>::new(125) < StrideImpl::<u8>::new(255)));
        assert!((StrideImpl::<u8>::new(129) < StrideImpl::<u8>::new(255)));
        assert!((StrideImpl::<u8>::new(0) < StrideImpl::<u8>::new(127)));

        let mut tasks = (0..=8)
            .map(|i| Task::new(i as usize, 2 + i))
            .collect::<Vec<_>>();

        while tasks.iter().map(|t| t.real_stride).max().unwrap() < u32::MAX.into() {
            let id = tasks
                .iter()
                .min_by_key(|t| (t.stride, t.id))
                .map(|t| t.id)
                .unwrap();
            let id_expect = tasks
                .iter()
                .min_by_key(|t| (t.real_stride, t.id))
                .map(|t| t.id)
                .unwrap();

            assert_eq!(
                id, id_expect,
                "expect {:?} < {:?}: {tasks:#?}",
                tasks[id_expect].stride.stride.0, tasks[id].stride.stride.0
            );

            let task = &mut tasks[id];
            task.stride
                .step(PriorityImpl::<u8>::try_from(task.priority as isize).unwrap());
            task.real_stride += (u8::BIG_STRIDE / task.priority) as u64;
        }
    }
}
