//! Priority definition

type PriorityInner = u32;

/// Task Priority
pub type Priority = PriorityImpl<PriorityInner>;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PriorityImpl<T>(pub(super) T);

impl<T> PriorityImpl<T> {
    const DEFAULT: isize = 16;

    /// new with priority
    pub fn new(value: T) -> Self {
        Self(value)
    }
}

impl<T> Default for PriorityImpl<T>
where
    T: TryFrom<isize>,
    <T as TryFrom<isize>>::Error: core::fmt::Debug,
{
    fn default() -> Self {
        Self(T::try_from(Self::DEFAULT).unwrap())
    }
}

impl<T> TryFrom<isize> for PriorityImpl<T>
where
    T: TryFrom<isize>,
    <T as TryFrom<isize>>::Error: core::fmt::Debug,
{
    type Error = ();

    fn try_from(value: isize) -> Result<Self, Self::Error> {
        match value {
            value @ 2..=isize::MAX => Ok(Self(T::try_from(value).unwrap())),
            _ => Err(()),
        }
    }
}
