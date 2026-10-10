use std::fmt::format;
use std::fs::File;
use std::io::{BufRead, BufReader};

use crate::instr::InstructionKind;

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

    Comma,
    Colon,
}

impl TokenKind {
    #[inline]
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
    }
}

#[derive(PartialEq, Debug)]
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
    DotInAddress,
    UnclosedString,
    DollarWithoutAddress,
    UnexpectedTokens,
}

struct Assembler {
    pub filename: String
}

#[inline]
fn unexpected_token_message(unexpected: TokenKind, expected: &[TokenKind]) -> String {
    let mut msg: String;

    if expected.is_empty() {
        msg = format!(
            "unexpected {}",
            unexpected.description(false)
        );
    } else {
        let mut iter = expected.iter().enumerate();
        let (_, first) = iter.next().unwrap();

        msg = format!(
            "unexpected {}, expected {}",
            unexpected.description(false),
            first.description(true)
        );

        for (pos, kind) in iter {
            let connector = if pos + 1 == expected.len() {
                " or "
            } else {
                ", "
            };

            msg += format!("{connector} {}", kind.description(true)).as_str();
        }
    }

    msg
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

                    '$' => {
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

        Ok(tokens)
    }

    pub fn generate_instructions(&self, file: File) -> Result<Vec<Instruction>, ErrorKind> {
        let mut code: Vec<Instruction> = Vec::new();

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

            let tokens: Vec<Token> = self.generate_tokens(line, line_number)?;

            let first_token = if let Some(tok) = tokens.first() {
                tok
            } else {
                // the Vec is empty
                continue;
            };

            use TokenKind::*;
            match &first_token.kind {
                Identifier(ident) => {}

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

    #[test]
    fn test_token_generator() {
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
}
