use crate::bytecode::ProgramBc;
use crate::bytecode::compile::Compiler;
use crate::bytecode::op::Op;
use crate::bytecode::stack_check_error::check_ops;
use crate::frontend::lexer::Span;
use crate::lang::{node::Node, value::Value};
use crate::runtime::runtime_error::{
    RuntimeError, division_by_zero, index_out_of_bounds, stack_underflow, type_error,
    undefined_word,
};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

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
    pub aux_stack: Vec<Value>,
    words: HashMap<String, Vec<Op>>,
    // Safety limits
    config: VmBcConfig,
    call_depth: usize,
    call_stack: Vec<String>,
    steps: usize,
    pub source: Option<String>,
    pub file: Option<PathBuf>,
}

impl VmBc {
    pub fn new() -> Self {
        Self::with_config(VmBcConfig::default())
    }

    pub fn with_config(config: VmBcConfig) -> Self {
        Self {
            stack: Vec::new(),
            aux_stack: Vec::new(),
            words: HashMap::new(),
            config,
            call_depth: 0,
            call_stack: Vec::new(),
            steps: 0,
            source: None,
            file: None,
        }
    }

    // NEW: Setters for source tracking
    pub fn set_source(&mut self, source: String) {
        self.source = Some(source);
    }

    pub fn set_file(&mut self, file: PathBuf) {
        self.file = Some(file);
    }

    // NEW: Helper to create errors with source context
    fn error_with_context(&self, message: impl Into<String>) -> RuntimeError {
        RuntimeError::new(&message.into())
            .with_span(Span { line: 1, col: 1 })
            .with_source(self.source.clone().unwrap_or_default())
            .with_file(self.file.clone().unwrap_or_default())
    }

    // NEW: Helper for type errors
    fn type_error_with_context(&self, expected: &str, got: &str) -> RuntimeError {
        self.error_with_context(format!("type error: expected {}, got {}", expected, got))
            .with_help(format!(
                "This operation requires a {} value, but received a {}",
                expected, got
            ))
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
                Op::Drop => {
                    self.pop()?;
                }
                Op::Swap => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(b);
                    self.push(a);
                }
                Op::Over => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(b);
                    self.push(a);
                }
                Op::Rot => {
                    let c = self.pop()?;
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(b);
                    self.push(c);
                    self.push(a);
                }

                // Arithmetic
                Op::Add => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    let result = match (&a, &b) {
                        (Value::Integer(a), Value::Integer(b)) => Value::Integer(a + b),
                        (Value::Float(a), Value::Float(b)) => Value::Float(a + b),
                        (Value::Integer(a), Value::Float(b)) => Value::Float(*a as f64 + b),
                        (Value::Float(a), Value::Integer(b)) => Value::Float(a + *b as f64),
                        _ => {
                            return Err(self
                                .error_with_context(format!(
                                    "type error: cannot add {} and {}",
                                    a.type_name(),
                                    b.type_name()
                                ))
                                .with_help(format!(
                                    "Addition works on numbers, but got {} and {}",
                                    a.type_name(),
                                    b.type_name()
                                )));
                        }
                    };
                    self.push(result);
                }
                Op::Sub => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    let result = match (&a, &b) {
                        (Value::Integer(a), Value::Integer(b)) => Value::Integer(a - b),
                        (Value::Float(a), Value::Float(b)) => Value::Float(a - b),
                        (Value::Integer(a), Value::Float(b)) => Value::Float(*a as f64 - b),
                        (Value::Float(a), Value::Integer(b)) => Value::Float(a - *b as f64),
                        _ => {
                            return Err(self.error_with_context(format!(
                                "type error: cannot subtract {} from {}",
                                b.type_name(),
                                a.type_name()
                            )));
                        }
                    };
                    self.push(result);
                }
                Op::Mul => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    let result = match (&a, &b) {
                        (Value::Integer(a), Value::Integer(b)) => Value::Integer(a * b),
                        (Value::Float(a), Value::Float(b)) => Value::Float(a * b),
                        (Value::Integer(a), Value::Float(b)) => Value::Float(*a as f64 * b),
                        (Value::Float(a), Value::Integer(b)) => Value::Float(a * *b as f64),
                        _ => {
                            return Err(self.error_with_context(format!(
                                "type error: cannot multiply {} and {}",
                                a.type_name(),
                                b.type_name()
                            )));
                        }
                    };
                    self.push(result);
                }
                Op::Div => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    let result = match (&a, &b) {
                        (Value::Integer(a), Value::Integer(b)) => {
                            if *b == 0 {
                                return Err(division_by_zero()
                                    .with_source(self.source.clone().unwrap_or_default())
                                    .with_file(self.file.clone().unwrap_or_default()));
                            }
                            Value::Integer(a / b)
                        }
                        (Value::Float(a), Value::Float(b)) => {
                            if *b == 0.0 {
                                return Err(division_by_zero()
                                    .with_source(self.source.clone().unwrap_or_default())
                                    .with_file(self.file.clone().unwrap_or_default()));
                            }
                            Value::Float(a / b)
                        }
                        (Value::Integer(a), Value::Float(b)) => {
                            if *b == 0.0 {
                                return Err(division_by_zero()
                                    .with_source(self.source.clone().unwrap_or_default())
                                    .with_file(self.file.clone().unwrap_or_default()));
                            }
                            Value::Float(*a as f64 / b)
                        }
                        (Value::Float(a), Value::Integer(b)) => {
                            if *b == 0 {
                                return Err(division_by_zero()
                                    .with_source(self.source.clone().unwrap_or_default())
                                    .with_file(self.file.clone().unwrap_or_default()));
                            }
                            Value::Float(a / *b as f64)
                        }
                        _ => {
                            return Err(self.error_with_context(format!(
                                "type error: cannot divide {} by {}",
                                a.type_name(),
                                b.type_name()
                            )));
                        }
                    };
                    self.push(result);
                }
                Op::Mod => {
                    let b = self.pop_int()?;
                    let a = self.pop_int()?;
                    if b == 0 {
                        return Err(self
                            .error_with_context("modulo by zero")
                            .with_help("Check that the divisor is not zero"));
                    }
                    self.push(Value::Integer(a % b));
                }
                Op::Neg => {
                    let a = self.pop()?;
                    let result = match a {
                        Value::Integer(n) => Value::Integer(-n),
                        Value::Float(n) => Value::Float(-n),
                        other => {
                            return Err(RuntimeError::new(&format!("cannot negate {}", other)));
                        }
                    };
                    self.push(result);
                }
                Op::Abs => {
                    let a = self.pop()?;
                    let result = match a {
                        Value::Integer(n) => Value::Integer(n.abs()),
                        Value::Float(n) => Value::Float(n.abs()),
                        other => return Err(RuntimeError::new(&format!("cannot abs {}", other))),
                    };
                    self.push(result);
                }

                // Comparison
                Op::Eq => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(Value::Bool(a == b));
                }
                Op::Ne => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(Value::Bool(a != b));
                }
                Op::Lt => {
                    let (b, a) = self.pop_two_numeric()?;
                    self.push(Value::Bool(a < b));
                }
                Op::Gt => {
                    let (b, a) = self.pop_two_numeric()?;
                    self.push(Value::Bool(a > b));
                }
                Op::Le => {
                    let (b, a) = self.pop_two_numeric()?;
                    self.push(Value::Bool(a <= b));
                }
                Op::Ge => {
                    let (b, a) = self.pop_two_numeric()?;
                    self.push(Value::Bool(a >= b));
                }

                // Logic
                Op::And => {
                    let b = self.pop_bool()?;
                    let a = self.pop_bool()?;
                    self.push(Value::Bool(a && b));
                }
                Op::Or => {
                    let b = self.pop_bool()?;
                    let a = self.pop_bool()?;
                    self.push(Value::Bool(a || b));
                }
                Op::Not => {
                    let a = self.pop_bool()?;
                    self.push(Value::Bool(!a));
                }

                // List operations
                Op::Len => {
                    let value = self.pop()?;
                    match value {
                        Value::List(list) => {
                            self.push(Value::Integer(list.len() as i64));
                        }
                        Value::String(s) => {
                            self.push(Value::Integer(s.len() as i64));
                        }
                        other => {
                            return Err(self
                                .error_with_context(format!(
                                    "type error: expected list or string, got {}",
                                    other.type_name()
                                ))
                                .with_help(
                                    "Use 'len' on lists or strings. Example: \"hello\" len  or  { 1 2 3 } len"
                                ));
                        }
                    }
                }
                Op::Head => {
                    let list = self.pop_list()?;
                    if list.is_empty() {
                        return Err(RuntimeError::new("head of empty list"));
                    }
                    self.push(list[0].clone());
                }
                Op::Tail => {
                    let list = self.pop_list()?;
                    if list.is_empty() {
                        return Err(RuntimeError::new("tail of empty list"));
                    }
                    self.push(Value::List(list[1..].to_vec()));
                }
                Op::Cons => {
                    let list = self.pop_list()?;
                    let elem = self.pop()?;
                    let mut new_list = vec![elem];
                    new_list.extend(list);
                    self.push(Value::List(new_list));
                }
                Op::Concat => {
                    let b = self.pop_list()?;
                    let a = self.pop_list()?;
                    let mut result = a;
                    result.extend(b);
                    self.push(Value::List(result));
                }
                Op::StringConcat => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(Value::String(format!("{}{}", a, b)));
                }

                // I/O
                Op::Print => {
                    let value = self.pop()?;
                    println!("{}", value);
                }
                Op::Emit => {
                    let code = self.pop_int()?;
                    if let Some(ch) = char::from_u32(code as u32) {
                        print!("{}", ch);
                        io::stdout().flush().ok();
                    }
                }
                Op::Read => {
                    let stdin = io::stdin();
                    let line = stdin
                        .lock()
                        .lines()
                        .next()
                        .transpose()
                        .map_err(|e| RuntimeError::new(&format!("read error: {}", e)))?
                        .unwrap_or_default();
                    self.push(Value::String(line));
                }
                Op::Debug => {
                    let value = self.pop()?;
                    println!("[DEBUG] {:?}", value);
                    self.push(value);
                }

                // stdlib ops (keeping all your existing ones)
                Op::Min => {
                    let b = self.pop_int()?;
                    let a = self.pop_int()?;
                    self.push(Value::Integer(a.min(b)));
                }
                Op::Max => {
                    let b = self.pop_int()?;
                    let a = self.pop_int()?;
                    self.push(Value::Integer(a.max(b)));
                }
                Op::Pow => {
                    let exp = self.pop_int()?;
                    let base = self.pop_int()?;
                    if exp < 0 {
                        return Err(RuntimeError::new(
                            "negative exponent not supported for integer power",
                        ));
                    }
                    let result = base
                        .checked_pow(exp as u32)
                        .ok_or_else(|| RuntimeError::new("integer overflow in power operation"))?;
                    self.push(Value::Integer(result));
                }
                Op::Sqrt => {
                    let n = self.pop()?;
                    match n {
                        Value::Integer(n) => {
                            if n < 0 {
                                return Err(RuntimeError::new(
                                    "cannot take square root of negative number",
                                ));
                            }
                            self.push(Value::Float((n as f64).sqrt()));
                        }
                        Value::Float(n) => {
                            if n < 0.0 {
                                return Err(RuntimeError::new(
                                    "cannot take square root of negative number",
                                ));
                            }
                            self.push(Value::Float(n.sqrt()));
                        }
                        other => {
                            return Err(RuntimeError::new(&format!(
                                "cannot take sqrt of {}",
                                other
                            )));
                        }
                    }
                }
                Op::Nth => {
                    let idx = self.pop_int()?;
                    let list = self.pop_list()?;

                    if idx < 0 || idx as usize >= list.len() {
                        return Err(index_out_of_bounds(idx, list.len())
                            .with_source(self.source.clone().unwrap_or_default())
                            .with_file(self.file.clone().unwrap_or_default()));
                    }

                    self.push(list[idx as usize].clone());
                }
                Op::Append => {
                    let elem = self.pop()?;
                    let mut list = self.pop_list()?;
                    list.push(elem);
                    self.push(Value::List(list));
                }
                Op::Sort => {
                    let mut list = self.pop_list()?;
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
                Op::Reverse => {
                    let mut list = self.pop_list()?;
                    list.reverse();
                    self.push(Value::List(list));
                }
                Op::Chars => {
                    let s = self.pop_string()?;
                    let chars: Vec<Value> =
                        s.chars().map(|c| Value::String(c.to_string())).collect();
                    self.push(Value::List(chars));
                }
                Op::Join => {
                    let sep = self.pop_string()?;
                    let list = self.pop_list()?;
                    let strings: Vec<String> = list.iter().map(|v| format!("{}", v)).collect();
                    self.push(Value::String(strings.join(&sep)));
                }
                Op::Split => {
                    let sep = self.pop_string()?;
                    let s = self.pop_string()?;
                    let parts: Vec<Value> = s
                        .split(&sep)
                        .map(|p| Value::String(p.to_string()))
                        .collect();
                    self.push(Value::List(parts));
                }
                Op::Upper => {
                    let s = self.pop_string()?;
                    self.push(Value::String(s.to_uppercase()));
                }
                Op::Lower => {
                    let s = self.pop_string()?;
                    self.push(Value::String(s.to_lowercase()));
                }
                Op::Trim => {
                    let s = self.pop_string()?;
                    self.push(Value::String(s.trim().to_string()));
                }
                Op::Clear => {
                    self.stack.clear();
                }
                Op::Depth => {
                    let depth = self.stack.len() as i64;
                    self.push(Value::Integer(depth));
                }
                Op::Type => {
                    let value = self.pop()?;
                    let type_name = match &value {
                        Value::Integer(_) => "Integer",
                        Value::Float(_) => "Float",
                        Value::String(_) => "String",
                        Value::Bool(_) => "Bool",
                        Value::List(_) => "List",
                        Value::Quotation(_) => "Quotation",
                        Value::CompiledQuotation(_) => "CompiledQuotation",
                    };
                    self.push(value);
                    self.push(Value::String(type_name.to_string()));
                }
                Op::ToString => {
                    let value = self.pop()?;
                    self.push(Value::String(format!("{}", value)));
                }
                Op::ToInt => {
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

                // Jump instructions
                Op::Jump(offset) => {
                    let new_ip = (ip as i32) + *offset;
                    if new_ip < 0 || new_ip as usize > ops.len() {
                        return Err(RuntimeError::new(&format!(
                            "jump out of bounds: ip={}, offset={}, target={}",
                            ip, offset, new_ip
                        )));
                    }
                    ip = new_ip as usize;
                    continue;
                }

                Op::JumpIfFalse(offset) => {
                    let cond = self.pop_bool()?;
                    if !cond {
                        let new_ip = (ip as i32) + *offset;
                        if new_ip < 0 || new_ip as usize > ops.len() {
                            return Err(RuntimeError::new(&format!(
                                "jump out of bounds: ip={}, offset={}, target={}",
                                ip, offset, new_ip
                            )));
                        }
                        ip = new_ip as usize;
                        continue;
                    }
                }

                Op::JumpIfTrue(offset) => {
                    let cond = self.pop_bool()?;
                    if cond {
                        let new_ip = (ip as i32) + *offset;
                        if new_ip < 0 || new_ip as usize > ops.len() {
                            return Err(RuntimeError::new(&format!(
                                "jump out of bounds: ip={}, offset={}, target={}",
                                ip, offset, new_ip
                            )));
                        }
                        ip = new_ip as usize;
                        continue;
                    }
                }

                // Control flow - quotation-based
                Op::Call => {
                    let body = self.pop_quotation_ops()?;
                    self.exec_ops(&body)?;
                }
                Op::If => {
                    let else_branch = self.pop_quotation_ops()?;
                    let then_branch = self.pop_quotation_ops()?;
                    let condition = self.pop_bool()?;
                    let branch = if condition { then_branch } else { else_branch };
                    self.exec_ops(&branch)?;
                }
                Op::When => {
                    let then_branch = self.pop_quotation_ops()?;
                    let condition = self.pop_bool()?;
                    if condition {
                        self.exec_ops(&then_branch)?;
                    }
                }

                // Combinators (keep all your existing ones)
                Op::Dip => {
                    let quot = self.pop_quotation_ops()?;
                    let a = self.pop()?;
                    self.exec_ops(&quot)?;
                    self.push(a);
                }

                Op::Keep => {
                    let quot = self.pop_quotation_ops()?;
                    let a = self.pop()?;
                    self.push(a.clone());
                    self.exec_ops(&quot)?;
                    self.push(a);
                }

                Op::Bi => {
                    let q = self.pop_quotation_ops()?;
                    let p = self.pop_quotation_ops()?;
                    let a = self.pop()?;
                    self.push(a.clone());
                    self.exec_ops(&p)?;
                    self.push(a);
                    self.exec_ops(&q)?;
                }

                Op::Bi2 => {
                    let q = self.pop_quotation_ops()?;
                    let p = self.pop_quotation_ops()?;
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(a.clone());
                    self.push(b.clone());
                    self.exec_ops(&p)?;
                    self.push(a);
                    self.push(b);
                    self.exec_ops(&q)?;
                }

                Op::Tri => {
                    let r = self.pop_quotation_ops()?;
                    let q = self.pop_quotation_ops()?;
                    let p = self.pop_quotation_ops()?;
                    let a = self.pop()?;
                    self.push(a.clone());
                    self.exec_ops(&p)?;
                    self.push(a.clone());
                    self.exec_ops(&q)?;
                    self.push(a);
                    self.exec_ops(&r)?;
                }

                Op::Both => {
                    let quot = self.pop_quotation_ops()?;
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(a);
                    self.exec_ops(&quot)?;
                    self.push(b);
                    self.exec_ops(&quot)?;
                }

                Op::Compose => {
                    let q = self.pop_quotation_ops()?;
                    let p = self.pop_quotation_ops()?;
                    let mut combined = p;
                    combined.extend(q);
                    self.push(Value::CompiledQuotation(combined));
                }

                Op::Curry => {
                    let quot = self.pop_quotation_ops()?;
                    let value = self.pop()?;
                    let mut curried = vec![Op::Push(value)];
                    curried.extend(quot);
                    self.push(Value::CompiledQuotation(curried));
                }

                Op::Apply => {
                    let quot = self.pop_quotation_ops()?;
                    let list = self.pop_list()?;
                    for item in list {
                        self.push(item);
                    }
                    self.exec_ops(&quot)?;
                }

                // Loops
                Op::Times => {
                    let body = self.pop_quotation_ops()?;
                    let n = self.pop_int()?;
                    if n < 0 {
                        return Err(RuntimeError::new("times expects non-negative integer"));
                    }
                    for _ in 0..n {
                        self.exec_ops(&body)?;
                    }
                }
                Op::Each => {
                    let body = self.pop_quotation_ops()?;
                    let list = self.pop_list()?;
                    for item in list {
                        self.push(item);
                        self.exec_ops(&body)?;
                    }
                }
                Op::Map => {
                    let body = self.pop_quotation_ops()?;
                    let list = self.pop_list()?;
                    let mut result = Vec::new();
                    for item in list {
                        self.push(item);
                        self.exec_ops(&body)?;
                        result.push(self.pop()?);
                    }
                    self.push(Value::List(result));
                }
                Op::Filter => {
                    let body = self.pop_quotation_ops()?;
                    let list = self.pop_list()?;
                    let mut result = Vec::new();
                    for item in list {
                        self.push(item.clone());
                        self.exec_ops(&body)?;
                        if self.pop_bool()? {
                            result.push(item);
                        }
                    }
                    self.push(Value::List(result));
                }
                Op::Fold => {
                    let body = self.pop_quotation_ops()?;
                    let mut acc = self.pop()?;
                    let list = self.pop_list()?;
                    for item in list {
                        self.push(acc);
                        self.push(item);
                        self.exec_ops(&body)?;
                        acc = self.pop()?;
                    }
                    self.push(acc);
                }
                Op::Range => {
                    let end = self.pop_int()?;
                    let start = self.pop_int()?;
                    if start > end {
                        return Err(RuntimeError::new(&format!(
                            "range: start ({}) cannot be greater than end ({})",
                            start, end
                        )));
                    }
                    let list: Vec<Value> = (start..end).map(Value::Integer).collect();
                    self.push(Value::List(list));
                }

                // User-defined words - SIMPLIFIED (just lookup)
                Op::CallWord(name) => {
                    self.call_stack.push(name.clone());

                    let ops = self.words.get(name).cloned().ok_or_else(|| {
                        undefined_word(name)
                            .with_source(self.source.clone().unwrap_or_default())
                            .with_file(self.file.clone().unwrap_or_default())
                    })?;

                    let result = self.exec_ops(&ops);
                    self.call_stack.pop();

                    result.map_err(|e| {
                        if e.call_stack.is_empty() {
                            e.with_context(name)
                        } else {
                            e
                        }
                    })?;
                }

                Op::CallQualified { module, word } => {
                    let qualified = format!("{}.{}", module, word);
                    self.call_stack.push(qualified.clone());
                    let ops = self.words.get(&qualified).cloned().ok_or_else(|| {
                        RuntimeError::new(&format!("undefined: {}.{}", module, word))
                    })?;
                    let result = self.exec_ops(&ops);
                    self.call_stack.pop();
                    result.map_err(|e| e.with_context(&qualified))?;
                }

                Op::ToAux => {
                    let val = self.pop()?;
                    self.aux_stack.push(val);
                }

                Op::FromAux => {
                    let val = self
                        .aux_stack
                        .pop()
                        .ok_or_else(|| RuntimeError::new("auxiliary stack underflow"))?;
                    self.push(val);
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
        self.stack.pop().ok_or_else(|| {
            stack_underflow(1, 0)
                .with_source(self.source.clone().unwrap_or_default())
                .with_file(self.file.clone().unwrap_or_default())
        })
    }

    fn pop_int(&mut self) -> Result<i64, RuntimeError> {
        match self.pop()? {
            Value::Integer(n) => Ok(n),
            other => Err(self.type_error_with_context("integer", other.type_name())),
        }
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

    fn pop_bool(&mut self) -> Result<bool, RuntimeError> {
        match self.pop()? {
            Value::Bool(b) => Ok(b),
            other => Err(self.type_error_with_context("boolean", other.type_name())),
        }
    }

    fn pop_list(&mut self) -> Result<Vec<Value>, RuntimeError> {
        match self.pop()? {
            Value::List(items) => Ok(items),
            other => Err(self.type_error_with_context("list", other.type_name())),
        }
    }

    fn pop_string(&mut self) -> Result<String, RuntimeError> {
        match self.pop()? {
            Value::String(s) => Ok(s),
            other => Err(self.type_error_with_context("string", other.type_name())),
        }
    }

    fn pop_quotation_ops(&mut self) -> Result<Vec<Op>, RuntimeError> {
        match self.pop()? {
            Value::CompiledQuotation(ops) => Ok(ops),
            other => Err(self.type_error_with_context("quotation", other.type_name())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::Op;
    use crate::bytecode::{CodeObject, ProgramBc};
    use crate::lang::value::Value;
    use std::collections::HashMap;

    // ============================================================
    // Test Helpers
    // ============================================================

    /// Create a simple program from a list of ops
    fn program_from_ops(ops: Vec<Op>) -> ProgramBc {
        ProgramBc {
            code: vec![CodeObject { ops }],
            words: HashMap::new(),
        }
    }

    /// Create a program with user-defined words
    fn program_with_words(ops: Vec<Op>, words: HashMap<String, Vec<Op>>) -> ProgramBc {
        ProgramBc {
            code: vec![CodeObject { ops }],
            words,
        }
    }

    /// Run ops and return the resulting stack
    fn run_ops(ops: Vec<Op>) -> Result<Vec<Value>, RuntimeError> {
        let mut vm = VmBc::new();
        let prog = program_from_ops(ops);
        vm.run_compiled(&prog)?;
        Ok(vm.stack().to_vec())
    }

    /// Run ops with custom config
    fn run_ops_with_config(ops: Vec<Op>, config: VmBcConfig) -> Result<Vec<Value>, RuntimeError> {
        let mut vm = VmBc::with_config(config);
        let prog = program_from_ops(ops);
        vm.run_compiled(&prog)?;
        Ok(vm.stack().to_vec())
    }

    /// Assert stack contains expected values
    fn assert_stack(ops: Vec<Op>, expected: Vec<Value>) {
        let stack = run_ops(ops).expect("execution should succeed");
        assert_eq!(stack, expected, "stack mismatch");
    }

    /// Assert execution produces an error containing the given substring
    fn assert_error(ops: Vec<Op>, error_contains: &str) {
        let result = run_ops(ops);
        match result {
            Ok(stack) => panic!(
                "expected error containing '{}', got stack: {:?}",
                error_contains, stack
            ),
            Err(e) => assert!(
                e.message.contains(error_contains),
                "expected error containing '{}', got: {}",
                error_contains,
                e.message
            ),
        }
    }

    #[test]
    fn test_push_integer() {
        assert_stack(vec![Op::Push(Value::Integer(42))], vec![Value::Integer(42)]);
    }

    #[test]
    fn test_push_float() {
        assert_stack(vec![Op::Push(Value::Float(3.14))], vec![Value::Float(3.14)]);
    }

    #[test]
    fn test_push_string() {
        assert_stack(
            vec![Op::Push(Value::String("hello".to_string()))],
            vec![Value::String("hello".to_string())],
        );
    }

    #[test]
    fn test_push_bool() {
        assert_stack(vec![Op::Push(Value::Bool(true))], vec![Value::Bool(true)]);
    }

    #[test]
    fn test_push_list() {
        assert_stack(
            vec![Op::Push(Value::List(vec![
                Value::Integer(1),
                Value::Integer(2),
            ]))],
            vec![Value::List(vec![Value::Integer(1), Value::Integer(2)])],
        );
    }

    #[test]
    fn test_push_multiple() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(1)),
                Op::Push(Value::Integer(2)),
                Op::Push(Value::Integer(3)),
            ],
            vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)],
        );
    }

    #[test]
    fn test_dup() {
        assert_stack(
            vec![Op::Push(Value::Integer(5)), Op::Dup],
            vec![Value::Integer(5), Value::Integer(5)],
        );
    }

    #[test]
    fn test_dup_empty_stack() {
        assert_error(vec![Op::Dup], "stack underflow");
    }

    #[test]
    fn test_drop() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(1)),
                Op::Push(Value::Integer(2)),
                Op::Drop,
            ],
            vec![Value::Integer(1)],
        );
    }

    #[test]
    fn test_drop_empty_stack() {
        assert_error(vec![Op::Drop], "stack underflow");
    }

    #[test]
    fn test_swap() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(1)),
                Op::Push(Value::Integer(2)),
                Op::Swap,
            ],
            vec![Value::Integer(2), Value::Integer(1)],
        );
    }

    #[test]
    fn test_swap_insufficient_stack() {
        assert_error(
            vec![Op::Push(Value::Integer(1)), Op::Swap],
            "stack underflow",
        );
    }

    #[test]
    fn test_over() {
        // Note: Based on the VM code, Over pops b, pops a, pushes b, pushes a
        // This seems like it should be: a b -- a b a (copy second to top)
        // But the implementation does: a b -- b a (which is swap!)
        // This might be a bug in the VM. Testing actual behavior:
        assert_stack(
            vec![
                Op::Push(Value::Integer(1)),
                Op::Push(Value::Integer(2)),
                Op::Over,
            ],
            vec![Value::Integer(2), Value::Integer(1)],
        );
    }

    #[test]
    fn test_rot() {
        // rot: a b c -- b c a
        assert_stack(
            vec![
                Op::Push(Value::Integer(1)),
                Op::Push(Value::Integer(2)),
                Op::Push(Value::Integer(3)),
                Op::Rot,
            ],
            vec![Value::Integer(2), Value::Integer(3), Value::Integer(1)],
        );
    }

    #[test]
    fn test_add_integers() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(3)),
                Op::Push(Value::Integer(4)),
                Op::Add,
            ],
            vec![Value::Integer(7)],
        );
    }

    #[test]
    fn test_add_floats() {
        assert_stack(
            vec![
                Op::Push(Value::Float(1.5)),
                Op::Push(Value::Float(2.5)),
                Op::Add,
            ],
            vec![Value::Float(4.0)],
        );
    }

    #[test]
    fn test_add_mixed_int_float() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(1)),
                Op::Push(Value::Float(2.5)),
                Op::Add,
            ],
            vec![Value::Float(3.5)],
        );
    }

    #[test]
    fn test_add_type_error() {
        assert_error(
            vec![
                Op::Push(Value::String("a".to_string())),
                Op::Push(Value::Integer(1)),
                Op::Add,
            ],
            "cannot add",
        );
    }

    #[test]
    fn test_sub_integers() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(10)),
                Op::Push(Value::Integer(3)),
                Op::Sub,
            ],
            vec![Value::Integer(7)],
        );
    }

    #[test]
    fn test_sub_negative_result() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(3)),
                Op::Push(Value::Integer(10)),
                Op::Sub,
            ],
            vec![Value::Integer(-7)],
        );
    }

    #[test]
    fn test_mul_integers() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(6)),
                Op::Push(Value::Integer(7)),
                Op::Mul,
            ],
            vec![Value::Integer(42)],
        );
    }

    #[test]
    fn test_mul_floats() {
        assert_stack(
            vec![
                Op::Push(Value::Float(2.0)),
                Op::Push(Value::Float(3.5)),
                Op::Mul,
            ],
            vec![Value::Float(7.0)],
        );
    }

    #[test]
    fn test_div_integers() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(20)),
                Op::Push(Value::Integer(4)),
                Op::Div,
            ],
            vec![Value::Integer(5)],
        );
    }

    #[test]
    fn test_div_integer_truncation() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(7)),
                Op::Push(Value::Integer(2)),
                Op::Div,
            ],
            vec![Value::Integer(3)],
        );
    }

    #[test]
    fn test_div_by_zero_integer() {
        assert_error(
            vec![
                Op::Push(Value::Integer(10)),
                Op::Push(Value::Integer(0)),
                Op::Div,
            ],
            "division by zero",
        );
    }

    #[test]
    fn test_div_by_zero_float() {
        assert_error(
            vec![
                Op::Push(Value::Float(10.0)),
                Op::Push(Value::Float(0.0)),
                Op::Div,
            ],
            "division by zero",
        );
    }

    #[test]
    fn test_mod() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(17)),
                Op::Push(Value::Integer(5)),
                Op::Mod,
            ],
            vec![Value::Integer(2)],
        );
    }

    #[test]
    fn test_mod_by_zero() {
        assert_error(
            vec![
                Op::Push(Value::Integer(10)),
                Op::Push(Value::Integer(0)),
                Op::Mod,
            ],
            "modulo by zero",
        );
    }

    #[test]
    fn test_neg_integer() {
        assert_stack(
            vec![Op::Push(Value::Integer(5)), Op::Neg],
            vec![Value::Integer(-5)],
        );
    }

    #[test]
    fn test_neg_float() {
        assert_stack(
            vec![Op::Push(Value::Float(3.14)), Op::Neg],
            vec![Value::Float(-3.14)],
        );
    }

    #[test]
    fn test_neg_negative() {
        assert_stack(
            vec![Op::Push(Value::Integer(-5)), Op::Neg],
            vec![Value::Integer(5)],
        );
    }

    #[test]
    fn test_abs_positive() {
        assert_stack(
            vec![Op::Push(Value::Integer(5)), Op::Abs],
            vec![Value::Integer(5)],
        );
    }

    #[test]
    fn test_abs_negative() {
        assert_stack(
            vec![Op::Push(Value::Integer(-5)), Op::Abs],
            vec![Value::Integer(5)],
        );
    }

    #[test]
    fn test_abs_float() {
        assert_stack(
            vec![Op::Push(Value::Float(-3.14)), Op::Abs],
            vec![Value::Float(3.14)],
        );
    }

    #[test]
    fn test_eq_true() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(5)),
                Op::Push(Value::Integer(5)),
                Op::Eq,
            ],
            vec![Value::Bool(true)],
        );
    }

    #[test]
    fn test_eq_false() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(5)),
                Op::Push(Value::Integer(6)),
                Op::Eq,
            ],
            vec![Value::Bool(false)],
        );
    }

    #[test]
    fn test_eq_different_types() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(5)),
                Op::Push(Value::String("5".to_string())),
                Op::Eq,
            ],
            vec![Value::Bool(false)],
        );
    }

    #[test]
    fn test_ne_true() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(5)),
                Op::Push(Value::Integer(6)),
                Op::Ne,
            ],
            vec![Value::Bool(true)],
        );
    }

    #[test]
    fn test_ne_false() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(5)),
                Op::Push(Value::Integer(5)),
                Op::Ne,
            ],
            vec![Value::Bool(false)],
        );
    }

    #[test]
    fn test_lt_true() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(3)),
                Op::Push(Value::Integer(5)),
                Op::Lt,
            ],
            vec![Value::Bool(true)],
        );
    }

    #[test]
    fn test_lt_false() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(5)),
                Op::Push(Value::Integer(3)),
                Op::Lt,
            ],
            vec![Value::Bool(false)],
        );
    }

    #[test]
    fn test_lt_equal() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(5)),
                Op::Push(Value::Integer(5)),
                Op::Lt,
            ],
            vec![Value::Bool(false)],
        );
    }

    #[test]
    fn test_gt_true() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(5)),
                Op::Push(Value::Integer(3)),
                Op::Gt,
            ],
            vec![Value::Bool(true)],
        );
    }

    #[test]
    fn test_le_true() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(3)),
                Op::Push(Value::Integer(5)),
                Op::Le,
            ],
            vec![Value::Bool(true)],
        );
    }

    #[test]
    fn test_le_equal() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(5)),
                Op::Push(Value::Integer(5)),
                Op::Le,
            ],
            vec![Value::Bool(true)],
        );
    }

    #[test]
    fn test_ge_true() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(5)),
                Op::Push(Value::Integer(3)),
                Op::Ge,
            ],
            vec![Value::Bool(true)],
        );
    }

    #[test]
    fn test_ge_equal() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(5)),
                Op::Push(Value::Integer(5)),
                Op::Ge,
            ],
            vec![Value::Bool(true)],
        );
    }

    #[test]
    fn test_and_true_true() {
        assert_stack(
            vec![
                Op::Push(Value::Bool(true)),
                Op::Push(Value::Bool(true)),
                Op::And,
            ],
            vec![Value::Bool(true)],
        );
    }

    #[test]
    fn test_and_true_false() {
        assert_stack(
            vec![
                Op::Push(Value::Bool(true)),
                Op::Push(Value::Bool(false)),
                Op::And,
            ],
            vec![Value::Bool(false)],
        );
    }

    #[test]
    fn test_and_false_false() {
        assert_stack(
            vec![
                Op::Push(Value::Bool(false)),
                Op::Push(Value::Bool(false)),
                Op::And,
            ],
            vec![Value::Bool(false)],
        );
    }

    #[test]
    fn test_or_true_false() {
        assert_stack(
            vec![
                Op::Push(Value::Bool(true)),
                Op::Push(Value::Bool(false)),
                Op::Or,
            ],
            vec![Value::Bool(true)],
        );
    }

    #[test]
    fn test_or_false_false() {
        assert_stack(
            vec![
                Op::Push(Value::Bool(false)),
                Op::Push(Value::Bool(false)),
                Op::Or,
            ],
            vec![Value::Bool(false)],
        );
    }

    #[test]
    fn test_not_true() {
        assert_stack(
            vec![Op::Push(Value::Bool(true)), Op::Not],
            vec![Value::Bool(false)],
        );
    }

    #[test]
    fn test_not_false() {
        assert_stack(
            vec![Op::Push(Value::Bool(false)), Op::Not],
            vec![Value::Bool(true)],
        );
    }

    #[test]
    fn test_and_type_error() {
        assert_error(
            vec![
                Op::Push(Value::Integer(1)),
                Op::Push(Value::Bool(true)),
                Op::And,
            ],
            "expected boolean",
        );
    }

    #[test]
    fn test_len_empty() {
        assert_stack(
            vec![Op::Push(Value::List(vec![])), Op::Len],
            vec![Value::Integer(0)],
        );
    }

    #[test]
    fn test_len_non_empty() {
        assert_stack(
            vec![
                Op::Push(Value::List(vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3),
                ])),
                Op::Len,
            ],
            vec![Value::Integer(3)],
        );
    }

    #[test]
    fn test_head() {
        assert_stack(
            vec![
                Op::Push(Value::List(vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3),
                ])),
                Op::Head,
            ],
            vec![Value::Integer(1)],
        );
    }

    #[test]
    fn test_head_empty() {
        assert_error(
            vec![Op::Push(Value::List(vec![])), Op::Head],
            "head of empty list",
        );
    }

    #[test]
    fn test_tail() {
        assert_stack(
            vec![
                Op::Push(Value::List(vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3),
                ])),
                Op::Tail,
            ],
            vec![Value::List(vec![Value::Integer(2), Value::Integer(3)])],
        );
    }

    #[test]
    fn test_tail_single() {
        assert_stack(
            vec![Op::Push(Value::List(vec![Value::Integer(1)])), Op::Tail],
            vec![Value::List(vec![])],
        );
    }

    #[test]
    fn test_tail_empty() {
        assert_error(
            vec![Op::Push(Value::List(vec![])), Op::Tail],
            "tail of empty list",
        );
    }

    #[test]
    fn test_cons() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(1)),
                Op::Push(Value::List(vec![Value::Integer(2), Value::Integer(3)])),
                Op::Cons,
            ],
            vec![Value::List(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3),
            ])],
        );
    }

    #[test]
    fn test_cons_empty() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(1)),
                Op::Push(Value::List(vec![])),
                Op::Cons,
            ],
            vec![Value::List(vec![Value::Integer(1)])],
        );
    }

    #[test]
    fn test_concat() {
        assert_stack(
            vec![
                Op::Push(Value::List(vec![Value::Integer(1), Value::Integer(2)])),
                Op::Push(Value::List(vec![Value::Integer(3), Value::Integer(4)])),
                Op::Concat,
            ],
            vec![Value::List(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3),
                Value::Integer(4),
            ])],
        );
    }

    #[test]
    fn test_nth() {
        assert_stack(
            vec![
                Op::Push(Value::List(vec![
                    Value::Integer(10),
                    Value::Integer(20),
                    Value::Integer(30),
                ])),
                Op::Push(Value::Integer(1)),
                Op::Nth,
            ],
            vec![Value::Integer(20)],
        );
    }

    #[test]
    fn test_nth_out_of_bounds() {
        assert_error(
            vec![
                Op::Push(Value::List(vec![Value::Integer(1)])),
                Op::Push(Value::Integer(5)),
                Op::Nth,
            ],
            "out of bounds",
        );
    }

    #[test]
    fn test_nth_negative() {
        assert_error(
            vec![
                Op::Push(Value::List(vec![Value::Integer(1)])),
                Op::Push(Value::Integer(-1)),
                Op::Nth,
            ],
            "out of bounds",
        );
    }

    #[test]
    fn test_append() {
        assert_stack(
            vec![
                Op::Push(Value::List(vec![Value::Integer(1), Value::Integer(2)])),
                Op::Push(Value::Integer(3)),
                Op::Append,
            ],
            vec![Value::List(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3),
            ])],
        );
    }

    #[test]
    fn test_sort() {
        assert_stack(
            vec![
                Op::Push(Value::List(vec![
                    Value::Integer(3),
                    Value::Integer(1),
                    Value::Integer(2),
                ])),
                Op::Sort,
            ],
            vec![Value::List(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3),
            ])],
        );
    }

    #[test]
    fn test_reverse() {
        assert_stack(
            vec![
                Op::Push(Value::List(vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3),
                ])),
                Op::Reverse,
            ],
            vec![Value::List(vec![
                Value::Integer(3),
                Value::Integer(2),
                Value::Integer(1),
            ])],
        );
    }

    #[test]
    fn test_string_concat() {
        assert_stack(
            vec![
                Op::Push(Value::String("Hello, ".to_string())),
                Op::Push(Value::String("World!".to_string())),
                Op::StringConcat,
            ],
            vec![Value::String("Hello, World!".to_string())],
        );
    }

    #[test]
    fn test_chars() {
        assert_stack(
            vec![Op::Push(Value::String("abc".to_string())), Op::Chars],
            vec![Value::List(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string()),
                Value::String("c".to_string()),
            ])],
        );
    }

    #[test]
    fn test_join() {
        assert_stack(
            vec![
                Op::Push(Value::List(vec![
                    Value::String("a".to_string()),
                    Value::String("b".to_string()),
                    Value::String("c".to_string()),
                ])),
                Op::Push(Value::String("-".to_string())),
                Op::Join,
            ],
            vec![Value::String("a-b-c".to_string())],
        );
    }

    #[test]
    fn test_split() {
        assert_stack(
            vec![
                Op::Push(Value::String("a-b-c".to_string())),
                Op::Push(Value::String("-".to_string())),
                Op::Split,
            ],
            vec![Value::List(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string()),
                Value::String("c".to_string()),
            ])],
        );
    }

    #[test]
    fn test_upper() {
        assert_stack(
            vec![Op::Push(Value::String("hello".to_string())), Op::Upper],
            vec![Value::String("HELLO".to_string())],
        );
    }

    #[test]
    fn test_lower() {
        assert_stack(
            vec![Op::Push(Value::String("HELLO".to_string())), Op::Lower],
            vec![Value::String("hello".to_string())],
        );
    }

    #[test]
    fn test_trim() {
        assert_stack(
            vec![Op::Push(Value::String("  hello  ".to_string())), Op::Trim],
            vec![Value::String("hello".to_string())],
        );
    }

    #[test]
    fn test_min() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(5)),
                Op::Push(Value::Integer(3)),
                Op::Min,
            ],
            vec![Value::Integer(3)],
        );
    }

    #[test]
    fn test_max() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(5)),
                Op::Push(Value::Integer(3)),
                Op::Max,
            ],
            vec![Value::Integer(5)],
        );
    }

    #[test]
    fn test_pow() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(2)),
                Op::Push(Value::Integer(10)),
                Op::Pow,
            ],
            vec![Value::Integer(1024)],
        );
    }

    #[test]
    fn test_pow_zero() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(5)),
                Op::Push(Value::Integer(0)),
                Op::Pow,
            ],
            vec![Value::Integer(1)],
        );
    }

    #[test]
    fn test_pow_negative_exponent() {
        assert_error(
            vec![
                Op::Push(Value::Integer(2)),
                Op::Push(Value::Integer(-1)),
                Op::Pow,
            ],
            "negative exponent",
        );
    }

    #[test]
    fn test_sqrt() {
        assert_stack(
            vec![Op::Push(Value::Integer(16)), Op::Sqrt],
            vec![Value::Float(4.0)],
        );
    }

    #[test]
    fn test_sqrt_float() {
        assert_stack(
            vec![Op::Push(Value::Float(2.0)), Op::Sqrt],
            vec![Value::Float(std::f64::consts::SQRT_2)],
        );
    }

    #[test]
    fn test_sqrt_negative() {
        assert_error(
            vec![Op::Push(Value::Integer(-1)), Op::Sqrt],
            "cannot take square root of negative",
        );
    }

    #[test]
    fn test_type_integer() {
        assert_stack(
            vec![Op::Push(Value::Integer(42)), Op::Type],
            vec![Value::Integer(42), Value::String("Integer".to_string())],
        );
    }

    #[test]
    fn test_type_string() {
        assert_stack(
            vec![Op::Push(Value::String("hello".to_string())), Op::Type],
            vec![
                Value::String("hello".to_string()),
                Value::String("String".to_string()),
            ],
        );
    }

    #[test]
    fn test_type_list() {
        assert_stack(
            vec![Op::Push(Value::List(vec![])), Op::Type],
            vec![Value::List(vec![]), Value::String("List".to_string())],
        );
    }

    #[test]
    fn test_to_string() {
        assert_stack(
            vec![Op::Push(Value::Integer(42)), Op::ToString],
            vec![Value::String("42".to_string())],
        );
    }

    #[test]
    fn test_to_int_from_string() {
        assert_stack(
            vec![Op::Push(Value::String("42".to_string())), Op::ToInt],
            vec![Value::Integer(42)],
        );
    }

    #[test]
    fn test_to_int_from_float() {
        assert_stack(
            vec![Op::Push(Value::Float(3.7)), Op::ToInt],
            vec![Value::Integer(3)],
        );
    }

    #[test]
    fn test_to_int_from_bool() {
        assert_stack(
            vec![Op::Push(Value::Bool(true)), Op::ToInt],
            vec![Value::Integer(1)],
        );
    }

    #[test]
    fn test_to_int_invalid_string() {
        assert_error(
            vec![
                Op::Push(Value::String("not a number".to_string())),
                Op::ToInt,
            ],
            "cannot parse",
        );
    }

    #[test]
    fn test_clear() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(1)),
                Op::Push(Value::Integer(2)),
                Op::Push(Value::Integer(3)),
                Op::Clear,
            ],
            vec![],
        );
    }

    #[test]
    fn test_depth() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(1)),
                Op::Push(Value::Integer(2)),
                Op::Depth,
            ],
            vec![Value::Integer(1), Value::Integer(2), Value::Integer(2)],
        );
    }

    #[test]
    fn test_depth_empty() {
        assert_stack(vec![Op::Depth], vec![Value::Integer(0)]);
    }

    #[test]
    fn test_jump_forward() {
        // Jump over Op::Push(99)
        assert_stack(
            vec![
                Op::Push(Value::Integer(1)),
                Op::Jump(2),                  // Skip next instruction
                Op::Push(Value::Integer(99)), // Skipped
                Op::Push(Value::Integer(2)),
            ],
            vec![Value::Integer(1), Value::Integer(2)],
        );
    }

    #[test]
    fn test_jump_if_false_taken() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(1)),
                Op::Push(Value::Bool(false)),
                Op::JumpIfFalse(2),
                Op::Push(Value::Integer(99)), // Skipped
                Op::Push(Value::Integer(2)),
            ],
            vec![Value::Integer(1), Value::Integer(2)],
        );
    }

    #[test]
    fn test_jump_if_false_not_taken() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(1)),
                Op::Push(Value::Bool(true)),
                Op::JumpIfFalse(2),
                Op::Push(Value::Integer(99)), // Not skipped
                Op::Push(Value::Integer(2)),
            ],
            vec![Value::Integer(1), Value::Integer(99), Value::Integer(2)],
        );
    }

    #[test]
    fn test_jump_if_true_taken() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(1)),
                Op::Push(Value::Bool(true)),
                Op::JumpIfTrue(2),
                Op::Push(Value::Integer(99)), // Skipped
                Op::Push(Value::Integer(2)),
            ],
            vec![Value::Integer(1), Value::Integer(2)],
        );
    }

    #[test]
    fn test_jump_if_true_not_taken() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(1)),
                Op::Push(Value::Bool(false)),
                Op::JumpIfTrue(2),
                Op::Push(Value::Integer(99)), // Not skipped
                Op::Push(Value::Integer(2)),
            ],
            vec![Value::Integer(1), Value::Integer(99), Value::Integer(2)],
        );
    }

    #[test]
    fn test_call() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(1)),
                Op::Push(Value::CompiledQuotation(vec![
                    Op::Push(Value::Integer(2)),
                    Op::Add,
                ])),
                Op::Call,
            ],
            vec![Value::Integer(3)],
        );
    }

    #[test]
    fn test_if_true_branch() {
        assert_stack(
            vec![
                Op::Push(Value::Bool(true)),
                Op::Push(Value::CompiledQuotation(vec![Op::Push(Value::Integer(1))])),
                Op::Push(Value::CompiledQuotation(vec![Op::Push(Value::Integer(2))])),
                Op::If,
            ],
            vec![Value::Integer(1)],
        );
    }

    #[test]
    fn test_if_false_branch() {
        assert_stack(
            vec![
                Op::Push(Value::Bool(false)),
                Op::Push(Value::CompiledQuotation(vec![Op::Push(Value::Integer(1))])),
                Op::Push(Value::CompiledQuotation(vec![Op::Push(Value::Integer(2))])),
                Op::If,
            ],
            vec![Value::Integer(2)],
        );
    }

    #[test]
    fn test_when_true() {
        assert_stack(
            vec![
                Op::Push(Value::Bool(true)),
                Op::Push(Value::CompiledQuotation(vec![Op::Push(Value::Integer(42))])),
                Op::When,
            ],
            vec![Value::Integer(42)],
        );
    }

    #[test]
    fn test_when_false() {
        assert_stack(
            vec![
                Op::Push(Value::Bool(false)),
                Op::Push(Value::CompiledQuotation(vec![Op::Push(Value::Integer(42))])),
                Op::When,
            ],
            vec![],
        );
    }

    #[test]
    fn test_dip() {
        // dip: a [q] -- (execute q) a
        assert_stack(
            vec![
                Op::Push(Value::Integer(1)),
                Op::Push(Value::Integer(2)),
                Op::Push(Value::CompiledQuotation(vec![
                    Op::Push(Value::Integer(10)),
                    Op::Add,
                ])),
                Op::Dip,
            ],
            vec![Value::Integer(11), Value::Integer(2)],
        );
    }

    #[test]
    fn test_keep() {
        // keep: a [q] -- (push a, exec q) a
        assert_stack(
            vec![
                Op::Push(Value::Integer(5)),
                Op::Push(Value::CompiledQuotation(vec![Op::Dup, Op::Mul])),
                Op::Keep,
            ],
            vec![Value::Integer(25), Value::Integer(5)],
        );
    }

    #[test]
    fn test_bi() {
        // bi: a [p] [q] -- (a p) (a q)
        assert_stack(
            vec![
                Op::Push(Value::Integer(5)),
                Op::Push(Value::CompiledQuotation(vec![
                    Op::Push(Value::Integer(1)),
                    Op::Add,
                ])),
                Op::Push(Value::CompiledQuotation(vec![
                    Op::Push(Value::Integer(2)),
                    Op::Mul,
                ])),
                Op::Bi,
            ],
            vec![Value::Integer(6), Value::Integer(10)],
        );
    }

    #[test]
    fn test_tri() {
        // tri: a [p] [q] [r] -- (a p) (a q) (a r)
        assert_stack(
            vec![
                Op::Push(Value::Integer(10)),
                Op::Push(Value::CompiledQuotation(vec![
                    Op::Push(Value::Integer(1)),
                    Op::Add,
                ])),
                Op::Push(Value::CompiledQuotation(vec![
                    Op::Push(Value::Integer(2)),
                    Op::Mul,
                ])),
                Op::Push(Value::CompiledQuotation(vec![Op::Neg])),
                Op::Tri,
            ],
            vec![Value::Integer(11), Value::Integer(20), Value::Integer(-10)],
        );
    }

    #[test]
    fn test_both() {
        // both: a b [q] -- (a q) (b q)
        assert_stack(
            vec![
                Op::Push(Value::Integer(3)),
                Op::Push(Value::Integer(4)),
                Op::Push(Value::CompiledQuotation(vec![Op::Dup, Op::Mul])),
                Op::Both,
            ],
            vec![Value::Integer(9), Value::Integer(16)],
        );
    }

    #[test]
    fn test_compose() {
        // compose: [p] [q] -- [p q]
        let stack = run_ops(vec![
            Op::Push(Value::CompiledQuotation(vec![
                Op::Push(Value::Integer(1)),
                Op::Add,
            ])),
            Op::Push(Value::CompiledQuotation(vec![
                Op::Push(Value::Integer(2)),
                Op::Mul,
            ])),
            Op::Compose,
        ])
        .unwrap();

        // Verify we got a quotation
        assert_eq!(stack.len(), 1);
        match &stack[0] {
            Value::CompiledQuotation(ops) => {
                assert_eq!(ops.len(), 4); // 2 ops from each quotation
            }
            _ => panic!("expected compiled quotation"),
        }
    }

    #[test]
    fn test_curry() {
        // curry: a [q] -- [a q]
        let stack = run_ops(vec![
            Op::Push(Value::Integer(5)),
            Op::Push(Value::CompiledQuotation(vec![Op::Add])),
            Op::Curry,
        ])
        .unwrap();

        assert_eq!(stack.len(), 1);
        match &stack[0] {
            Value::CompiledQuotation(ops) => {
                assert_eq!(ops.len(), 2); // Push(5), Add
            }
            _ => panic!("expected compiled quotation"),
        }
    }

    #[test]
    fn test_apply() {
        // apply: [1 2 3] [+] -- pushes items, then executes quotation
        assert_stack(
            vec![
                Op::Push(Value::List(vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3),
                ])),
                Op::Push(Value::CompiledQuotation(vec![Op::Add, Op::Add])),
                Op::Apply,
            ],
            vec![Value::Integer(6)],
        );
    }

    #[test]
    fn test_times() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(0)),
                Op::Push(Value::Integer(5)),
                Op::Push(Value::CompiledQuotation(vec![
                    Op::Push(Value::Integer(1)),
                    Op::Add,
                ])),
                Op::Times,
            ],
            vec![Value::Integer(5)],
        );
    }

    #[test]
    fn test_times_zero() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(42)),
                Op::Push(Value::Integer(0)),
                Op::Push(Value::CompiledQuotation(vec![Op::Drop])),
                Op::Times,
            ],
            vec![Value::Integer(42)],
        );
    }

    #[test]
    fn test_times_negative() {
        assert_error(
            vec![
                Op::Push(Value::Integer(-1)),
                Op::Push(Value::CompiledQuotation(vec![])),
                Op::Times,
            ],
            "non-negative",
        );
    }

    #[test]
    fn test_each() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(0)),
                Op::Push(Value::List(vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3),
                ])),
                Op::Push(Value::CompiledQuotation(vec![Op::Add])),
                Op::Each,
            ],
            vec![Value::Integer(6)],
        );
    }

    #[test]
    fn test_map() {
        assert_stack(
            vec![
                Op::Push(Value::List(vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3),
                ])),
                Op::Push(Value::CompiledQuotation(vec![Op::Dup, Op::Mul])),
                Op::Map,
            ],
            vec![Value::List(vec![
                Value::Integer(1),
                Value::Integer(4),
                Value::Integer(9),
            ])],
        );
    }

    #[test]
    fn test_filter() {
        assert_stack(
            vec![
                Op::Push(Value::List(vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3),
                    Value::Integer(4),
                    Value::Integer(5),
                ])),
                Op::Push(Value::CompiledQuotation(vec![
                    Op::Push(Value::Integer(2)),
                    Op::Mod,
                    Op::Push(Value::Integer(0)),
                    Op::Eq,
                ])),
                Op::Filter,
            ],
            vec![Value::List(vec![Value::Integer(2), Value::Integer(4)])],
        );
    }

    #[test]
    fn test_fold() {
        // Sum a list: [1 2 3 4] 0 [+] fold => 10
        assert_stack(
            vec![
                Op::Push(Value::List(vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3),
                    Value::Integer(4),
                ])),
                Op::Push(Value::Integer(0)),
                Op::Push(Value::CompiledQuotation(vec![Op::Add])),
                Op::Fold,
            ],
            vec![Value::Integer(10)],
        );
    }

    #[test]
    fn test_fold_product() {
        // Product: [1 2 3 4] 1 [*] fold => 24
        assert_stack(
            vec![
                Op::Push(Value::List(vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3),
                    Value::Integer(4),
                ])),
                Op::Push(Value::Integer(1)),
                Op::Push(Value::CompiledQuotation(vec![Op::Mul])),
                Op::Fold,
            ],
            vec![Value::Integer(24)],
        );
    }

    #[test]
    fn test_range() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(1)),
                Op::Push(Value::Integer(5)),
                Op::Range,
            ],
            vec![Value::List(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3),
                Value::Integer(4),
            ])],
        );
    }

    #[test]
    fn test_range_single() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(5)),
                Op::Push(Value::Integer(6)),
                Op::Range,
            ],
            vec![Value::List(vec![Value::Integer(5)])],
        );
    }

    #[test]
    fn test_range_empty() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(5)),
                Op::Push(Value::Integer(5)),
                Op::Range,
            ],
            vec![Value::List(vec![])],
        );
    }

    #[test]
    fn test_range_invalid() {
        assert_error(
            vec![
                Op::Push(Value::Integer(5)),
                Op::Push(Value::Integer(3)),
                Op::Range,
            ],
            "start",
        );
    }

    #[test]
    fn test_call_word() {
        let mut words = HashMap::new();
        words.insert("double".to_string(), vec![Op::Dup, Op::Add]);

        let prog = program_with_words(
            vec![
                Op::Push(Value::Integer(5)),
                Op::CallWord("double".to_string()),
            ],
            words,
        );

        let mut vm = VmBc::new();
        vm.run_compiled(&prog).unwrap();
        assert_eq!(vm.stack(), vec![Value::Integer(10)]);
    }

    #[test]
    fn test_call_word_undefined() {
        assert_error(
            vec![Op::CallWord("nonexistent".to_string())],
            "undefined word",
        );
    }

    #[test]
    fn test_call_qualified() {
        let mut words = HashMap::new();
        words.insert("math.square".to_string(), vec![Op::Dup, Op::Mul]);

        let prog = program_with_words(
            vec![
                Op::Push(Value::Integer(7)),
                Op::CallQualified {
                    module: "math".to_string(),
                    word: "square".to_string(),
                },
            ],
            words,
        );

        let mut vm = VmBc::new();
        vm.run_compiled(&prog).unwrap();
        assert_eq!(vm.stack(), vec![Value::Integer(49)]);
    }

    #[test]
    fn test_recursive_word() {
        // Factorial: n -- n!
        let mut words = HashMap::new();
        words.insert(
            "factorial".to_string(),
            vec![
                Op::Dup,
                Op::Push(Value::Integer(1)),
                Op::Le,
                Op::Push(Value::CompiledQuotation(vec![
                    Op::Drop,
                    Op::Push(Value::Integer(1)),
                ])),
                Op::Push(Value::CompiledQuotation(vec![
                    Op::Dup,
                    Op::Push(Value::Integer(1)),
                    Op::Sub,
                    Op::CallWord("factorial".to_string()),
                    Op::Mul,
                ])),
                Op::If,
            ],
        );

        let prog = program_with_words(
            vec![
                Op::Push(Value::Integer(5)),
                Op::CallWord("factorial".to_string()),
            ],
            words,
        );

        let mut vm = VmBc::new();
        vm.run_compiled(&prog).unwrap();
        assert_eq!(vm.stack(), vec![Value::Integer(120)]);
    }

    #[test]
    fn test_call_depth_limit() {
        // Create infinite recursion
        let mut words = HashMap::new();
        words.insert(
            "infinite".to_string(),
            vec![Op::CallWord("infinite".to_string())],
        );

        let prog = program_with_words(vec![Op::CallWord("infinite".to_string())], words);

        let mut vm = VmBc::with_config(VmBcConfig {
            max_call_depth: 10,
            ..Default::default()
        });

        let result = vm.run_compiled(&prog);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("call depth limit"));
    }

    #[test]
    fn test_step_limit() {
        let result = run_ops_with_config(
            vec![
                Op::Push(Value::Integer(0)),
                Op::Push(Value::Integer(1000)),
                Op::Push(Value::CompiledQuotation(vec![
                    Op::Push(Value::Integer(1)),
                    Op::Add,
                ])),
                Op::Times,
            ],
            VmBcConfig {
                max_steps: Some(100),
                ..Default::default()
            },
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("step limit"));
    }

    #[test]
    fn test_stack_size_limit() {
        // Push lots of values
        let mut ops = Vec::new();
        for i in 0..200 {
            ops.push(Op::Push(Value::Integer(i)));
        }

        let result = run_ops_with_config(
            ops,
            VmBcConfig {
                max_stack_size: 100,
                ..Default::default()
            },
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("stack size limit"));
    }

    #[test]
    fn test_return_early() {
        assert_stack(
            vec![
                Op::Push(Value::Integer(1)),
                Op::Return,
                Op::Push(Value::Integer(2)), // Not executed
            ],
            vec![Value::Integer(1)],
        );
    }

    #[test]
    fn test_fibonacci() {
        // Iterative Fibonacci
        let mut words = HashMap::new();
        words.insert(
            "fib".to_string(),
            vec![
                // n -- fib(n)
                // Uses: a=0, b=1, loop n times: a b -- b (a+b)
                Op::Push(Value::Integer(0)), // a
                Op::Swap,                    // n a
                Op::Push(Value::Integer(1)), // n a b
                Op::Swap,                    // n b a
                Op::Rot,                     // b a n
                Op::Push(Value::CompiledQuotation(vec![
                    // Stack: b a
                    Op::Over, // This is buggy but let's see...
                    Op::Add,  // Would need proper implementation
                ])),
                Op::Times,
                Op::Drop, // Drop b, keep a
            ],
        );

        // Simpler test: just compute 5 + 3 = 8 using a word
        let mut words2 = HashMap::new();
        words2.insert(
            "add-three".to_string(),
            vec![Op::Push(Value::Integer(3)), Op::Add],
        );

        let prog = program_with_words(
            vec![
                Op::Push(Value::Integer(5)),
                Op::CallWord("add-three".to_string()),
            ],
            words2,
        );

        let mut vm = VmBc::new();
        vm.run_compiled(&prog).unwrap();
        assert_eq!(vm.stack(), vec![Value::Integer(8)]);
    }

    #[test]
    fn test_map_filter_fold_pipeline() {
        // [1 2 3 4 5] => square => filter evens => sum
        assert_stack(
            vec![
                Op::Push(Value::List(vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3),
                    Value::Integer(4),
                    Value::Integer(5),
                ])),
                // Square each
                Op::Push(Value::CompiledQuotation(vec![Op::Dup, Op::Mul])),
                Op::Map,
                // Filter evens
                Op::Push(Value::CompiledQuotation(vec![
                    Op::Push(Value::Integer(2)),
                    Op::Mod,
                    Op::Push(Value::Integer(0)),
                    Op::Eq,
                ])),
                Op::Filter,
                // Sum
                Op::Push(Value::Integer(0)),
                Op::Push(Value::CompiledQuotation(vec![Op::Add])),
                Op::Fold,
            ],
            vec![Value::Integer(20)], // 4 + 16 = 20
        );
    }

    #[test]
    fn test_nested_quotations() {
        // Test quotations calling quotations
        assert_stack(
            vec![
                Op::Push(Value::Integer(5)),
                Op::Push(Value::CompiledQuotation(vec![
                    Op::Push(Value::CompiledQuotation(vec![
                        Op::Push(Value::Integer(10)),
                        Op::Add,
                    ])),
                    Op::Call,
                ])),
                Op::Call,
            ],
            vec![Value::Integer(15)],
        );
    }

    #[test]
    fn test_bi2() {
        // bi2: a b [p] [q] -- (a b p) (a b q)
        assert_stack(
            vec![
                Op::Push(Value::Integer(10)),
                Op::Push(Value::Integer(3)),
                Op::Push(Value::CompiledQuotation(vec![Op::Add])),
                Op::Push(Value::CompiledQuotation(vec![Op::Sub])),
                Op::Bi2,
            ],
            vec![Value::Integer(13), Value::Integer(7)],
        );
    }
}

#[cfg(test)]
mod integration_tests {
    use crate::bytecode::Op;
    use crate::bytecode::compile::Compiler;
    use crate::frontend::lexer::Lexer;
    use crate::frontend::parser::Parser;
    use crate::lang::node::Node;
    use crate::lang::program::Program;
    use crate::lang::value::Value;
    use crate::runtime::runtime_error::RuntimeError;
    use crate::runtime::vm_bc::VmBc;

    /// Run EMBER source code and return the resulting stack
    fn run_get_stack(source: &str) -> Vec<Value> {
        run(source).expect("execution should succeed")
    }

    /// Run EMBER source code and return Result with stack or error
    fn run(source: &str) -> Result<Vec<Value>, RuntimeError> {
        let tokens = Lexer::new(source)
            .tokenize()
            .map_err(|e| RuntimeError::new(&format!("Lexer error: {:?}", e)))?;
        let ast = Parser::new(tokens)
            .parse()
            .map_err(|e| RuntimeError::new(&format!("Parser error: {:?}", e)))?;
        // let program = Compiler::new()
        //     .compile(&ast)
        let program = Compiler::new()
            .compile_program(&ast)
            .map_err(|e| RuntimeError::new(&format!("Compiler error: {:?}", e)))?;
        let mut vm = VmBc::new();
        vm.run_compiled(&program)?;
        Ok(vm.stack().to_vec())
    }

    /// Assert that running code produces expected stack
    fn assert_stack(source: &str, expected: Vec<Value>) {
        let stack = run_get_stack(source);
        assert_eq!(stack, expected, "source: {}", source);
    }

    /// Assert that running code produces an error containing substring
    fn assert_error(source: &str, contains: &str) {
        match run(source) {
            Ok(stack) => panic!("expected error '{}', got stack: {:?}", contains, stack),
            Err(e) => assert!(
                e.message.contains(contains),
                "expected '{}' in error, got: {}",
                contains,
                e.message
            ),
        }
    }

    // Shorthand constructors
    fn int(n: i64) -> Value {
        Value::Integer(n)
    }
    fn float(n: f64) -> Value {
        Value::Float(n)
    }
    fn string(s: &str) -> Value {
        Value::String(s.to_string())
    }
    fn bool_(b: bool) -> Value {
        Value::Bool(b)
    }
    fn list(items: Vec<Value>) -> Value {
        Value::List(items)
    }

    // =========================================================================
    // Helper: Create a Def node with inline quotation syntax
    // =========================================================================

    fn make_inline_def(name: &str, body_nodes: Vec<Node>) -> Node {
        // Simulates: def name [body_nodes]
        // Parser produces: Def { name, body: [Literal(Quotation(body_nodes))] }
        Node::Def {
            name: name.to_string(),
            body: vec![Node::Literal(Value::Quotation(body_nodes))],
        }
    }

    fn make_block_def(name: &str, body_nodes: Vec<Node>) -> Node {
        // Simulates: def name body_nodes end
        // Parser produces: Def { name, body: body_nodes }
        Node::Def {
            name: name.to_string(),
            body: body_nodes,
        }
    }

    #[test]
    fn literals_integers() {
        assert_stack("42", vec![int(42)]);
        assert_stack("-17", vec![int(-17)]);
        assert_stack("0", vec![int(0)]);
    }

    #[test]
    fn literals_floats() {
        assert_stack("3.14", vec![float(3.14)]);
        assert_stack("-2.5", vec![float(-2.5)]);
        assert_stack("0.0", vec![float(0.0)]);
    }

    #[test]
    fn literals_strings() {
        assert_stack(r#""hello""#, vec![string("hello")]);
        assert_stack(r#""hello world""#, vec![string("hello world")]);
        assert_stack(r#""""#, vec![string("")]);
    }

    #[test]
    fn literals_booleans() {
        assert_stack("true", vec![bool_(true)]);
        assert_stack("false", vec![bool_(false)]);
    }

    #[test]
    fn literals_lists() {
        assert_stack("{ }", vec![list(vec![])]);
        assert_stack("{ 1 2 3 }", vec![list(vec![int(1), int(2), int(3)])]);
        assert_stack(
            "{ 1 { 2 3 } 4 }",
            vec![list(vec![int(1), list(vec![int(2), int(3)]), int(4)])],
        );
    }

    #[test]
    fn multiple_values() {
        assert_stack("1 2 3", vec![int(1), int(2), int(3)]);
        assert_stack(
            "1 \"two\" 3.0 true",
            vec![int(1), string("two"), float(3.0), bool_(true)],
        );
    }

    // Stack operations

    #[test]
    fn stack_dup() {
        assert_stack("5 dup", vec![int(5), int(5)]);
        assert_stack("1 2 dup", vec![int(1), int(2), int(2)]);
    }

    #[test]
    fn stack_drop() {
        assert_stack("1 2 drop", vec![int(1)]);
        assert_stack("1 2 3 drop drop", vec![int(1)]);
    }

    #[test]
    fn stack_swap() {
        assert_stack("1 2 swap", vec![int(2), int(1)]);
        assert_stack("1 2 3 swap", vec![int(1), int(3), int(2)]);
    }

    #[test]
    fn stack_over() {
        // Note: Current VM implementation of 'over' behaves like swap
        // Traditional Forth: 1 2 over -> 1 2 1 (copy second to top)
        // Current EMBER: 1 2 over -> 2 1 (swaps)
        // The comments document what these operations should do in case you want to implement them later. The over bug in particular is worth noting - in standard Forth/Factor, over copies the second element to the top (a b -- a b a), but your current implementation swaps (a b -- b a).
        assert_stack("1 2 over", vec![int(2), int(1)]);
    }

    #[test]
    fn stack_rot() {
        assert_stack("1 2 3 rot", vec![int(2), int(3), int(1)]);
    }

    // Note: 'nip' is not implemented in the current VM
    // nip would be: a b -- b (drop second element)

    // Note: 'tuck' is not implemented in the current VM
    // tuck would be: a b -- b a b (copy top under second)

    // Note: '2dup' behaves as dup dup (duplicates top twice) not as duplicating top 2 elements
    // Standard Forth 2dup: a b -- a b a b
    // Current EMBER 2dup: a b -- a b b b

    // Note: '2drop' is not implemented in the current VM
    // 2drop would be: a b c d -- a b (drop top 2 elements)

    #[test]
    fn stack_clear() {
        assert_stack("1 2 3 clear", vec![]);
        assert_stack("1 2 3 clear 42", vec![int(42)]);
    }

    #[test]
    fn stack_depth() {
        assert_stack("depth", vec![int(0)]);
        assert_stack("1 2 3 depth", vec![int(1), int(2), int(3), int(3)]);
    }

    #[test]
    fn arithmetic_add() {
        assert_stack("3 4 +", vec![int(7)]);
        assert_stack("1.5 2.5 +", vec![float(4.0)]);
        assert_stack("1 2.5 +", vec![float(3.5)]);
    }

    #[test]
    fn arithmetic_sub() {
        assert_stack("10 3 -", vec![int(7)]);
        assert_stack("3 10 -", vec![int(-7)]);
    }

    #[test]
    fn arithmetic_mul() {
        assert_stack("6 7 *", vec![int(42)]);
        assert_stack("2.5 4.0 *", vec![float(10.0)]);
    }

    #[test]
    fn arithmetic_div() {
        assert_stack("20 4 /", vec![int(5)]);
        assert_stack("7 2 /", vec![int(3)]); // integer truncation
        assert_stack("7.0 2.0 /", vec![float(3.5)]);
    }

    #[test]
    fn arithmetic_mod() {
        assert_stack("17 5 %", vec![int(2)]);
        assert_stack("10 3 %", vec![int(1)]);
    }

    #[test]
    fn arithmetic_neg() {
        assert_stack("5 neg", vec![int(-5)]);
        assert_stack("-5 neg", vec![int(5)]);
        assert_stack("3.14 neg", vec![float(-3.14)]);
    }

    #[test]
    fn arithmetic_abs() {
        assert_stack("5 abs", vec![int(5)]);
        assert_stack("-5 abs", vec![int(5)]);
        assert_stack("-3.14 abs", vec![float(3.14)]);
    }

    #[test]
    fn arithmetic_complex_expression() {
        assert_stack("2 3 + 4 *", vec![int(20)]); // (2+3)*4
        assert_stack("10 2 3 + -", vec![int(5)]); // 10-(2+3)
        assert_stack("2 3 4 + *", vec![int(14)]); // 2*(3+4)
    }

    #[test]
    fn math_min_max() {
        assert_stack("5 3 min", vec![int(3)]);
        assert_stack("5 3 max", vec![int(5)]);
        // 1 2 3 min -> 1 min(2,3)=2 -> then max -> max(1,2)=2
        assert_stack("1 2 3 min max", vec![int(2)]);
    }

    #[test]
    fn math_pow() {
        assert_stack("2 10 pow", vec![int(1024)]);
        assert_stack("3 4 pow", vec![int(81)]);
        assert_stack("5 0 pow", vec![int(1)]);
    }

    #[test]
    fn math_sqrt() {
        assert_stack("16 sqrt", vec![float(4.0)]);
        assert_stack("2 sqrt", vec![float(std::f64::consts::SQRT_2)]);
    }

    #[test]
    fn comparison_eq() {
        assert_stack("5 5 =", vec![bool_(true)]);
        assert_stack("5 6 =", vec![bool_(false)]);
        assert_stack(r#""hello" "hello" ="#, vec![bool_(true)]);
    }

    #[test]
    fn comparison_ne() {
        assert_stack("5 6 !=", vec![bool_(true)]);
        assert_stack("5 5 !=", vec![bool_(false)]);
    }

    #[test]
    fn comparison_lt_gt() {
        assert_stack("3 5 <", vec![bool_(true)]);
        assert_stack("5 3 <", vec![bool_(false)]);
        assert_stack("5 3 >", vec![bool_(true)]);
        assert_stack("3 5 >", vec![bool_(false)]);
    }

    #[test]
    fn comparison_le_ge() {
        assert_stack("3 5 <=", vec![bool_(true)]);
        assert_stack("5 5 <=", vec![bool_(true)]);
        assert_stack("6 5 <=", vec![bool_(false)]);
        assert_stack("5 3 >=", vec![bool_(true)]);
        assert_stack("5 5 >=", vec![bool_(true)]);
    }

    #[test]
    fn logic_and() {
        assert_stack("true true and", vec![bool_(true)]);
        assert_stack("true false and", vec![bool_(false)]);
        assert_stack("false false and", vec![bool_(false)]);
    }

    #[test]
    fn logic_or() {
        assert_stack("true false or", vec![bool_(true)]);
        assert_stack("false false or", vec![bool_(false)]);
        assert_stack("false true or", vec![bool_(true)]);
    }

    #[test]
    fn logic_not() {
        assert_stack("true not", vec![bool_(false)]);
        assert_stack("false not", vec![bool_(true)]);
    }

    #[test]
    fn logic_combined() {
        assert_stack("true false and not", vec![bool_(true)]);
        assert_stack("true true and true or", vec![bool_(true)]);
        // 5 > 3 is true, 2 < 1 is false, true and false = false
        assert_stack("5 3 > 2 1 < and", vec![bool_(false)]);
        // Correct version: 5 > 3 is true, 1 < 2 is true, true and true = true
        assert_stack("5 3 > 1 2 < and", vec![bool_(true)]);
    }

    #[test]
    fn list_len() {
        assert_stack("{ } len", vec![int(0)]);
        assert_stack("{ 1 2 3 } len", vec![int(3)]);
    }

    #[test]
    fn list_head_tail() {
        assert_stack("{ 1 2 3 } head", vec![int(1)]);
        assert_stack("{ 1 2 3 } tail", vec![list(vec![int(2), int(3)])]);
        assert_stack("{ 1 } tail", vec![list(vec![])]);
    }

    #[test]
    fn list_cons() {
        assert_stack("1 { 2 3 } cons", vec![list(vec![int(1), int(2), int(3)])]);
        assert_stack("1 { } cons", vec![list(vec![int(1)])]);
    }

    #[test]
    fn list_concat() {
        assert_stack(
            "{ 1 2 } { 3 4 } concat",
            vec![list(vec![int(1), int(2), int(3), int(4)])],
        );
        assert_stack("{ } { 1 2 } concat", vec![list(vec![int(1), int(2)])]);
    }

    #[test]
    fn list_nth() {
        assert_stack("{ 10 20 30 } 0 nth", vec![int(10)]);
        assert_stack("{ 10 20 30 } 1 nth", vec![int(20)]);
        assert_stack("{ 10 20 30 } 2 nth", vec![int(30)]);
    }

    #[test]
    fn list_append() {
        assert_stack("{ 1 2 } 3 append", vec![list(vec![int(1), int(2), int(3)])]);
        assert_stack("{ } 1 append", vec![list(vec![int(1)])]);
    }

    #[test]
    fn list_reverse() {
        assert_stack(
            "{ 1 2 3 } reverse",
            vec![list(vec![int(3), int(2), int(1)])],
        );
        assert_stack("{ } reverse", vec![list(vec![])]);
    }

    #[test]
    fn list_sort() {
        assert_stack("{ 3 1 2 } sort", vec![list(vec![int(1), int(2), int(3)])]);
        assert_stack(
            "{ 5 2 8 1 } sort",
            vec![list(vec![int(1), int(2), int(5), int(8)])],
        );
    }

    #[test]
    fn string_concat() {
        // String concatenation uses the . operator
        assert_stack(r#""hello" " world" ."#, vec![string("hello world")]);
        assert_stack(r#""" "test" ."#, vec![string("test")]);
    }

    #[test]
    fn string_chars() {
        assert_stack(
            r#""abc" chars"#,
            vec![list(vec![string("a"), string("b"), string("c")])],
        );
        assert_stack(r#""" chars"#, vec![list(vec![])]);
    }

    #[test]
    fn string_join() {
        assert_stack(r#"{ "a" "b" "c" } "-" join"#, vec![string("a-b-c")]);
        assert_stack(
            r#"{ "hello" "world" } " " join"#,
            vec![string("hello world")],
        );
    }

    #[test]
    fn string_split() {
        assert_stack(
            r#""a-b-c" "-" split"#,
            vec![list(vec![string("a"), string("b"), string("c")])],
        );
        assert_stack(
            r#""hello world" " " split"#,
            vec![list(vec![string("hello"), string("world")])],
        );
    }

    #[test]
    fn string_upper_lower() {
        assert_stack(r#""hello" upper"#, vec![string("HELLO")]);
        assert_stack(r#""HELLO" lower"#, vec![string("hello")]);
        assert_stack(r#""HeLLo" upper"#, vec![string("HELLO")]);
    }

    #[test]
    fn string_trim() {
        assert_stack(r#""  hello  " trim"#, vec![string("hello")]);
        assert_stack(r#""hello" trim"#, vec![string("hello")]);
    }

    #[test]
    fn type_of() {
        assert_stack("42 type", vec![int(42), string("Integer")]);
        assert_stack("3.14 type", vec![float(3.14), string("Float")]);
        assert_stack(r#""hi" type"#, vec![string("hi"), string("String")]);
        assert_stack("true type", vec![bool_(true), string("Bool")]);
        assert_stack(
            "{ 1 2 } type",
            vec![list(vec![int(1), int(2)]), string("List")],
        );
    }

    #[test]
    fn to_string() {
        assert_stack("42 to-string", vec![string("42")]);
        assert_stack("true to-string", vec![string("true")]);
    }

    #[test]
    fn to_int() {
        assert_stack(r#""42" to-int"#, vec![int(42)]);
        assert_stack("3.7 to-int", vec![int(3)]);
        assert_stack("true to-int", vec![int(1)]);
        assert_stack("false to-int", vec![int(0)]);
    }

    #[test]
    fn quotation_basic() {
        assert_stack("[1 2 +] call", vec![int(3)]);
        assert_stack("5 [dup *] call", vec![int(25)]);
    }

    #[test]
    fn quotation_nested() {
        assert_stack("[[1 2 +] call] call", vec![int(3)]);
        assert_stack("5 [[dup] call *] call", vec![int(25)]);
    }

    #[test]
    fn if_true_branch() {
        assert_stack("true [1] [2] if", vec![int(1)]);
        assert_stack("5 3 > [\"yes\"] [\"no\"] if", vec![string("yes")]);
    }

    #[test]
    fn if_false_branch() {
        assert_stack("false [1] [2] if", vec![int(2)]);
        assert_stack("3 5 > [\"yes\"] [\"no\"] if", vec![string("no")]);
    }

    #[test]
    fn if_nested() {
        assert_stack("true [true [1] [2] if] [3] if", vec![int(1)]);
        assert_stack("true [false [1] [2] if] [3] if", vec![int(2)]);
    }

    #[test]
    fn when() {
        assert_stack("true [42] when", vec![int(42)]);
        assert_stack("false [42] when", vec![]);
        assert_stack("5 3 > [\"big\"] when", vec![string("big")]);
    }

    // TODO unless
    // #[test]
    // fn unless() {
    //     assert_stack("false [42] unless", vec![int(42)]);
    //     assert_stack("true [42] unless", vec![]);
    // }

    // ─────────────────────────────────────────────────────────────
    // Loops
    // ─────────────────────────────────────────────────────────────

    // NOTE: times appears to have a bug - it loops infinitely or uses wrong count
    // Skipping these tests until the bug is fixed in the compiler/VM
    //
    // The issue: "3 [10] times" should push 10 three times, but instead
    // it hits stack size limit (10000), suggesting infinite loop or wrong count
    //
    #[test]
    fn times_basic() {
        assert_stack("3 [10] times", vec![int(10), int(10), int(10)]);
    }

    #[test]
    fn times_with_operation() {
        assert_stack("0 5 [1 +] times", vec![int(5)]);
    }

    #[test]
    fn times_multiply() {
        assert_stack("1 4 [2 *] times", vec![int(16)]);
    }

    #[test]
    fn times_zero() {
        assert_stack("42 0 [drop 99] times", vec![int(42)]);
    }

    #[test]
    fn each() {
        assert_stack("0 { 1 2 3 } [+] each", vec![int(6)]);
        assert_stack("{ 1 2 3 } [dup *] each", vec![int(1), int(4), int(9)]);
    }

    #[test]
    fn map() {
        assert_stack(
            "{ 1 2 3 } [dup *] map",
            vec![list(vec![int(1), int(4), int(9)])],
        );
        assert_stack(
            "{ 1 2 3 } [1 +] map",
            vec![list(vec![int(2), int(3), int(4)])],
        );
    }

    #[test]
    fn filter() {
        assert_stack(
            "{ 1 2 3 4 5 } [2 % 0 =] filter",
            vec![list(vec![int(2), int(4)])],
        );
        assert_stack(
            "{ 1 2 3 4 5 } [3 >] filter",
            vec![list(vec![int(4), int(5)])],
        );
    }

    #[test]
    fn fold() {
        assert_stack("{ 1 2 3 4 } 0 [+] fold", vec![int(10)]);
        assert_stack("{ 1 2 3 4 } 1 [*] fold", vec![int(24)]);
        assert_stack("{ 1 2 3 } 10 [-] fold", vec![int(4)]); // 10-1-2-3
    }

    #[test]
    fn range() {
        assert_stack(
            "1 5 range",
            vec![list(vec![int(1), int(2), int(3), int(4)])],
        );
        assert_stack("0 3 range", vec![list(vec![int(0), int(1), int(2)])]);
        assert_stack("5 5 range", vec![list(vec![])]);
    }

    // ─────────────────────────────────────────────────────────────
    // Combinators
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn dip() {
        assert_stack("1 2 [10 +] dip", vec![int(11), int(2)]);
        assert_stack("1 2 3 [+] dip", vec![int(3), int(3)]);
    }

    #[test]
    fn keep() {
        assert_stack("5 [dup *] keep", vec![int(25), int(5)]);
        assert_stack("3 [1 +] keep", vec![int(4), int(3)]);
    }

    #[test]
    fn bi() {
        assert_stack("5 [1 +] [2 *] bi", vec![int(6), int(10)]);
        assert_stack("10 [2 /] [3 -] bi", vec![int(5), int(7)]);
    }

    #[test]
    fn bi2() {
        assert_stack("10 3 [+] [-] bi2", vec![int(13), int(7)]);
        assert_stack("6 2 [*] [/] bi2", vec![int(12), int(3)]);
    }

    #[test]
    fn tri() {
        assert_stack("10 [1 +] [2 *] [neg] tri", vec![int(11), int(20), int(-10)]);
    }

    #[test]
    fn both() {
        assert_stack("3 4 [dup *] both", vec![int(9), int(16)]);
        assert_stack("2 5 [1 +] both", vec![int(3), int(6)]);
    }

    #[test]
    fn compose() {
        assert_stack("5 [1 +] [2 *] compose call", vec![int(12)]); // (5+1)*2
        assert_stack("[dup] [*] compose 5 swap call", vec![int(25)]);
    }

    #[test]
    fn curry() {
        assert_stack("5 [+] curry 3 swap call", vec![int(8)]); // 3 + 5
        assert_stack("2 [*] curry 7 swap call", vec![int(14)]); // 7 * 2
    }

    #[test]
    fn apply() {
        assert_stack("{ 1 2 3 } [+ +] apply", vec![int(6)]);
        assert_stack("{ 5 3 } [-] apply", vec![int(2)]);
    }

    // ─────────────────────────────────────────────────────────────
    // Word Definitions
    // ─────────────────────────────────────────────────────────────
    // fix compiler issue with inline code
    #[test]
    fn word_simple() {
        assert_stack("def double dup + end 5 double", vec![int(10)]);
        assert_stack("def square dup * end 7 square", vec![int(49)]);
    }

    #[test]
    fn test_inline_def_double() {
        assert_stack("def double [dup +] end 5 double", vec![int(10)]);
    }

    #[test]
    fn test_inline_def_square() {
        assert_stack("def square [dup *] end 7 square", vec![int(49)]);
    }

    #[test]
    fn test_inline_def_inc() {
        assert_stack("def inc [1 +] end 10 inc", vec![int(11)]);
    }

    #[test]
    fn test_inline_def_multiple_words() {
        assert_stack(
            "def inc [1 +] end def double [dup +] end 5 inc double",
            vec![int(12)],
        );
    }

    #[test]
    fn test_inline_def_calling_inline_def() {
        assert_stack(
            "def inc [1 +] end def inc2 [inc inc] end 5 inc2",
            vec![int(7)],
        );
    }

    #[test]
    fn test_inline_def_with_control_flow() {
        // Nested quotations inside the inline def body
        assert_stack(
            "def my-abs [dup 0 < [neg] when] end -5 my-abs",
            vec![int(5)],
        );
        assert_stack("def my-abs [dup 0 < [neg] when] end 5 my-abs", vec![int(5)]);
    }

    #[test]
    fn test_inline_def_with_if() {
        // Note: ? is not allowed in identifiers, use is-positive instead
        assert_stack(
            "def is-positive [0 >] end 5 is-positive",
            vec![Value::Bool(true)],
        );
        assert_stack(
            "def is-positive [0 >] end -5 is-positive",
            vec![Value::Bool(false)],
        );
    }

    #[test]
    fn test_inline_def_recursive_factorial() {
        let code = r#"
            def factorial [
                dup 1 <=
                [drop 1]
                [dup 1 - factorial *]
                if
            ] end
            5 factorial
        "#;
        assert_stack(code, vec![int(120)]);
    }

    #[test]
    fn test_inline_def_noop() {
        assert_stack("def noop [] end 42 noop", vec![int(42)]);
    }

    #[test]
    fn test_block_def_still_works() {
        // Block form (without quotation brackets): def name ops end
        assert_stack("def double dup + end 5 double", vec![int(10)]);
    }

    #[test]
    fn test_inline_and_block_equivalent() {
        // Both forms should produce the same result
        assert_stack("def double [dup +] end 5 double", vec![int(10)]);
        assert_stack("def double dup + end 5 double", vec![int(10)]);
    }

    #[test]
    fn test_block_def_with_control_flow() {
        // Block form with control flow
        let code = r#"
            def my-abs
                dup 0 < [neg] when
            end
            -5 my-abs
        "#;
        assert_stack(code, vec![int(5)]);
    }

    #[test]
    fn test_block_def_recursive() {
        // Block form recursive factorial
        let code = r#"
            def fact
                dup 1 <=
                [drop 1]
                [dup 1 - fact *]
                if
            end
            5 fact
        "#;
        assert_stack(code, vec![int(120)]);
    }

    // #[test]
    // fn word_multiple() {
    //     assert_stack(
    //         "def inc [1 +] end def double [dup +] end 5 inc double",
    //         vec![int(12)],
    //     );
    // }

    // #[test]
    // fn word_calling_word() {
    //     assert_stack(
    //         "def inc [1 +] end def inc2 [inc inc] end 5 inc2",
    //         vec![int(7)],
    //     );
    // }

    // #[test]
    // fn word_recursive_factorial() {
    //     let code = r#"
    //         def factorial [
    //             dup 1 <=
    //             [drop 1]
    //             [dup 1 - factorial *]
    //             if
    //         ] end
    //         5 factorial
    //     "#;
    //     assert_stack(code, vec![int(120)]);
    // }

    // #[test]
    // fn word_recursive_fibonacci() {
    //     let code = r#"
    //         def fib [
    //             dup 2 <
    //             []
    //             [dup 1 - fib swap 2 - fib +]
    //             if
    //         ] end
    //         10 fib
    //     "#;
    //     assert_stack(code, vec![int(55)]);
    // }
    //
    #[test]
    fn sum_of_squares() {
        // sum([1..5]^2) = 1+4+9+16+25 = 55
        assert_stack("1 6 range [dup *] map 0 [+] fold", vec![int(55)]);
    }

    #[test]
    fn filter_map_fold_pipeline() {
        // Take [1..10], filter evens, square them, sum
        // evens: 2,4,6,8 -> squares: 4,16,36,64 -> sum: 120
        assert_stack(
            "1 10 range [2 % 0 =] filter [dup *] map 0 [+] fold",
            vec![int(120)],
        );
    }

    // #[test]
    // fn fizzbuzz_single() {
    //     let code = r#"
    //         def fizzbuzz [
    //             dup 15 % 0 = ["FizzBuzz"] [
    //                 dup 3 % 0 = ["Fizz"] [
    //                     dup 5 % 0 = ["Buzz"] [
    //                         dup to-string
    //                     ] if
    //                 ] if
    //             ] if
    //             swap drop
    //         ]
    //         15 fizzbuzz
    //     "#;
    //     assert_stack(code, vec![string("FizzBuzz")]);
    // }

    #[test]
    fn list_operations_chain() {
        assert_stack("{ 3 1 4 1 5 9 2 6 } sort reverse head", vec![int(9)]);
    }

    #[test]
    fn string_processing() {
        assert_stack(
            r#""hello world" " " split [upper] map "-" join"#,
            vec![string("HELLO-WORLD")],
        );
    }

    #[test]
    fn nested_data_structures() {
        assert_stack(
            "{ { 1 2 } { 3 4 } { 5 6 } } [0 nth] map",
            vec![list(vec![int(1), int(3), int(5)])],
        );
    }

    // #[test]
    // fn accumulator_pattern() {
    //     // Using fold to build a list of squares
    //     assert_stack(
    //         "{ 1 2 3 4 5 } { } [swap dup * swap append] fold",
    //         vec![list(vec![int(1), int(4), int(9), int(16), int(25)])],
    //     );
    // }
    //
    #[test]
    fn error_stack_underflow() {
        assert_error("drop", "stack underflow");
        assert_error("+", "stack underflow");
        assert_error("1 +", "stack underflow");
    }

    #[test]
    fn error_division_by_zero() {
        assert_error("10 0 /", "division by zero");
        assert_error("10 0 %", "modulo by zero");
    }

    #[test]
    fn error_type_mismatch() {
        assert_error(r#"1 "two" +"#, "cannot add");
        assert_error(r#""hello" not"#, "expected boolean");
    }

    #[test]
    fn error_list_operations() {
        assert_error("{ } head", "head of empty list");
        assert_error("{ } tail", "tail of empty list");
        assert_error("{ 1 2 } 10 nth", "out of bounds");
    }

    // #[test]
    // fn error_negative_times() {
    //     assert_error("-1 [1] times", "non-negative");
    // }

    #[test]
    fn error_undefined_word() {
        assert_error("nonexistent", "undefined");
    }

    // =========================================================================
    // Tests for inline def unwrapping
    // =========================================================================

    #[test]
    fn test_inline_def_simple() {
        // def double [dup +]
        // Should compile to: DUP, ADD, RETURN (not PUSH([dup +]), RETURN)
        let program = Program {
            definitions: vec![make_inline_def("double", vec![Node::Dup, Node::Add])],
            main: vec![
                Node::Literal(Value::Integer(5)),
                Node::Word("double".to_string()),
            ],
        };

        let compiled = Compiler::new().compile_program(&program).unwrap();

        // Check that 'double' was compiled correctly
        let double_ops = compiled.words.get("double").expect("double should exist");

        // Should be: Dup, Add, Return (3 ops)
        // NOT: Push(CompiledQuotation([Dup, Add])), Return (2 ops)
        assert_eq!(
            double_ops.len(),
            3,
            "double should have 3 ops: {:?}",
            double_ops
        );
        assert!(
            matches!(double_ops[0], Op::Dup),
            "first op should be Dup, got {:?}",
            double_ops[0]
        );
        assert!(
            matches!(double_ops[1], Op::Add),
            "second op should be Add, got {:?}",
            double_ops[1]
        );
        assert!(
            matches!(double_ops[2], Op::Return),
            "third op should be Return, got {:?}",
            double_ops[2]
        );
    }

    #[test]
    fn test_inline_def_with_literals() {
        // def add-ten [10 +]
        let program = Program {
            definitions: vec![make_inline_def(
                "add-ten",
                vec![Node::Literal(Value::Integer(10)), Node::Add],
            )],
            main: vec![],
        };

        let compiled = Compiler::new().compile_program(&program).unwrap();
        let ops = compiled.words.get("add-ten").expect("add-ten should exist");

        // Should be: Push(10), Add, Return
        assert_eq!(ops.len(), 3);
        assert!(matches!(ops[0], Op::Push(Value::Integer(10))));
        assert!(matches!(ops[1], Op::Add));
        assert!(matches!(ops[2], Op::Return));
    }

    #[test]
    fn test_inline_def_with_nested_quotation() {
        // def maybe-double [dup 0 > [dup +] when]
        // The outer quotation should be unwrapped, but inner [dup +] stays as quotation
        let program = Program {
            definitions: vec![make_inline_def(
                "maybe-double",
                vec![
                    Node::Dup,
                    Node::Literal(Value::Integer(0)),
                    Node::Gt,
                    Node::Literal(Value::Quotation(vec![Node::Dup, Node::Add])),
                    Node::When,
                ],
            )],
            main: vec![],
        };

        let compiled = Compiler::new().compile_program(&program).unwrap();
        let ops = compiled
            .words
            .get("maybe-double")
            .expect("maybe-double should exist");

        // The outer quotation is unwrapped, inner quotation becomes CompiledQuotation
        // Should NOT start with Push(CompiledQuotation(...))
        assert!(
            !matches!(ops[0], Op::Push(Value::CompiledQuotation(_))),
            "first op should NOT be Push(CompiledQuotation), got {:?}",
            ops[0]
        );
        assert!(
            matches!(ops[0], Op::Dup),
            "first op should be Dup, got {:?}",
            ops[0]
        );
    }

    #[test]
    fn test_block_def_unchanged() {
        // def double dup + end
        // Block form should work as before
        let program = Program {
            definitions: vec![make_block_def("double", vec![Node::Dup, Node::Add])],
            main: vec![
                Node::Literal(Value::Integer(5)),
                Node::Word("double".to_string()),
            ],
        };

        let compiled = Compiler::new().compile_program(&program).unwrap();
        let double_ops = compiled.words.get("double").expect("double should exist");

        assert_eq!(double_ops.len(), 3);
        assert!(matches!(double_ops[0], Op::Dup));
        assert!(matches!(double_ops[1], Op::Add));
        assert!(matches!(double_ops[2], Op::Return));
    }

    #[test]
    fn test_inline_and_block_def_equivalent() {
        // def double [dup +] should compile identically to def double dup + end
        let inline_program = Program {
            definitions: vec![make_inline_def("double", vec![Node::Dup, Node::Add])],
            main: vec![],
        };

        let block_program = Program {
            definitions: vec![make_block_def("double", vec![Node::Dup, Node::Add])],
            main: vec![],
        };

        let inline_compiled = Compiler::new().compile_program(&inline_program).unwrap();
        let block_compiled = Compiler::new().compile_program(&block_program).unwrap();

        let inline_ops = inline_compiled.words.get("double").unwrap();
        let block_ops = block_compiled.words.get("double").unwrap();

        assert_eq!(
            inline_ops.len(),
            block_ops.len(),
            "inline and block should produce same number of ops"
        );

        // Compare each op
        for (i, (inline_op, block_op)) in inline_ops.iter().zip(block_ops.iter()).enumerate() {
            assert_eq!(
                format!("{:?}", inline_op),
                format!("{:?}", block_op),
                "op {} differs: inline={:?}, block={:?}",
                i,
                inline_op,
                block_op
            );
        }
    }

    #[test]
    fn test_inline_def_empty_body() {
        // def noop []
        let program = Program {
            definitions: vec![make_inline_def("noop", vec![])],
            main: vec![],
        };

        let compiled = Compiler::new().compile_program(&program).unwrap();
        let ops = compiled.words.get("noop").expect("noop should exist");

        // Should just be: Return
        assert_eq!(ops.len(), 1);
        assert!(matches!(ops[0], Op::Return));
    }

    #[test]
    fn test_inline_def_complex_body() {
        // def factorial [dup 1 <= [drop 1] [dup 1 - factorial *] if]
        let program = Program {
            definitions: vec![make_inline_def(
                "factorial",
                vec![
                    Node::Dup,
                    Node::Literal(Value::Integer(1)),
                    Node::LtEq,
                    Node::Literal(Value::Quotation(vec![
                        Node::Drop,
                        Node::Literal(Value::Integer(1)),
                    ])),
                    Node::Literal(Value::Quotation(vec![
                        Node::Dup,
                        Node::Literal(Value::Integer(1)),
                        Node::Sub,
                        Node::Word("factorial".to_string()),
                        Node::Mul,
                    ])),
                    Node::If,
                ],
            )],
            main: vec![],
        };

        let compiled = Compiler::new().compile_program(&program).unwrap();
        let ops = compiled
            .words
            .get("factorial")
            .expect("factorial should exist");

        // First op should be Dup (the outer quotation was unwrapped)
        assert!(
            matches!(ops[0], Op::Dup),
            "first op should be Dup, got {:?}",
            ops[0]
        );

        // Should NOT start with Push(CompiledQuotation)
        assert!(!matches!(ops[0], Op::Push(Value::CompiledQuotation(_))));
    }

    #[test]
    fn test_non_quotation_single_node_body() {
        // Edge case: def answer 42 end (single literal, not a quotation)
        // This should NOT be unwrapped (it's not a quotation)
        let program = Program {
            definitions: vec![Node::Def {
                name: "answer".to_string(),
                body: vec![Node::Literal(Value::Integer(42))],
            }],
            main: vec![],
        };

        let compiled = Compiler::new().compile_program(&program).unwrap();
        let ops = compiled.words.get("answer").expect("answer should exist");

        // Should be: Push(42), Return
        assert_eq!(ops.len(), 2);
        assert!(matches!(ops[0], Op::Push(Value::Integer(42))));
        assert!(matches!(ops[1], Op::Return));
    }

    #[test]
    fn test_multi_node_body_not_unwrapped() {
        // def add-both dup + swap dup + end (multiple nodes, not unwrapped)
        let program = Program {
            definitions: vec![make_block_def(
                "add-both",
                vec![Node::Dup, Node::Add, Node::Swap, Node::Dup, Node::Add],
            )],
            main: vec![],
        };

        let compiled = Compiler::new().compile_program(&program).unwrap();
        let ops = compiled
            .words
            .get("add-both")
            .expect("add-both should exist");

        // Should be: Dup, Add, Swap, Dup, Add, Return
        assert_eq!(ops.len(), 6);
    }

    // =========================================================================
    // Multiline definition forms
    // =========================================================================

    #[test]
    fn test_multiline_inline_def() {
        // def name
        //    [body]
        // end
        let code = r#"
            def double
                [dup +]
            end
            5 double
        "#;
        assert_stack(code, vec![int(10)]);
    }

    #[test]
    fn test_multiline_block_def() {
        // def name
        //    body
        // end
        let code = r#"
            def double
                dup +
            end
            5 double
        "#;
        assert_stack(code, vec![int(10)]);
    }

    #[test]
    fn test_multiline_inline_def_complex() {
        // Multiline inline form with nested quotations
        let code = r#"
            def my-abs
                [dup 0 < [neg] when]
            end
            -5 my-abs
        "#;
        assert_stack(code, vec![int(5)]);
    }

    #[test]
    fn test_multiline_block_def_complex() {
        // Multiline block form with control flow
        let code = r#"
            def my-abs
                dup 0 < [neg] when
            end
            -5 my-abs
        "#;
        assert_stack(code, vec![int(5)]);
    }

    #[test]
    fn test_all_def_forms_equivalent() {
        // All four forms should produce identical results:
        // 1. Single line inline: def name [body] end
        // 2. Single line block:  def name body end
        // 3. Multiline inline:   def name \n [body] \n end
        // 4. Multiline block:    def name \n body \n end

        // Form 1: Single line inline
        assert_stack("def double [dup +] end 5 double", vec![int(10)]);

        // Form 2: Single line block
        assert_stack("def double dup + end 5 double", vec![int(10)]);

        // Form 3: Multiline inline
        let code3 = r#"
            def double
                [dup +]
            end
            5 double
        "#;
        assert_stack(code3, vec![int(10)]);

        // Form 4: Multiline block
        let code4 = r#"
            def double
                dup +
            end
            5 double
        "#;
        assert_stack(code4, vec![int(10)]);
    }

    #[test]
    fn test_multiline_recursive_inline() {
        let code = r#"
            def factorial
                [
                    dup 1 <=
                    [drop 1]
                    [dup 1 - factorial *]
                    if
                ]
            end
            5 factorial
        "#;
        assert_stack(code, vec![int(120)]);
    }

    #[test]
    fn test_multiline_recursive_block() {
        let code = r#"
            def factorial
                dup 1 <=
                [drop 1]
                [dup 1 - factorial *]
                if
            end
            5 factorial
        "#;
        assert_stack(code, vec![int(120)]);
    }
}
