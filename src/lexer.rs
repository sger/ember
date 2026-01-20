use crate::token::Token;

#[derive(Debug, Clone)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct Spanned {
    pub token: Token,
    pub span: Span,
}

#[derive(Debug)]
pub struct LexerError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl std::fmt::Display for LexerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.col, self.message)
    }
}

pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn current(&self) -> Option<char> {
        self.source.get(self.pos).copied()
    }

    fn peek(&self) -> Option<char> {
        self.source.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.current();
        if ch == Some('\n') {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        self.pos += 1;
        ch
    }

    fn span(&self) -> Span {
        Span {
            line: self.line,
            col: self.col,
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current() {
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_comment(&mut self) -> Token {
        self.advance();
        let mut comment = String::new();
        while let Some(ch) = self.current() {
            if ch == '\n' {
                break;
            }
            comment.push(ch);
            self.advance();
        }
        Token::Comment(comment.trim().to_string())
    }

    fn read_string(&mut self) -> Result<Token, LexerError> {
        let start_line = self.line;
        let start_col = self.col;
        self.advance();

        let mut string = String::new();
        loop {
            match self.current() {
                Some('"') => {
                    self.advance();
                    return Ok(Token::String(string));
                }
                Some('\\') => {
                    self.advance();
                    match self.current() {
                        Some('n') => string.push('\n'),
                        Some('t') => string.push('\t'),
                        Some('r') => string.push('\r'),
                        Some('\\') => string.push('\\'),
                        Some('"') => string.push('"'),
                        Some('0') => string.push('\0'),
                        Some(ch) => {
                            return Err(LexerError {
                                message: format!("unknown escape sequence: \\{}", ch),
                                line: self.line,
                                col: self.col,
                            });
                        }
                        None => {
                            return Err(LexerError {
                                message: "unexpected EOF in escape sequence".to_string(),
                                line: self.line,
                                col: self.col,
                            });
                        }
                    }
                    self.advance();
                }
                Some('\n') => {
                    return Err(LexerError {
                        message: "unterminated string (newline before closing quote)".to_string(),
                        line: start_line,
                        col: start_col,
                    });
                }
                Some(ch) => {
                    string.push(ch);
                    self.advance();
                }
                None => {
                    return Err(LexerError {
                        message: "unterminated string literal".to_string(),
                        line: start_line,
                        col: start_col,
                    });
                }
            }
        }
    }

    fn read_number(&mut self) -> Result<Token, LexerError> {
        // Remember where the number started (better error locations)
        let start_line = self.line;
        let start_col = self.col;

        // Handle leading '-': keep it separate from the digit buffer
        let is_negative = self.current() == Some('-');
        if is_negative {
            self.advance(); // consume '-'
        }

        // Hex: 0x... or 0X...
        if self.current() == Some('0') && matches!(self.peek(), Some('x') | Some('X')) {
            self.advance(); // '0'
            self.advance(); // 'x' or 'X'

            let mut hex = String::new();
            while let Some(ch) = self.current() {
                if ch.is_ascii_hexdigit() {
                    hex.push(ch);
                    self.advance();
                } else {
                    break;
                }
            }

            if hex.is_empty() {
                return Err(LexerError {
                    message: "expected hex digits after 0x".to_string(),
                    line: start_line,
                    col: start_col,
                });
            }

            let mut value = i64::from_str_radix(&hex, 16).map_err(|_| LexerError {
                message: format!("invalid hex number: 0x{}", hex),
                line: start_line,
                col: start_col,
            })?;

            if is_negative {
                value = -value;
            }

            return Ok(Token::Integer(value));
        }

        // Decimal int/float
        let mut digits = String::new();
        let mut has_dot = false;

        while let Some(ch) = self.current() {
            if ch.is_ascii_digit() {
                digits.push(ch);
                self.advance();
            } else if ch == '.' && !has_dot {
                // Only treat '.' as a decimal point if followed by a digit
                if self.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    has_dot = true;
                    digits.push('.');
                    self.advance();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if digits.is_empty() {
            return Err(LexerError {
                message: "expected digits".to_string(),
                line: start_line,
                col: start_col,
            });
        }

        if has_dot {
            let mut value: f64 = digits.parse().map_err(|_| LexerError {
                message: format!("invalid float: {}", digits),
                line: start_line,
                col: start_col,
            })?;
            if is_negative {
                value = -value;
            }
            Ok(Token::Float(value))
        } else {
            let mut value: i64 = digits.parse().map_err(|_| LexerError {
                message: format!("invalid integer: {}", digits),
                line: start_line,
                col: start_col,
            })?;
            if is_negative {
                value = -value;
            }
            Ok(Token::Integer(value))
        }
    }

    fn read_identifier(&mut self) -> Token {
        let mut ident = String::new();
        while let Some(ch) = self.current() {
            if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        match ident.as_str() {
            // Booleans
            "true" => Token::Bool(true),
            "false" => Token::Bool(false),

            // Stack ops
            "dup" => Token::Dup,
            "drop" => Token::Drop,
            "swap" => Token::Swap,
            "over" => Token::Over,
            "rot" => Token::Rot,

            // Arithmetic
            "neg" => Token::Neg,
            "abs" => Token::Abs,

            // Logic
            "and" => Token::And,
            "or" => Token::Or,
            "not" => Token::Not,

            // Control flow
            "if" => Token::If,
            "when" => Token::When,
            "cond" => Token::Cond,
            "call" => Token::Call,

            // Loops & higher-order
            "times" => Token::Times,
            "each" => Token::Each,
            "map" => Token::Map,
            "filter" => Token::Filter,
            "fold" => Token::Fold,
            "range" => Token::Range,

            // List ops
            "len" => Token::Len,
            "head" => Token::Head,
            "tail" => Token::Tail,
            "cons" => Token::Cons,
            "concat" => Token::Concat,

            // I/O
            "print" => Token::Print,
            "emit" => Token::Emit,
            "read" => Token::Read,
            "debug" => Token::Debug,

            // Definition
            "def" => Token::Def,
            "end" => Token::End,
            "import" => Token::Import,
            "module" => Token::Module,
            "use" => Token::Use,

            // User-defined word
            _ => Token::Ident(ident),
        }
    }

    fn read_operator(&mut self) -> Option<Token> {
        let ch = self.current()?;
        let next = self.peek();

        let token = match (ch, next) {
            ('!', Some('=')) => {
                self.advance();
                self.advance();
                Token::NotEq
            }
            ('<', Some('=')) => {
                self.advance();
                self.advance();
                Token::LtEq
            }
            ('>', Some('=')) => {
                self.advance();
                self.advance();
                Token::GtEq
            }
            ('+', _) => {
                self.advance();
                Token::Plus
            }
            ('-', _) => {
                self.advance();
                Token::Minus
            }
            ('*', _) => {
                self.advance();
                Token::Star
            }
            ('/', _) => {
                self.advance();
                Token::Slash
            }
            ('%', _) => {
                self.advance();
                Token::Percent
            }
            ('=', _) => {
                self.advance();
                Token::Eq
            }
            ('<', _) => {
                self.advance();
                Token::Lt
            }
            ('>', _) => {
                self.advance();
                Token::Gt
            }
            ('.', _) => {
                self.advance();
                Token::Dot
            }
            _ => return None,
        };

        Some(token)
    }

    pub fn tokenize(&mut self) -> Result<Vec<Spanned>, LexerError> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace();
            let span = self.span();

            match self.current() {
                None => {
                    tokens.push(Spanned {
                        token: Token::Eof,
                        span,
                    });
                    break;
                }
                Some('\n') => {
                    tokens.push(Spanned {
                        token: Token::Newline,
                        span,
                    });
                    self.advance();
                }
                Some(';') => {
                    let token = self.read_comment();
                    tokens.push(Spanned { token, span });
                }
                Some('"') => {
                    let token = self.read_string()?;
                    tokens.push(Spanned { token, span });
                }
                Some('[') => {
                    self.advance();
                    tokens.push(Spanned {
                        token: Token::LBracket,
                        span,
                    });
                }
                Some(']') => {
                    self.advance();
                    tokens.push(Spanned {
                        token: Token::RBracket,
                        span,
                    });
                }
                Some('{') => {
                    self.advance();
                    tokens.push(Spanned {
                        token: Token::LBrace,
                        span,
                    });
                }
                Some('}') => {
                    self.advance();
                    tokens.push(Spanned {
                        token: Token::RBrace,
                        span,
                    });
                }
                Some('-') if self.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) => {
                    let token = self.read_number()?;
                    tokens.push(Spanned { token, span });
                }
                Some(ch) if ch.is_ascii_digit() => {
                    let token = self.read_number()?;
                    tokens.push(Spanned { token, span });
                }
                Some(ch) if ch.is_alphabetic() || ch == '_' => {
                    let token = self.read_identifier();
                    tokens.push(Spanned { token, span });
                }
                Some(ch) if "+-*/%=<>!.".contains(ch) => {
                    if let Some(token) = self.read_operator() {
                        tokens.push(Spanned { token, span });
                    } else {
                        return Err(LexerError {
                            message: format!("unexpected character: '{}'", ch),
                            line: self.line,
                            col: self.col,
                        });
                    }
                }
                Some(ch) => {
                    return Err(LexerError {
                        message: format!("unexpected character: '{}'", ch),
                        line: self.line,
                        col: self.col,
                    });
                }
            }
        }

        Ok(tokens)
    }

    pub fn tokenize_clean(&mut self) -> Result<Vec<Spanned>, LexerError> {
        let tokens = self.tokenize()?;
        Ok(tokens
            .into_iter()
            .filter(|t| !matches!(t.token, Token::Comment(_) | Token::Newline))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(source: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(source);
        lexer
            .tokenize_clean()
            .unwrap()
            .into_iter()
            .map(|s| s.token)
            .filter(|t| !matches!(t, Token::Eof))
            .collect()
    }

    fn tokens_raw(source: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(source);
        lexer
            .tokenize()
            .unwrap()
            .into_iter()
            .map(|s| s.token)
            .collect()
    }

    #[test]
    fn test_hello_world() {
        let t = tokens(r#""Hello, World!" print"#);
        assert_eq!(
            t,
            vec![Token::String("Hello, World!".to_string()), Token::Print]
        );
    }

    #[test]
    fn test_arithmetic() {
        let t = tokens("10 20 + 5 *");
        assert_eq!(
            t,
            vec![
                Token::Integer(10),
                Token::Integer(20),
                Token::Plus,
                Token::Integer(5),
                Token::Star
            ]
        );
    }

    #[test]
    fn test_comparison() {
        let t = tokens("= != < > <= >=");
        assert_eq!(
            t,
            vec![
                Token::Eq,
                Token::NotEq,
                Token::Lt,
                Token::Gt,
                Token::LtEq,
                Token::GtEq
            ]
        )
    }

    #[test]
    fn test_definition() {
        let t = tokens("def square dup * end");
        assert_eq!(
            t,
            vec![
                Token::Def,
                Token::Ident("square".to_string()),
                Token::Dup,
                Token::Star,
                Token::End
            ]
        )
    }

    #[test]
    fn test_quotation() {
        let t = tokens("[dup *] call");
        assert_eq!(
            t,
            vec![
                Token::LBracket,
                Token::Dup,
                Token::Star,
                Token::RBracket,
                Token::Call
            ]
        )
    }

    #[test]
    fn test_list() {
        let t = tokens("{ 1 2 3 }");
        assert_eq!(
            t,
            vec![
                Token::LBrace,
                Token::Integer(1),
                Token::Integer(2),
                Token::Integer(3),
                Token::RBrace
            ]
        )
    }

    #[test]
    fn test_floats() {
        let t = tokens("3.14 -2.5");
        assert_eq!(t, vec![Token::Float(3.14), Token::Float(-2.5)]);
    }

    #[test]
    fn test_booleans() {
        let t = tokens("true false and or not");
        assert_eq!(
            t,
            vec![
                Token::Bool(true),
                Token::Bool(false),
                Token::And,
                Token::Or,
                Token::Not
            ]
        );
    }

    #[test]
    fn test_higher_order() {
        let t = tokens("{ 1 2 3 } [dup *] map");
        assert_eq!(
            t,
            vec![
                Token::LBrace,
                Token::Integer(1),
                Token::Integer(2),
                Token::Integer(3),
                Token::RBrace,
                Token::LBracket,
                Token::Dup,
                Token::Star,
                Token::RBracket,
                Token::Map
            ]
        );
    }

    #[test]
    fn test_string_concat() {
        let t = tokens(r#""Hello " "World" . print"#);
        assert_eq!(
            t,
            vec![
                Token::String("Hello ".to_string()),
                Token::String("World".to_string()),
                Token::Dot,
                Token::Print
            ]
        );
    }

    #[test]
    fn test_control_flow() {
        let t = tokens("true [1] [2] if");
        assert_eq!(
            t,
            vec![
                Token::Bool(true),
                Token::LBracket,
                Token::Integer(1),
                Token::RBracket,
                Token::LBracket,
                Token::Integer(2),
                Token::RBracket,
                Token::If
            ]
        );
    }

    #[test]
    fn test_user_defined_word() {
        let t = tokens("def my-word 42 end my-word");
        assert_eq!(
            t,
            vec![
                Token::Def,
                Token::Ident("my-word".to_string()),
                Token::Integer(42),
                Token::End,
                Token::Ident("my-word".to_string())
            ]
        );
    }

    #[test]
    fn test_escape_sequences() {
        let t = tokens(r#""hello\nworld\t!""#);
        assert_eq!(t, vec![Token::String("hello\nworld\t!".to_string())]);
    }

    #[test]
    fn test_all_stack_ops_keywords() {
        let t = tokens("dup drop swap over rot");
        assert_eq!(
            t,
            vec![
                Token::Dup,
                Token::Drop,
                Token::Swap,
                Token::Over,
                Token::Rot
            ]
        );
    }

    #[test]
    fn test_all_logic_keywords() {
        let t = tokens("and or not true false");
        assert_eq!(
            t,
            vec![
                Token::And,
                Token::Or,
                Token::Not,
                Token::Bool(true),
                Token::Bool(false)
            ]
        );
    }

    #[test]
    fn test_all_control_flow_keywords() {
        let t = tokens("if when cond call def end");
        assert_eq!(
            t,
            vec![
                Token::If,
                Token::When,
                Token::Cond,
                Token::Call,
                Token::Def,
                Token::End
            ]
        );
    }

    #[test]
    fn test_all_loops_keywords() {
        let t = tokens("times each map filter fold range");
        assert_eq!(
            t,
            vec![
                Token::Times,
                Token::Each,
                Token::Map,
                Token::Filter,
                Token::Fold,
                Token::Range
            ]
        );
    }

    #[test]
    fn test_all_list_ops_keywords() {
        let t = tokens("len head tail cons concat .");
        assert_eq!(
            t,
            vec![
                Token::Len,
                Token::Head,
                Token::Tail,
                Token::Cons,
                Token::Concat,
                Token::Dot
            ]
        );
    }

    #[test]
    fn test_all_io_keywords() {
        let t = tokens("print emit read debug");
        assert_eq!(
            t,
            vec![Token::Print, Token::Emit, Token::Read, Token::Debug]
        );
    }

    // --------------------
    // Operators & delims
    // --------------------

    #[test]
    fn test_comparison_operators() {
        let t = tokens("1=2 1!=2 1<2 1<=2 1>2 1>=2");
        assert_eq!(
            t,
            vec![
                Token::Integer(1),
                Token::Eq,
                Token::Integer(2),
                Token::Integer(1),
                Token::NotEq,
                Token::Integer(2),
                Token::Integer(1),
                Token::Lt,
                Token::Integer(2),
                Token::Integer(1),
                Token::LtEq,
                Token::Integer(2),
                Token::Integer(1),
                Token::Gt,
                Token::Integer(2),
                Token::Integer(1),
                Token::GtEq,
                Token::Integer(2),
            ]
        );
    }

    #[test]
    fn test_arithmetic_operators() {
        let t = tokens("10 2 + 3 - 4 * 5 / 6 %");
        assert_eq!(
            t,
            vec![
                Token::Integer(10),
                Token::Integer(2),
                Token::Plus,
                Token::Integer(3),
                Token::Minus,
                Token::Integer(4),
                Token::Star,
                Token::Integer(5),
                Token::Slash,
                Token::Integer(6),
                Token::Percent,
            ]
        );
    }

    #[test]
    fn test_brackets_and_braces() {
        let t = tokens("[ ] { }");
        assert_eq!(
            t,
            vec![
                Token::LBracket,
                Token::RBracket,
                Token::LBrace,
                Token::RBrace
            ]
        );
    }

    // --------------------
    // Numbers
    // --------------------

    #[test]
    fn test_hex_numbers() {
        let t = tokens("0x2a 0xFF");
        assert_eq!(t, vec![Token::Integer(42), Token::Integer(255)]);
    }

    #[test]
    fn test_negative_numbers() {
        let t = tokens("-123 -4.5 -0x2A");
        assert_eq!(
            t,
            vec![
                Token::Integer(-123),
                Token::Float(-4.5),
                Token::Integer(-42)
            ]
        );
    }

    #[test]
    fn test_dot_operator_vs_float() {
        // float
        let t1 = tokens("1.25");
        assert_eq!(t1, vec![Token::Float(1.25)]);

        // dot operator (because '.' not followed by digit)
        let t2 = tokens("1.");
        assert_eq!(t2, vec![Token::Integer(1), Token::Dot]);

        // concat operator between strings
        let t3 = tokens(r#""a" . "b""#);
        assert_eq!(
            t3,
            vec![
                Token::String("a".to_string()),
                Token::Dot,
                Token::String("b".to_string()),
            ]
        );
    }

    // --------------------
    // Strings
    // --------------------

    #[test]
    fn test_string_escapes() {
        let t = tokens(r#""a\nb\tc\r\\\"""#);
        assert_eq!(t, vec![Token::String("a\nb\tc\r\\\"".to_string())]);
    }

    #[test]
    fn test_unterminated_string_newline_error() {
        let mut lexer = Lexer::new("\"hello\nworld\"");
        let err = lexer.tokenize().unwrap_err();
        assert!(
            err.message.contains("unterminated string"),
            "msg was: {}",
            err.message
        );
    }

    #[test]
    fn test_unknown_escape_error() {
        let mut lexer = Lexer::new(r#""\q""#);
        let err = lexer.tokenize().unwrap_err();
        assert!(
            err.message.contains("unknown escape sequence"),
            "msg was: {}",
            err.message
        );
    }

    // --------------------
    // Identifiers / keywords boundary
    // --------------------

    #[test]
    fn test_keyword_vs_ident() {
        // exact matches become keywords, others remain identifiers
        let t = tokens("print printer dup dupx if iff");
        assert_eq!(
            t,
            vec![
                Token::Print,
                Token::Ident("printer".to_string()),
                Token::Dup,
                Token::Ident("dupx".to_string()),
                Token::If,
                Token::Ident("iff".to_string()),
            ]
        );
    }

    #[test]
    fn test_identifier_with_dash() {
        let t = tokens("foo-bar");
        assert_eq!(t, vec![Token::Ident("foo-bar".to_string())]);
    }

    // --------------------
    // Raw mode: comments/newlines/eof
    // --------------------

    #[test]
    fn test_comments_newlines_eof_raw() {
        let t = tokens_raw("; hello \n1\n");
        assert_eq!(
            t,
            vec![
                Token::Comment("hello".to_string()), // trim() removes surrounding spaces
                Token::Newline,
                Token::Integer(1),
                Token::Newline,
                Token::Eof
            ]
        );
    }

    // --------------------
    // Errors
    // --------------------

    #[test]
    fn test_invalid_hex_error() {
        let mut lexer = Lexer::new("0x");
        let err = lexer.tokenize().unwrap_err();
        assert!(
            err.message.contains("expected hex digits"),
            "msg was: {}",
            err.message
        );
    }

    #[test]
    fn test_unexpected_character_error() {
        let mut lexer = Lexer::new("@");
        let err = lexer.tokenize().unwrap_err();
        assert!(
            err.message.contains("unexpected character"),
            "msg was: {}",
            err.message
        );
    }

    #[test]
    fn test_tokens_and_spans_raw() {
        // Source with:
        // - comment + newline
        // - string + print + newline
        // - numbers + operator + newline
        let src = ";hi\n\"a\" print\n10 20 +\n";

        let mut lexer = Lexer::new(src);
        let sp = lexer.tokenize().unwrap();

        // Helper macro to assert token + (line,col) quickly
        macro_rules! at {
            ($i:expr, $tok:expr, $line:expr, $col:expr) => {{
                assert_eq!(sp[$i].token, $tok, "token mismatch at index {}", $i);
                assert_eq!(sp[$i].span.line, $line, "line mismatch at index {}", $i);
                assert_eq!(sp[$i].span.col, $col, "col mismatch at index {}", $i);
            }};
        }

        // We expect 8 tokens total: comment, nl, string, print, nl, 10, 20, +, nl, eof
        // Actually that's 10 tokens.
        assert_eq!(sp.len(), 10, "unexpected token count: {:?}", sp);

        // Line 1: ";hi\n"
        at!(0, Token::Comment("hi".to_string()), 1, 1);
        at!(1, Token::Newline, 1, 4); // ';'(1) 'h'(2) 'i'(3) '\n'(4)

        // Line 2: "\"a\" print\n"
        at!(2, Token::String("a".to_string()), 2, 1);
        at!(3, Token::Print, 2, 5); // after "\"a\"" (3 chars) + space => print starts col 5
        at!(4, Token::Newline, 2, 10); // 'print' is 5 chars starting at col 5 => newline at col 10

        // Line 3: "10 20 +\n"
        at!(5, Token::Integer(10), 3, 1);
        at!(6, Token::Integer(20), 3, 4); // "10"(cols 1-2), space at col 3, "20" starts col 4
        at!(7, Token::Plus, 3, 7); // "20"(cols 4-5), space col 6, '+' at col 7
        at!(8, Token::Newline, 3, 8); // after '+' at col 7 => newline at col 8

        // Line 4: EOF (because final newline advanced to next line)
        at!(9, Token::Eof, 4, 1);
    }

    #[test]
    fn test_ops_delims_float_vs_dot_and_spans_raw() {
        let src = "{ [ ] } 1!=2 1<=2 1>=2\n1.2 \"a\" . \"b\"\n";

        let mut lexer = Lexer::new(src);
        let sp = lexer.tokenize().unwrap();

        macro_rules! at {
            ($i:expr, $tok:expr, $line:expr, $col:expr) => {{
                assert_eq!(sp[$i].token, $tok, "token mismatch at index {}", $i);
                assert_eq!(sp[$i].span.line, $line, "line mismatch at index {}", $i);
                assert_eq!(sp[$i].span.col, $col, "col mismatch at index {}", $i);
            }};
        }

        // Token list (raw):
        // Line 1: "{ [ ] } 1!=2 1<=2 1>=2\n"
        //   0: LBrace      @ 1:1
        //   1: LBracket    @ 1:3
        //   2: RBracket    @ 1:5
        //   3: RBrace      @ 1:7
        //   4: Integer(1)  @ 1:9
        //   5: NotEq       @ 1:10  (the '!' in "!=")
        //   6: Integer(2)  @ 1:12
        //   7: Integer(1)  @ 1:14
        //   8: LtEq        @ 1:15  (the '<' in "<=")
        //   9: Integer(2)  @ 1:17
        //  10: Integer(1)  @ 1:19
        //  11: GtEq        @ 1:20  (the '>' in ">=")
        //  12: Integer(2)  @ 1:22
        //  13: Newline     @ 1:23
        //
        // Line 2: "1.2 \"a\" . \"b\"\n"
        //  14: Float(1.2)  @ 2:1
        //  15: String("a") @ 2:5
        //  16: Dot         @ 2:9
        //  17: String("b") @ 2:11
        //  18: Newline     @ 2:14
        //
        // Line 3:
        //  19: Eof         @ 3:1

        assert_eq!(sp.len(), 20, "unexpected token count: {:?}", sp);

        // Line 1
        at!(0, Token::LBrace, 1, 1);
        at!(1, Token::LBracket, 1, 3);
        at!(2, Token::RBracket, 1, 5);
        at!(3, Token::RBrace, 1, 7);

        at!(4, Token::Integer(1), 1, 9);
        at!(5, Token::NotEq, 1, 10);
        at!(6, Token::Integer(2), 1, 12);

        at!(7, Token::Integer(1), 1, 14);
        at!(8, Token::LtEq, 1, 15);
        at!(9, Token::Integer(2), 1, 17);

        at!(10, Token::Integer(1), 1, 19);
        at!(11, Token::GtEq, 1, 20);
        at!(12, Token::Integer(2), 1, 22);

        at!(13, Token::Newline, 1, 23);

        // Line 2
        at!(14, Token::Float(1.2), 2, 1);
        at!(15, Token::String("a".to_string()), 2, 5);
        at!(16, Token::Dot, 2, 9);
        at!(17, Token::String("b".to_string()), 2, 11);
        at!(18, Token::Newline, 2, 14);

        // Line 3
        at!(19, Token::Eof, 3, 1);
    }
}
