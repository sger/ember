use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::bytecode::op::Op;
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

        // Process all definitions (including imports, modules, use statements)
        for def in &program.definitions {
            self.process_definition(def)?;
        }
    }
}
