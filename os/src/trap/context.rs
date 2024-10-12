use riscv::register::sstatus::{self, Sstatus, SPP};

#[repr(C)]
#[derive(Debug)]
/// trap context structure containing sstatus, sepc and registers
pub struct TrapContext {
    /// General-Purpose Register x0-31
    pub x: [usize; 32],
    /// Supervisor Status Register
    pub sstatus: Sstatus,
    /// Supervisor Exception Program Counter
    pub sepc: usize,
}

impl TrapContext {
    /// put the sp(stack pointer) into x\[2\] field of TrapContext
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }
    /// init the trap context of an application
    pub fn app_init_context(entry: usize, sp: usize) -> Self {
        let mut sstatus = sstatus::read(); // CSR sstatus
        sstatus.set_spp(SPP::User); //previous privilege mode: user mode
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry, // entry point of app
        };
        cx.set_sp(sp); // app's user stack pointer
        cx // return initial Trap Context of app
    }
}

impl TrapContext {
    /// helper function for get gpr
    pub const fn gpr(&self, name: &'static [u8]) -> usize {
        match name {
            b"x0" | b"zero" => self.x[0],       // Hard-wired zero
            b"x1" | b"ra" => self.x[1],         // Return address                   | Caller
            b"x2" | b"sp" => self.x[2],         // Stack pointer                    | Callee
            b"x3" | b"gp" => self.x[3],         // Global pointer
            b"x4" | b"tp" => self.x[4],         // Thread pointer
            b"x5" | b"t0" => self.x[5],         // Temporaries                      | Caller
            b"x6" | b"t1" => self.x[6],         // Temporaries                      | Caller
            b"x7" | b"t2" => self.x[7],         // Temporaries                      | Caller
            b"x8" | b"s0" | b"fp" => self.x[8], // Saved register/frame pointer     | Callee
            b"x9" | b"s1" => self.x[9],         // Saved register                   | Callee
            b"x10" | b"a0" => self.x[10],       // Function arguments/return values | Caller
            b"x11" | b"a1" => self.x[11],       // Function arguments/return values | Caller
            b"x12" | b"a2" => self.x[12],       // Function arguments               | Caller
            b"x13" | b"a3" => self.x[13],       // Function arguments               | Caller
            b"x14" | b"a4" => self.x[14],       // Function arguments               | Caller
            b"x15" | b"a5" => self.x[15],       // Function arguments               | Caller
            b"x16" | b"a6" => self.x[16],       // Function arguments               | Caller
            b"x17" | b"a7" => self.x[17],       // Function arguments               | Caller
            b"x18" | b"s2" => self.x[18],       // Saved registers                  | Callee
            b"x19" | b"s3" => self.x[19],       // Saved registers                  | Callee
            b"x20" | b"s4" => self.x[20],       // Saved registers                  | Callee
            b"x21" | b"s5" => self.x[21],       // Saved registers                  | Callee
            b"x22" | b"s6" => self.x[22],       // Saved registers                  | Callee
            b"x23" | b"s7" => self.x[23],       // Saved registers                  | Callee
            b"x24" | b"s8" => self.x[24],       // Saved registers                  | Callee
            b"x25" | b"s9" => self.x[25],       // Saved registers                  | Callee
            b"x26" | b"s10" => self.x[26],      // Saved registers                  | Callee
            b"x27" | b"s11" => self.x[27],      // Saved registers                  | Callee
            b"x28" | b"t3" => self.x[28],       // Temporaries                      | Caller
            b"x29" | b"t4" => self.x[29],       // Temporaries                      | Caller
            b"x30" | b"t5" => self.x[30],       // Temporaries                      | Caller
            b"x31" | b"t6" => self.x[31],       // Temporaries                      | Caller
            _ => unimplemented!(),
        }
    }
}
