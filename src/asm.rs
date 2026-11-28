use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fs::File;
use std::io::{BufRead, BufReader};

use execution_time::ExecutionTime;

use crate::instr::InstructionKind;
use crate::magic::MAGIC;

const MAX_IDENTIFIER_LENGTH: usize = 32;

const DEBUG: bool = true;

const VERSION: u8 = 0;

struct Instruction {
    pub kind: InstructionKind,
}

#[derive(PartialEq, Debug, Clone)]
#[repr(u8)]
enum Token {
    Identifier(String),
    String(String),
    Number(f32),
    Address(u32),
    Register(u8),

    Colon,
}

impl Token {
    /* #[inline]
    fn description(&self, with_article: bool) -> &str {
        if with_article {
            match self {
                Identifier(..) => "an identifier",
                String(..) => "a string literal",
                Number(..) => "a number",
                Address(..) => "an address `$number`",

                Comma => "a comma",
                Colon => "a colon",
            }
        } else {
            match self {
                Identifier(..) => "identifier",
                String(..) => "string literal",
                Number(..) => "number",
                Address(..) => "address `$number`",

                Comma => "comma",
                Colon => "colon",
            }
        }
    } */
}

type MacrosMap = HashMap<String, Token>;

#[inline]
fn insert_trubeta_macros(macros: &mut MacrosMap) {
    macro_rules! add_macro {
        ($str:expr, $kind:ident, $value:expr) => {
            macros.insert(String::from($str), Token::$kind($value))
        };
    }

    add_macro!("BETA", Address, 0);

    /* blit flags ------------------------------------------------ */

    // using address because binary floats are not supported by rustc.
    add_macro!("SKIP_TRANSP", Address, 0b0001);

    /* screen control -------------------------------------------- */

    // 0 is nop.
    add_macro!("BETA_STARTFRAME", Address, 1);
    add_macro!("BETA_ENDFRAME",   Address, 2);
    // 3-9

    /* graphics primitives --------------------------------------- */

    add_macro!("BETA_DOT",  Address, 10);
    add_macro!("BETA_LINE", Address, 11);
    add_macro!("BETA_BOX",  Address, 12);
    add_macro!("BETA_FBOX", Address, 13);
    // 14-19

    /* textures manipulation ------------------------------------- */

    add_macro!("BETA_BLIT",   Address, 20);
    add_macro!("BETA_BLITS",  Address, 21);
    add_macro!("BETA_FBLITS", Address, 22);
    // 23-..
}

enum KValue {
    Number(f32),
    Address(u32),
    String(String)
}


/// struct containing the Trufcode itself and its metadata
struct Trufcode {
    pub code: Vec<u8>,
    pub code_len: usize
}

struct Class {
    pub code: Vec<Instruction>,
    pub boot_start: u32,
    pub constants: Vec<KValue>
}

impl Class {
    pub fn into_trufcode(self) -> Result<Trufcode, ErrorKind> {
        let mut chunk: Vec<u8> = Vec::new();

        macro_rules! write_u32 {
            ($num:expr) => {{
                chunk.push((($num >> 24) & 0xFF) as u8);
                chunk.push((($num >> 16) & 0xFF) as u8);
                chunk.push((($num >> 8)  & 0xFF) as u8);
                chunk.push(($num & 0xFF) as u8);
            }};
        }

        macro_rules! write_u16 {
            ($num:expr) => {{
                chunk.push((($num >> 8) & 0xFF) as u8);
                chunk.push(($num & 0xFF) as u8);
            }};
        }

        macro_rules! write_u8 {
            ($num:expr) => {
                chunk.push($num);
            };
        }

        macro_rules! write_f32 {
            ($num:expr) => {
                write_u32!($num.to_bits())
            };
        }

        chunk.extend_from_slice(&MAGIC);
        chunk.push(VERSION);

        /*
            constants type codes:
            0 = break (end of table)
            1 = number
            2 = address
            3 = string
        */

        for (cid, constant) in self.constants.into_iter().enumerate() {
            write_u8!(match constant {
                KValue::Number(_) => 1,
                KValue::Address(_) => 2,
                KValue::String(_) => 3
            });

            write_u16!(cid as u16);

            match constant {
                KValue::Number(num) => write_f32!(num),
                KValue::Address(addr) => write_u32!(addr),
                KValue::String(str) => {
                    let bytes = str.as_bytes();

                    let len: u32 = bytes.len() as u32;
                    write_u32!(len);

                    for byte in bytes {
                        write_u8!(*byte);
                    }
                }
            }
        }

        // end the constants table
        write_u8!(0);
        write_u32!(self.boot_start);
        let code_offset = chunk.len();

        for instr in self.code {
            let kind = instr.kind;

            macro_rules! r3 {
                ($opcode:expr, $r0:ident, $r1:ident, $r2:ident) => {{
                    write_u8!($opcode);
                    write_u8!($r0);
                    write_u8!($r1);
                    write_u8!($r2);
                }};
            }

            use InstructionKind::*;
            match kind {
                Nop => {
                    write_u8!(0);
                }

                Mov { r0, r1 } => {
                    write_u8!(1);
                    write_u8!(r0);
                    write_u8!(r1);
                }

                MovK { reg, kaddr } => {
                    write_u8!(2);
                    write_u8!(reg);
                    write_u16!(kaddr);
                }

                MVoid { r0, rn } => {
                    write_u8!(3);
                    write_u8!(r0);
                    write_u8!(rn);
                }

                Add { r0, r1, r2 } => r3!(4,  r0, r1, r2),
                Sub { r0, r1, r2 } => r3!(5,  r0, r1, r2),
                Mul { r0, r1, r2 } => r3!(6,  r0, r1, r2),
                Div { r0, r1, r2 } => r3!(7,  r0, r1, r2),
                Rem { r0, r1, r2 } => r3!(8,  r0, r1, r2),
                And { r0, r1, r2 } => r3!(9,  r0, r1, r2),
                Or  { r0, r1, r2 } => r3!(10, r0, r1, r2),

                Not { r0, r1 } => {
                    write_u8!(11);
                    write_u8!(r0);
                    write_u8!(r1);
                }

                Shl { r0, r1, r2 } => r3!(12, r0, r1, r2),
                Shr { r0, r1, r2 } => r3!(13, r0, r1, r2),
                Eq  { r0, r1, r2 } => r3!(14, r0, r1, r2),
                Neq { r0, r1, r2 } => r3!(15, r0, r1, r2),
                Lt  { r0, r1, r2 } => r3!(16, r0, r1, r2),
                Lte { r0, r1, r2 } => r3!(17, r0, r1, r2),

                Test { reg } => {
                    write_u8!(18);
                    write_u8!(reg);
                }

                Goto { pos } => {
                    write_u8!(19);
                    write_u8!(pos);
                }

                GotoK { kaddr } => {
                    write_u8!(20);
                    write_u16!(kaddr);
                }

                Call { retr, pos, argr } => r3!(21, retr, pos, argr),

                NewCont { reg } => {
                    write_u8!(22);
                    write_u8!(reg);
                }

                GetCont { reg, cont, pos } => r3!(23, reg, cont, pos),
                SetCont { cont, pos, value } => r3!(24, cont, pos, value),
                UpdCont { cont, pos, value } => r3!(25, cont, pos, value),
                
                DelCont { cont, pos } => {
                    write_u8!(26);
                    write_u8!(cont);
                    write_u8!(pos);
                }

                DropCont { cont } => {
                    write_u8!(27);
                    write_u8!(cont);
                }

                Int { port, msg } => {
                    write_u8!(28);
                    write_u8!(port);
                    write_u8!(msg);
                }

                RetV => {
                    write_u8!(29);
                }

                RetK { kaddr } => {
                    write_u8!(30);
                    write_u16!(kaddr);
                }

                Ret { reg } => {
                    write_u8!(31);
                    write_u8!(reg);
                }

                Halt => {
                    write_u8!(32);
                }
            }
        }

        let trufcode = Trufcode {
            code_len: chunk.len() - code_offset,
            code: chunk,
        };

        Ok(trufcode)
    }
}

#[derive(Debug)]
#[repr(u8)]
pub enum ErrorKind {
    UnrecognizedCharacter,
    NumberOverflow,
    TooLongIdentifier,
    RedundantDot,
    UnclosedString,
    TooLongString,
    UnexpectedTokens,
    ExpectedArgument,
    InvalidArity,
    MalformedNumber,
    InvalidOpcode,
    ReachedCodeLengthLimit,
    LabelRedefinition,
    MacroRedefinition,
    FullConstantPool,
    InstructionBeforeLabel,
    BootNotDefined,
    IOError
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

    pub fn generate_tokens(&self, line: String, line_number: u32) -> Result<Vec<Token>, ErrorKind> {
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
                $ch.to_digit(10).expect(format!("expected digit, got: `{}`", $ch).as_str())
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

        macro_rules! is_identifier {
            ($ch:expr) => {
                ($ch.is_ascii_alphabetic() || $ch == '_')
            };
        }

        macro_rules! is_digit {
            ($ch:expr) => {
                $ch.is_ascii_digit()
            };
        }

        macro_rules! next_char {
            () => {{
                chars.next()
            }};
        }

        while let Some(char) = next_char!() {
            if char.is_ascii_whitespace() {
                continue
            }

            let is_identifier: bool = is_identifier!(char);
            let is_digit: bool = is_digit!(char);
            let is_string: bool = char == '"' || char == '\'';
            let is_line_comment: bool = char == '#';

            let mut new_token: Option<Token> = None;

            if is_identifier {
                let mut ident: String = String::with_capacity(MAX_IDENTIFIER_LENGTH);

                ident.push(char);

                while let Some(ch) = chars.peek() {
                    let ch = *ch;

                    if ident.len() > MAX_IDENTIFIER_LENGTH {
                        print_error!(TooLongIdentifier, format!(
                            "too long identifier (max length is {} symbols)",
                            MAX_IDENTIFIER_LENGTH
                        ));
                    }

                    if ch.is_ascii_digit() && (
                        ident == "r" ||
                        ident == "a"
                    ) {
                        break;
                    }

                    if is_identifier!(ch) || ch.is_ascii_digit() {
                        ident.push(ch);
                        next_char!();
                    } else {
                        break;
                    }
                }

                new_token = Some(Token::Identifier(ident));
            }

            if is_digit {
                let last = tokens.last();

                new_token = if let Some(Token::Identifier(ident)) = last {
                    match ident.as_str() {
                        "a" => {
                            let mut num: u32 = to_digit!(char);

                            while let Some(char) = chars.peek() {
                                let char = *char;

                                if is_digit!(char) {
                                    let digit = to_digit!(char);
                                    assert!(digit < 10);

                                    num_op!(num, checked_mul, 10);
                                    num_op!(num, checked_add, digit);

                                    next_char!();
                                } else {
                                    if char == '.' {
                                        print_error!(MalformedNumber, "unexpected dot in an address");
                                    } else {
                                        break;
                                    }
                                }
                            }

                            tokens.pop();

                            Some(Token::Address(num))
                        }

                        "r" => {
                            let mut num: u8 = to_digit!(char).try_into().unwrap();

                            while let Some(char) = chars.peek() {
                                let char = *char;

                                if is_digit!(char) {
                                    let digit: u8 = to_digit!(char).try_into().unwrap();
                                    assert!(digit < 10);

                                    num_op!(num, checked_mul, 10);
                                    num_op!(num, checked_add, digit);

                                    next_char!();
                                } else {
                                    if char == '.' {
                                        print_error!(MalformedNumber, "unexpected dot in an address");
                                    } else {
                                        break;
                                    }
                                }
                            }

                            tokens.pop();

                            Some(Token::Register(num))
                        }

                        _ => None
                    }
                } else {
                    None
                };

                if new_token.is_none() {
                    // TODO: use one String to store parsed number.

                    let mut num_a: u32 = to_digit!(char);
                    let mut num_b: String = String::new();
                    let mut collecting_a = true;

                    while let Some(char) = chars.peek() {
                        let char = *char;

                        if is_digit!(char) {
                            let digit = to_digit!(char);
                            assert!(digit < 10);

                            if collecting_a {
                                num_op!(num_a, checked_mul, 10);
                                num_op!(num_a, checked_add, digit);
                            } else {
                                num_b.push(char);
                            }

                            next_char!();
                        } else {
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
                    }

                    let snum: String = format!("{num_a}.{num_b}");
                    let num: f32 = snum.parse().unwrap();

                    new_token = Some(Token::Number(num));
                }
            }

            if is_string {
                let mut string: String = String::new();
                let mut closed = false;
                let started_quote_mark = char;

                while let Some(char) = next_char!() {
                    if u32::try_from(string.len()).is_err() {
                        print_error!(TooLongString, "reached string size limit (2**32 max bytes)");
                    }

                    if char == started_quote_mark {
                        closed = true;
                        break;
                    }

                    string.push(char);
                }

                if closed {
                    let token = Token::String(string);
                    new_token = Some(token);
                } else {
                    print_error!(UnclosedString, "unclosed string");
                }
            }

            if is_line_comment {
                break;
            }

            if new_token.is_none() {
                let kind = match char {
                    ':' => Some(Token::Colon),

                    // whitespaces and tabs are skipped in the beginning
                    _ => None
                };

                if let Some(kind) = kind {
                    new_token = Some(kind);
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

        // tokens = self.parse_numbers(tokens, line_number)?;

        Ok(tokens)
    }

    pub fn unwrap_constants(
        &self,
        constants: &mut Vec<KValue>,
        tokens: Vec<Token>,
        line_number: u32
    ) -> Result<Vec<Token>, ErrorKind> {
        macro_rules! print_error {
            ($err_kind:ident, $msg:expr) => {{
                eprintln!("{}: line {}:", self.filename, line_number);
                eprintln!("  error: {}", $msg);
                return Err(ErrorKind::$err_kind);
            }};
        }

        let mut final_tokens: Vec<Token> = Vec::new();

        macro_rules! get_or_add_constant {
            ($val:expr, $kind:ident) => {{
                // итоговый адрес константы
                let mut kaddr: Option<u32> = None;

                // пробегаемся по всем существующим константам.
                for (kpos, kval) in constants.iter().enumerate() {
                    // если текущая константа соответствует типу и значением...
                    if let KValue::$kind(kval) = kval && *kval == $val {
                        // ...сохраняем адрес этой константы.
                        kaddr = Some(kpos as u32);
                        break;
                    }
                }

                // если нет требуемого значения из пула констант, создаём новую.
                if kaddr.is_none() {
                    // длина пула будет служить адресом новой константы.
                    let len: u32 = constants.len() as u32;

                    // проверяем на переполнение в 16-битном адресном режиме.
                    if len >= 2u32.pow(16) {
                        print_error!(FullConstantPool, "constant pool is full");
                    }

                    constants.push(KValue::$kind($val));
                    kaddr = Some(len);
                }

                kaddr.unwrap()
            }};
        }

        for token in tokens {
            use Token::*;
            let new_token = match token {
                Number(num) => {
                    let kaddr: u32 = get_or_add_constant!(num, Number);
                    Address(kaddr)
                }

                String(string) => {
                    let kaddr: u32 = get_or_add_constant!(string, String);
                    Address(kaddr)
                }

                Address(addr) => {
                    let kaddr: u32 = get_or_add_constant!(addr, Address);
                    Address(kaddr)
                }

                _ => token
            };

            final_tokens.push(new_token);
        }

        Ok(final_tokens)
    }

    pub fn unwrap_macros(&self, tokens: Vec<Token>, macros: &MacrosMap) -> Result<Vec<Token>, ErrorKind> {
        let mut result: Vec<Token> = Vec::new();

        for gtoken in tokens {
            let token = if let Token::Identifier(ident) = &gtoken
            && let Some(mtk) = macros.get(ident) {
                mtk.clone()
            } else {
                gtoken
            };

            result.push(token);
        }

        Ok(result)
    }
    
    pub fn make_class_from_file(&self, file: File) -> Result<Class, ErrorKind> {
        macro_rules! print_error {
            ($err_kind:ident, $msg:expr) => {{
                eprintln!("{}:", self.filename);
                eprintln!("  error: {}", $msg);
                return Err(ErrorKind::$err_kind);
            }};
        }
        
        let mut code: Vec<Instruction> = Vec::new();
        let mut code_length: u32 = 0;
        let mut labels: HashMap<String, u32> = HashMap::new();
        let mut boot_start: Option<u32> = None;

        let mut constants: Vec<KValue> = Vec::new();

        let mut macros: MacrosMap = HashMap::new();
        insert_trubeta_macros(&mut macros);

        let reader = BufReader::new(file);
        let input_lines = reader.lines();

        for (line_number, line) in (1u32..).zip(input_lines) {
            macro_rules! print_error {
                ($err_kind:ident, $msg:expr) => {{
                    eprintln!("{}: line {}:", self.filename, line_number);
                    eprintln!("  error: {}", $msg);
                    return Err(ErrorKind::$err_kind);
                }};
            }

            let line = match line {
                Ok(string) => string,
                Err(err) => print_error!(IOError, err.kind())
            };

            let generated_tokens: Vec<Token> = self.generate_tokens(line, line_number)?;
            let unmacrofied_tokens: Vec<Token> = self.unwrap_macros(generated_tokens, &macros)?;
            let tokens: Vec<Token> = self.unwrap_constants(&mut constants, unmacrofied_tokens, line_number)?;

            let first_token = if let Some(tok) = tokens.first() {
                tok
            } else {
                // empty
                continue;
            };

            use Token::*;
            use InstructionKind as Instr;
            match &first_token {
                // INSTRUCTIONS ONLY!!
                Identifier(ident) => {
                    if labels.is_empty() {
                        print_error!(InstructionBeforeLabel, "a label must be declared before any instructions");
                    }

                    let ident = ident.to_lowercase();
                    let ident: &str = ident.as_str();

                    let args_count = tokens.len() - 1;

                    macro_rules! get_arg {
                        ($pos:expr, $tok_kind:ident) => {
                            match tokens.get(1 + $pos) {
                                Some(tok) => if let Token::$tok_kind(value) = tok {
                                    Some(*value)
                                } else {
                                    None
                                },

                                None => None
                            }
                        };
                    }

                    macro_rules! check_arity {
                        ($required:expr) => {
                            if args_count != $required {
                                print_error!(InvalidArity, format!("expected {} argument(s), but got {args_count}", $required));
                            }
                        };
                    }

                    macro_rules! mathop3 {
                        ($kind:ident) => {{
                            check_arity!(3);

                            let r0: u8 = if let Some(value) = get_arg!(0, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #0");
                            };

                            let r1: u8 = if let Some(value) = get_arg!(1, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #1");
                            };

                            let r2: u8 = if let Some(value) = get_arg!(2, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #2");
                            };

                            Instr::$kind { r0, r1, r2 }
                        }};
                    }

                    macro_rules! mathop2 {
                        ($kind:ident) => {{
                            check_arity!(2);

                            let r0: u8 = if let Some(value) = get_arg!(0, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #0");
                            };

                            let r1: u8 = if let Some(value) = get_arg!(1, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #1");
                            };

                            Instr::$kind { r0, r1 }
                        }};
                    }

                    // this hell should be simplified
                    let kind = match ident {
                        "nop" => {
                            check_arity!(0);
                            Instr::Nop
                        }

                        "mov" => {
                            check_arity!(2);

                            let r0: u8 = if let Some(value) = get_arg!(0, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #0");
                            };

                            let r1: u8 = if let Some(value) = get_arg!(1, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #1");
                            };

                            Instr::Mov { r0, r1 }
                        }

                        "movk" => {
                            check_arity!(2);

                            let reg: u8 = if let Some(value) = get_arg!(0, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #0");
                            };

                            let kaddr32: u32 = if let Some(value) = get_arg!(1, Address) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a constant address at argument #1");
                            };

                            let kaddr: u16 = if let Ok(num) = kaddr32.try_into() {
                                num
                            } else {
                                print_error!(MalformedNumber, "constant address out of bounds");
                            };

                            Instr::MovK { reg, kaddr }
                        }

                        "mvoid" => {
                            check_arity!(2);

                            let r0: u8 = if let Some(value) = get_arg!(0, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #0");
                            };

                            let rn: u8 = if let Some(value) = get_arg!(1, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #1");
                            };

                            Instr::MVoid { r0, rn }
                        }

                        "add" => { mathop3!(Add) }
                        "sub" => { mathop3!(Sub) }
                        "mul" => { mathop3!(Mul) }
                        "div" => { mathop3!(Div) }
                        "rem" => { mathop3!(Rem) }
                        "and" => { mathop3!(And) }
                        "or"  => { mathop3!(Or)  }
                        "not" => { mathop2!(Not) }
                        "shl" => { mathop3!(Shl) }
                        "shr" => { mathop3!(Shr) }
                        "eq"  => { mathop3!(Eq)  }
                        "neq" => { mathop3!(Neq) }
                        "lt"  => { mathop3!(Lt)  }
                        "lte" => { mathop3!(Lte) }

                        "test" => {
                            check_arity!(1);

                            let reg: u8 = if let Some(value) = get_arg!(0, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #0");
                            };

                            Instr::Test { reg }
                        }

                        "goto" => {
                            check_arity!(1);

                            let pos: u8 = if let Some(value) = get_arg!(0, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #0");
                            };

                            Instr::Goto { pos }
                        }

                        "gotok" => {
                            check_arity!(1);

                            let kaddr32: u32 = if let Some(value) = get_arg!(0, Address) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a constant address at argument #0");
                            };

                            let kaddr: u16 = if let Ok(num) = kaddr32.try_into() {
                                num
                            } else {
                                print_error!(MalformedNumber, "constant address out of bounds");
                            };

                            Instr::GotoK { kaddr }
                        }

                        "call" => {
                            check_arity!(3);

                            let retr: u8 = if let Some(value) = get_arg!(0, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #0");
                            };

                            let pos: u8 = if let Some(value) = get_arg!(1, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #1");
                            };

                            let argr: u8 = if let Some(value) = get_arg!(2, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #2");
                            };

                            Instr::Call { retr, pos, argr }
                        }

                        "newcont" => {
                            check_arity!(1);

                            let reg: u8 = if let Some(value) = get_arg!(0, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #0");
                            };

                            Instr::NewCont { reg }
                        }

                        "getcont" => {
                            check_arity!(3);

                            let reg: u8 = if let Some(value) = get_arg!(0, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #0");
                            };

                            let cont: u8 = if let Some(value) = get_arg!(1, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #1");
                            };

                            let pos: u8 = if let Some(value) = get_arg!(2, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #2");
                            };

                            Instr::GetCont { reg, cont, pos }
                        }

                        "setcont" | "updcont" => {
                            check_arity!(3);

                            let cont: u8 = if let Some(value) = get_arg!(0, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #0");
                            };

                            let pos: u8 = if let Some(value) = get_arg!(1, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #1");
                            };

                            let value: u8 = if let Some(value) = get_arg!(2, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #2");
                            };

                            if ident == "setcont" {
                                Instr::SetCont { cont, pos, value }
                            } else {
                                Instr::UpdCont { cont, pos, value }
                            }
                        }

                        "delcont" => {
                            check_arity!(2);

                            let cont: u8 = if let Some(value) = get_arg!(0, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #0");
                            };

                            let pos: u8 = if let Some(value) = get_arg!(1, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #1");
                            };

                            Instr::DelCont { cont, pos }
                        }

                        "dropcont" => {
                            check_arity!(1);

                            let cont: u8 = if let Some(value) = get_arg!(0, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #0");
                            };

                            Instr::DropCont { cont }
                        }

                        "int" => {
                            check_arity!(2);

                            let port: u8 = if let Some(value) = get_arg!(0, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #0");
                            };

                            let msg: u8 = if let Some(value) = get_arg!(1, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #1");
                            };

                            Instr::Int { port, msg }
                        }

                        "retv" => {
                            check_arity!(0);
                            Instr::RetV
                        },

                        "retk" => {
                            check_arity!(1);

                            let kaddr32: u32 = if let Some(value) = get_arg!(0, Address) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a constant address at argument #0");
                            };

                            let kaddr: u16 = if let Ok(num) = kaddr32.try_into() {
                                num
                            } else {
                                print_error!(MalformedNumber, "constant address out of bounds");
                            };

                            Instr::RetK { kaddr }
                        }

                        "ret" => {
                            check_arity!(1);

                            let reg: u8 = if let Some(value) = get_arg!(0, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #0");
                            };

                            Instr::Ret { reg }
                        }

                        "halt" => {
                            check_arity!(0);
                            Instr::Halt
                        },

                        _ => {
                            print_error!(InvalidOpcode, "invalid opcode");
                        }
                    };

                    // additional instruction checks?

                    let instr = Instruction { kind };

                    code.push(instr);
                    if let Some(len) = code_length.checked_add(1) {
                        code_length = len;
                    } else {
                        print_error!(ReachedCodeLengthLimit, "reached code length limit (2**32 max instructions)");
                    }
                }

                Colon => {
                    let name = if let Some(Token::Identifier(ident)) = tokens.get(1) {
                        ident.clone()
                    } else {
                        print_error!(UnexpectedTokens, "expected an identifier after colon");
                    };

                    let is_boot_label = name == "boot";

                    // проверяем, существует ли такая метка.
                    match labels.entry(name.clone()) {
                        // ключ занят.
                        Entry::Occupied(_) => {
                            print_error!(LabelRedefinition, "duplicate label");
                        }

                        // такой метки нет. теперь проверям в макросах.
                        Entry::Vacant(label_entry) => match macros.entry(name) {
                            // ключ в макросах занят.
                            Entry::Occupied(_) => {
                                print_error!(MacroRedefinition, "a macro defined with the same name");
                            }

                            // ключ в макросах свободен.
                            Entry::Vacant(macro_entry) => {
                                if is_boot_label {
                                    boot_start = Some(code_length);
                                }

                                macro_entry.insert(Address(code_length));
                                label_entry.insert(code_length);
                            }
                        }
                    }
                }

                _ => {
                    print_error!(UnexpectedTokens, "expected an identifier or a colon");
                }
            }
        }

        let boot_start: u32 = if let Some(pos) = boot_start {
            pos
        } else {
            print_error!(BootNotDefined, "`boot` label not defined");
        };

        let class = Class { code, boot_start, constants };

        Ok(class)
    }
}

pub fn compile_from_file(file: File, filename: String) -> Result<Vec<u8>, ErrorKind> {
    let assembler = Assembler::new(filename);

    let timer = ExecutionTime::start();
    let class = assembler.make_class_from_file(file)?;
    let codegen_time = timer.get_duration();

    let code_length = class.code.len();
    let consts_count = class.constants.len();
    
    if DEBUG {
        println!("--------- INSTRUCTIONS ---------");

        for (index, instr) in class.code.iter().enumerate() {
            println!("[{}] {:#?}", index, instr.kind);
        }
        
        println!("--------------------------------\n");
    }

    println!("codegen took ~{} ms", codegen_time.as_millis());

    println!(
        "generated {} instructio{} and {} constan{}",

        code_length,
        if code_length != 1 { "ns" } else { "n" },

        consts_count,
        if consts_count != 1 { "ts" } else { "t" },
    );

    let trufcode = class.into_trufcode()?;
    let code_size = trufcode.code_len;
    
    if code_size > 1024 {
        let code_size_kibs = code_size as f32 / 1024.;
        println!("code takes {:.1} KiB ({} bytes) of Trualpha memory", code_size_kibs, code_size);
    } else {
        println!("code takes {} bytes of Trualpha memory", code_size);
    }

    Ok(trufcode.code)
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! token {
        ($kind:ident) => {
            Token::$kind
        };

        ($kind:ident, $value:expr) => {
            Token::$kind($value)
        };
    }

    #[test]
    fn test_token_generator() {
        let input = "ident 123.45 r123 a456";

        let expected = vec![
            token!(Identifier, String::from("ident")),
            token!(Number, 123.45),
            token!(Register, 123),
            token!(Address, 456),
        ];

        let asm = Assembler::new(String::from("<waffle>"));

        let result = asm.generate_tokens(String::from(input), 0).unwrap();

        /* println!("EXPECTED >>> {expected:#?}");
        println!("RESULT >>> {result:#?}"); */

        assert_eq!(result, expected)
    }

    #[test]
    fn test_number_parser() {
        let input = "r8 a777 ident 123";

        let expected = vec![
            token!(Register, 8),
            token!(Address, 777),
            token!(Identifier, String::from("ident")),
            token!(Number, 123.),
        ];

        let asm = Assembler::new(String::from("<waffle>"));

        let result = asm.generate_tokens(String::from(input), 0).unwrap();

        /* println!("EXPECTED >>> {expected:#?}");
        println!("RESULT >>> {result:#?}"); */

        assert_eq!(result, expected)
    }

    #[test]
    fn test_number_parser_error() {
        let input = "r8.0 a777.7 a";

        let asm = Assembler::new(String::from("<waffle>"));

        let result = asm.generate_tokens(String::from(input), 0);

        assert!(result.is_err())
    }
}
