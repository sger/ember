use crate::frontend::lexer::Span;
use std::fmt;
use std::path::PathBuf;

/// Type alias for Result with a boxed RuntimeError.
/// This keeps the Result size small (pointer-sized error variant).
pub type RuntimeResult<T> = Result<T, Box<RuntimeError>>;

#[derive(Debug)]
pub struct RuntimeError {
    pub message: String,
    pub span: Option<Span>,
    pub source: Option<String>,
    pub file: Option<PathBuf>,
    pub call_stack: Vec<String>,
    pub help: Option<String>,
}

impl RuntimeError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            span: None,
            source: None,
            file: None,
            call_stack: Vec::new(),
            help: None,
        }
    }

    pub fn boxed(self) -> Box<Self> {
        Box::new(self)
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_source(mut self, source: String) -> Self {
        self.source = Some(source);
        self
    }

    pub fn with_file(mut self, file: PathBuf) -> Self {
        self.file = Some(file);
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn with_context(mut self, word: &str) -> Self {
        if !word.is_empty() {
            self.call_stack.push(word.to_string());
        }
        self
    }

    /// Get the source line text if available
    fn get_line_text(&self) -> Option<String> {
        if let (Some(span), Some(source)) = (&self.span, &self.source) {
            source
                .lines()
                .nth(span.line.saturating_sub(1))
                .map(|s| s.to_string())
        } else {
            None
        }
    }

    /// Format error with beautiful context
    pub fn display_with_context(&self) -> String {
        let mut output = String::new();

        // Error header
        output.push_str(&format!("\n❌ Runtime Error: {}\n", self.message));

        // Location
        if let Some(span) = &self.span {
            if let Some(file) = &self.file {
                output.push_str(&format!(
                    "  --> {}:{}:{}\n",
                    file.display(),
                    span.line,
                    span.col
                ));
            } else {
                output.push_str(&format!("  --> line {}:{}\n", span.line, span.col));
            }

            // Source context
            if let Some(source) = &self.source {
                let lines: Vec<&str> = source.lines().collect();
                if span.line > 0 && span.line <= lines.len() {
                    let line_idx = span.line - 1;

                    // Show line before (if exists)
                    if line_idx > 0 {
                        output.push_str(&format!(
                            "  {:>4} | {}\n",
                            span.line - 1,
                            lines[line_idx - 1]
                        ));
                    }

                    // Show error line
                    output.push_str(&format!("  {:>4} | {}\n", span.line, lines[line_idx]));

                    // Show error pointer (^^^)
                    let spaces = " ".repeat(span.col.saturating_sub(1));
                    output.push_str(&format!("       | {}^\n", spaces));

                    // Show line after (if exists)
                    if line_idx + 1 < lines.len() {
                        output.push_str(&format!(
                            "  {:>4} | {}\n",
                            span.line + 1,
                            lines[line_idx + 1]
                        ));
                    }
                }
            } else if let Some(line_text) = self.get_line_text() {
                // Fallback: just show the line without context
                output.push_str(&format!("  {:>4} | {}\n", span.line, line_text));
                let spaces = " ".repeat(span.col.saturating_sub(1));
                output.push_str(&format!("       | {}^\n", spaces));
            }
        }

        // Call stack
        if !self.call_stack.is_empty() {
            output.push_str("\n📚 Call stack:\n");
            for (i, frame) in self.call_stack.iter().enumerate() {
                output.push_str(&format!("  {} {}\n", i, frame));
            }
        }

        // Help message
        if let Some(help) = &self.help {
            output.push_str(&format!("\n💡 Help: {}\n", help));
        }

        output
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.display_with_context())
    }
}

impl std::error::Error for RuntimeError {}

// Helper functions for common error types

pub fn stack_underflow(expected: usize, actual: usize) -> RuntimeError {
    RuntimeError::new(&format!(
        "stack underflow: expected {} values, found {}",
        expected, actual
    ))
    .with_help("Check that all operations have enough arguments on the stack")
}

#[allow(dead_code)]
pub fn type_error(expected: &str, got: &str) -> RuntimeError {
    RuntimeError::new(&format!("type error: expected {}, got {}", expected, got)).with_help(
        format!(
            "This operation requires a {} value, but received a {}",
            expected, got
        ),
    )
}

pub fn undefined_word(word: &str) -> RuntimeError {
    RuntimeError::new(&format!("undefined word: {}", word)).with_help(format!(
        "The word '{}' is not defined. Check spelling or define it with: def {} ... end",
        word, word
    ))
}

pub fn division_by_zero() -> RuntimeError {
    RuntimeError::new("division by zero")
        .with_help("Check that the divisor is not zero before dividing")
}

pub fn index_out_of_bounds(index: i64, length: usize) -> RuntimeError {
    RuntimeError::new(&format!(
        "index {} out of bounds for list of length {}",
        index, length
    ))
    .with_help(format!(
        "Valid indices are 0 to {}",
        length.saturating_sub(1)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_error() {
        let err = RuntimeError::new("something went wrong");
        assert_eq!(err.message, "something went wrong");
    }

    #[test]
    fn test_error_with_span() {
        let span = Span { line: 5, col: 10 };
        let err = RuntimeError::new("test error").with_span(span);
        assert!(err.span.is_some());
        assert_eq!(err.span.unwrap().line, 5);
    }

    #[test]
    fn test_error_with_source() {
        let source = "line 1\nline 2\nline 3";
        let span = Span { line: 2, col: 3 };
        let err = RuntimeError::new("test error")
            .with_span(span)
            .with_source(source.to_string());

        let output = err.display_with_context();
        assert!(output.contains("line 2"));
        assert!(output.contains("line 2:3"));
    }

    #[test]
    fn test_helper_functions() {
        let err = stack_underflow(2, 0);
        assert!(err.message.contains("stack underflow"));
        assert!(err.help.is_some());

        let err = type_error("integer", "string");
        assert!(err.message.contains("expected integer, got string"));

        let err = undefined_word("foo");
        assert!(err.message.contains("undefined word: foo"));
        assert!(err.help.unwrap().contains("def foo"));
    }
}
