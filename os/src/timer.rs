//! RISC-V timer-related functionality

use crate::config::CLOCK_FREQ;
use crate::sbi::set_timer;
use riscv::register::time;
/// The number of milliseconds per second
pub const MSEC_PER_SEC: usize = 1000;
/// The number of microseconds per second
pub const MICRO_PER_SEC: usize = 1_000_000;

/// Timer Tick
pub type Tick = usize;

/// Get the current time in ticks
pub fn get_time_tick() -> Tick {
    time::read()
}

/// get current time in milliseconds
pub fn get_time_ms() -> usize {
    time::read() * MSEC_PER_SEC / CLOCK_FREQ
}

/// get current time in microseconds
pub fn get_time_us() -> usize {
    time::read() * MICRO_PER_SEC / CLOCK_FREQ
}

/// Set the next timer interrupt
pub fn set_next_trigger() {
    // set timer after 10ms
    set_timer(get_time_tick() + 10 * CLOCK_FREQ / MSEC_PER_SEC);
}
