use crate::frontend::lexer::Spanned;
use crate::frontend::token::Token;

pub struct TokenDumper {
    pub color: bool,
    pub show_debug_repr: bool, // if false, prints a nicer value for some tokens
}

impl Default for TokenDumper {
    fn default() -> Self {
        Self {
            color: true,
            show_debug_repr: true,
        }
    }
}

impl TokenDumper {
    // ANSI colors
    const RESET: &'static str = "\x1b[0m";
    const DIM: &'static str = "\x1b[2m";
    const GRN: &'static str = "\x1b[32m";
    const YEL: &'static str = "\x1b[33m";
    const CYN: &'static str = "\x1b[36m";
    const MAG: &'static str = "\x1b[35m";

    pub fn new() -> Self {
        Self::default()
    }

    pub fn no_color(mut self) -> Self {
        self.color = false;
        self
    }

    pub fn pretty(mut self) -> Self {
        self.show_debug_repr = false;
        self
    }

    pub fn dump(&self, tokens: &[Spanned]) {
        for s in tokens {
            self.print_one(s);
        }
    }

    fn print_one(&self, s: &Spanned) {
        let line = s.span.line;
        let col = s.span.col;

        let kind = self.kind(&s.token);
        let colr = if self.color { self.color(&s.token) } else { "" };
        let reset = if self.color { Self::RESET } else { "" };

        if self.show_debug_repr {
            // Uniform: always print Debug token
            println!(
                "[{:02}:{:02}] {}{:<8} {:?}{}",
                line, col, colr, kind, s.token, reset
            );
        } else {
            // Pretty: special cases for a couple of tokens
            match &s.token {
                Token::Comment(c) => {
                    println!(
                        "[{:02}:{:02}] {}{:<8} COMMENT: {}{}",
                        line, col, colr, kind, c, reset
                    );
                }
                Token::Newline => {
                    println!(
                        "[{:02}:{:02}] {}{:<8} NEWLINE{}",
                        line, col, colr, kind, reset
                    );
                }
                _ => {
                    println!(
                        "[{:02}:{:02}] {}{:<8} {:?}{}",
                        line, col, colr, kind, s.token, reset
                    );
                }
            }
        }
    }

    fn kind(&self, t: &Token) -> &'static str {
        use Token::*;
        match t {
            // common specials
            Newline => "NEWLINE",
            Comment(_) => "COMMENT",
            Eof => "EOF",

            // literals
            Integer(_) => "INT",
            Float(_) => "FLOAT",
            String(_) => "STRING",
            Bool(_) => "BOOL",

            // names
            Ident(_) => "IDENT",

            // structure
            LBracket | RBracket => "BRACKET",
            LBrace | RBrace => "BRACE",

            // ops / comparisons
            Plus | Minus | Star | Slash | Percent | Dot => "OP",
            Eq | NotEq | Lt | LtEq | Gt | GtEq => "CMP",

            // everything else = keyword/builtin
            _ => "KEYWORD",
        }
    }

    fn color(&self, t: &Token) -> &'static str {
        use Token::*;
        match t {
            Newline | Comment(_) | Eof => Self::DIM,
            String(_) => Self::GRN,
            Integer(_) | Float(_) | Bool(_) => Self::CYN,
            Ident(_) => Self::YEL,
            Plus | Minus | Star | Slash | Percent | Dot => Self::MAG,
            Eq | NotEq | Lt | LtEq | Gt | GtEq => Self::MAG,
            _ => Self::RESET,
        }
    }
}
