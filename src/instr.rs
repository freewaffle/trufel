// remove this lint in the future!!!!!!
#[allow(dead_code)]
#[derive(Debug)]
#[repr(u8)]
pub enum InstructionKind {
    // - `r` = register
    // - `k` = constant
    // - `c` = container
    // - `sp` = sprite slot

    Nop,

    /// `r(r0) = r(r1)`
    Mov { r0: u8, r1: u8 },
    /// `r(reg) = k(kn)`
    MovK { reg: u8, kn: u16 },
    /// `r(r0 ..= rn) = void`
    MovV { r0: u8, rn: u8 },

    /// `r(r0) = r(r1) + r(r2)`
    Add { r0: u8, r1: u8, r2: u8 },
    /// `r(r0) = r(r1) - r(r2)`
    Sub { r0: u8, r1: u8, r2: u8 },
    /// `r(r0) = r(r1) * r(r2)`
    Mul { r0: u8, r1: u8, r2: u8 },
    /// `r(r0) = r(r1) / r(r2)`
    Div { r0: u8, r1: u8, r2: u8 },
    /// `r(r0) = r(r1) % r(r2)`
    Rem { r0: u8, r1: u8, r2: u8 },
    /// `r(r0) = r(r1) && r(r2)`
    And { r0: u8, r1: u8, r2: u8 },
    /// `r(r0) = r(r1) || r(r2)`
    Or  { r0: u8, r1: u8, r2: u8 },
    // Xor { r0: u8, r1: u8, r2: u8 },
    /// `r(r0) = !r(r1)`
    Not { r0: u8, r1: u8 },
    /// `r(r0) = r(r1) << r(r2)`
    Shl { r0: u8, r1: u8, r2: u8 },
    /// `r(r0) = r(r1) >> r(r2)`
    Shr { r0: u8, r1: u8, r2: u8 },
    /// `r(r0) = r(r1) == r(r2)`
    Eq  { r0: u8, r1: u8, r2: u8 },
    /// `r(r0) = r(r1) != r(r2)`
    Neq { r0: u8, r1: u8, r2: u8 },
    /// `r(r0) = r(r1) < r(r2)`
    Lt  { r0: u8, r1: u8, r2: u8 },
    /// `r(r0) = r(r1) <= r(r2)`
    Lte { r0: u8, r1: u8, r2: u8 },
    
    /// if `reg > 0`: `pc += 1`
    Test { reg: u8 },
    Goto  { pos: u32 },
    Call { pos: u32 },

    /// `r(reg) = newcont(size)`
    NewCont { reg: u8 },
    /// `r(reg) = c(cont)[r(pos)]`
    GetCont { reg: u8, cont: u8, pos: u8 },
    /// `c(cont)[r(pos)] = r(value)`
    SetCont { cont: u8, pos: u8, value: u8 },
    /// `c(cont)[r(pos)] = r(value)`
    /// 
    /// same as `SetCont`, but panics if field `pos`
    /// wasn't set before.
    UpdCont { cont: u8, pos: u8, value: u8 },
    /// deletes field `c(cont)[r(pos)]`
    DelCont { cont: u8, pos: u8 },
    DropCont { cont: u8 },

    Int { port: u8, msg: u8 },

    /*
        NewSpr { slot: u8, xsize: u8, ysize: u8 },
        /// `sp(slot)[r(pos)] = r(value)`
        SetSpr { slot: u8, pos: u8, value: u8},

        /// if got a new event: `r(reg) = event_container`
        /// 
        /// else: `r(reg) = void`
        NextEvt { reg: u8 },
    */
    
    Ret,
    Halt,
}
