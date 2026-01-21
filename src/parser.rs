use crate::ast::{Node, Program, UseItem, Value};
use crate::lexer::Spanned;
use crate::parser_error::ParserError;
use crate::token::Token;

/// Recursive-descent parser for Ember.
///
/// The parser consumes a stream of lexed `Spanned` tokens and produces a `Program`:
/// - `definitions`: top-level `def`, `import`, `module`, and `use` forms
/// - `main`: the remaining nodes (top-level executable code)
///
/// Notes:
/// - Comments and newlines are filtered out in `Parser::new`.
/// - Qualified words are recognized only in the strict form `Ident "." Ident`.
///   Any other use of `.` is parsed as the `StringConcat` operator.
pub struct Parser {
    tokens: Vec<Spanned>,
    pos: usize,
    /// Span of the most recently consumed token.
    ///
    /// Used to provide stable source locations for errors that occur after
    /// advancing past the last token or at end-of-file.
    last_span: Option<crate::lexer::Span>,
}

impl Parser {
    /// Creates a new parser from lexer output.
    ///
    /// The parser filters out `Token::Comment(_)` and `Token::Newline` to simplify
    /// parsing. (This keeps line/col information intact, since spans come from
    /// the original tokens.)
    pub fn new(tokens: Vec<Spanned>) -> Self {
        // Filter out comments and newlines
        let tokens: Vec<Spanned> = tokens
            .into_iter()
            .filter(|t| !matches!(t.token, Token::Comment(_) | Token::Newline))
            .collect();
        Parser {
            tokens,
            pos: 0,
            last_span: None,
        }
    }

    /// Returns the current token without consuming it.
    ///
    /// Returns `None` when the parser position is beyond the token list.
    fn current(&self) -> Option<&Spanned> {
        self.tokens.get(self.pos)
    }

    /// Advances the token stream by one and returns the consumed token.
    ///
    /// This also updates `last_span` to the consumed token's span so that
    /// EOF-related errors can still report a useful location.
    fn advance(&mut self) -> Option<&Spanned> {
        let token = self.tokens.get(self.pos);
        if let Some(s) = token {
            self.last_span = Some(s.span.clone());
        }
        self.pos += 1;
        token
    }

    /// Peeks the current token kind without consuming it.
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|s| &s.token)
    }

    /// Peeks the next token kind without consuming anything.
    fn peek_next(&self) -> Option<&Token> {
        self.tokens.get(self.pos + 1).map(|s| &s.token)
    }

    /// Constructs a `ParserError` at the most relevant location.
    ///
    /// Priority:
    /// 1. If `current()` exists, use its span.
    /// 2. Else, use `last_span` (e.g. after consuming EOF or falling off the end).
    /// 3. Else, default to (1,1) for truly empty input.
    fn error(&self, message: &str) -> ParserError {
        if let Some(spanned) = self.current() {
            ParserError {
                message: message.to_string(),
                line: spanned.span.line,
                col: spanned.span.col,
            }
        } else if let Some(span) = &self.last_span {
            // We fell off the end (or are past EOF). Use last known location.
            ParserError {
                message: message.to_string(),
                line: span.line,
                col: span.col,
            }
        } else {
            // Empty input case
            ParserError {
                message: message.to_string(),
                line: 1,
                col: 1,
            }
        }
    }

    /// Parses a complete Ember program.
    ///
    /// Top-level forms are split into:
    /// - `definitions`: `def`, `import`, `module`, `use`
    /// - `main`: everything else
    ///
    /// The parser stops when it reaches `Token::Eof`.
    pub fn parse(&mut self) -> Result<Program, ParserError> {
        let mut definitions = Vec::new();
        let mut main = Vec::new();

        while let Some(spanned) = self.current() {
            if matches!(spanned.token, Token::Eof) {
                break;
            }

            match &spanned.token {
                Token::Def => {
                    let def = self.parse_definition()?;
                    definitions.push(def);
                }
                Token::Import => {
                    let import = self.parse_import()?;
                    definitions.push(import);
                }
                Token::Module => {
                    let module = self.parse_module()?;
                    definitions.push(module);
                }
                Token::Use => {
                    let use_statement = self.parse_use()?;
                    definitions.push(use_statement);
                }
                _ => {
                    let node = self.parse_node()?;
                    main.push(node);
                }
            }
        }

        Ok(Program { definitions, main })
    }

    /// Parses a word definition:
    ///
    /// ```text
    /// def <name> <body...> end
    /// ```
    ///
    /// Returns `Node::Def { name, body }`.
    ///
    /// # Errors
    /// - If `<name>` is missing or not an identifier.
    /// - If EOF is reached before `end`.
    fn parse_definition(&mut self) -> Result<Node, ParserError> {
        self.advance(); // consume 'def'

        let name = match self.advance() {
            Some(Spanned {
                token: Token::Ident(name),
                ..
            }) => name.clone(),
            _ => return Err(self.error("expected word name after 'def'")),
        };

        let mut body = Vec::new();

        while let Some(spanned) = self.current() {
            if matches!(spanned.token, Token::End) {
                self.advance(); // consume 'end'
                break;
            }

            if matches!(spanned.token, Token::Eof) {
                return Err(self.error("unexpected EOF, expected 'end'"));
            }

            let node = self.parse_node()?;
            body.push(node);
        }

        Ok(Node::Def { name, body })
    }

    /// Parses an import statement:
    ///
    /// ```text
    /// import "path"
    /// ```
    ///
    /// Returns `Node::Import(path)`.
    ///
    /// # Errors
    /// - If the path is missing or not a string literal.
    fn parse_import(&mut self) -> Result<Node, ParserError> {
        self.advance(); // consume 'import'

        match self.advance() {
            Some(Spanned {
                token: Token::String(path),
                ..
            }) => Ok(Node::Import(path.clone())),
            _ => Err(self.error("expected string path after 'import'")),
        }
    }

    /// Parses a module block:
    ///
    /// ```text
    /// module <Name>
    ///   def ...
    ///   def ...
    /// end
    /// ```
    ///
    /// The terminating `end` is treated as optional; the module also ends when
    /// the parser sees another `module` or EOF, or when it hits non-definition code.
    ///
    /// Returns `Node::Module { name, definitions }`.
    fn parse_module(&mut self) -> Result<Node, ParserError> {
        self.advance(); // consume 'module'

        let name = match self.advance() {
            Some(Spanned {
                token: Token::Ident(name),
                ..
            }) => name.clone(),
            _ => return Err(self.error("expected module name after 'module'")),
        };

        let mut definitions = Vec::new();

        // Parse definitions until we reach the end, another module, or EOF
        while let Some(spanned) = self.current() {
            match &spanned.token {
                Token::Def => {
                    let def = self.parse_definition()?;
                    definitions.push(def);
                }
                Token::End => {
                    self.advance(); // consume 'end' (optional module terminator)
                    break;
                }
                Token::Module | Token::Eof => break,
                // If we see somethings thats not a def, end, or module we've hit main code
                _ => break,
            }
        }

        Ok(Node::Module { name, definitions })
    }

    /// Parses a `use` statement:
    ///
    /// ```text
    /// use Module.word
    /// use Module.*
    /// ```
    ///
    /// Returns `Node::Use { module, item }`.
    ///
    /// # Errors
    /// - Missing module identifier
    /// - Missing `.` after module name
    /// - Missing item identifier or `*`
    fn parse_use(&mut self) -> Result<Node, ParserError> {
        self.advance(); // consume 'use'

        let module = match self.advance() {
            Some(Spanned {
                token: Token::Ident(name),
                ..
            }) => name.clone(),
            _ => return Err(self.error("expected module name after 'use'")),
        };

        // Expect '.'
        match self.advance() {
            Some(Spanned {
                token: Token::Dot, ..
            }) => {}
            _ => return Err(self.error("expected '.' after module name in 'use'")),
        }

        // Expect identifier or '*'
        let item = match self.advance() {
            Some(Spanned {
                token: Token::Star, ..
            }) => UseItem::All,
            Some(Spanned {
                token: Token::Ident(name),
                ..
            }) => UseItem::Single(name.clone()),
            _ => return Err(self.error("expected word name or '*' after 'Module.'")),
        };

        Ok(Node::Use { module, item })
    }

    /// Parses a single executable node (literal, builtin, word call, etc.).
    ///
    /// This is the core "token to AST" mapping. Most tokens map directly to a
    /// corresponding `Node` variant.
    ///
    /// Special case: qualified words.
    /// - `Ident "." Ident` becomes `Node::QualifiedWord { module, word }`
    /// - otherwise the initial `Ident` becomes `Node::Word(name)` and `.` (if any)
    ///   is handled later as `Node::StringConcat`.
    fn parse_node(&mut self) -> Result<Node, ParserError> {
        let spanned = self.current().ok_or_else(|| self.error("unexpected EOF"))?;

        let node = match &spanned.token {
            // Literals
            Token::Integer(n) => {
                let n = *n;
                self.advance();
                Node::Literal(Value::Integer(n))
            }
            Token::Float(n) => {
                let n = *n;
                self.advance();
                Node::Literal(Value::Float(n))
            }
            Token::String(s) => {
                let s = s.clone();
                self.advance();
                Node::Literal(Value::String(s))
            }
            Token::Bool(b) => {
                let b = *b;
                self.advance();
                Node::Literal(Value::Bool(b))
            }

            // Quotation
            Token::LBracket => {
                let quotation = self.parse_quotation()?;
                Node::Literal(quotation)
            }

            // List
            Token::LBrace => {
                let list = self.parse_list()?;
                Node::Literal(list)
            }

            // Stack operations
            Token::Dup => {
                self.advance();
                Node::Dup
            }
            Token::Drop => {
                self.advance();
                Node::Drop
            }
            Token::Swap => {
                self.advance();
                Node::Swap
            }
            Token::Over => {
                self.advance();
                Node::Over
            }
            Token::Rot => {
                self.advance();
                Node::Rot
            }

            // Arithmetic
            Token::Plus => {
                self.advance();
                Node::Add
            }
            Token::Minus => {
                self.advance();
                Node::Sub
            }
            Token::Star => {
                self.advance();
                Node::Mul
            }
            Token::Slash => {
                self.advance();
                Node::Div
            }
            Token::Percent => {
                self.advance();
                Node::Mod
            }
            Token::Neg => {
                self.advance();
                Node::Neg
            }
            Token::Abs => {
                self.advance();
                Node::Abs
            }

            // Comparison
            Token::Eq => {
                self.advance();
                Node::Eq
            }
            Token::NotEq => {
                self.advance();
                Node::NotEq
            }
            Token::Lt => {
                self.advance();
                Node::Lt
            }
            Token::Gt => {
                self.advance();
                Node::Gt
            }
            Token::LtEq => {
                self.advance();
                Node::LtEq
            }
            Token::GtEq => {
                self.advance();
                Node::GtEq
            }

            // Logic
            Token::And => {
                self.advance();
                Node::And
            }
            Token::Or => {
                self.advance();
                Node::Or
            }
            Token::Not => {
                self.advance();
                Node::Not
            }

            // Control flow
            Token::If => {
                self.advance();
                Node::If
            }
            Token::When => {
                self.advance();
                Node::When
            }
            Token::Call => {
                self.advance();
                Node::Call
            }

            // Loops & higher-order
            Token::Times => {
                self.advance();
                Node::Times
            }
            Token::Each => {
                self.advance();
                Node::Each
            }
            Token::Map => {
                self.advance();
                Node::Map
            }
            Token::Filter => {
                self.advance();
                Node::Filter
            }
            Token::Fold => {
                self.advance();
                Node::Fold
            }
            Token::Range => {
                self.advance();
                Node::Range
            }

            // List operations
            Token::Len => {
                self.advance();
                Node::Len
            }
            Token::Head => {
                self.advance();
                Node::Head
            }
            Token::Tail => {
                self.advance();
                Node::Tail
            }
            Token::Cons => {
                self.advance();
                Node::Cons
            }
            Token::Concat => {
                self.advance();
                Node::Concat
            }
            Token::Dot => {
                self.advance();
                Node::StringConcat
            }

            // I/O
            Token::Print => {
                self.advance();
                Node::Print
            }
            Token::Emit => {
                self.advance();
                Node::Emit
            }
            Token::Read => {
                self.advance();
                Node::Read
            }
            Token::Debug => {
                self.advance();
                Node::Debug
            }

            // Additional builtins
            Token::Min => {
                self.advance();
                Node::Min
            }
            Token::Max => {
                self.advance();
                Node::Max
            }
            Token::Pow => {
                self.advance();
                Node::Pow
            }
            Token::Sqrt => {
                self.advance();
                Node::Sqrt
            }
            Token::Nth => {
                self.advance();
                Node::Nth
            }
            Token::Append => {
                self.advance();
                Node::Append
            }
            Token::Sort => {
                self.advance();
                Node::Sort
            }
            Token::Reverse => {
                self.advance();
                Node::Reverse
            }
            Token::Chars => {
                self.advance();
                Node::Chars
            }
            Token::Join => {
                self.advance();
                Node::Join
            }
            Token::Split => {
                self.advance();
                Node::Split
            }
            Token::Upper => {
                self.advance();
                Node::Upper
            }
            Token::Lower => {
                self.advance();
                Node::Lower
            }
            Token::Trim => {
                self.advance();
                Node::Trim
            }
            Token::Clear => {
                self.advance();
                Node::Clear
            }
            Token::Depth => {
                self.advance();
                Node::Depth
            }
            Token::Type => {
                self.advance();
                Node::Type
            }
            Token::ToString => {
                self.advance();
                Node::ToString
            }
            Token::ToInt => {
                self.advance();
                Node::ToInt
            }

            // Concatenative Combinators
            Token::Dip => {
                self.advance();
                Node::Dip
            }
            Token::Keep => {
                self.advance();
                Node::Keep
            }
            Token::Bi => {
                self.advance();
                Node::Bi
            }
            Token::Bi2 => {
                self.advance();
                Node::Bi2
            }
            Token::Tri => {
                self.advance();
                Node::Tri
            }
            Token::Both => {
                self.advance();
                Node::Both
            }
            Token::Compose => {
                self.advance();
                Node::Compose
            }
            Token::Curry => {
                self.advance();
                Node::Curry
            }
            Token::Apply => {
                self.advance();
                Node::Apply
            }

            // User-defined word
            Token::Ident(name) => {
                let name = name.clone();
                self.advance();

                // Check if this is a qualified word (Module.word)
                if matches!(self.peek(), Some(Token::Dot)) {
                    // Peek ahead to see if followed by an identifier
                    if matches!(self.peek_next(), Some(Token::Ident(_))) {
                        self.advance(); // consume '.'
                        if let Some(Spanned {
                            token: Token::Ident(word),
                            ..
                        }) = self.advance()
                        {
                            Node::QualifiedWord {
                                module: name,
                                word: word.clone(),
                            }
                        } else {
                            // Shouldn't happen given our peek check
                            Node::Word(name)
                        }
                    } else {
                        // Dot not followed by identifier, treat as regular word
                        Node::Word(name)
                    }
                } else {
                    Node::Word(name)
                }
            }

            // Unexpected
            _ => {
                return Err(self.error(&format!("unexpected token: {:?}", spanned.token)));
            }
        };
        Ok(node)
    }

    /// Parses a list literal:
    ///
    /// ```text
    /// { 1 2 3 }
    /// { 1 { 2 3 } 4 }   // nested lists allowed
    /// ```
    ///
    /// Lists may contain only literal values (numbers, strings, bools, lists).
    /// They do not contain arbitrary nodes.
    ///
    /// # Errors
    /// - Unexpected token inside the list
    /// - EOF before `}`
    fn parse_list(&mut self) -> Result<Value, ParserError> {
        self.advance(); // consume '{'

        let mut items = Vec::new();

        while let Some(spanned) = self.current() {
            match &spanned.token {
                Token::RBrace => {
                    self.advance(); // consume '}'
                    return Ok(Value::List(items));
                }
                Token::Integer(n) => {
                    items.push(Value::Integer(*n));
                    self.advance();
                }
                Token::Float(n) => {
                    items.push(Value::Float(*n));
                    self.advance();
                }
                Token::String(s) => {
                    items.push(Value::String(s.clone()));
                    self.advance();
                }
                Token::Bool(b) => {
                    items.push(Value::Bool(*b));
                    self.advance();
                }
                Token::LBrace => {
                    let nested = self.parse_list()?;
                    items.push(nested);
                }
                Token::Eof => {
                    return Err(self.error("unexpected EOF, expected '}'"));
                }
                _ => {
                    return Err(
                        self.error(&format!("unexpected token in list: {:?}", spanned.token))
                    );
                }
            }
        }

        Err(self.error("unexpected EOF, expected '}'"))
    }

    /// Parses a quotation:
    ///
    /// ```text
    /// [ dup * ]
    /// [ 1 [2] if ]
    /// ```
    ///
    /// Quotations contain full `Node`s (not just literal values).
    ///
    /// # Errors
    /// - EOF before `]`
    fn parse_quotation(&mut self) -> Result<Value, ParserError> {
        self.advance(); // consume '['

        let mut body = Vec::new();

        while let Some(spanned) = self.current() {
            if matches!(spanned.token, Token::RBracket) {
                self.advance(); // consume ']'
                return Ok(Value::Quotation(body));
            }

            if matches!(spanned.token, Token::Eof) {
                return Err(self.error("unexpected EOF, expected ']'"));
            }

            let node = self.parse_node()?;
            body.push(node);
        }

        Err(self.error("unexpected EOF, expected ']'"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(source: &str) -> Program {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse().unwrap()
    }

    fn parse_err(source: &str) -> ParserError {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse().unwrap_err()
    }

    #[test]
    fn test_hello_world() {
        let program = parse(r#""Hello, World!" print"#);
        assert_eq!(program.main.len(), 2);
        assert!(
            matches!(&program.main[0], Node::Literal(Value::String(s)) if s == "Hello, World!")
        );
        assert!(matches!(program.main[1], Node::Print));
    }

    #[test]
    fn test_arithmetic() {
        let program = parse("10 20 + 5 *");
        assert_eq!(program.main.len(), 5);
    }

    #[test]
    fn test_definition() {
        let program = parse("def square dup * end 5 square");
        assert_eq!(program.definitions.len(), 1);
        assert!(
            matches!(&program.definitions[0], Node::Def { name, body } if name == "square" && body.len() == 2)
        );
    }

    #[test]
    fn test_quotation() {
        let prog = parse("[dup *] call");
        assert_eq!(prog.main.len(), 2);
        assert!(matches!(
            &prog.main[0],
            Node::Literal(Value::Quotation(body)) if body.len() == 2
        ));
    }

    #[test]
    fn test_list() {
        let prog = parse("{ 1 2 3 }");
        assert_eq!(prog.main.len(), 1);
        assert!(matches!(
            &prog.main[0],
            Node::Literal(Value::List(items)) if items.len() == 3
        ));
    }

    #[test]
    fn test_filters_comments_and_newlines() {
        let program = parse(
            r#"
            ; comment line
            "hi"
            print
            "#,
        );
        assert_eq!(program.main.len(), 2);
        assert!(matches!(&program.main[0], Node::Literal(Value::String(s)) if s == "hi"));
        assert!(matches!(&program.main[1], Node::Print));
    }

    #[test]
    fn test_import_parses_into_definitions() {
        let program = parse(r#"import "player""#);

        assert_eq!(program.definitions.len(), 1);
        println!("definitions = {:?}", program.definitions);

        assert!(matches!(
            &program.definitions[0],
            Node::Import(path) if path == "player"
        ));
        assert_eq!(program.main.len(), 0);
    }

    #[test]
    fn test_use_single_item() {
        let program = parse("use Player.create");
        assert_eq!(program.definitions.len(), 1);
        assert!(
            matches!(&program.definitions[0], Node::Use { module, item } if module == "Player" && matches!(item, UseItem::Single(name) if name == "create")
            )
        );
    }

    #[test]
    fn test_use_all_item() {
        let program = parse("use Enemy.*");
        assert_eq!(program.definitions.len(), 1);
        assert!(
            matches!(&program.definitions[0], Node::Use { module, item } if module == "Enemy" && matches!(item, UseItem::All)
            )
        );
    }

    #[test]
    fn test_module_with_multiple_defs() {
        let program = parse(
            r#"
            module Player
                def create 100 end
                def damage swap - end
            end
            "#,
        );

        assert_eq!(program.definitions.len(), 1);

        match &program.definitions[0] {
            Node::Module { name, definitions } => {
                assert_eq!(name, "Player");
                assert_eq!(definitions.len(), 2);
                assert!(matches!(&definitions[0], Node::Def { name, .. } if name == "create"));
                assert!(matches!(&definitions[1], Node::Def { name, .. } if name == "damage"));
            }
            other => panic!("expected Node::Module, got {other:?}"),
        }
    }

    #[test]
    fn test_module_terminator_optional() {
        // parse_module treats 'end' as optional terminator.
        // This asserts the parser doesn't crash and stops module parsing at EOF.
        let program = parse(
            r#"
            module Enemy
                def goblin 30 end
            "#,
        );

        assert_eq!(program.definitions.len(), 1);
        assert!(matches!(&program.definitions[0], Node::Module { name, .. } if name == "Enemy"));
    }

    #[test]
    fn test_qualified_word_parses() {
        let program = parse("Enemy.goblin");

        assert_eq!(program.main.len(), 1);
        assert!(
            matches!(&program.main[0], Node::QualifiedWord { module, word } if module == "Enemy" && word == "goblin")
        );
    }

    #[test]
    fn test_ident_dot_not_followed_by_ident_is_not_qualified() {
        // If "Foo . 123" ever becomes possible, this test ensures your
        // current "only qualify if next is Ident" logic stays stable.

        let program = parse("Foo .");
        assert_eq!(program.main.len(), 2);
        assert!(matches!(&program.main[0], Node::Word(w) if w == "Foo"));
        assert!(matches!(&program.main[1], Node::StringConcat));
    }

    #[test]
    fn test_nested_list_parses() {
        let program = parse("{ 1 { 2 3 } 4 }");
        assert_eq!(program.main.len(), 1);

        match &program.main[0] {
            Node::Literal(Value::List(items)) => {
                assert_eq!(items.len(), 3);
                assert!(matches!(&items[0], Value::Integer(1)));
                assert!(matches!(&items[1], Value::List(xs) if xs.len() == 2));
                assert!(matches!(&items[2], Value::Integer(4)));
            }
            other => panic!("expected list literal, got {other:?}"),
        }
    }

    #[test]
    fn test_quotation_contains_nodes_in_order() {
        let prog = parse("[ 1 dup * ]");
        assert_eq!(prog.main.len(), 1);

        match &prog.main[0] {
            Node::Literal(Value::Quotation(body)) => {
                assert_eq!(body.len(), 3);
                assert!(matches!(&body[0], Node::Literal(Value::Integer(1))));
                assert!(matches!(&body[1], Node::Dup));
                assert!(matches!(&body[2], Node::Mul));
            }
            other => panic!("expected quotation literal, got {other:?}"),
        }
    }

    #[test]
    fn test_definition_unexpected_eof_missing_end() {
        let err = parse_err("def square dup *");
        assert!(err.message.contains("unexpected EOF"));
        assert!(err.message.contains("expected 'end'"));
    }

    #[test]
    fn test_list_unexpected_eof_missing_rbrace() {
        let err = parse_err("{ 1 2 3 ");
        assert!(err.message.contains("unexpected EOF"));
        assert!(err.message.contains("expected '}'"));
    }

    #[test]
    fn test_quotation_unexpected_eof_missing_rbracket() {
        let err = parse_err("[ 1 dup ");
        assert!(err.message.contains("unexpected EOF"));
        assert!(err.message.contains("expected ']'"));
    }

    #[test]
    fn test_use_missing_dot_errors() {
        let err = parse_err("use Player create");
        assert!(err.message.contains("expected '.'"));
    }

    #[test]
    fn test_use_missing_item_errors() {
        let err = parse_err("use Player.");
        assert!(err.message.contains("expected word name or '*'"));
    }

    #[test]
    fn test_import_requires_string() {
        let err = parse_err("import player");
        assert!(err.message.contains("expected string path"));
    }

    #[test]
    fn test_unknown_token_reports_unexpected() {
        let err = parse_err("cond");
        assert!(err.message.contains("unexpected token"));
    }

    #[test]
    fn test_error_line_for_missing_end_in_def() {
        let src = r#"
    def square
      dup *
    "#;

        let err = parse_err(src);

        assert!(
            err.message.contains("expected 'end'"),
            "message = {}",
            err.message
        );

        // Expect the error at EOF line (often last line of the source)
        let expected_line = src.lines().count();
        assert_eq!(err.line, expected_line, "err = {:?}", err);
        assert!(err.col > 0);
    }

    #[test]
    fn test_error_points_at_unexpected_token() {
        let err = parse_err("1 2 }");

        assert!(err.message.contains("unexpected token"));

        // '}' is the third token, same line
        assert_eq!(err.line, 1);
        assert!(err.col > 0);
    }

    #[test]
    fn test_error_line_for_unterminated_quotation() {
        let src = r#"
    [ 1
      dup
    "#;

        let err = parse_err(src);
        assert!(err.message.contains("expected ']'"), "msg={}", err.message);

        let expected_line = src.lines().count();
        assert_eq!(err.line, expected_line, "err={:?}", err);
        assert!(err.col > 0);
    }

    #[test]
    fn test_error_column_is_not_zero() {
        let err = parse_err("use Player.");

        assert!(err.message.contains("expected"));

        assert!(err.col > 0, "column should not be zero; got {}", err.col);
    }

    #[test]
    fn test_exact_column_for_simple_error() {
        let err = parse_err("}");

        assert_eq!(err.line, 1);
        assert_eq!(err.col, 1);
    }
}
