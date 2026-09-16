#[repr(u8)]
pub enum Instruction {
    Mov { r0: u8, c0: u16 },
    Add { r0: u8, r1: u8, r2: u8 },
    Sub { r0: u8, r1: u8, r2: u8 },
    Mul { r0: u8, r1: u8, r2: u8 },
    Div { r0: u8, r1: u8, r2: u8 },
    Mod { r0: u8, r1: u8, r2: u8 },
    And { r0: u8, r1: u8, r2: u8 },
    Or  { r0: u8, r1: u8, r2: u8 },
    Xor { r0: u8, r1: u8, r2: u8 },
    Not { r0: u8, r1: u8 },
    Shl { r0: u8, r1: u8, r2: u8 },
    Shr { r0: u8, r1: u8, r2: u8 },
    Eq  { r0: u8, r1: u8, r2: u8 },
    Neq { r0: u8, r1: u8, r2: u8 },
    Lt  { r0: u8, r1: u8, r2: u8 },
    Lte { r0: u8, r1: u8, r2: u8 },
    Gt  { r0: u8, r1: u8, r2: u8 },
    Gte { r0: u8, r1: u8, r2: u8 },
    Jmp { pos: u16 },
    Cmp,
    Call { pos: u16 },
    Ret,
    Store { r0: u8, r1: u8 },
    Fetch { r0: u8, r1: u8 },
    Int,
    Halt,
}

struct Value {
    pub payload: u64,
    pub tag: u8
}
