file target/riscv64gc-unknown-none-elf/release/os
set arch riscv:rv64
target remote localhost:1234

layout asm
break *0x1000
break *0x80000000
break *0x80200000
break rust_main