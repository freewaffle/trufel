use std::fs::File;
use std::io::{BufRead, BufReader};

use crate::vm;

const MAX_IDENTIFIER_LENGTH: usize = 32;
const MAX_EXPRESSION_DEPTH: u8 = 128;

const DEBUG: bool = true;

const POSSIBLE_OPS: [TokenKind; 5] = [
    TokenKind::Equality,
    TokenKind::Add,
    TokenKind::Sub,
    TokenKind::Mul,
    TokenKind::Div
];

#[derive(PartialEq, Debug, Clone)]
#[repr(u8)]
enum TokenKind {
    Identifier(String),
    DoubleIdentifier(String, String),
    String(String),
    Number(f32),

    InlineFunctionCall {
        name: String,
        args: Vec<Vec<Token>>
    },

    Add,
    Sub,
    Mul,
    Div,

    Equality,
    DoubleEquality,

    Comma,
    Colon,
    OpenParen,
    ClosedParen
}

impl TokenKind {
    /* #[inline]
    fn description(&self) -> &str {
        use TokenKind::*;
        match self {
            Identifier(..) => "an identifier",
            DoubleIdentifier(..) => "a typed identifier `name:type`",
            String(..) => "a string literal",
            Number(..) => "a number",

            InlineFunctionCall { .. } => "an inline function call",

            Add => "an addition symbol '+'",
            Sub => "a substraction symbol '-'",
            Mul => "a multiplication symbol '*'",
            Div => "a division symbol '/'",

            Equality => "an equality symbol '='",
            DoubleEquality => "a double equality symbol '=='",

            Comma => "a comma ','",
            Colon => "a colon ':'",
            OpenParen => "an open parenthesis '('",
            ClosedParen => "a closed parenthesis ')'",
        }
    } */
}

#[derive(PartialEq, Debug, Clone)]
struct Token {
    pub line_pos: usize,
    pub kind: TokenKind,
}

#[repr(u8)]
#[derive(Debug, Clone)]
enum VariableType {
    Number
}

impl VariableType {
    #[inline]
    pub fn from_string(string: String) -> Option<Self> {
        use VariableType::*;
        match string.as_str() {
            "number" => Some(Number),
            _ => None
        }
    }
}

#[derive(Debug, Clone)]
struct TypedIdentifier {
    pub ident: String,
    pub ty: VariableType
}

#[repr(u8)]
#[derive(Debug, Clone)]
enum CommandKind {
    FunctionHeader {
        name: String,
        args: Vec<TypedIdentifier>,
        return_type: String
    },
    FunctionCall {
        name: String,
        args: Vec<Vec<Token>>
    },
    Return {
        expr: Vec<Token>
    },
    End,
    VariableDecl {
        shadowing: bool,
        name: TypedIdentifier,
        op: TokenKind,
        expr: Vec<Token>
    },
    VariableUpdate {
        name: String,
        op: TokenKind,
        expr: Vec<Token>
    },
}

#[derive(Clone)]
struct Command {
    pub kind: CommandKind,
    pub line_pos: usize
}

#[derive(Debug)]
#[repr(u8)]
enum InstructionKind {
    Nop,

    /// `r(r0) = r(r1)`
    Mov { r0: u8, r1: u8 },
    /// `r(reg) = const(cn)`
    MovC { reg: u8, cn: u16 },
    /// `r(reg) = void`
    MovV { reg: u8 },

    /// `r(r0) = r(r1) + r(r2)`
    Add { r0: u8, r1: u8, r2: u8 },
    /// `r(r0) = r(r1) - r(r2)`
    Sub { r0: u8, r1: u8, r2: u8 },
    /// `r(r0) = r(r1) * r(r2)`
    Mul { r0: u8, r1: u8, r2: u8 },
    /// `r(r0) = r(r1) / r(r2)`
    Div { r0: u8, r1: u8, r2: u8 },
    /// `r(r0) = r(r1) % r(r2)`
    Mod { r0: u8, r1: u8, r2: u8 },
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
    
    Test { reg: u8 },
    Jmp  { pos: u16 },
    Call { pos: u16 },

    NewSpr { r: u8, xsize: u8, ysize: u8 },
    /// `s(spr)[r(pos)] = r(value)`
    SetSpr { spr: u8, pos: u8, value: u8},
    
    Int,
    Ret,
    Halt,
}

struct Instruction {
    pub kind: InstructionKind,
}

struct FunctionPrototype {
    pub name: String,
    pub args: Vec<TypedIdentifier>,
    pub body: Vec<Command>,
    pub return_type: String
}

#[repr(u8)]
enum Register {
    Free,
    Variable {
        name: String,
        ty: VariableType,
    },
}

#[repr(u8)]
pub enum ErrorKind {
    UnrecognizedCharacter,
    IntegerOverflow,
    TooLongIdentifier,
    RedundantDot,
    UnclosedString,
    ExpectedTokens,
    RedundantTokens,
    EmptyExpression,
    DuplicateName,
    NestedFunctionDefinition,
    UnclosedFunction,
    UnknownType,
    FreeRegisterNotFound
}

struct Compiler {
    pub filename: String
}

impl Compiler {
    #[inline]
    pub fn new(filename: String) -> Self {
        Self {
            filename
        }
    }

    pub fn parse_line(&self, line_str: String, line_pos: usize) -> Result<Vec<Token>, ErrorKind> {
        let mut chars = line_str.chars().peekable();
        let mut line: Vec<Token> = Vec::new();

        macro_rules! print_error {
            ($err_kind:ident, $msg:expr) => {{
                eprintln!("[{}]: line {}:", self.filename, line_pos);
                eprintln!("  error: {}", $msg);
                return Err(ErrorKind::$err_kind);
            }};
        }

        macro_rules! to_digit {
            ($ch:ident) => {
                $ch.to_digit(10).unwrap()
            };
        }

        macro_rules! num_op {
            ($var:ident, $func:ident, $func_expr:expr) => {
                if let Some(result) = $var.$func($func_expr) {
                    $var = result;
                } else {
                    print_error!(IntegerOverflow, "integer overflow");
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

        macro_rules! next_char {
            () => {{
                chars.next()
            }};
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

        // convenience macro that automatically fills `line_pos` field
        macro_rules! token {
            ($kind:ident) => {
                Token {
                    line_pos,
                    kind: TokenKind::$kind,
                }
            };

            ($kind:ident, $expr:expr) => {
                Token {
                    line_pos,
                    kind: TokenKind::$kind($expr),
                }
            };
        }

        // `for ch in chars` takes ownership
        while let Some(ch) = next_char!() {
            if ch.is_ascii_whitespace() {
                continue
            }

            let identifier: bool = is_identifier_token!(ch);
            let number: bool = ch.is_ascii_digit();
            let string: bool = is_string_token!(ch);
            let line_comment: bool = ch == '#';

            let mut new_token: Option<Token> = None;

            if identifier {
                let ident: String = collect_identifier!(ch);
                let token = token!(Identifier, ident);
                new_token = Some(token);
            }

            if number {
                let mut num_a: u32 = to_digit!(ch);
                let mut num_b: u32 = 0;
                let mut collecting_a = true;

                while let Some(ch) = chars.peek() {
                    let ch = *ch;

                    if !ch.is_ascii_digit() {
                        if ch == '.' {
                            if collecting_a {
                                collecting_a = false;
                                chars.next();
                                continue;
                            } else {
                                print_error!(RedundantDot, "redundant dot near number");
                            }
                        } else {
                            break;
                        }
                    }

                    let digit = to_digit!(ch);
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

                let token = token!(Number, num);
                new_token = Some(token);
            }

            if string {
                let mut string: String = String::new();
                let mut closed = false;

                while let Some(ch) = next_char!() {
                    if is_string_token!(ch) {
                        closed = true;
                        break;
                    }

                    string.push(ch);
                }

                if closed {
                    let token = token!(String, string);
                    new_token = Some(token);
                } else {
                    print_error!(UnclosedString, "unclosed string");
                }
            }

            if line_comment {
                break;
            }

            if new_token.is_none() {
                let kind = match ch {
                    '+' => Some(TokenKind::Add),
                    '-' => Some(TokenKind::Sub),
                    '*' => Some(TokenKind::Mul),
                    '/' => Some(TokenKind::Div),

                    '=' => {
                        if chars.peek().is_some_and(|ch| *ch == '=') {
                            next_char!();
                            Some(TokenKind::DoubleEquality)
                        } else {
                            Some(TokenKind::Equality)
                        }
                    }

                    ',' => Some(TokenKind::Comma),

                    ':' => {
                        let mut remove_last_token = false;

                        let tok = if chars.peek().is_some_and(|ch| is_identifier_token!(*ch)) {
                            if let Some(token) = line.last()
                            && let TokenKind::Identifier(left_ident) = &token.kind {
                                remove_last_token = true;
                                let ch = next_char!().unwrap();
                                let right_ident = collect_identifier!(ch);
                                TokenKind::DoubleIdentifier(left_ident.to_owned(), right_ident)
                            } else {
                                TokenKind::Colon
                            }
                        } else {
                            TokenKind::Colon
                        };

                        if remove_last_token {
                            line.pop();
                        }

                        Some(tok)
                    },

                    '(' => Some(TokenKind::OpenParen),
                    ')' => Some(TokenKind::ClosedParen),

                    // whitespaces and tabs are skipped in the beginning
                    _ => None
                };

                if let Some(kind) = kind {
                    new_token = Some(Token {
                        line_pos,
                        kind,
                    });
                } // else None
            }

            if let Some(tok) = new_token {
                line.push(tok);
            } else {
                print_error!(UnrecognizedCharacter, format!(
                    "unrecognized character: '{}'",
                    ch
                ));
            }
        }

        Ok(line)
    }

    pub fn parse_file(&self, file: File) -> Result<Vec<Vec<Token>>, ErrorKind> {
        let reader = BufReader::new(file);
        let input_lines = reader.lines().map(|line| line.unwrap());

        let mut lines: Vec<Vec<Token>> = Vec::new();
        
        for (current_line, input_line) in (1..).zip(input_lines) {
            match self.parse_line(input_line, current_line) {
                Ok(line) => {
                    lines.push(line);
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }

        Ok(lines)
    }

    fn check_expression(&self, expr: &[Token], line_pos: usize) -> Result<(), ErrorKind> {
        if DEBUG {
            println!("[check_expression] >>> {expr:#?}");
        }

        macro_rules! print_error {
            ($err_kind:ident, $msg:expr) => {{
                eprintln!("[{}]: line {}:", self.filename, line_pos);
                eprintln!("  error: {}", $msg);

                if DEBUG {
                    eprintln!("[check_expression] ERROR");
                }

                return Err(ErrorKind::$err_kind);
            }};
        }

        if expr.is_empty() {
            /*
                we are checking expression emptyness outside of this function where
                it's need to, so this error isn't required:

                print_error!(EmptyExpression, "empty expression");
            */
            
            return Ok(());
        }

        /*
            expecting...
            if true:  number, identifier or open paren
            if false: operator or closed paren
        */
        let mut expecting_number = true;

        macro_rules! invert {
            () => {
                expecting_number = !expecting_number;
            };
        }

        macro_rules! malformed {
            () => {{
                let what = if expecting_number {
                    "number, identifier, function call, literal string or open paren"
                } else {
                    "operator or closed paren"
                };
                print_error!(ExpectedTokens, format!("malformed expression: expected {what}"));
            }};
        }

        macro_rules! assert_malformed {
            ($cond:expr) => {
                if $cond {
                    invert!();
                } else {
                    malformed!();
                }
            };
        }
        
        /*
            increments on open paren,
            decrements on closed paren
        */
        let mut paren_counter: u8 = 0;
        
        for token in expr {
            use TokenKind::*;

            // 1. check for malformness
            match token.kind {
                Number(..) | Identifier(..) | InlineFunctionCall { .. } | String(..) => {
                    assert_malformed!(expecting_number);
                }

                OpenParen => {
                    // same as `assert_malformed!`, but without `invert!`
                    if !expecting_number {
                        malformed!();
                    }
                }

                ClosedParen => {
                    // same as `assert_malformed!`, but without `invert!`
                    if expecting_number {
                        malformed!();
                    }
                }

                Add | Sub | Mul | Div | DoubleEquality => {
                    assert_malformed!(!expecting_number);
                }

                _ => {
                    malformed!();
                }
            }

            // 2. count parenthesis
            match token.kind {
                OpenParen => {
                    if let Some(sum) = paren_counter.checked_add(1)
                    && sum < MAX_EXPRESSION_DEPTH {
                        paren_counter = sum;
                    } else {
                        print_error!(ExpectedTokens, "too many nested parentheses");
                    }
                }

                ClosedParen => {
                    if let Some(diff) = paren_counter.checked_sub(1) {
                        paren_counter = diff;
                    } else {
                        print_error!(ExpectedTokens, "unmatched closed paren ')'");
                    }
                }

                _ => {}
            }
        }

        if paren_counter > 0 {
            print_error!(ExpectedTokens, "missing closed parentheses ')'");
        }
        
        if expecting_number {
            malformed!();
        }

        if DEBUG {
            eprintln!("[check_expression] OK");
        }
        
        Ok(())
    }

    fn calculate_depths(&self, expr: &[Token], line_pos: usize) -> Vec<u8> {
        let mut error: Option<ErrorKind> = None;

        macro_rules! try_set_error {
            ($kind:ident) => {
                if error.is_none() {
                    error = Some(ErrorKind::$kind);
                }
            };
        }

        macro_rules! print_error {
            ($err_kind:ident, $msg:expr) => {{
                eprintln!("[{}]: line {}:", self.filename, line_pos);
                eprintln!("  error: {}", $msg);
                try_set_error!($err_kind);
            }};
        }

        if expr.is_empty() {
            print_error!(EmptyExpression, "empty expression");
        }
        
        let mut depth_stack: Vec<u8> = Vec::new();

        let mut peeker = expr.iter().peekable();

        let mut last_depth: u8 = 0;
        let mut paren_stack: Vec<u8> = Vec::new();

        macro_rules! get_last_paren_counter {
            () => {
                paren_stack.last_mut().expect("paren counter must exist")
            };
        }
        
        /*
            `paren_stack` помогает определять, где конкретно заканчивается IFC.
            после каждого `I(` создаётся новый счетчик через `push`, и самый
            последний счётчик считается активным.

            пример:
                FUNC( 1 + (3 * 3) )
            PC: ____0_____1_____0_X
        */

        while let Some(token) = peeker.next() {
            let parsing_ifc = !paren_stack.is_empty();

            macro_rules! push_ifc {
                () => {
                    last_depth += 1;
                    depth_stack.push(last_depth);

                    paren_stack.push(0);

                    // skip OpenParen
                    peeker.next();
                    depth_stack.push(0);

                    continue;
                };
            }

            macro_rules! pop_ifc {
                () => {
                    last_depth -= 1;
                    depth_stack.push(last_depth);

                    paren_stack.pop();

                    continue;
                };
            }
            
            use TokenKind::*;
            match &token.kind {
                OpenParen if parsing_ifc => {
                    let paren_counter = get_last_paren_counter!();

                    if let Some(sum) = paren_counter.checked_add(1)
                    && sum < MAX_EXPRESSION_DEPTH {
                        *paren_counter = sum;
                    } else {
                        print_error!(ExpectedTokens, "too many nested parentheses");
                    }
                }

                ClosedParen if parsing_ifc => {
                    let paren_counter = get_last_paren_counter!();

                    if let Some(diff) = paren_counter.checked_sub(1) {
                        *paren_counter = diff;
                    } else {
                        pop_ifc!();
                    }
                }

                Identifier(..)
                    if peeker.peek().is_some_and(|tok| tok.kind == OpenParen) => {
                        push_ifc!();
                    }

                _ => {}
            }

            // default depth (no IFC)
            depth_stack.push(0);
        }

        depth_stack
    }

    fn collect_expr(&self, tokens: &[Token], line_pos: usize) -> Result<Vec<Token>, ErrorKind> {
        macro_rules! print_error {
            ($err_kind:ident, $msg:expr) => {{
                eprintln!("[{}]: line {}:", self.filename, line_pos);
                eprintln!("  error: {}", $msg);
                return Err(ErrorKind::$err_kind);
            }};
        }
        
        let mut expr: Vec<Token> = tokens.to_vec();
                    
        /* for token in tokens {
            expr.push(token.clone());
        } */

        /* if expr.is_empty() {
            print_error!(EmptyExpression, "empty expression");
        } */
        
        /*
            стек глубины `depth_stack` имеет одинаковый размер с `expr`
            и показывает, в каких местах начинается IFC.

            допустимые значения глубины:
                0   : нет IFC
                1.. : больше = глубже
            
            сначала обрабатываются самые глубокие IFC (максимально
            достигнутая глубина берётся из `max_depth`).

            пример:
                FUNC( 1 + (3 * 3) + SOMETHING() )
            D:  1   0 0 0 00 0 00 0 2           0
        */
        
        let mut depth_stack: Vec<u8> = self.calculate_depths(&expr, line_pos);

        let mut max_depth: u8 = 0;

        for depth in depth_stack.iter() {
            let depth = *depth;
            if depth > max_depth {
                max_depth = depth;
            }
        }

        let has_ifcs = max_depth > 0;

        if DEBUG && has_ifcs {
            println!("[{line_pos}]:");

            print!("  depth: ");
            for depth in depth_stack.iter() {
                print!("{:?}, ", depth);
            }
            println!();
        }

        assert_eq!(depth_stack.len(), expr.len(), "`depth_stack` and `expr` must have equal lengths");

        if has_ifcs {
            /*
                A: read-only
                B: mutable
            */
            
            let mut expr_a: Vec<Token> = expr.clone();
            let mut expr_b: Vec<Token> = Vec::new();

            for target_depth in (1..=max_depth).rev() {
                let mut peeker = depth_stack.iter().peekable();
                let mut dpos: usize = 0;

                macro_rules! next {
                    () => {{
                        let next = peeker.next();
                        if next.is_some() {
                            dpos += 1;
                        }
                        next
                    }};
                }

                macro_rules! pos {
                    () => {
                        dpos.checked_sub(1).unwrap_or(0)
                    }
                }

                macro_rules! get_token_at_dpos {
                    () => {
                        expr_a[pos!()].clone()
                    }
                }

                while let Some(depth) = next!() {
                    let token = get_token_at_dpos!();

                    if *depth == target_depth {
                        let name = if let TokenKind::Identifier(ident) = token.kind {
                            ident
                        } else {
                            // this is parser's fault, so it's better to panic, i think
                            panic!("depth marker not pointing to an Identifier (got {:?})", token.kind);
                        };

                        let mut args: Vec<Vec<Token>> = vec![
                            Vec::new()
                        ];

                        /*
                            increments on open paren,
                            decrements on closed paren
                        */
                        let mut paren_counter: u8 = 0;

                        // skip OpenParen
                        next!();

                        while next!().is_some() {
                            let expr = args.last_mut().unwrap();
                            let token = get_token_at_dpos!();

                            use TokenKind::*;
                            match token.kind {
                                OpenParen => {
                                    if let Some(sum) = paren_counter.checked_add(1)
                                    && sum < MAX_EXPRESSION_DEPTH {
                                        paren_counter = sum;
                                    } else {
                                        print_error!(ExpectedTokens, "too many nested parentheses");
                                    }
                                }

                                ClosedParen => {
                                    if let Some(diff) = paren_counter.checked_sub(1) {
                                        paren_counter = diff;
                                    } else {
                                        break;
                                    }
                                }

                                Comma => {
                                    args.push(Vec::new());
                                    continue;
                                }

                                _ => {}
                            }

                            expr.push(token);
                        }

                        for expr in args.iter() {
                            if expr.is_empty() {
                                print_error!(EmptyExpression, "empty expression");
                            }

                            self.check_expression(expr, line_pos)?;
                        }

                        let ifc = Token {
                            kind: TokenKind::InlineFunctionCall {
                                name, args
                            },
                            line_pos: token.line_pos
                        };

                        expr_b.push(ifc);
                    } else {
                        expr_b.push(token);
                    }
                }

                expr_a = expr_b;
                expr_b = Vec::new();

                depth_stack = self.calculate_depths(&expr_a, line_pos);
                assert_eq!(depth_stack.len(), expr_a.len(), "`depth_stack` and `expr_a` must have equal lengths");
            }

            expr = expr_a;
        }

        if DEBUG && has_ifcs {
            println!("[{line_pos}]:");

            print!("  expr: ");
            for token in expr.iter() {
                print!("{:#?}, ", token.kind);
            }
            println!();
        }

        self.check_expression(&expr, line_pos)?;

        Ok(expr)
    }

    pub fn parse_tokens(&self, tokens: Vec<Vec<Token>>) -> Result<Vec<Command>, ErrorKind> {
        let mut commands: Vec<Command> = Vec::new();
        
        'tokens: for tokens in tokens {
            if tokens.is_empty() {
                continue 'tokens;
            }

            let first_token = tokens.first().unwrap();
            let line_pos = first_token.line_pos;

            let new_command: Option<Command>;

            macro_rules! print_error {
                ($err_kind:ident, $msg:expr) => {{
                    eprintln!("[{}]: line {}:", self.filename, line_pos);
                    eprintln!("  error: {}", $msg);
                    return Err(ErrorKind::$err_kind);
                }};
            }

            macro_rules! get_token_value {
                ($index:expr, $kind:ident) => {
                    match tokens.get($index) {
                        Some(Token {
                            kind: TokenKind::$kind(ident),
                            ..
                        }) => Some(ident),
                        _ => None,
                    }
                };
            }

            macro_rules! redundant_tokens_error {
                () => {
                    print_error!(RedundantTokens, "redundant tokens");
                };
            }

            macro_rules! command {
                ($kind:expr) => {
                    Command {
                        kind: $kind,
                        line_pos
                    }
                };
            }

            macro_rules! collect_expr {
                ($tokens:expr) => {{
                    match self.collect_expr($tokens, line_pos) {
                        Ok(vec) => vec,
                        Err(err) => {
                            return Err(err);
                        }
                    }
                }};
            }

            macro_rules! try_type_from_string {
                ($str:expr) => {
                    if let Some(ty) = VariableType::from_string($str) {
                        ty
                    } else {
                        print_error!(UnknownType, format!("unknown type `{}`", $str));
                    }
                };
            }

            if let TokenKind::Identifier(ident) = &first_token.kind {
                // as_str() shouldn't clone string, looks like it just
                // does nothing but changes the type.

                // don't be afraid of `clone`: the `compile_from_file` function,
                // which is preferred compilation way, transfers ownership of
                // produced tokens to this function, dropping them after this
                // function finishes.

                let ident = ident.as_str();

                new_command = match ident {
                    "func" => {
                        let name = if let Some(ident) = get_token_value!(1, Identifier) {
                            ident.clone()
                        } else {
                            print_error!(ExpectedTokens, "expected identifier");
                        };

                        let mut args: Vec<TypedIdentifier> = Vec::new();
                        
                        if tokens.get(2).is_some_and(|tok|tok.kind != TokenKind::OpenParen) {
                            print_error!(ExpectedTokens, "expected open paren '('");
                        }

                        let mut left_tokens = tokens.get(3..).unwrap().iter();
                        let mut redundant_pos: usize = 3;
                        let mut has_closed_paren = false;

                        macro_rules! next_token {
                            () => {{
                                redundant_pos += 1;
                                left_tokens.next()
                            }};
                        }

                        while let Some(token) = next_token!() {
                            match &token.kind {
                                TokenKind::ClosedParen => {
                                    has_closed_paren = true;
                                    break;
                                }
                                TokenKind::DoubleIdentifier(ident, ty) => {
                                    let arg = TypedIdentifier {
                                        ident: ident.clone(),
                                        ty: try_type_from_string!(ty.clone())
                                    };

                                    args.push(arg);

                                    if next_token!().is_some_and(|tok| tok.kind != TokenKind::Comma) {
                                        print_error!(ExpectedTokens, "expected comma ',' after argument");
                                    }
                                }
                                _ => {
                                    print_error!(ExpectedTokens, "expected typed identifier `name:type`");
                                }
                            }
                        }

                        if !has_closed_paren {
                            print_error!(ExpectedTokens, "expected closed paren ')' after arguments list");
                        }

                        let return_type: String = if let Some(token) = next_token!()
                        && let TokenKind::Identifier(ident) = &token.kind {
                            ident.clone()
                        } else {
                            print_error!(ExpectedTokens, "expected return type");
                        };

                        // checking for redundant tokens
                        {
                            // decrementing length, because we check it with an index,
                            // which starts from 0
                            let len = tokens.len().saturating_sub(1);
                            let pos = redundant_pos - 1;
                            let diff = len.checked_sub(pos);
                            if diff.is_some_and(|diff| diff > 0) {
                                redundant_tokens_error!();
                            }
                        }

                        Some(command!(CommandKind::FunctionHeader { name, args, return_type }))
                    }

                    "end" => {
                        // `end` doesn't have arguments at all
                        if tokens.len() > 1 {
                            redundant_tokens_error!();
                        }

                        Some(command!(CommandKind::End))
                    }

                    "let" | "set" | "shadow" => {
                        let operator = if let Some(op) = tokens.get(2) {
                            op.clone()
                        } else {
                            print_error!(ExpectedTokens, "expected operator");
                        };

                        let expr_tokens = if let Some(tokens) = tokens.get(3..) {
                            tokens
                        } else {
                            print_error!(ExpectedTokens, "expected expression");
                        };

                        let expr: Vec<Token> = collect_expr!(expr_tokens);

                        let command = match ident {
                            "let" | "shadow" => {
                                let shadowing = ident == "shadow";

                                let next_token = match tokens.get(1) {
                                    Some(Token {
                                        kind: TokenKind::DoubleIdentifier(ident, ty),
                                        ..
                                    }) => Some(TypedIdentifier {
                                        ident: ident.clone(),
                                        ty: try_type_from_string!(ty.clone())
                                    }),
                                    _ => None,
                                };

                                let name: TypedIdentifier = if let Some(name) = next_token {
                                    name
                                } else {
                                    print_error!(ExpectedTokens, "expected typed identifier `name:type`");
                                };

                                let op: TokenKind = if operator.kind == TokenKind::Equality {
                                    operator.kind
                                } else {
                                    print_error!(ExpectedTokens, "expected equality symbol '=' operator");
                                };

                                command!(CommandKind::VariableDecl { shadowing, name, op, expr })
                            }

                            "set" => {
                                let name: String = if let Some(ident) = get_token_value!(1, Identifier) {
                                    ident.clone()
                                } else {
                                    print_error!(ExpectedTokens, "expected identifier");
                                };

                                let op: TokenKind = if POSSIBLE_OPS.contains(&operator.kind) {
                                    operator.kind
                                } else {
                                    print_error!(ExpectedTokens, "invalid operator");
                                };

                                command!(CommandKind::VariableUpdate { name, op, expr })
                            }

                            _ => unreachable!()
                        };

                        Some(command)
                    }

                    _ => {
                        let name: String = if let Some(ident) = get_token_value!(0, Identifier) {
                            ident.clone()
                        } else {
                            print_error!(ExpectedTokens, "expected identifier");
                        };

                        let mut args: Vec<Vec<Token>> = Vec::new();
                        
                        if let Some(tokens) = tokens.get(1..) {
                            let mut parts: Vec<Vec<Token>> = vec![Vec::new()];

                            /*
                                increments on open paren,
                                decrements on closed paren
                            */
                            let mut paren_counter: u8 = 0;

                            for token in tokens {
                                use TokenKind::*;
                                match token.kind {
                                    OpenParen => {
                                        if let Some(sum) = paren_counter.checked_add(1)
                                        && sum < MAX_EXPRESSION_DEPTH {
                                            paren_counter = sum;
                                        } else {
                                            print_error!(ExpectedTokens, "too many nested parentheses");
                                        }
                                    }

                                    ClosedParen => {
                                        if let Some(diff) = paren_counter.checked_sub(1) {
                                            paren_counter = diff;
                                        } else {
                                            print_error!(ExpectedTokens, "unmatched closed paren ')'");
                                        }
                                    }

                                    Comma if paren_counter == 0 => {
                                        parts.push(Vec::new());
                                        continue;
                                    }

                                    _ => {}
                                }

                                let part = parts.last_mut().unwrap();
                                part.push(token.clone());
                            }

                            let is_empty = parts.len() == 1 && parts[0].is_empty();

                            if !is_empty {
                                for part in parts {
                                    if part.is_empty() {
                                        print_error!(ExpectedTokens, "expected expression");
                                    }

                                    let expr: Vec<Token> = collect_expr!(&part);

                                    args.push(expr);
                                }
                            }
                        }
                        
                        Some(command!(CommandKind::FunctionCall { name, args }))
                    }
                }
            } else {
                print_error!(ExpectedTokens, "expected identifier");
            }

            if let Some(command) = new_command {
                commands.push(command);
            }
        }

        Ok(commands)
    }

    pub fn generate_instructions(&self, commands: Vec<Command>) -> Result<Vec<Instruction>, ErrorKind> {
        macro_rules! print_error {
            ($line_pos:expr, $err_kind:ident, $msg:expr) => {{
                eprintln!("[{}]: line {}:", self.filename, $line_pos);
                eprintln!("  error: {}", $msg);
                return Err(ErrorKind::$err_kind);
            }};
        }

        let mut functions: Vec<FunctionPrototype> = Vec::new();
        let mut boot: Vec<Command> = Vec::new();

        let mut registers: [Register; vm::MAX_REGISTERS_COUNT] = [const { Register::Free }; vm::MAX_REGISTERS_COUNT];

        let mut instrs: Vec<Instruction> = Vec::new();

        macro_rules! find_free_register {
            () => {{
                let mut free_register: Option<usize> = None;

                for (index, reg) in registers.iter().enumerate() {
                    if let Register::Free = reg {
                        free_register = Some(index);
                        break;
                    }
                }

                free_register
            }};
        }

        {
            let mut cmds_iter = commands.into_iter();

            while let Some(cmd) = cmds_iter.next() {
                if let CommandKind::FunctionHeader { name, args, return_type } = cmd.kind {
                    for proto in functions.iter() {
                        if proto.name == name {
                            print_error!(cmd.line_pos, DuplicateName, format!("duplicate function name `{name}`"));
                        }
                    }

                    let mut body: Vec<Command> = Vec::new();
                    let mut closed = false;

                    for cmd in cmds_iter.by_ref() {
                        use CommandKind::*;
                        match cmd.kind {
                            FunctionHeader { .. } => {
                                print_error!(cmd.line_pos, NestedFunctionDefinition, "function can't be nested into another");
                            }
                            End => {
                                closed = true;
                                break;
                            }
                            _ => {
                                body.push(cmd);
                            }
                        }
                    }

                    if !closed {
                        print_error!(cmd.line_pos, UnclosedFunction, format!("unclosed function `{name}`"));
                    }

                    let proto = FunctionPrototype { name, args, body, return_type };
                    functions.push(proto);
                } else {
                    boot.push(cmd);
                }
            }
        }

        for command in boot {
            let line_pos = command.line_pos;

            use CommandKind::*;
            match command.kind {
                FunctionHeader { .. } => {
                    panic!("FunctionHeader inside a function/boot");
                }
                VariableDecl { shadowing, name, op, expr } => {
                    if shadowing {
                        todo!();
                    } else {
                        let free_register: usize = if let Some(index) = find_free_register!() {
                            index
                        } else {
                            print_error!(line_pos, FreeRegisterNotFound, format!(
                                "unable to find free VM register for variable `{}`", name.ident
                            ));
                        };
                    }
                }
                _ => {
                    todo!();
                }
            }
        }

        Ok(Vec::new())
    }
}

pub fn compile_from_file(file: File, filename: String) -> Result<Vec<u8>, ErrorKind> {
    let compiler = Compiler::new(filename);

    let tokens = compiler.parse_file(file)?;

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
    }

    // Ok(bytecode)
    Ok(Vec::new())
}
