#[derive(Debug)]
pub struct RuntimeError {
    pub message: String,
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "runtime error: {}", self.message)
    }
}

impl RuntimeError {
    fn new(msg: &str) -> Self {
        RuntimeError {
            message: msg.to_string(),
        }
    }
}
