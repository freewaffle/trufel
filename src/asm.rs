use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fs::File;
use std::io::{BufRead, BufReader};

use crate::instr::InstructionKind;
use crate::vm::MAX_REGISTERS_COUNT;

const MAX_IDENTIFIER_LENGTH: usize = 32;

const DEBUG: bool = true;

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
    Register(u8),

    Comma,
    Colon,
}

impl TokenKind {
    /* #[inline]
    fn description(&self, with_article: bool) -> &str {
        use TokenKind::*;

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

type MacrosMap = HashMap<String, TokenKind>;

#[inline]
fn insert_trubeta_macros(macros: &mut MacrosMap) {
    macro_rules! add_macro {
        ($str:expr, $kind:ident, $value:expr) => {
            macros.insert(String::from($str), TokenKind::$kind($value))
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

#[derive(PartialEq, Debug, Clone)]
struct Token {
    pub kind: TokenKind,
}

enum KValue {
    Number(f32),
    Address(u32),
    String(String)
}

struct Class {
    pub code: Vec<Instruction>,
    pub boot_start: u32,
    pub constants: Vec<KValue>
}

#[derive(Debug)]
#[repr(u8)]
pub enum ErrorKind {
    UnrecognizedCharacter,
    NumberOverflow,
    TooLongIdentifier,
    RedundantDot,
    UnclosedString,
    UnexpectedTokens,
    ExpectedTokens,
    ExpectedArgument,
    MalformedNumber,
    InvalidOpcode,
    ReachedCodeLengthLimit,
    LabelRedefinition,
    MacroRedefinition,
    FullConstantPool,
    InstructionBeforeLabel,
    BootNotDefined
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

        macro_rules! is_string_quote {
            ($ch:expr) => {
                ($ch == '"' || $ch == '\'')
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
            let is_string: bool = is_string_quote!(char);
            let is_line_comment: bool = char == '#';

            let mut new_token: Option<Token> = None;

            if is_identifier {
                let mut ident: String = String::with_capacity(MAX_IDENTIFIER_LENGTH);

                ident.push(char);

                while let Some(ch) = chars.peek() {
                    let ch = *ch;

                    if ident.len() >= MAX_IDENTIFIER_LENGTH {
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

                let token = Token { kind: TokenKind::Identifier(ident) };
                new_token = Some(token);
            }

            if is_digit {
                let mut num_a: u32 = to_digit!(char);
                let mut num_b: u32 = 0;
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
                            num_op!(num_b, checked_mul, 10);
                            num_op!(num_b, checked_add, digit);
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

                let token = Token { kind: TokenKind::Number(num) };
                new_token = Some(token);
            }

            if is_string {
                let mut string: String = String::new();
                let mut closed = false;

                while let Some(char) = next_char!() {
                    if is_string_quote!(char) {
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

            if is_line_comment {
                break;
            }

            if new_token.is_none() {
                let kind = match char {
                    ',' => Some(TokenKind::Comma),
                    ':' => Some(TokenKind::Colon),

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

        tokens = self.parse_numbers(tokens, line_number)?;

        Ok(tokens)
    }

    fn parse_numbers(&self, tokens: Vec<Token>, line_number: u32) -> Result<Vec<Token>, ErrorKind> {
        macro_rules! print_error {
            ($err_kind:ident, $msg:expr) => {{
                eprintln!("{}: line {}:", self.filename, line_number);
                eprintln!("  error: {}", $msg);
                return Err(ErrorKind::$err_kind);
            }};
        }

        let mut iter = tokens.into_iter().peekable();
        let mut tokens: Vec<Token> = Vec::new();

        while let Some(token) = iter.next() {
            let prev_kind = token.kind;
            let kind: TokenKind = if let TokenKind::Identifier(ident) = prev_kind {
                // если текущий токен это идент, то проверяем его значение
                use TokenKind::*;
                match ident.as_str() {
                    // регистр
                    "r" => {
                        // если есть следующий токен, и это число:
                        if let Some(nt) = iter.next() &&
                        let TokenKind::Number(num) = nt.kind {
                            let index_32: u32 = if num % 1. == 0. {
                                num as u32
                            } else {
                                print_error!(MalformedNumber, "register index with floating point");
                            };

                            let index_8: u8 = if let Ok(num) = index_32.try_into() {
                                num
                            } else {
                                print_error!(MalformedNumber, "register index out of bounds");
                            };

                            if index_8 < MAX_REGISTERS_COUNT as u8 {
                                Register(index_8)
                            } else {
                                print_error!(MalformedNumber, "register index out of bounds");
                            }
                        } else {
                            print_error!(ExpectedTokens, "expected a register index after 'r' label");
                        }
                    }

                    // адрес
                    "a" => {
                        // если есть следующий токен, и это число:
                        if let Some(nt) = iter.next() &&
                        let TokenKind::Number(num) = nt.kind {
                            let address: u32 = if num % 1. == 0. {
                                num as u32
                            } else {
                                print_error!(MalformedNumber, "address with floating point");
                            };

                            Address(address)
                        } else {
                            print_error!(ExpectedTokens, "expected an address after 'r' label");
                        }
                    }

                    // в остальных случаях оставляем как есть
                    _ => Identifier(ident)
                }
            } else {
                // если это не идент, то оставляем как есть
                prev_kind
            };

            tokens.push(Token { kind });
        }

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
            use TokenKind::*;
            let kind = match token.kind {
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

                _ => token.kind
            };

            let new_token = Token { kind };

            final_tokens.push(new_token);
        }

        Ok(final_tokens)
    }

    pub fn unwrap_macros(&self, tokens: Vec<Token>, macros: &MacrosMap) -> Result<Vec<Token>, ErrorKind> {
        let mut result: Vec<Token> = Vec::new();

        for gtoken in tokens {
            let token = if let TokenKind::Identifier(ident) = &gtoken.kind
            && let Some(mtk) = macros.get(ident) {
                Token { kind: mtk.clone() }
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
        let input_lines = reader.lines().map(|line| line.unwrap());

        for (line_number, line) in (1u32..).zip(input_lines) {
            macro_rules! print_error {
                ($err_kind:ident, $msg:expr) => {{
                    eprintln!("{}: line {}:", self.filename, line_number);
                    eprintln!("  error: {}", $msg);
                    return Err(ErrorKind::$err_kind);
                }};
            }

            let generated_tokens: Vec<Token> = self.generate_tokens(line, line_number)?;
            let unmacrofied_tokens: Vec<Token> = self.unwrap_macros(generated_tokens, &macros)?;
            let tokens: Vec<Token> = self.unwrap_constants(&mut constants, unmacrofied_tokens, line_number)?;

            let first_token = if let Some(tok) = tokens.first() {
                tok
            } else {
                // the Vec is empty
                continue;
            };

            let mut args: Vec<Token> = Vec::new();

            if let Some(tokens) = tokens.get(1..) {
                for slice in tokens.split(|tok| tok.kind == TokenKind::Comma) {
                    if slice.len() == 1 {
                        args.push(slice[0].clone());
                    } else {
                        // mov $0 $1, $2
                        //       ^^^ error
                        print_error!(
                            UnexpectedTokens,
                            "multiple tokens in an argument. did you forget inserting a comma?"
                        );
                    }
                }
            }

            use TokenKind::*;
            use InstructionKind as Instr;
            match &first_token.kind {
                // INSTRUCTIONS ONLY!!
                Identifier(ident) => {
                    if labels.is_empty() {
                        print_error!(InstructionBeforeLabel, "a label must be declared before any instructions");
                    }

                    let ident = ident.to_lowercase();
                    let ident: &str = ident.as_str();

                    macro_rules! get_arg {
                        ($pos:expr, $tok_kind:ident) => {
                            match args.get($pos) {
                                Some(tok) => if let TokenKind::$tok_kind(value) = tok.kind {
                                    Some(value)
                                } else {
                                    None
                                },

                                None => None
                            }
                        };
                    }

                    macro_rules! mathop3 {
                        ($kind:ident) => {{
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

                    let kind = match ident {
                        "nop" => { Instr::Nop }

                        "mov" => {
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
                            let reg: u8 = if let Some(value) = get_arg!(0, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #0");
                            };

                            Instr::Test { reg }
                        }

                        "goto" => {
                            /* let pos: u32 = if let Some(token) = args.get(0) {
                                match token.kind {
                                    Identifier(label_name) => {
                                        todo!()
                                    }

                                    Address(addr) => {
                                        if addr < MAX_GOTO_LENGTH {
                                            addr
                                        } else {
                                            print_error!(MalformedNumber, "instruction address out of bounds");
                                        }
                                    }

                                    _ => {
                                        print_error!(ExpectedArgument, "expected an instruction address or a label at argument #0");
                                    }
                                }
                            } else {
                                print_error!(ExpectedArgument, "expected an instruction address or a label at argument #0");
                            }; */

                            let pos: u8 = if let Some(value) = get_arg!(0, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #0");
                            };

                            Instr::Goto { pos }
                        }

                        "gotok" => {
                            let kaddr32: u32 = if let Some(value) = get_arg!(0, Address) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a constant address at argument #1");
                            };

                            let kaddr: u16 = if let Ok(num) = kaddr32.try_into() {
                                num
                            } else {
                                print_error!(MalformedNumber, "constant address out of bounds");
                            };

                            Instr::GotoK { kaddr }
                        }

                        "call" => {
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
                            let reg: u8 = if let Some(value) = get_arg!(0, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #0");
                            };

                            Instr::NewCont { reg }
                        }

                        "getcont" => {
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
                            let cont: u8 = if let Some(value) = get_arg!(0, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #0");
                            };

                            Instr::DropCont { cont }
                        }

                        "int" => {
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

                        "retv" => Instr::RetV,

                        "retk" => {
                            let kaddr32: u32 = if let Some(value) = get_arg!(0, Address) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a constant address at argument #1");
                            };

                            let kaddr: u16 = if let Ok(num) = kaddr32.try_into() {
                                num
                            } else {
                                print_error!(MalformedNumber, "constant address out of bounds");
                            };

                            Instr::RetK { kaddr }
                        }

                        "ret" => {
                            let reg: u8 = if let Some(value) = get_arg!(0, Register) {
                                value
                            } else {
                                print_error!(ExpectedArgument, "expected a register index at argument #0");
                            };

                            Instr::Ret { reg }
                        }

                        "halt" => Instr::Halt,

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
                    let name = if let Some(token) = tokens.get(1)
                    && let Identifier(ident) = &token.kind {
                        ident.clone()
                    } else {
                        print_error!(UnexpectedTokens, "expected an identifier after colon");
                    };

                    let is_boot_label = name == "boot";

                    // проверяем, занят (определён) ли ключ в `labels`.
                    match labels.entry(name.clone()) {
                        // ключ занят.
                        Entry::Occupied(_) => {
                            print_error!(LabelRedefinition, "duplicate label");
                        }

                        // ключ свободен. теперь проверям в макросах.
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

    let class = assembler.make_class_from_file(file)?;

    if DEBUG {
        println!("--------- INSTRUCTIONS ---------");

        for (index, instr) in class.code.iter().enumerate() {
            println!("[{}] {:#?}", index, instr.kind);
        }

        println!("--------------------------------\n");
    }

    // Ok(bytecode)
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! token {
        ($kind:ident) => {
            Token {
                kind: TokenKind::$kind
            }
        };

        ($kind:ident, $value:expr) => {
            Token {
                kind: TokenKind::$kind($value)
            }
        };
    }

    #[test]
    fn test_token_generator() {
        let input = "ident1 123.45, ident2, $567";

        let expected = vec![
            token!(Identifier, String::from("ident1")),
            token!(Number, 123.45),
            token!(Comma),
            token!(Identifier, String::from("ident2")),
            token!(Comma),
            token!(Address, 567),
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
