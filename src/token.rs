#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Integer(i64),
    Float(f64),
    String(std::string::String),
    Bool(bool),

    // Stack operations
    Dup,
    Drop,
    Swap,
    Over,
    Rot,

    // Arithmetic
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Neg,
    Abs,

    // Comparison
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,

    // Logic
    And,
    Or,
    Not,

    // Control flow
    If,
    When,
    Cond,
    Call,

    // Loops and higher-order
    Times,
    Each,
    Map,
    Filter,
    Fold,
    Range,

    // List operations
    Len,
    Head,
    Tail,
    Cons,
    Concat,
    Dot, // string concat

    // I/O
    Print,
    Emit,
    Read,
    Debug,

    // Definition
    Def,
    End,
    Import,
    Module,
    Use,

    // Delimiters
    LBracket, // [
    RBracket, // ]
    LBrace,   // {
    RBrace,   // }

    // Identifier (user-defined word)
    Ident(std::string::String),

    // Special
    Comment(std::string::String),
    Newline,
    Eof,
    // TODO stdlib
}

impl Token {
    /// Returns true if this token is a built-in word
    #[allow(dead_code)]
    pub fn is_builtin_word(&self) -> bool {
        matches!(
            self,
            Token::Dup
                | Token::Drop
                | Token::Swap
                | Token::Over
                | Token::Rot
                | Token::Plus
                | Token::Minus
                | Token::Star
                | Token::Slash
                | Token::Percent
                | Token::Neg
                | Token::Abs
                | Token::Eq
                | Token::NotEq
                | Token::Lt
                | Token::Gt
                | Token::LtEq
                | Token::GtEq
                | Token::And
                | Token::Or
                | Token::Not
                | Token::If
                | Token::When
                | Token::Cond
                | Token::Call
                | Token::Times
                | Token::Each
                | Token::Map
                | Token::Filter
                | Token::Fold
                | Token::Range
                | Token::Len
                | Token::Head
                | Token::Tail
                | Token::Cons
                | Token::Concat
                | Token::Dot
                | Token::Print
                | Token::Emit
                | Token::Read
                | Token::Debug
        )
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Integer(n) => write!(f, "{}", n),
            Token::Float(n) => write!(f, "{}", n),
            Token::String(s) => write!(f, "\"{}\"", s),
            Token::Bool(b) => write!(f, "{}", b),
            Token::Dup => write!(f, "dup"),
            Token::Drop => write!(f, "drop"),
            Token::Swap => write!(f, "swap"),
            Token::Over => write!(f, "over"),
            Token::Rot => write!(f, "rot"),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::Neg => write!(f, "neg"),
            Token::Abs => write!(f, "abs"),
            Token::Eq => write!(f, "="),
            Token::NotEq => write!(f, "!="),
            Token::Lt => write!(f, "<"),
            Token::Gt => write!(f, ">"),
            Token::LtEq => write!(f, "<="),
            Token::GtEq => write!(f, ">="),
            Token::And => write!(f, "and"),
            Token::Or => write!(f, "or"),
            Token::Not => write!(f, "not"),
            Token::If => write!(f, "if"),
            Token::When => write!(f, "when"),
            Token::Cond => write!(f, "cond"),
            Token::Call => write!(f, "call"),
            Token::Times => write!(f, "times"),
            Token::Each => write!(f, "each"),
            Token::Map => write!(f, "map"),
            Token::Filter => write!(f, "filter"),
            Token::Fold => write!(f, "fold"),
            Token::Range => write!(f, "range"),
            Token::Len => write!(f, "len"),
            Token::Head => write!(f, "head"),
            Token::Tail => write!(f, "tail"),
            Token::Cons => write!(f, "cons"),
            Token::Concat => write!(f, "concat"),
            Token::Dot => write!(f, "."),
            Token::Print => write!(f, "print"),
            Token::Emit => write!(f, "emit"),
            Token::Read => write!(f, "read"),
            Token::Debug => write!(f, "debug"),
            Token::Def => write!(f, "def"),
            Token::End => write!(f, "end"),
            Token::Import => write!(f, "import"),
            Token::Module => write!(f, "module"),
            Token::Use => write!(f, "use"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::Ident(s) => write!(f, "{}", s),
            Token::Comment(s) => write!(f, "; {}", s),
            Token::Newline => write!(f, "\\n"),
            Token::Eof => write!(f, "EOF"),
        }
    }
}
