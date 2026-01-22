use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::bytecode::ProgramBc;
use crate::bytecode::compile::Compiler;
use crate::bytecode::op::Op;
use crate::bytecode::stack_check_error::check_ops;
use crate::lang::program::Program;
use crate::lang::{node::Node, value::Value};
use crate::runtime::runtime_error::RuntimeError;

#[derive(Debug, Clone)]
pub struct VmBcConfig {
    pub max_call_depth: usize,
    pub max_steps: Option<usize>,
    pub max_stack_size: usize,
}

impl Default for VmBcConfig {
    fn default() -> Self {
        VmBcConfig {
            max_call_depth: 1000,
            max_steps: None,
            max_stack_size: 10_000,
        }
    }
}

pub struct VmBc {
    stack: Vec<Value>,
    // Word definitions (AST form, from parser/resolver)
    words: HashMap<String, Vec<Node>>,
    // Aliases from 'use' statements: "word" -> "Module.word"
    aliases: HashMap<String, String>,
    // Compilation cache: fully qualified name -> compiled ops
    compiled_cache: HashMap<String, Vec<Op>>,

    // Import tracking
    imported: HashSet<PathBuf>,
    current_dir: Option<PathBuf>,
    imported_programs: Vec<(PathBuf, Program)>,

    // Safety limits
    config: VmBcConfig,
    call_depth: usize,
    call_stack: Vec<String>,
    steps: usize,
}

impl VmBc {
    pub fn new() -> Self {
        Self::with_config(VmBcConfig::default())
    }

    pub fn with_config(config: VmBcConfig) -> Self {
        Self {
            stack: Vec::new(),
            words: HashMap::new(),
            aliases: HashMap::new(),
            compiled_cache: HashMap::new(),
            imported: HashSet::new(),
            current_dir: None,
            imported_programs: Vec::new(),
            config,
            call_depth: 0,
            call_stack: Vec::new(),
            steps: 0,
        }
    }

    #[allow(dead_code)]
    pub fn stack(&self) -> &[Value] {
        &self.stack
    }

    pub fn words_snapshot(&self) -> HashMap<String, Vec<Node>> {
        self.words.clone()
    }

    pub fn aliases_snapshot(&self) -> HashMap<String, String> {
        self.aliases.clone()
    }

    pub fn imported_programs_snapshot(&self) -> Vec<(PathBuf, Program)> {
        self.imported_programs.clone()
    }

    #[allow(dead_code)]
    pub fn clear_cache(&mut self) {
        self.compiled_cache.clear();
    }

    #[allow(dead_code)]
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.compiled_cache.len(), self.words.len())
    }

    pub fn set_current_dir(&mut self, path: &Path) {
        self.current_dir = if path.is_dir() {
            Some(path.to_path_buf())
        } else {
            path.parent().map(|p| p.to_path_buf())
        }
    }

    pub fn reset_execution_state(&mut self) {
        self.steps = 0;
        self.call_depth = 0;
        self.call_stack.clear();
    }

    pub fn run(&mut self, program: &Program) -> Result<(), RuntimeError> {
        self.reset_execution_state();

        for def in &program.definitions {
            self.process_definition(def)?;
        }

        let main_ops = Self::compile_nodes(&program.main)?;
        check_ops(&main_ops).map_err(|e| RuntimeError::new(&e.message))?;
        self.exec_ops(&main_ops)
    }

    pub fn run_compiled(&mut self, prog: &ProgramBc) -> Result<(), RuntimeError> {
        self.compiled_cache.extend(prog.words.clone());

        let main = prog
            .code
            .get(0)
            .ok_or_else(|| RuntimeError::new("Bytecode program has no main code object"))?;

        check_ops(&main.ops).map_err(|e| RuntimeError::new(&e.message))?;

        self.reset_execution_state();
        self.exec_ops(&main.ops)
    }

    // Execution

    fn check_limits(&mut self) -> Result<(), RuntimeError> {
        self.steps += 1;

        if let Some(max) = self.config.max_steps {
            if self.steps > max {
                return Err(RuntimeError::new(&format!(
                    "execution step limit exceeded ({})",
                    max
                )));
            }
        }

        if self.stack.len() > self.config.max_stack_size {
            return Err(RuntimeError::new(&format!(
                "stack size limit exceeded ({})",
                self.config.max_stack_size
            )));
        }

        Ok(())
    }

    fn exec_ops(&mut self, ops: &[Op]) -> Result<(), RuntimeError> {
        self.call_depth += 1;

        if self.call_depth > self.config.max_call_depth {
            let context = self.call_stack.last().cloned().unwrap_or_default();

            return Err(RuntimeError::new(&format!(
                "call depth limit exceeded ({}) - possible infinite recursion{}",
                self.config.max_call_depth,
                if context.is_empty() {
                    String::new()
                } else {
                    format!(" in '{}'", context)
                }
            )));
        }

        let result = self.exec_ops_inner(ops);

        self.call_depth -= 1;
        result
    }

    fn exec_ops_inner(&mut self, ops: &[Op]) -> Result<(), RuntimeError> {
        let mut ip: usize = 0;

        while ip < ops.len() {
            self.check_limits()?;

            match &ops[ip] {
                // Literals
                Op::Push(v) => self.push(v.clone()),

                _ => {
                    println!("error");
                }
            }

            ip += 1;
        }

        Ok(())
    }

    // Stack helpers

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    // Compilation

    fn compile_nodes(nodes: &[Node]) -> Result<Vec<Op>, RuntimeError> {
        let mut compiler = Compiler::new();
        compiler
            .compile_nodes(nodes)
            .map_err(|e| RuntimeError::new(&e.to_string()))
    }

    // Definition processing

    fn process_definition(&mut self, def: &Node) -> Result<(), RuntimeError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::frontend::{lexer::Lexer, parser::Parser};

    use super::*;

    fn run(source: &str) -> VmBc {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        let mut vm = VmBc::new();
        vm.run(&program).unwrap();
        vm
    }

    fn run_expect_error(source: &str) -> RuntimeError {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        let mut vm = VmBc::new();
        vm.run(&program)
            .expect_err("Expected an error but got success")
    }

    fn run_get_stack(source: &str) -> Vec<Value> {
        run(source).stack
    }
}
