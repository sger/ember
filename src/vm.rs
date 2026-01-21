use crate::ast::{Node, Program, UseItem, Value};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::runtime_error::RuntimeError;
use std::collections::{HashMap, HashSet};
use std::f64;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct VMConfig {
    pub max_call_depth: usize,
    pub max_steps: Option<usize>,
    pub max_stack_size: usize,
}

impl Default for VMConfig {
    fn default() -> Self {
        VMConfig {
            max_call_depth: 1000,
            max_steps: None,
            max_stack_size: 10_000,
        }
    }
}

pub struct VM {
    stack: Vec<Value>,
    // "word" or "Module.word" -> body
    words: HashMap<String, Vec<Node>>,
    // "word" -> "Module.word" (from 'use')
    aliases: HashMap<String, String>,
    imported: HashSet<PathBuf>,
    current_dir: Option<PathBuf>,
    // Print full abstract syntax tree
    imported_programs: Vec<(PathBuf, Program)>,
    config: VMConfig,
    call_depth: usize,
    call_stack: Vec<String>,
    steps: usize,
}

impl VM {
    pub fn new() -> Self {
        Self::with_config(VMConfig::default())
    }

    pub fn with_config(config: VMConfig) -> Self {
        VM {
            stack: Vec::new(),
            words: HashMap::new(),
            aliases: HashMap::new(),
            imported: HashSet::new(),
            current_dir: None,
            imported_programs: Vec::new(),
            config,
            call_depth: 0,
            call_stack: Vec::new(),
            steps: 0,
        }
    }

    pub fn imported_programs_snapshot(&self) -> Vec<(PathBuf, Program)> {
        self.imported_programs.clone()
    }

    /// Convenience: pretty-print all ASTs (main + imported) for `--ast-full`.
    pub fn print_ast_full(&self, main_path: Option<&Path>, main_program: &Program) {
        // Main file/program
        if let Some(p) = main_path {
            println!("== AST (main: {}) ==", p.display());
        } else {
            println!("== AST (main) ==");
        }
        println!("{:#?}", main_program);

        // Imported files/programs
        for (path, program) in &self.imported_programs {
            println!();
            println!("== AST (import: {}) ==", path.display());
            println!("{:#?}", program);
        }
    }

    pub fn words_snapshot(&self) -> std::collections::HashMap<String, Vec<crate::ast::Node>> {
        self.words.clone()
    }

    pub fn aliases_snapshot(&self) -> std::collections::HashMap<String, String> {
        self.aliases.clone()
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn pop(&mut self) -> Result<Value, RuntimeError> {
        self.stack
            .pop()
            .ok_or_else(|| RuntimeError::new("stack underflow"))
    }

    fn pop_int(&mut self) -> Result<i64, RuntimeError> {
        match self.pop()? {
            Value::Integer(n) => Ok(n),
            other => Err(RuntimeError::new(&format!(
                "expected integer, got {}",
                other
            ))),
        }
    }

    fn pop_bool(&mut self) -> Result<bool, RuntimeError> {
        match self.pop()? {
            Value::Bool(b) => Ok(b),
            other => Err(RuntimeError::new(&format!(
                "expected boolean, got {}",
                other
            ))),
        }
    }

    // TODO Handle CompiledQuotation (cannot execute in AST interpreter)
    fn pop_quotation(&mut self) -> Result<Vec<Node>, RuntimeError> {
        match self.pop()? {
            Value::Quotation(body) => Ok(body),
            other => Err(RuntimeError::new(&format!(
                "expected quotation, got {}",
                other
            ))),
        }
    }

    fn pop_list(&mut self) -> Result<Vec<Value>, RuntimeError> {
        match self.pop()? {
            Value::List(items) => Ok(items),
            other => Err(RuntimeError::new(&format!("expected list, got {}", other))),
        }
    }

    fn pop_string(&mut self) -> Result<String, RuntimeError> {
        match self.pop()? {
            Value::String(s) => Ok(s),
            other => Err(RuntimeError::new(&format!(
                "expected string, for {}",
                other
            ))),
        }
    }

    /// Pop a numeric value, returning (value as f64, was_integer)
    fn pop_numeric(&mut self) -> Result<(f64, bool), RuntimeError> {
        match self.pop()? {
            Value::Integer(n) => Ok((n as f64, true)),
            Value::Float(n) => Ok((n, false)),
            other => Err(RuntimeError::new(&format!(
                "expected number, got {}",
                other
            ))),
        }
    }

    /// Apply a binary numeric operation, preserving integer type when possible
    fn numeric_binop<F>(&mut self, op: F, op_name: &str) -> Result<(), RuntimeError>
    where
        F: Fn(f64, f64) -> f64,
    {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = match (&a, &b) {
            (Value::Integer(a), Value::Integer(b)) => {
                let r = op(*a as f64, *b as f64);
                if r.fract() == 0.0 && r >= i64::MIN as f64 && r <= i64::MAX as f64 {
                    Value::Integer(r as i64)
                } else {
                    Value::Float(r)
                }
            }
            (Value::Float(a), Value::Float(b)) => Value::Float(op(*a, *b)),
            (Value::Integer(a), Value::Float(b)) => Value::Float(op(*a as f64, *b)),
            (Value::Float(a), Value::Integer(b)) => Value::Float(op(*a, *b as f64)),
            _ => {
                return Err(RuntimeError::new(&format!(
                    "cannot {} {} and {}",
                    op_name, a, b
                )));
            }
        };
        self.push(result);
        Ok(())
    }

    /// Compare two numeric values
    fn numeric_compare<F>(&mut self, op: F) -> Result<(), RuntimeError>
    where
        F: Fn(f64, f64) -> bool,
    {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = match (&a, &b) {
            (Value::Integer(a), Value::Integer(b)) => op(*a as f64, *b as f64),
            (Value::Float(a), Value::Float(b)) => op(*a, *b),
            (Value::Integer(a), Value::Float(b)) => op(*a as f64, *b),
            (Value::Float(a), Value::Integer(b)) => op(*a, *b as f64),
            _ => {
                return Err(RuntimeError::new(&format!(
                    "cannot compare {} and {}",
                    a, b
                )));
            }
        };
        self.push(Value::Bool(result));
        Ok(())
    }

    pub fn set_current_dir(&mut self, path: &Path) {
        self.current_dir = if path.is_dir() {
            Some(path.to_path_buf())
        } else {
            path.parent().map(|p| p.to_path_buf())
        }
    }

    pub fn load(&mut self, program: &Program) -> Result<(), RuntimeError> {
        for def in &program.definitions {
            self.process_definition(def)?;
        }
        Ok(())
    }

    pub fn run(&mut self, program: &Program) -> Result<(), RuntimeError> {
        for def in &program.definitions {
            self.process_definition(def)?;
        }

        self.execute(&program.main)
    }

    pub fn execute(&mut self, nodes: &[Node]) -> Result<(), RuntimeError> {
        for node in nodes {
            self.execute_node(node)?;
        }
        Ok(())
    }

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

    fn execute_node(&mut self, node: &Node) -> Result<(), RuntimeError> {
        self.check_limits()?;

        match node {
            // Literals
            Node::Literal(value) => {
                self.push(value.clone());
            }

            // Stack operations
            Node::Dup => {
                let a = self.pop()?;
                self.push(a.clone());
                self.push(a);
            }
            Node::Drop => {
                self.pop()?;
            }
            Node::Swap => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(b);
                self.push(a);
            }
            Node::Over => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(a.clone());
                self.push(b);
                self.push(a);
            }
            Node::Rot => {
                let c = self.pop()?;
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(b);
                self.push(c);
                self.push(a);
            }

            // Arithmetic
            Node::Add => self.numeric_binop(|a, b| a + b, "add")?,
            Node::Sub => self.numeric_binop(|a, b| a - b, "subtract")?,
            Node::Mul => self.numeric_binop(|a, b| a * b, "multiply")?,
            Node::Div => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = match (a, b) {
                    (Value::Integer(a), Value::Integer(b)) => {
                        if b == 0 {
                            return Err(RuntimeError::new(&format!("division by zero")));
                        }
                        Value::Integer(a / b)
                    }
                    (Value::Float(a), Value::Float(b)) => Value::Float(a / b),
                    (Value::Integer(a), Value::Float(b)) => Value::Float(a as f64 / b),
                    (Value::Float(a), Value::Integer(b)) => Value::Float(a / b as f64),
                    (a, b) => {
                        return Err(RuntimeError::new(&format!("cannot divide {} and {}", a, b)));
                    }
                };
                self.push(result);
            }
            Node::Mod => {
                let b = self.pop_int()?;
                let a = self.pop_int()?;
                if b == 0 {
                    return Err(RuntimeError::new("modulo by zero"));
                }
                self.push(Value::Integer(a % b));
            }
            Node::Neg => {
                let a = self.pop()?;
                let result = match a {
                    Value::Integer(n) => Value::Integer(-n),
                    Value::Float(n) => Value::Float(-n),
                    other => return Err(RuntimeError::new(&format!("cannot negate {}", other))),
                };
                self.push(result);
            }
            Node::Abs => {
                let a = self.pop()?;
                let result = match a {
                    Value::Integer(n) => Value::Integer(n.abs()),
                    Value::Float(n) => Value::Float(n.abs()),
                    other => {
                        return Err(RuntimeError::new(&format!("cannot take abs of {}", other)));
                    }
                };
                self.push(result);
            }

            // Comparison
            Node::Eq => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(Value::Bool(a == b));
            }
            Node::NotEq => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(Value::Bool(a != b));
            }
            Node::Lt => self.numeric_compare(|a, b| a < b)?,
            Node::Gt => self.numeric_compare(|a, b| a > b)?,
            Node::LtEq => self.numeric_compare(|a, b| a <= b)?,
            Node::GtEq => self.numeric_compare(|a, b| a >= b)?,

            // Logic
            Node::And => {
                let b = self.pop_bool()?;
                let a = self.pop_bool()?;
                self.push(Value::Bool(a && b));
            }
            Node::Or => {
                let b = self.pop_bool()?;
                let a = self.pop_bool()?;
                self.push(Value::Bool(a || b));
            }
            Node::Not => {
                let a = self.pop_bool()?;
                self.push(Value::Bool(!a));
            }

            // Control flow
            Node::If => {
                let else_branch = self.pop_quotation()?;
                let then_branch = self.pop_quotation()?;
                let condition = self.pop_bool()?;
                if condition {
                    self.execute(&then_branch)?;
                } else {
                    self.execute(&else_branch)?;
                }
            }
            Node::When => {
                let then_branch = self.pop_quotation()?;
                let condition = self.pop_bool()?;
                if condition {
                    self.execute(&then_branch)?;
                }
            }
            Node::Call => {
                let quotation = self.pop_quotation()?;
                self.execute(&quotation)?;
            }

            // Loops & higher-order
            Node::Times => {
                let body = self.pop_quotation()?;
                let n = self.pop_int()?;
                if n < 0 {
                    return Err(RuntimeError::new("times expects non-negative integer"));
                }
                for i in 0..(n as i64) {
                    self.push(Value::Integer(i)); // optional: include index; or omit if you prefer
                    self.execute(&body)?;
                }
            }
            Node::Each => {
                let body = self.pop_quotation()?;
                let list = self.pop_list()?;
                for item in list {
                    self.push(item);
                    self.execute(&body)?;
                }
            }
            Node::Map => {
                let body = self.pop_quotation()?;
                let list = self.pop_list()?;
                let mut result = Vec::new();

                for item in list {
                    self.push(item);
                    self.execute(&body)?;
                    result.push(self.pop()?);
                }
                self.push(Value::List(result));
            }
            Node::Filter => {
                let body = self.pop_quotation()?;
                let list = self.pop_list()?;
                let mut result = Vec::new();

                for item in list {
                    self.push(item.clone());
                    self.execute(&body)?;
                    if self.pop_bool()? {
                        result.push(item);
                    }
                }
                self.push(Value::List(result));
            }
            Node::Fold => {
                let body = self.pop_quotation()?;
                let mut acc = self.pop()?;
                let list = self.pop_list()?;

                for item in list {
                    self.push(acc);
                    self.push(item);
                    self.execute(&body)?;
                    acc = self.pop()?;
                }
                self.push(acc);
            }
            Node::Range => {
                let end = self.pop_int()?;
                let start = self.pop_int()?;
                let list: Vec<Value> = (start..end).map(Value::Integer).collect();
                self.push(Value::List(list));
            }

            // List operations
            Node::Len => {
                let list = self.pop_list()?;
                self.push(Value::Integer(list.len() as i64));
            }
            Node::Head => {
                let list = self.pop_list()?;
                if list.is_empty() {
                    return Err(RuntimeError::new("head of empty list"));
                }
                self.push(list[0].clone());
            }
            Node::Tail => {
                let list = self.pop_list()?;
                if list.is_empty() {
                    return Err(RuntimeError::new("tail of empty list"));
                }
                self.push(Value::List(list[1..].to_vec()));
            }
            Node::Cons => {
                let list = self.pop_list()?;
                let elem = self.pop()?;
                let mut new_list = vec![elem];
                new_list.extend(list);
                self.push(Value::List(new_list));
            }
            Node::Concat => {
                let b = self.pop_list()?;
                let a = self.pop_list()?;
                let mut result = a;
                result.extend(b);
                self.push(Value::List(result));
            }
            Node::StringConcat => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = format!("{}{}", a, b);
                self.push(Value::String(result));
            }

            // I/O
            Node::Print => {
                let value = self.pop()?;
                println!("{}", value);
            }
            Node::Emit => {
                let code = self.pop_int()?;
                if let Some(ch) = char::from_u32(code as u32) {
                    print!("{}", ch);
                    io::stdout().flush().ok();
                }
            }
            Node::Read => {
                let stdin = io::stdin();
                let line = stdin
                    .lock()
                    .lines()
                    .next()
                    .transpose()
                    .map_err(|e| return RuntimeError::new(&format!("read error: {}", e)))?
                    .unwrap_or_default();
                self.push(Value::String(line));
            }
            Node::Debug => {
                let value = self.pop()?;
                println!("[DEBUG] {:?}", value);
                self.push(value);
            }

            // Additional builtins
            Node::Min => {
                let b = self.pop_int()?;
                let a = self.pop_int()?;
                self.push(Value::Integer(a.min(b)));
            }
            Node::Max => {
                let b = self.pop_int()?;
                let a = self.pop_int()?;
                self.push(Value::Integer(a.max(b)));
            }
            Node::Pow => {
                let exp = self.pop_int()?;
                let base = self.pop_int()?;
                if exp < 0 {
                    return Err(RuntimeError::new("negative exponent"));
                }
                self.push(Value::Integer(base.pow(exp as u32)));
            }
            Node::Sqrt => {
                let n = self.pop()?;
                match n {
                    Value::Integer(n) => {
                        self.push(Value::Float((n as f64).sqrt()));
                    }
                    Value::Float(n) => {
                        self.push(Value::Float(n.sqrt()));
                    }
                    other => {
                        return Err(RuntimeError::new(&format!("cannot take sqrt of {}", other)));
                    }
                }
            }
            Node::Nth => {
                let idx = self.pop_int()?;
                let list = self.pop_list()?;
                if idx < 0 || idx as usize >= list.len() {
                    return Err(RuntimeError::new(&format!(
                        "index {} out of bounds for list of length {}",
                        idx,
                        list.len()
                    )));
                }
                self.push(list[idx as usize].clone());
            }
            Node::Append => {
                let elem = self.pop()?;
                let mut list = self.pop_list()?;
                list.push(elem);
                self.push(Value::List(list));
            }
            Node::Sort => {
                let mut list = self.pop_list()?;
                // Only sort if all integers
                let all_ints = list.iter().all(|v| matches!(v, Value::Integer(_)));
                if all_ints {
                    list.sort_by(|a, b| {
                        if let (Value::Integer(a), Value::Integer(b)) = (a, b) {
                            a.cmp(b)
                        } else {
                            std::cmp::Ordering::Equal
                        }
                    });
                }
                self.push(Value::List(list));
            }
            Node::Reverse => {
                let mut list = self.pop_list()?;
                list.reverse();
                self.push(Value::List(list));
            }
            Node::Chars => {
                let s = self.pop_string()?;
                let chars: Vec<Value> = s.chars().map(|c| Value::String(c.to_string())).collect();
                self.push(Value::List(chars));
            }
            Node::Join => {
                let sep = self.pop_string()?;
                let list = self.pop_list()?;
                let strings: Vec<String> = list.iter().map(|v| format!("{}", v)).collect();
                self.push(Value::String(strings.join(&sep)));
            }
            Node::Split => {
                let sep = self.pop_string()?;
                let s = self.pop_string()?;
                let parts: Vec<Value> = s
                    .split(&sep)
                    .map(|p| Value::String(p.to_string()))
                    .collect();
                self.push(Value::List(parts));
            }
            Node::Upper => {
                let s = self.pop_string()?;
                self.push(Value::String(s.to_uppercase()));
            }
            Node::Lower => {
                let s = self.pop_string()?;
                self.push(Value::String(s.to_lowercase()));
            }
            Node::Trim => {
                let s = self.pop_string()?;
                self.push(Value::String(s.trim().to_string()));
            }
            Node::Clear => {
                self.stack.clear();
            }
            Node::Depth => {
                let depth = self.stack.len() as i64;
                self.push(Value::Integer(depth));
            }
            Node::Type => {
                let value = self.pop()?;
                let type_name = match &value {
                    Value::Integer(_) => "Integer",
                    Value::Float(_) => "Float",
                    Value::String(_) => "String",
                    Value::Bool(_) => "Bool",
                    Value::List(_) => "List",
                    Value::Quotation(_) => "Quotation",
                    // CompiledQuotation
                };
                self.push(value);
                self.push(Value::String(type_name.to_string()));
            }
            Node::ToString => {
                let value = self.pop()?;
                self.push(Value::String(format!("{}", value)));
            }
            Node::ToInt => {
                let value = self.pop()?;
                match value {
                    Value::Integer(n) => self.push(Value::Integer(n)),
                    Value::Float(n) => self.push(Value::Integer(n as i64)),
                    Value::String(s) => {
                        let n: i64 = s.trim().parse().map_err(|_| {
                            RuntimeError::new(&format!("cannot parse '{}' as integer", s))
                        })?;
                        self.push(Value::Integer(n));
                    }
                    Value::Bool(b) => self.push(Value::Integer(if b { 1 } else { 0 })),
                    other => {
                        return Err(RuntimeError::new(&format!(
                            "cannot convert {} to integer",
                            other
                        )));
                    }
                }
            }

            // User-defined word (checks aliases first)
            Node::Word(name) => {
                let body = self
                    .lookup_word(name)
                    .ok_or_else(|| RuntimeError::new(&format!("undefined word: {}", name)))?
                    .to_vec();

                self.call_depth += 1;

                if self.call_depth > self.config.max_call_depth {
                    return Err(RuntimeError::new(&format!(
                        "call depth limit exceeded ({}) - possible infinite recursion in '{}'",
                        self.config.max_call_depth, name
                    )));
                }

                self.call_stack.push(name.clone());

                let result = self.execute(&body);

                self.call_stack.pop();
                self.call_depth -= 1;

                result.map_err(|e| e.with_context(name))?;
            }

            // Qualified word (Module.word)
            Node::QualifiedWord { module, word } => {
                let qualified = format!("{}.{}", module, word);
                let body = self
                    .words
                    .get(&qualified)
                    .ok_or_else(|| RuntimeError::new(&format!("undefined: {}.{}", module, word)))?
                    .clone();

                self.call_depth += 1;
                if self.call_depth > self.config.max_call_depth {
                    return Err(RuntimeError::new(&format!(
                        "call depth limit exceeded ({}) - possible infinite recursion in '{}'",
                        self.config.max_call_depth, qualified
                    )));
                }
                self.call_stack.push(qualified.clone());

                let result = self.execute(&body);

                self.call_stack.pop();
                self.call_depth -= 1;

                result.map_err(|e| e.with_context(&qualified))?;
            }

            // Concatenative Combinators
            Node::Dip => {
                // Execute quotation with top of stack temporarily hidden
                let quot = self.pop_quotation()?;
                let saved = self.pop()?;
                self.execute(&quot)?;
                self.push(saved);
            }
            Node::Keep => {
                // Execute quotation but preserve the input value
                let quot = self.pop_quotation()?;
                let a = self.pop()?;
                self.push(a.clone());
                self.execute(&quot)?;
                self.push(a);
            }
            Node::Bi => {
                // Apply two quotations to the same value
                let q = self.pop_quotation()?;
                let p = self.pop_quotation()?;
                let a = self.pop()?;
                self.push(a.clone());
                self.execute(&p)?;
                self.push(a);
                self.execute(&q)?;
            }
            Node::Bi2 => {
                // Apply two quotations to two values
                let q = self.pop_quotation()?;
                let p = self.pop_quotation()?;
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(a.clone());
                self.push(b.clone());
                self.execute(&p)?;
                self.push(a);
                self.push(b);
                self.execute(&q)?;
            }
            Node::Tri => {
                // Apply three quotations to the same value
                let r = self.pop_quotation()?;
                let q = self.pop_quotation()?;
                let p = self.pop_quotation()?;
                let a = self.pop()?;
                self.push(a.clone());
                self.execute(&p)?;
                self.push(a.clone());
                self.execute(&q)?;
                self.push(a);
                self.execute(&r)?;
            }
            Node::Both => {
                // Apply the same quotation to two values
                let quot = self.pop_quotation()?;
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(a);
                self.execute(&quot)?;
                self.push(b);
                self.execute(&quot)?;
            }
            Node::Compose => {
                // Create a new quotation that runs quot1 then quot2
                let q2 = self.pop_quotation()?;
                let q1 = self.pop_quotation()?;
                let mut combined = q1;
                combined.extend(q2);
                self.push(Value::Quotation(combined));
            }
            Node::Curry => {
                // Create a new quotation with value baked in at the front
                let quot = self.pop_quotation()?;
                let value = self.pop()?;
                let mut curried = vec![Node::Literal(value)];
                curried.extend(quot);
                self.push(Value::Quotation(curried));
            }
            Node::Apply => {
                // Push all list elements onto stack, then execute quotation
                let quot = self.pop_quotation()?;
                let args = self.pop_list()?;
                for arg in args {
                    self.push(arg);
                }
                self.execute(&quot)?;
            }

            // Definition (shouldn't be executed directly)
            Node::Def { .. } => {}

            // Module (handled during program loading)
            Node::Module { .. } => {}

            // Use (handled during program loading)
            Node::Use { .. } => {}

            // Import (handled during program loading)
            Node::Import(_) => {}

            // Node::Def { .. } | Node::Module { .. } | Node::Use { .. } | Node::Import(_) => {
            //     self.process_definition(node)?;
            // }
            other => {
                return Err(RuntimeError::new(&format!("unhandled node: {:?}", other)));
            }
        }

        Ok(())
    }

    fn lookup_word(&self, name: &str) -> Option<&[Node]> {
        if let Some(qualified) = self.aliases.get(name) {
            return self.words.get(qualified).map(|v| v.as_slice());
        }
        self.words.get(name).map(|v| v.as_slice())
    }

    fn process_definition(&mut self, def: &Node) -> Result<(), RuntimeError> {
        match def {
            Node::Def { name, body } => {
                self.words.insert(name.clone(), body.clone());
            }
            Node::Import(path) => {
                self.import_file(path)?;
            }
            Node::Module { name, definitions } => {
                self.register_module(name, definitions)?;
            }
            Node::Use { module, item } => {
                self.handle_use(module, item)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn import_file(&mut self, path: &str) -> Result<(), RuntimeError> {
        // Enforce/normalize extension: allow no extension => .em, otherwise must be .em
        let mut import_rel = PathBuf::from(path);

        if import_rel.extension().is_none() {
            import_rel.set_extension("em");
        } else if import_rel.extension().and_then(|e| e.to_str()) != Some("em") {
            return Err(RuntimeError::new(&format!(
                "imports must use .em files (or omit extension), got '{}'",
                path
            )));
        }

        // Resolve path relative to current file directory (or CWD if none)
        let resolved = if let Some(ref current_dir) = self.current_dir {
            current_dir.join(&import_rel)
        } else {
            import_rel.clone()
        };

        // Canonicalize to handle relative paths
        let canonical = resolved.canonicalize().map_err(|e| {
            RuntimeError::new(&format!(
                "cannot resolve import '{}' (tried '{}'): {}",
                path,
                resolved.display(),
                e
            ))
        })?;

        // Check for circular / repeated imports
        if self.imported.contains(&canonical) {
            return Ok(()); // Already imported, skip
        }
        self.imported.insert(canonical.clone());

        // Read file
        let source = std::fs::read_to_string(&canonical).map_err(|e| {
            RuntimeError::new(&format!(
                "cannot read import '{}' ('{}'): {}",
                path,
                canonical.display(),
                e
            ))
        })?;

        // Parse
        let mut lexer = Lexer::new(&source);
        let tokens = lexer.tokenize().map_err(|e| {
            RuntimeError::new(&format!("in import '{}': {}", canonical.display(), e))
        })?;

        let mut parser = Parser::new(tokens);
        let program = parser.parse().map_err(|e| {
            RuntimeError::new(&format!("in import '{}': {}", canonical.display(), e))
        })?;

        // record AST for --ast-full
        self.imported_programs
            .push((canonical.clone(), program.clone()));

        // Save current dir, set to imported file's dir (for nested imports)
        let prev_dir = self.current_dir.clone();
        self.current_dir = canonical.parent().map(|p| p.to_path_buf());

        // Process definitions from imported file (including modules)
        for def in &program.definitions {
            self.process_definition(def)?;
        }

        // Restore current dir
        self.current_dir = prev_dir;

        Ok(())
    }

    fn register_module(
        &mut self,
        module_name: &str,
        definitions: &[Node],
    ) -> Result<(), RuntimeError> {
        for def in definitions {
            if let Node::Def {
                name: word_name,
                body,
            } = def
            {
                // Register with qualified name (Module.word)
                let qualified = format!("{}.{}", module_name, word_name);
                self.words.insert(qualified.clone(), body.clone());

                // Also register with unqualified name for intra-module calls
                // (can be overridden by later definitions or use statements)
                if !self.words.contains_key(word_name) {
                    self.words.insert(word_name.clone(), body.clone());
                }
            }
        }
        Ok(())
    }

    fn handle_use(&mut self, module: &str, item: &UseItem) -> Result<(), RuntimeError> {
        match item {
            UseItem::Single(word) => {
                let qualified = format!("{}.{}", module, word);
                if !self.words.contains_key(&qualified) {
                    return Err(RuntimeError::new(&format!(
                        "undefined: {}.{}",
                        module, word
                    )));
                }
                self.aliases.insert(word.clone(), qualified);
            }
            UseItem::All => {
                let prefix = format!("{}.", module);
                let to_alias: Vec<(String, String)> = self
                    .words
                    .keys()
                    .filter(|k| k.starts_with(&prefix))
                    .map(|qualified| {
                        let word = qualified.strip_prefix(&prefix).unwrap().to_string();
                        (word, qualified.clone())
                    })
                    .collect();

                if to_alias.is_empty() {
                    return Err(RuntimeError::new(&format!(
                        "no definitions found in module '{}'",
                        module
                    )));
                }

                for (word, qualified) in to_alias {
                    self.aliases.insert(word, qualified);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn run(source: &str) -> VM {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        let mut vm = VM::new();
        vm.run(&program).unwrap();
        vm
    }

    fn run_get_stack(source: &str) -> Vec<Value> {
        run(source).stack
    }

    #[test]
    fn test_arithmetic() {
        let stack = run_get_stack("10 20 +");
        assert_eq!(stack, vec![Value::Integer(30)]);

        let stack = run_get_stack("10 3 *");
        assert_eq!(stack, vec![Value::Integer(30)]);

        let stack = run_get_stack("20 3 /");
        assert_eq!(stack, vec![Value::Integer(6)]);

        let stack = run_get_stack("20 3 %");
        assert_eq!(stack, vec![Value::Integer(2)]);
    }

    #[test]
    fn test_stack_ops() {
        let stack = run_get_stack("1 2 3 swap");
        assert_eq!(
            stack,
            vec![Value::Integer(1), Value::Integer(3), Value::Integer(2)]
        );

        let stack = run_get_stack("1 dup");
        assert_eq!(stack, vec![Value::Integer(1), Value::Integer(1)]);

        let stack = run_get_stack("1 2 drop");
        assert_eq!(stack, vec![Value::Integer(1)]);
    }

    #[test]
    fn test_comparison() {
        let stack = run_get_stack("5 3 <");
        assert_eq!(stack, vec![Value::Bool(false)]);

        let stack = run_get_stack("3 5 <");
        assert_eq!(stack, vec![Value::Bool(true)]);

        let stack = run_get_stack("5 5 =");
        assert_eq!(stack, vec![Value::Bool(true)]);
    }

    #[test]
    fn test_if() {
        let stack = run_get_stack("true [1] [2] if");
        assert_eq!(stack, vec![Value::Integer(1)]);

        let stack = run_get_stack("false [1] [2] if");
        assert_eq!(stack, vec![Value::Integer(2)]);
    }

    #[test]
    fn test_definition() {
        let stack = run_get_stack("def square dup * end 5 square");
        assert_eq!(stack, vec![Value::Integer(25)]);
    }

    #[test]
    fn test_range() {
        let stack = run_get_stack("1 5 range");
        assert_eq!(
            stack,
            vec![Value::List(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3),
                Value::Integer(4),
            ])]
        );
    }

    #[test]
    fn test_map() {
        let stack = run_get_stack("{ 1 2 3 } [dup *] map");
        assert_eq!(
            stack,
            vec![Value::List(vec![
                Value::Integer(1),
                Value::Integer(4),
                Value::Integer(9),
            ])]
        );
    }

    #[test]
    fn test_filter() {
        let stack = run_get_stack("{ 1 2 3 4 5 6 } [ 2 % 0 =] filter");
        assert_eq!(
            stack,
            vec![Value::List(vec![
                Value::Integer(2),
                Value::Integer(4),
                Value::Integer(6),
            ])]
        );
    }

    #[test]
    fn test_fold() {
        let stack = run_get_stack("{ 1 2 3 4 5 } 0 [+] fold");
        assert_eq!(stack, vec![Value::Integer(15)]);
    }

    #[test]
    fn test_factorial() {
        let stack = run_get_stack(
            "def factorial dup 1 <= [drop 1] [dup 1 - factorial *] if end 5 factorial",
        );
        assert_eq!(stack, vec![Value::Integer(120)]);
    }

    #[test]
    fn test_fibonacci() {
        let stack = run_get_stack(
            "def fib \
                dup 1 <= \
                [ drop 1 ] \
                [ dup 1 - fib swap 2 - fib + ] \
             if end \
             10 fib",
        );

        assert_eq!(stack, vec![Value::Integer(89)]);
    }

    #[test]
    fn test_fibonacci_base_cases() {
        let stack0 =
            run_get_stack("def fib dup 1 <= [drop 1] [dup 1 - fib swap 2 - fib +] if end 0 fib");
        assert_eq!(stack0, vec![Value::Integer(1)]);

        let stack1 =
            run_get_stack("def fib dup 1 <= [drop 1] [dup 1 - fib swap 2 - fib +] if end 1 fib");
        assert_eq!(stack1, vec![Value::Integer(1)]);
    }

    #[test]
    fn test_min_max() {
        let stack = run_get_stack("5 10 min");
        assert_eq!(stack, vec![Value::Integer(5)]);

        let stack = run_get_stack("5 10 max");
        assert_eq!(stack, vec![Value::Integer(10)]);
    }

    #[test]
    fn test_pow() {
        let stack = run_get_stack("2 10 pow");
        assert_eq!(stack, vec![Value::Integer(1024)]);
    }

    #[test]
    fn test_reverse() {
        let stack = run_get_stack("{ 1 2 3 } reverse");
        assert_eq!(
            stack,
            vec![Value::List(vec![
                Value::Integer(3),
                Value::Integer(2),
                Value::Integer(1),
            ])]
        );
    }

    #[test]
    fn test_sort() {
        let stack = run_get_stack("{ 3 1 4 1 5 } sort");
        assert_eq!(
            stack,
            vec![Value::List(vec![
                Value::Integer(1),
                Value::Integer(1),
                Value::Integer(3),
                Value::Integer(4),
                Value::Integer(5),
            ])]
        );
    }

    #[test]
    fn test_nth() {
        let stack = run_get_stack("{ 10 20 30 } 1 nth");
        assert_eq!(stack, vec![Value::Integer(20)]);
    }

    #[test]
    fn test_append() {
        let stack = run_get_stack("{ 1 2 } 3 append");
        assert_eq!(
            stack,
            vec![Value::List(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3),
            ])]
        );
    }

    #[test]
    fn test_upper_lower() {
        let stack = run_get_stack(r#""Hello" upper"#);
        assert_eq!(stack, vec![Value::String("HELLO".to_string())]);

        let stack = run_get_stack(r#""Hello" lower"#);
        assert_eq!(stack, vec![Value::String("hello".to_string())]);
    }

    #[test]
    fn test_split_join() {
        let stack = run_get_stack(r#""a,b,c" "," split"#);
        assert_eq!(
            stack,
            vec![Value::List(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string()),
                Value::String("c".to_string()),
            ])]
        );

        let stack = run_get_stack(r#"{ "a" "b" "c" } "-" join"#);
        assert_eq!(stack, vec![Value::String("a-b-c".to_string())]);
    }

    #[test]
    fn test_depth_clear() {
        let stack = run_get_stack("1 2 3 depth");
        assert_eq!(
            stack,
            vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3),
                Value::Integer(3),
            ]
        );

        let stack = run_get_stack("1 2 3 clear depth");
        assert_eq!(stack, vec![Value::Integer(0)]);
    }

    #[test]
    fn test_type() {
        let stack = run_get_stack("42 type");
        assert_eq!(
            stack,
            vec![Value::Integer(42), Value::String("Integer".to_string())]
        );
    }

    #[test]
    fn test_to_string_to_int() {
        let stack = run_get_stack("42 to-string");
        assert_eq!(stack, vec![Value::String("42".to_string())]);

        let stack = run_get_stack(r#""123" to-int"#);
        assert_eq!(stack, vec![Value::Integer(123)]);
    }

    #[test]
    fn test_dip() {
        let stack = run_get_stack("1 2 [3 +] dip");
        assert_eq!(stack, vec![Value::Integer(4), Value::Integer(2)]);

        let stack = run_get_stack("10 20 [5 *] dip");
        assert_eq!(stack, vec![Value::Integer(50), Value::Integer(20)]);
    }

    #[test]
    fn test_keep() {
        // keep: execute quotation, but preserve original value
        let stack = run_get_stack("5 [dup *] keep");
        assert_eq!(stack, vec![Value::Integer(25), Value::Integer(5)]);

        let stack = run_get_stack("10 [2 /] keep");
        assert_eq!(stack, vec![Value::Integer(5), Value::Integer(10)]);
    }

    #[test]
    fn test_bi() {
        // bi: apply two quotations to same value
        let stack = run_get_stack("5 [2 +] [3 *] bi");
        assert_eq!(stack, vec![Value::Integer(7), Value::Integer(15)]);

        let stack = run_get_stack("100 [10 /] [1 +] bi");
        assert_eq!(stack, vec![Value::Integer(10), Value::Integer(101)]);
    }

    #[test]
    fn test_bi2() {
        // bi2: apply two quotations to two values
        let stack = run_get_stack("3 4 [+] [*] bi2");
        assert_eq!(stack, vec![Value::Integer(7), Value::Integer(12)]);

        // Compare two numbers: get both difference and ratio
        let stack = run_get_stack("10 2 [-] [/] bi2");
        assert_eq!(stack, vec![Value::Integer(8), Value::Integer(5)]);
    }

    #[test]
    fn test_tri() {
        // tri: apply three quotations to same value
        let stack = run_get_stack("10 [1 +] [2 *] [dup *] tri");
        assert_eq!(
            stack,
            vec![
                Value::Integer(11),  // 10 + 1
                Value::Integer(20),  // 10 * 2
                Value::Integer(100), // 10 * 10
            ]
        );
    }

    #[test]
    fn test_both() {
        // both: apply same quotation to two values
        let stack = run_get_stack("3 4 [dup *] both");
        assert_eq!(stack, vec![Value::Integer(9), Value::Integer(16)]);

        // Square two numbers
        let stack = run_get_stack("5 12 [dup *] both");
        assert_eq!(stack, vec![Value::Integer(25), Value::Integer(144)]);
    }

    #[test]
    fn test_compose() {
        // compose: create combined quotation
        let stack = run_get_stack("[2 +] [3 *] compose 5 swap call");
        assert_eq!(stack, vec![Value::Integer(21)]); // (5 + 2) * 3 = 21

        // Order matters: first quot runs first
        let stack = run_get_stack("[3 *] [2 +] compose 5 swap call");
        assert_eq!(stack, vec![Value::Integer(17)]); // (5 * 3) + 2 = 17
    }

    #[test]
    fn test_curry() {
        // curry: partial application
        let stack = run_get_stack("10 [+] curry 5 swap call");
        assert_eq!(stack, vec![Value::Integer(15)]); // 5 + 10 = 15

        // Curry a multiplier
        let stack = run_get_stack("3 [*] curry 7 swap call");
        assert_eq!(stack, vec![Value::Integer(21)]); // 7 * 3 = 21
    }

    #[test]
    fn test_apply() {
        // apply: use list as arguments
        let stack = run_get_stack("{ 3 4 } [+] apply");
        assert_eq!(stack, vec![Value::Integer(7)]);

        // Multiple operations: 2 3 4 -> (3 + 4) * 2 = 14
        let stack = run_get_stack("{ 2 3 4 } [+ *] apply");
        assert_eq!(stack, vec![Value::Integer(14)]);
    }

    #[test]
    fn test_combinator_composition() {
        // Combine combinators for more complex patterns
        // cleave pattern: apply multiple quotations, collect results
        let stack = run_get_stack("10 [1 +] [2 *] [3 -] tri");
        assert_eq!(
            stack,
            vec![Value::Integer(11), Value::Integer(20), Value::Integer(7),]
        );

        // Pythagorean: 3² + 4² = 25
        let stack = run_get_stack("3 4 [dup *] both +");
        assert_eq!(stack, vec![Value::Integer(25)]);
    }

    #[test]
    fn test_curry_with_filter() {
        // Create a "greater than 5" filter using curry
        let stack = run_get_stack("{ 1 3 5 7 9 } 5 [>] curry filter");
        assert_eq!(
            stack,
            vec![Value::List(vec![Value::Integer(7), Value::Integer(9),])]
        );
    }

    #[test]
    fn test_compose_pipeline() {
        // Build a processing pipeline: add 1, then double
        let stack = run_get_stack("[1 +] [2 *] compose 10 swap call");
        assert_eq!(stack, vec![Value::Integer(22)]); // (10 + 1) * 2 = 22
    }
}
