//! SBI call wrappers

use core::arch::asm;

const SBI_CONSOLE_PUTCHAR: usize = 1;

// https://docs.rs/rustsbi/latest/rustsbi/#call-sbi-in-different-programming-languages
/// general sbi call
#[inline(always)]
fn sbi_call(which: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut ret;
    unsafe {
        asm!(
            "li a6, 0",
            "ecall",
            inlateout("a0") arg0 => ret,
            in("a1") arg1,
            in("a2") arg2,
            in("a7") which,
        );
    }
    ret
}

// https://github.com/riscv-non-isa/riscv-sbi-doc/blob/master/src/ext-legacy.adoc#extension-console-getchar-eid-0x02
/// use sbi call to putchar in console (qemu uart handler)
pub fn console_putchar(c: usize) {
    sbi_call(SBI_CONSOLE_PUTCHAR, c, 0, 0);
}

use crate::board::QEMUExit;
/// use sbi call to shutdown the kernel
pub fn shutdown() -> ! {
    crate::board::QEMU_EXIT_HANDLE.exit_failure();
}
