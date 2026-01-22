#[derive(Debug)]
pub struct RuntimeError {
    pub message: String,
    pub call_stack: Vec<String>,
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "runtime error: {}", self.message)?;

        if !self.call_stack.is_empty() {
            write!(f, "\n  call stack:")?;

            for (i, frame) in self.call_stack.iter().rev().enumerate() {
                write!(f, "\n    {}: {}", i, frame)?;
            }
        }
        Ok(())
    }
}

impl RuntimeError {
    pub fn new(msg: &str) -> Self {
        RuntimeError {
            message: msg.to_string(),
            call_stack: Vec::new(),
        }
    }

    pub fn with_context(mut self, context: &str) -> Self {
        self.call_stack.push(context.to_string());
        self
    }
}
