use std::collections::HashMap;

use crate::{
    ast::{node::Node, value::Value},
    vm::op::Op,
};

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
}
