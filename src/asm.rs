use std::collections::HashMap;
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

#[derive(PartialEq, Debug, Clone)]
struct Token {
    pub kind: TokenKind,
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

                    /* '$' => {
                        if let Some(char) = next_char!() && char.is_ascii_digit() {
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
                                        print_error!(DotInAddress, "dots cannot appear in addresses, as they cannot be float");
                                    } else {
                                        break;
                                    }
                                }
                            }

                            Some(TokenKind::Address(num))
                        } else {
                            print_error!(DollarWithoutAddress, "dollar sign without a following address");
                        }
                    } */

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

    pub fn generate_instructions(&self, file: File) -> Result<Vec<Instruction>, ErrorKind> {
        let mut code: Vec<Instruction> = Vec::new();

        let mut macros: HashMap<String, TokenKind> = HashMap::new();

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

            let mut unmacrofied_tokens: Vec<Token> = Vec::new();

            for gtoken in generated_tokens {
                let token = if let TokenKind::Identifier(ident) = &gtoken.kind
                && let Some(mtk) = macros.get(ident) {
                    Token {
                        kind: mtk.clone(),
                        // other_field: gtoken.other_field
                    }
                } else {
                    gtoken
                };

                unmacrofied_tokens.push(token);
            }

            let tokens: Vec<Token> = unmacrofied_tokens;

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
                Identifier(ident) => {
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

                    // additional checks?

                    let instr = Instruction { kind };

                    code.push(instr);
                }

                Colon => {}

                _ => {
                    print_error!(UnexpectedTokens, "expected an identifier or a colon");
                }
            }
        }

        Ok(code)
    }
}

pub fn compile_from_file(file: File, filename: String) -> Result<Vec<u8>, ErrorKind> {
    let assembler = Assembler::new(filename);

    let instrs = assembler.generate_instructions(file)?;

    if DEBUG {
        println!("--------- INSTRUCTIONS ---------");

        for (index, instr) in instrs.iter().enumerate() {
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
