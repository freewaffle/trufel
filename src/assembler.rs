use std::fs::File;
use std::io::{BufRead, BufReader};

const MAX_IDENTIFIER_LENGTH: usize = 32;

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
    Address(u32),

    Comma,
    Colon,
    Dollar,
}

struct Token {
    pub kind: TokenKind,
}

#[repr(u8)]
pub enum ErrorKind {
    UnrecognizedCharacter,
    NumberOverflow,
    TooLongIdentifier,
    RedundantDot,
    DotInAddress,
    UnclosedString,
    DollarWithoutAddress
}

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
            let mut tokens: Vec<Token> = Vec::new();

            let mut chars = line.chars().peekable();

            macro_rules! print_error {
                ($err_kind:ident, $msg:expr) => {{
                    eprintln!("{}: line {}:", self.filename, line_number);
                    eprintln!("  error: {}", $msg);
                    return Err(ErrorKind::$err_kind);
                }};
            }

            macro_rules! to_digit {
                ($ch:expr) => {
                    $ch.to_digit(10).unwrap()
                };
            }

            macro_rules! num_op {
                ($var:ident, $func:ident, $func_expr:expr) => {
                    if let Some(result) = $var.$func($func_expr) {
                        $var = result;
                    } else {
                        print_error!(NumberOverflow, "number overflow");
                    }
                };
            }

            macro_rules! is_string_token {
                ($ch:expr) => {
                    ($ch == '"' || $ch == '\'')
                };
            }

            macro_rules! is_identifier_token {
                ($ch:expr) => {
                    ($ch.is_ascii_alphabetic() || $ch == '_')
                };
            }

            macro_rules! collect_identifier {
                ($start_char:expr) => {{
                    let mut ident: String = String::with_capacity(MAX_IDENTIFIER_LENGTH);

                    ident.push($start_char);

                    while let Some(ch) = chars.peek() {
                        let ch = *ch;

                        if ident.len() >= MAX_IDENTIFIER_LENGTH {
                            print_error!(TooLongIdentifier, format!(
                                "too long identifier (max length is {} symbols)",
                                MAX_IDENTIFIER_LENGTH
                            ));
                        }

                        let is_char = is_identifier_token!(ch) || ch.is_ascii_digit();
                        if !is_char {
                            break;
                        }

                        ident.push(ch);
                        next_char!();
                    }

                    ident
                }};
            }

            macro_rules! collect_number {
                ($start_char:expr) => {{
                    let mut num_a: u32 = to_digit!($start_char);
                    let mut num_b: u32 = 0;
                    let mut collecting_a = true;

                    while let Some(char) = chars.peek() {
                        let char = *char;

                        if !char.is_ascii_digit() {
                            if char == '.' {
                                if collecting_a {
                                    collecting_a = false;
                                    next_char!();
                                    continue;
                                } else {
                                    print_error!(RedundantDot, "redundant dot near number");
                                }
                            } else {
                                break;
                            }
                        }

                        let digit = to_digit!(char);
                        assert!(digit < 10);

                        if collecting_a {
                            num_op!(num_a, checked_mul, 10);
                            num_op!(num_a, checked_add, digit);
                        } else {
                            num_op!(num_b, checked_mul, 10);
                            num_op!(num_b, checked_add, digit);
                        }

                        next_char!();
                    }

                    let snum: String = format!("{num_a}.{num_b}");
                    let num: f32 = snum.parse().unwrap();

                    num
                }};
            }

            macro_rules! collect_address_number {
                ($start_char:expr) => {{
                    let mut num: u32 = to_digit!($start_char);

                    while let Some(char) = chars.peek() {
                        let char = *char;

                        if !char.is_ascii_digit() {
                            if char == '.' {
                                print_error!(DotInAddress, "dots cannot appear in addresses, as they cannot be float");
                            } else {
                                break;
                            }
                        }

                        let digit = to_digit!(char);
                        assert!(digit < 10);

                        num_op!(num, checked_mul, 10);
                        num_op!(num, checked_add, digit);

                        next_char!();
                    }

                    num
                }};
            }

            macro_rules! next_char {
                () => {{
                    chars.next()
                }};
            }

            for char in next_char!() {
                if char.is_ascii_whitespace() {
                    continue
                }

                let identifier: bool = is_identifier_token!(char);
                let number: bool = char.is_ascii_digit();
                let string: bool = is_string_token!(char);
                let line_comment: bool = char == '#';

                let mut new_token: Option<Token> = None;

                if identifier {
                    let ident: String = collect_identifier!(char);
                    let token = Token { kind: TokenKind::Identifier(ident) };
                    new_token = Some(token);
                }

                if number {
                    let num: f32 = collect_number!(char);
                    let token = Token { kind: TokenKind::Number(num) };
                    new_token = Some(token);
                }

                if string {
                    let mut string: String = String::new();
                    let mut closed = false;

                    while let Some(char) = next_char!() {
                        if is_string_token!(char) {
                            closed = true;
                            break;
                        }

                        string.push(char);
                    }

                    if closed {
                        let token = Token { kind: TokenKind::String(string) };
                        new_token = Some(token);
                    } else {
                        print_error!(UnclosedString, "unclosed string");
                    }
                }

                if line_comment {
                    break;
                }

                if new_token.is_none() {
                    let kind = match char {
                        ',' => Some(TokenKind::Comma),
                        ':' => Some(TokenKind::Colon),

                        '$' => {
                            if next_char!().is_some_and(|char| char.is_ascii_digit()) {
                                let num: u32 = collect_address_number!(char);
                                Some(TokenKind::Address(num))
                            } else {
                                print_error!(DollarWithoutAddress, "dollar sign without a following address");
                            }
                        }

                        // whitespaces and tabs are skipped in the beginning
                        _ => None
                    };

                    if let Some(kind) = kind {
                        new_token = Some(Token { kind });
                    } // else None
                }

                if let Some(tok) = new_token {
                    tokens.push(tok);
                } else {
                    print_error!(UnrecognizedCharacter, format!(
                        "unrecognized character: '{}'",
                        char
                    ));
                }
            }
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
