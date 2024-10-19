file target/riscv64gc-unknown-none-elf/release/os
set arch riscv:rv64
target remote localhost:1234

# `layout asm` will trigger pagination
set pagination off
layout asm
focus cmd



set pagination on
