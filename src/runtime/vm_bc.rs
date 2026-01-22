use std::collections::HashMap;

use crate::bytecode::op::Op;
use crate::lang::{node::Node, value::Value};

pub struct VmBc {
    stack: Vec<Value>,
    // Word definitions (AST form, from parser/resolver)
    words: HashMap<String, Vec<Node>>,
    aliases: HashMap<String, String>,
    // Compilation cache for words
    compile_cache: HashMap<String, Vec<Op>>,
}

impl VmBc {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            words: HashMap::new(),
            aliases: HashMap::new(),
            compile_cache: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn stack(&self) -> &[Value] {
        &self.stack
    }
}
