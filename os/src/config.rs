//! Constants in the kernel

#[allow(unused)]

/// Physical address
pub type Address = usize;

/// user app's stack size (Byte)
pub const USER_STACK_SIZE: usize = 4096 * 2;
/// kernel stack size (Byte)
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;
/// kernel heap size (Byte)
pub const KERNEL_HEAP_SIZE: usize = 0x0200_0000;

/// page size : 4KB
pub const PAGE_SIZE: usize = 0x1000;
/// page size bits: 12
pub const PAGE_SIZE_BITS: usize = 0xc;
/// the max number of syscall
pub const MAX_SYSCALL_NUM: usize = 500;
/// the virtual addr of trapoline
pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
/// the virtual addr of trap context
pub const TRAP_CONTEXT_BASE: usize = TRAMPOLINE - PAGE_SIZE;
/// clock frequency (ticks per second)
pub const CLOCK_FREQ: usize = 12_500_000;
/// the physical memory end
pub const MEMORY_END: Address = 0x88000000;
