use std::fs::File;
use std::io::{BufRead, BufReader};

const DEBUG: bool = true;

#[allow(dead_code)]
#[derive(Debug)]
#[repr(u8)]
/// - `r` = register
/// - `k` = constant
/// - `c` = container
/// - `sp` = sprite slot
enum InstructionKind {
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
    Jmp  { pos: u32 },
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

struct Instruction {
    pub kind: InstructionKind,
}

#[derive(PartialEq, Debug, Clone)]
#[repr(u8)]
enum TokenKind {
    Identifier(String),
    String(String),
    Number(f32),
    RawNumber(u32),

    Comma,
}

struct Token {
    pub kind: TokenKind,
}

// #[repr(u8)]
pub enum ErrorKind {}

struct Assembler {
    pub filename: String
}

impl Assembler {
    #[inline]
    pub fn new(filename: String) -> Self {
        Self {
            filename
        }
    }

    pub fn generate_bytecode(&self, file: File) -> Result<Vec<u8>, ErrorKind> {
        let mut code: Vec<u8> = Vec::new();

        let reader = BufReader::new(file);
        let input_lines = reader.lines().map(|line| line.unwrap());

        for (line_number, line) in (1..).zip(input_lines) {
            for char in line.chars() {}
        }

        Ok(code)
    }
}

pub fn compile_from_file(file: File, filename: String) -> Result<Vec<u8>, ErrorKind> {
    let compiler = Assembler::new(filename);

    /* let tokens = compiler.parse_file(file)?;

    if DEBUG {
        println!("------------ TOKENS ------------");

        for line in tokens.iter() {
            if let Some(tok) = line.first() {
                print!("[{}] ", tok.line_pos);
            } else {
                continue;
            }

            for token in line {
                print!("{:?}, ", token.kind);
            }

            println!();
        }

        println!("--------------------------------\n");
    }

    let commands = compiler.parse_tokens(tokens)?;
    
    if DEBUG {
        println!("----------- COMMANDS -----------");

        for command in commands.iter() {
            println!("[{}] {:#?}", command.line_pos, command.kind);
        }

        println!("--------------------------------\n");
    }

    let bytecode: Vec<Instruction> = compiler.generate_instructions(commands)?;

    if DEBUG {
        println!("--------- INSTRUCTIONS ---------");

        for (index, instr) in bytecode.iter().enumerate() {
            println!("[{}] {:#?}", index, instr.kind);
        }

        println!("--------------------------------\n");
    } */

    // Ok(bytecode)
    Ok(Vec::new())
}
