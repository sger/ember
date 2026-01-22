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
    words: HashMap<String, Vec<Op>>,
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

    pub fn reset_execution_state(&mut self) {
        self.steps = 0;
        self.call_depth = 0;
        self.call_stack.clear();
    }

    pub fn run_compiled(&mut self, prog: &ProgramBc) -> Result<(), RuntimeError> {
        self.reset_execution_state();

        self.words = prog.words.clone();

        let main = prog
            .code
            .get(0)
            .ok_or_else(|| RuntimeError::new("bytecode program has no main code object"))?;

        check_ops(&main.ops).map_err(|e| RuntimeError::new(&e.message))?;

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

                // Stack operations
                Op::Dup => {
                    let a = self.pop()?;
                    self.push(a.clone());
                    self.push(a);
                }

                // Comparison
                Op::Eq => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(Value::Bool(a == b));
                }
                Op::Le => {
                    let (b, a) = self.pop_two_numeric()?;
                    self.push(Value::Bool(a <= b));
                }

                // I/O
                Op::Print => {
                    let value = self.pop()?;
                    println!("{}", value);
                }

                // User defined words
                Op::CallWord(name) => {
                    self.call_stack.push(name.clone());
                    let ops =
                        self.words.get(name).cloned().ok_or_else(|| {
                            RuntimeError::new(&format!("undefined word: {}", name))
                        })?;
                    let result = self.exec_ops(&ops);
                    self.call_stack.pop();
                    result.map_err(|e| e.with_context(name))?;
                }

                Op::Return => break,

                other => {
                    return Err(RuntimeError::new(&format!("unhandled node: {:?}", other)));
                }
            }

            ip += 1;
        }

        Ok(())
    }

    // Stack operations

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn pop(&mut self) -> Result<Value, RuntimeError> {
        self.stack
            .pop()
            .ok_or_else(|| RuntimeError::new("stack underflow"))
    }

    fn pop_two_numeric(&mut self) -> Result<(f64, f64), RuntimeError> {
        let b = self.pop()?;
        let a = self.pop()?;
        let b_f = match &b {
            Value::Integer(n) => *n as f64,
            Value::Float(n) => *n,
            other => {
                return Err(RuntimeError::new(&format!(
                    "expected number, got {}",
                    other
                )));
            }
        };
        let a_f = match &a {
            Value::Integer(n) => *n as f64,
            Value::Float(n) => *n,
            other => {
                return Err(RuntimeError::new(&format!(
                    "expected number, got {}",
                    other
                )));
            }
        };

        Ok((b_f, a_f))
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
