use crate::ast::{Node, Program, UseItem, Value};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::runtime_error::RuntimeError;
use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

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
}

impl VM {
    pub fn new() -> Self {
        VM {
            stack: Vec::new(),
            words: HashMap::new(),
            aliases: HashMap::new(),
            imported: HashSet::new(),
            current_dir: None,
            imported_programs: Vec::new(),
        }
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

    fn execute_node(&mut self, node: &Node) -> Result<(), RuntimeError> {
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
            Node::Add => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = match (a, b) {
                    (Value::Integer(a), Value::Integer(b)) => Value::Integer(a + b),
                    (Value::Float(a), Value::Float(b)) => Value::Float(a + b),
                    (Value::Integer(a), Value::Float(b)) => Value::Float(a as f64 + b),
                    (Value::Float(a), Value::Integer(b)) => Value::Float(a + b as f64),
                    (a, b) => {
                        return Err(RuntimeError::new(&format!("cannot add {} and {}", a, b)));
                    }
                };
                self.push(result);
            }
            Node::Sub => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = match (a, b) {
                    (Value::Integer(a), Value::Integer(b)) => Value::Integer(a - b),
                    (Value::Float(a), Value::Float(b)) => Value::Float(a - b),
                    (Value::Integer(a), Value::Float(b)) => Value::Float(a as f64 - b),
                    (Value::Float(a), Value::Integer(b)) => Value::Float(a - b as f64),
                    (a, b) => {
                        return Err(RuntimeError::new(&format!(
                            "cannot subtract {} and {}",
                            a, b
                        )));
                    }
                };
                self.push(result);
            }
            Node::Mul => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = match (a, b) {
                    (Value::Integer(a), Value::Integer(b)) => Value::Integer(a * b),
                    (Value::Float(a), Value::Float(b)) => Value::Float(a * b),
                    (Value::Integer(a), Value::Float(b)) => Value::Float(a as f64 * b),
                    (Value::Float(a), Value::Integer(b)) => Value::Float(a * b as f64),
                    (a, b) => {
                        return Err(RuntimeError::new(&format!(
                            "cannot multiply {} and {}",
                            a, b
                        )));
                    }
                };
                self.push(result);
            }
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
            Node::Lt => {
                let b = self.pop_int()?;
                let a = self.pop_int()?;
                self.push(Value::Bool(a < b));
            }
            Node::Gt => {
                let b = self.pop_int()?;
                let a = self.pop_int()?;
                self.push(Value::Bool(a > b));
            }
            Node::LtEq => {
                let b = self.pop_int()?;
                let a = self.pop_int()?;
                self.push(Value::Bool(a <= b));
            }
            Node::GtEq => {
                let b = self.pop_int()?;
                let a = self.pop_int()?;
                self.push(Value::Bool(a >= b));
            }

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
                let list = self.pop_list()?;
                for item in list {
                    self.push(item);
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
            // TODO

            // User-defined word (checks aliases first)
            Node::Word(name) => {
                let body = self
                    .lookup_word(name)
                    .ok_or_else(|| RuntimeError::new(&format!("undefined word: {}", name)))?;
                self.execute(&body)?;
            }

            // Qualified word (Module.word)
            Node::QualifiedWord { module, word } => {
                let qualified = format!("{}.{}", module, word);
                let body = self
                    .words
                    .get(&qualified)
                    .ok_or_else(|| RuntimeError::new(&format!("undefined: {}.{}", module, word)))?
                    .clone();
                self.execute(&body)?;
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

    fn lookup_word(&self, name: &str) -> Option<Vec<Node>> {
        if let Some(qualified) = self.aliases.get(name) {
            return self.words.get(qualified).cloned();
        }

        self.words.get(name).cloned()
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
}
