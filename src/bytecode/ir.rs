use crate::bytecode::Op;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A compiled bytecode program.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramBc {
    /// Collection of code objects (instruction streams).
    /// Convention: `code[0]` is always `main`.
    pub code: Vec<CodeObject>,

    /// Compiled word definitions: name -> ops
    pub words: HashMap<String, Vec<Op>>,
}

impl ProgramBc {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            code: vec![CodeObject::new()],
            words: HashMap::new(),
        }
    }
}

/// A single compiled instruction stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeObject {
    pub ops: Vec<Op>,
}

impl CodeObject {
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }
}
