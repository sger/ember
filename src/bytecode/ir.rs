use crate::bytecode::Op;
use std::collections::HashMap;

/// A compiled bytecode program.
#[derive(Debug, Clone)]
pub struct ProgramBc {
    /// Collection of code objects (instruction streams).
    /// Convention: `code[0]` is always `main`.
    pub code: Vec<CodeObject>,

    /// Compiled word definitions: name -> ops
    pub words: HashMap<String, Vec<Op>>,
}

impl ProgramBc {
    pub fn new() -> Self {
        Self {
            code: vec![CodeObject::new()],
            words: HashMap::new(),
        }
    }
}

/// A single compiled instruction stream.
#[derive(Debug, Clone)]
pub struct CodeObject {
    pub ops: Vec<Op>,
}

impl CodeObject {
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }
}
