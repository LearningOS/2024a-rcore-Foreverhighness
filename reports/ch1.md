# GBD
I download pre-build `riscv64-unknown-elf-gdb`, but it lack for `layout src`, `layout asm` command.

After exam [MIT 6.824](https://pdos.csail.mit.edu/6.828/2024/tools.html)'s preparation, I found an alternative is to install `gdb-multiarch`, which support `layout` command.

```bash
>>> sudo apt install gdb-multiarch
```

I also write `.gdbinit` file to speed up debugging.

```gdb
#.gdbinit
file target/riscv64gc-unknown-none-elf/release/os
set arch riscv:rv64
target remote localhost:1234
layout asm
break *0x1000
break *0x80000000
break *0x80200000
break rust_main
```

There is another question: gdb cannot disassemble rust code, so we need to use `rust-gdb`.

But the `rust-gdb` use original gdb, which cannot handle `riscv:rv64` arch, so I create an symbol link to it.

```bash
>>> sudo mv /usr/bin/gdb /usr/bin/gdb-original
>>> sudo ln -s /usr/bin/gdb-multiarch /usr/bin/gdb
```

# Linker Script

After reading linker scripts from [xv6-x86_64][1], [xv6-riscv][2] and [abstract-machine][3], I found that [`PROVIDE`][4] primitive is useful, and use `ALIGN(4K)` is more easier to understand than `ALIGN(0x1000)`.

[1]: https://github.com/mit-pdos/xv6-public/blob/master/kernel.ld "xv6-x86_64 linker script"
[2]: https://github.com/mit-pdos/xv6-riscv/blob/riscv/kernel/kernel.ld "xv6-riscv linker script"
[3]: https://github.com/NJU-ProjectN/abstract-machine/blob/master/scripts/linker.ld "abstract-machine linker script"
[4]: https://sourceware.org/binutils/docs/ld/PROVIDE.html "PROVIDE manual"