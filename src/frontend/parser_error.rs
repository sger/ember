/// A parsing error with source location.
///
/// `line` and `col` are 1-based positions coming from the lexer spans.
/// For EOF-ish errors (e.g. missing `end`, `]`, `}`), the parser will use the
/// last consumed token's span as a fallback so locations are never `0:0`.
#[derive(Debug)]
pub struct ParserError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl std::fmt::Display for ParserError {
    /// Formats as `line:col: message` for CLI-friendly diagnostics.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.col, self.message)
    }
}
