//! Synchronization and interior mutability primitives

mod condvar;
mod mutex;
mod semaphore;
mod up;

pub use condvar::Condvar;
pub use mutex::{Mutex, MutexBlocking, MutexSpin};
pub use semaphore::Semaphore;
pub use up::UPSafeCell;

mod deadlock_avoidance;
pub use deadlock_avoidance::{
    acquire, add_resource, disable, enable, record, release, RequestResult,
};
