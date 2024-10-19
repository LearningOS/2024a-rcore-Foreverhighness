//! Priority definition

type PriorityInner = isize;

/// Task Priority
pub type Priority = PriorityImpl<PriorityInner>;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PriorityImpl<T>(pub(super) T);

impl<T> PriorityImpl<T> {
    pub const DEFAULT: isize = 16;
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
{
    type Error = ();

    fn try_from(value: isize) -> Result<Self, Self::Error> {
        match value {
            value @ 2..=isize::MAX => T::try_from(value).map(Self).map_err(|_| ()),
            _ => Err(()),
        }
    }
}
