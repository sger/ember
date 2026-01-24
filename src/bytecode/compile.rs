use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use crate::{
    bytecode::{CodeObject, Op, ProgramBc, compile_error::CompileError},
    frontend::{lexer::Lexer, parser::Parser},
    lang::{node::Node, program::Program, use_item::UseItem, value::Value},
};

pub struct Compiler {
    /// Output bytecode program
    program_bc: ProgramBc,

    /// Accumulated word definitions (as AST nodes, for lazy compilation)
    words: HashMap<String, Vec<Node>>,

    /// Files already included (prevents duplicates)
    included: HashSet<PathBuf>,

    /// Aliases from 'use' statements
    aliases: HashMap<String, String>,
}

#[allow(dead_code)]
impl Compiler {
    pub fn new() -> Self {
        Self {
            program_bc: ProgramBc {
                code: vec![CodeObject::new()],
                words: HashMap::new(),
            },
            words: HashMap::new(),
            included: HashSet::new(),
            aliases: HashMap::new(),
        }
    }

    pub fn compile_from_file(mut self, path: &Path) -> Result<ProgramBc, CompileError> {
        // Load the file and all its imports (recursively)
        let main_program = self.load_file_recursive(path)?;

        // Clone the words HashMap to avoid borrow checker issues
        // (We need to iterate over words while calling compile_nodes which borrows self mutably)
        let words_to_compile: Vec<(String, Vec<Node>)> = self
            .words
            .iter()
            .map(|(name, body)| (name.clone(), body.clone()))
            .collect();

        // Now compile all words to bytecode
        for (name, body) in words_to_compile {
            let mut word_ops = self.compile_nodes(&body)?;
            word_ops.push(Op::Return);
            self.program_bc.words.insert(name, word_ops);
        }

        // Compile main
        let mut main_ops = self.compile_nodes(&main_program)?;
        main_ops.push(Op::Return);
        self.program_bc.code[0].ops = main_ops;

        Ok(self.program_bc)
    }

    /// Compile from AST (for backward compatibility, REPL, testing)
    /// Does NOT handle imports - use compile_from_file for that
    pub fn compile_program(mut self, program: &Program) -> Result<ProgramBc, CompileError> {
        // Process definitions
        for def in &program.definitions {
            self.process_definition(def, None)?;
        }

        // Clone words to avoid borrow checker issues
        let words_to_compile: Vec<(String, Vec<Node>)> = self
            .words
            .iter()
            .map(|(name, body)| (name.clone(), body.clone()))
            .collect();

        // Compile accumulated words
        for (name, body) in words_to_compile {
            let mut word_ops = self.compile_nodes(&body)?;
            word_ops.push(Op::Return);
            self.program_bc.words.insert(name, word_ops);
        }

        // Compile main
        let mut main_ops = self.compile_nodes(&program.main)?;
        main_ops.push(Op::Return);
        self.program_bc.code[0].ops = main_ops;

        Ok(self.program_bc)
    }

    fn load_file_recursive(&mut self, path: &Path) -> Result<Vec<Node>, CompileError> {
        // Normalize to .em extension
        let mut path_buf = path.to_path_buf();

        if path_buf.extension().is_none() {
            path_buf.set_extension("em");
        }

        // Canonicalize to absolute path
        let canonical = path_buf.canonicalize().map_err(|e| {
            CompileError::new(&format!("cannot find file '{}': {}", path.display(), e))
        })?;

        // Already included? Skip (prevents infinite loops and duplicate definitions)
        if !self.included.insert(canonical.clone()) {
            return Ok(Vec::new()); // Return empty - already processed
        }

        // Get base directory for resolving imports
        let base_dir = canonical
            .parent()
            .ok_or_else(|| CompileError::new("cannot get parent directory"))?;

        // Read and parse
        let source = std::fs::read_to_string(&canonical).map_err(|e| {
            CompileError::new(&format!("cannot read '{}': {}", canonical.display(), e))
        })?;

        let mut lexer = Lexer::new(&source);
        let tokens = lexer
            .tokenize()
            .map_err(|e| CompileError::new(&format!("in '{}': {}", canonical.display(), e)))?;

        let mut parser = Parser::new(tokens);
        let program = parser
            .parse()
            .map_err(|e| CompileError::new(&format!("in '{}': {}", canonical.display(), e)))?;

        // Process imports FIRST (depth-first, like Forth INCLUDE)
        for def in &program.definitions {
            if let Node::Import(import_path) = def {
                let import_full = base_dir.join(import_path);
                self.load_file_recursive(&import_full)?;
                // Note: we discard the result because definitions are accumulated
                // in self.words, not returned
            }
        }

        // Now process definitions from THIS file
        for def in &program.definitions {
            self.process_definition(def, Some(&canonical))?;
        }

        // Return main code (only meaningful for the top-level file)
        Ok(program.main)
    }

    fn process_definition(
        &mut self,
        def: &Node,
        source_file: Option<&Path>,
    ) -> Result<(), CompileError> {
        match def {
            Node::Def { name, body } => {
                if self.words.contains_key(name) {
                    // Allow redefinition with a warning (Forth-style)
                    eprintln!(
                        "Warning: redefining word '{}' {}",
                        name,
                        if let Some(path) = source_file {
                            format!("in {}", path.display())
                        } else {
                            String::new()
                        }
                    );
                }

                // FIX: Unwrap inline quotation syntax: def name [body]
                // If body is exactly one node and it's a quotation literal,
                // use the quotation's contents as the body instead.
                // This allows: def double [dup +]  to work like: def double dup + end
                let actual_body = if body.len() == 1 {
                    if let Node::Literal(Value::Quotation(inner)) = &body[0] {
                        inner.clone()
                    } else {
                        body.clone()
                    }
                } else {
                    body.clone()
                };

                self.words.insert(name.clone(), actual_body);
            }

            Node::Module {
                name: module_name,
                definitions,
            } => {
                for inner_def in definitions {
                    if let Node::Def {
                        name: word_name,
                        body,
                    } = inner_def
                    {
                        let qualified = format!("{}.{}", module_name, word_name);
                        self.words.insert(qualified, body.clone());
                    }
                }
            }

            Node::Use { module, item } => match item {
                UseItem::Single(word) => {
                    let qualified = format!("{}.{}", module, word);

                    self.aliases.insert(word.clone(), qualified);
                }

                UseItem::All => {
                    let prefix = format!("{}.", module);
                    let matching: Vec<_> = self
                        .words
                        .keys()
                        .filter(|k| k.starts_with(&prefix))
                        .cloned()
                        .collect();

                    for qualified in matching {
                        let word = qualified.strip_prefix(&prefix).unwrap();
                        self.aliases.insert(word.to_string(), qualified);
                    }
                }
            },

            Node::Import(_) => {}

            _ => {}
        }

        Ok(())
    }

    pub fn compile_nodes(&mut self, nodes: &[Node]) -> Result<Vec<Op>, CompileError> {
        let mut ops = Vec::new();
        for node in nodes {
            self.compile_node(node, &mut ops)?;
        }

        Ok(ops)
    }

    fn compile_module(
        &mut self,
        module_name: &str,
        definitions: &[Node],
    ) -> Result<(), CompileError> {
        for node in definitions {
            if let Node::Def { name, body } = node {
                let qualified_name = format!("{}.{}", module_name, name);
                let mut word_ops = self.compile_nodes(body)?;
                word_ops.push(Op::Return);
                self.program_bc.words.insert(qualified_name, word_ops);
            }
        }
        Ok(())
    }

    fn compile_node(&mut self, node: &Node, ops: &mut Vec<Op>) -> Result<(), CompileError> {
        match node {
            Node::Literal(value) => {
                let compiled_value = self.compile_value(value)?;
                ops.push(Op::Push(compiled_value));
            }

            // Stack ops
            Node::Dup => ops.push(Op::Dup),
            Node::Drop => ops.push(Op::Drop),
            Node::Swap => ops.push(Op::Swap),
            Node::Over => ops.push(Op::Over),
            Node::Rot => ops.push(Op::Rot),

            // Arithmetic
            Node::Add => ops.push(Op::Add),
            Node::Sub => ops.push(Op::Sub),
            Node::Mul => ops.push(Op::Mul),
            Node::Div => ops.push(Op::Div),
            Node::Mod => ops.push(Op::Mod),
            Node::Neg => ops.push(Op::Neg),
            Node::Abs => ops.push(Op::Abs),

            // Comparison
            Node::Eq => ops.push(Op::Eq),
            Node::NotEq => ops.push(Op::Ne),
            Node::Lt => ops.push(Op::Lt),
            Node::Gt => ops.push(Op::Gt),
            Node::LtEq => ops.push(Op::Le),
            Node::GtEq => ops.push(Op::Ge),

            // Logic
            Node::And => ops.push(Op::And),
            Node::Or => ops.push(Op::Or),
            Node::Not => ops.push(Op::Not),

            // Control flow - try jump optimization, fall back to quotation-based
            Node::If => {
                if !self.try_emit_if_jumps(ops) {
                    ops.push(Op::If);
                }
            }
            Node::When => {
                if !self.try_emit_when_jumps(ops) {
                    ops.push(Op::When);
                }
            }
            Node::Call => ops.push(Op::Call),

            // Loops - try jump optimization, fall back to quotation-based
            Node::Times => {
                if !self.try_emit_times_jumps(ops) {
                    ops.push(Op::Times);
                }
            }

            // These remain quotation-based for now (could optimize later)
            Node::Each => ops.push(Op::Each),
            Node::Map => ops.push(Op::Map),
            Node::Filter => ops.push(Op::Filter),
            Node::Fold => ops.push(Op::Fold),
            Node::Range => ops.push(Op::Range),

            // List ops
            Node::Len => ops.push(Op::Len),
            Node::Head => ops.push(Op::Head),
            Node::Tail => ops.push(Op::Tail),
            Node::Cons => ops.push(Op::Cons),
            Node::Concat => ops.push(Op::Concat),
            Node::StringConcat => ops.push(Op::StringConcat),

            // I/O
            Node::Print => ops.push(Op::Print),
            Node::Emit => ops.push(Op::Emit),
            Node::Read => ops.push(Op::Read),
            Node::Debug => ops.push(Op::Debug),

            // stdlib
            Node::Min => ops.push(Op::Min),
            Node::Max => ops.push(Op::Max),
            Node::Pow => ops.push(Op::Pow),
            Node::Sqrt => ops.push(Op::Sqrt),
            Node::Nth => ops.push(Op::Nth),
            Node::Append => ops.push(Op::Append),
            Node::Sort => ops.push(Op::Sort),
            Node::Reverse => ops.push(Op::Reverse),
            Node::Chars => ops.push(Op::Chars),
            Node::Join => ops.push(Op::Join),
            Node::Split => ops.push(Op::Split),
            Node::Upper => ops.push(Op::Upper),
            Node::Lower => ops.push(Op::Lower),
            Node::Trim => ops.push(Op::Trim),
            Node::Clear => ops.push(Op::Clear),
            Node::Depth => ops.push(Op::Depth),
            Node::Type => ops.push(Op::Type),
            Node::ToString => ops.push(Op::ToString),
            Node::ToInt => ops.push(Op::ToInt),

            // Combinators
            Node::Dip => ops.push(Op::Dip),
            Node::Keep => ops.push(Op::Keep),
            Node::Bi => ops.push(Op::Bi),
            Node::Bi2 => ops.push(Op::Bi2),
            Node::Tri => ops.push(Op::Tri),
            Node::Both => ops.push(Op::Both),
            Node::Compose => ops.push(Op::Compose),
            Node::Curry => ops.push(Op::Curry),
            Node::Apply => ops.push(Op::Apply),

            // Word calls
            Node::Word(name) => {
                // Check if this word has an alias (from 'use' statements)
                let resolved = self
                    .aliases
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| name.clone());
                ops.push(Op::CallWord(resolved));
            }

            Node::QualifiedWord { module, word } => ops.push(Op::CallQualified {
                module: module.clone(),
                word: word.clone(),
            }),

            // Definition-time constructs - specific error messages
            Node::Def { name, .. } => {
                return Err(CompileError::def_in_runtime(name));
            }

            Node::Module { name, .. } => {
                return Err(CompileError::module_in_runtime(name));
            }

            Node::Use { module, item } => {
                let item_name = match item {
                    UseItem::Single(name) => name.as_str(),
                    UseItem::All => "*",
                };
                return Err(CompileError::use_in_runtime(module, item_name));
            }

            Node::Import(path) => {
                return Err(CompileError::import_in_runtime(path));
            }
        }

        Ok(())
    }

    fn compile_value(&mut self, value: &Value) -> Result<Value, CompileError> {
        match value {
            Value::Quotation(nodes) => {
                let compiled_ops = self.compile_nodes(nodes)?;
                Ok(Value::CompiledQuotation(compiled_ops))
            }
            Value::CompiledQuotation(ops) => Ok(Value::CompiledQuotation(ops.clone())),
            Value::List(items) => {
                let compiled_items: Result<Vec<Value>, CompileError> =
                    items.iter().map(|it| self.compile_value(it)).collect();
                Ok(Value::List(compiled_items?))
            }
            Value::Integer(n) => Ok(Value::Integer(*n)),
            Value::Float(n) => Ok(Value::Float(*n)),
            Value::String(s) => Ok(Value::String(s.clone())),
            Value::Bool(b) => Ok(Value::Bool(*b)),
        }
    }

    // =========================================================================
    // Jump-based control flow optimization
    // =========================================================================

    /// Try to optimize `if` using jumps.
    /// Expects stack to have: ... then-quot else-quot
    /// Returns true if optimization succeeded, false to fall back to Op::If
    fn try_emit_if_jumps(&mut self, ops: &mut Vec<Op>) -> bool {
        if ops.len() < 2 {
            return false;
        }

        let len = ops.len();

        // Check if last two ops are compiled quotations
        let (then_ops, else_ops) = match (&ops[len - 2], &ops[len - 1]) {
            (
                Op::Push(Value::CompiledQuotation(then_ops)),
                Op::Push(Value::CompiledQuotation(else_ops)),
            ) => (then_ops.clone(), else_ops.clone()),
            _ => return false,
        };

        // Remove the two Push ops
        ops.pop();
        ops.pop();

        // Emit jump-based if:
        //   JumpIfFalse(then_len + 2)  ; skip then + jump
        //   <then_ops>
        //   Jump(else_len + 1)         ; skip else
        //   <else_ops>
        let then_len = then_ops.len() as i32;
        let else_len = else_ops.len() as i32;

        ops.push(Op::JumpIfFalse(then_len + 2));
        ops.extend(then_ops);
        ops.push(Op::Jump(else_len + 1));
        ops.extend(else_ops);

        true
    }

    /// Try to optimize `when` using jumps.
    /// Expects stack to have: ... then-quot
    /// Returns true if optimization succeeded, false to fall back to Op::When
    fn try_emit_when_jumps(&mut self, ops: &mut Vec<Op>) -> bool {
        if ops.is_empty() {
            return false;
        }

        let then_ops = match ops.last() {
            Some(Op::Push(Value::CompiledQuotation(then_ops))) => then_ops.clone(),
            _ => return false,
        };

        // Remove the Push op
        ops.pop();

        // Emit jump-based when:
        //   JumpIfFalse(then_len + 1)  ; skip then
        //   <then_ops>
        let then_len = then_ops.len() as i32;

        ops.push(Op::JumpIfFalse(then_len + 1));
        ops.extend(then_ops);

        true
    }

    /// Emit jump-based times loop if a compiled quotation is on top of ops.
    /// Returns true if optimization was applied, false otherwise.
    ///
    /// The generated structure uses ToAux/FromAux to preserve the counter
    /// while the body executes (which may push values onto the stack).
    ///
    /// Generated bytecode structure:
    /// ```text
    ///   Position   Instruction       Stack effect
    ///   --------   -----------       ------------
    ///   0:         Dup               n → n n
    ///   1:         Push(0)           n n → n n 0
    ///   2:         Le                n n 0 → n (n≤0)
    ///   3:         JumpIfTrue(exit)  n (n≤0) → n  [exit if counter ≤ 0]
    ///   4:         ToAux             n → ε  [aux: n]
    ///   5..5+B-1:  <body ops>        execute body, may push values
    ///   5+B:       FromAux           → n  [aux: ε]
    ///   6+B:       Push(1)           n → n 1
    ///   7+B:       Sub               n 1 → n-1
    ///   8+B:       Jump(back)        loop back to position 0
    ///   9+B:       Drop              n → ε  [cleanup counter]
    /// ```
    /// Where B = body_ops.len()
    fn try_emit_times_jumps(&mut self, ops: &mut Vec<Op>) -> bool {
        if ops.is_empty() {
            return false;
        }

        // Check if we have a compiled quotation on top
        let body_ops = match ops.last() {
            Some(Op::Push(Value::CompiledQuotation(body_ops))) => body_ops.clone(),
            _ => return false,
        };

        // Remove the Push(CompiledQuotation) op
        ops.pop();

        let body_len = body_ops.len() as i32;

        // Calculate positions (0-indexed from start of this loop construct):
        // 0: Dup
        // 1: Push(0)
        // 2: Le
        // 3: JumpIfTrue
        // 4: ToAux
        // 5 to 5+body_len-1: body
        // 5+body_len: FromAux
        // 6+body_len: Push(1)
        // 7+body_len: Sub
        // 8+body_len: Jump
        // 9+body_len: Drop (exit target)

        // JumpIfTrue at position 3 needs to reach Drop at position 9+body_len
        // With `continue` in VM (offset applied directly, no ip+=1):
        // target = current + offset → 9+body_len = 3 + offset → offset = 6+body_len
        let exit_offset = 6 + body_len;

        // Jump at position 8+body_len needs to reach Dup at position 0
        // target = current + offset → 0 = (8+body_len) + offset → offset = -(8+body_len)
        let jump_back = -(8 + body_len);

        // Emit the loop structure
        ops.push(Op::Dup); // 0
        ops.push(Op::Push(Value::Integer(0))); // 1
        ops.push(Op::Le); // 2
        ops.push(Op::JumpIfTrue(exit_offset)); // 3

        ops.push(Op::ToAux); // 4
        ops.extend(body_ops); // 5 to 5+body_len-1
        ops.push(Op::FromAux); // 5+body_len

        ops.push(Op::Push(Value::Integer(1))); // 6+body_len
        ops.push(Op::Sub); // 7+body_len
        ops.push(Op::Jump(jump_back)); // 8+body_len

        ops.push(Op::Drop); // 9+body_len

        true
    }

    // =========================================================================
    // Standalone jump compilation (for testing or explicit use)
    // =========================================================================

    #[allow(dead_code)]
    pub fn compile_if_jumps(
        &mut self,
        then_body: &[Node],
        else_body: &[Node],
    ) -> Result<Vec<Op>, CompileError> {
        let then_ops = self.compile_nodes(then_body)?;
        let else_ops = self.compile_nodes(else_body)?;

        let then_len = then_ops.len() as i32;
        let else_len = else_ops.len() as i32;

        let mut result = Vec::new();
        result.push(Op::JumpIfFalse(then_len + 2));
        result.extend(then_ops);
        result.push(Op::Jump(else_len + 1));
        result.extend(else_ops);
        Ok(result)
    }

    #[allow(dead_code)]
    pub fn compile_when_jumps(&mut self, then_body: &[Node]) -> Result<Vec<Op>, CompileError> {
        let then_ops = self.compile_nodes(then_body)?;
        let then_len = then_ops.len() as i32;

        let mut result = Vec::new();
        result.push(Op::JumpIfFalse(then_len + 1));
        result.extend(then_ops);
        Ok(result)
    }

    #[allow(dead_code)]
    pub fn compile_while_jumps(
        &mut self,
        condition_body: &[Node],
        loop_body: &[Node],
    ) -> Result<Vec<Op>, CompileError> {
        let cond_ops = self.compile_nodes(condition_body)?;
        let body_ops = self.compile_nodes(loop_body)?;

        let cond_len = cond_ops.len() as i32;
        let body_len = body_ops.len() as i32;

        let mut result = Vec::new();
        result.extend(cond_ops);
        result.push(Op::JumpIfFalse(body_len + 2));
        result.extend(body_ops);
        result.push(Op::Jump(-(cond_len + 1 + body_len + 1)));
        Ok(result)
    }

    /// Compile a times loop body into jump-based bytecode.
    /// This is the standalone version used for testing.
    ///
    /// Note: This generates the loop structure WITHOUT the initial counter push.
    /// The caller is responsible for ensuring the counter is on the stack.
    /// #[allow(dead_code)]
    pub fn compile_times_jumps(&mut self, loop_body: &[Node]) -> Result<Vec<Op>, CompileError> {
        let body_ops = self.compile_nodes(loop_body)?;
        let body_len = body_ops.len() as i32;

        let exit_offset = 6 + body_len;
        let jump_back = -(8 + body_len);

        let mut result = Vec::new();

        result.push(Op::Dup); // 0
        result.push(Op::Push(Value::Integer(0))); // 1
        result.push(Op::Le); // 2
        result.push(Op::JumpIfTrue(exit_offset)); // 3

        result.push(Op::ToAux); // 4
        result.extend(body_ops); // 5 to 5+body_len-1
        result.push(Op::FromAux); // 5+body_len

        result.push(Op::Push(Value::Integer(1))); // 6+body_len
        result.push(Op::Sub); // 7+body_len
        result.push(Op::Jump(jump_back)); // 8+body_len

        result.push(Op::Drop); // 9+body_len

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Basic compilation tests
    // =========================================================================

    #[test]
    fn test_compile_quotation() {
        let nodes = vec![Node::Literal(Value::Quotation(vec![
            Node::Literal(Value::Integer(1)),
            Node::Literal(Value::Integer(2)),
            Node::Add,
        ]))];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        assert_eq!(ops.len(), 1);

        match &ops[0] {
            Op::Push(Value::CompiledQuotation(inner)) => {
                assert_eq!(inner.len(), 3);
            }
            other => panic!("expected CompiledQuotation, got {:?}", other),
        }
    }

    #[test]
    fn test_compile_nested_quotation() {
        let inner = Value::Quotation(vec![
            Node::Literal(Value::Integer(1)),
            Node::Literal(Value::Integer(2)),
            Node::Add,
        ]);
        let outer = vec![Node::Literal(Value::Quotation(vec![
            Node::Literal(inner),
            Node::Call,
        ]))];

        let ops = Compiler::new().compile_nodes(&outer).unwrap();

        match &ops[0] {
            Op::Push(Value::CompiledQuotation(outer_ops)) => {
                assert!(matches!(
                    &outer_ops[0],
                    Op::Push(Value::CompiledQuotation(_))
                ));
            }
            _ => panic!("expected nested compiled quotation"),
        }
    }

    #[test]
    fn test_compile_list_with_quotations() {
        let list = Value::List(vec![
            Value::Integer(1),
            Value::Quotation(vec![Node::Literal(Value::Integer(2))]),
        ]);

        let compiled = Compiler::new().compile_value(&list).unwrap();

        match compiled {
            Value::List(items) => {
                assert_eq!(items.len(), 2);
                assert!(matches!(items[0], Value::Integer(1)));
                assert!(matches!(items[1], Value::CompiledQuotation(_)));
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn test_compile_definition_error() {
        let nodes = vec![Node::Def {
            name: "foo".to_string(),
            body: vec![],
        }];

        let result = Compiler::new().compile_nodes(&nodes);
        assert!(result.is_err());
    }

    #[test]
    fn test_compile_qualified_word() {
        let nodes = vec![Node::QualifiedWord {
            module: "math".to_string(),
            word: "sqrt".to_string(),
        }];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        assert!(matches!(
            &ops[0],
            Op::CallQualified { module, word } if module == "math" && word == "sqrt"
        ));
    }

    // =========================================================================
    // Standalone jump compilation tests (using compile_*_jumps methods)
    // =========================================================================

    #[test]
    fn test_compile_if_jumps() {
        let then_body = vec![Node::Literal(Value::Integer(10))];
        let else_body = vec![Node::Literal(Value::Integer(20))];

        let ops = Compiler::new()
            .compile_if_jumps(&then_body, &else_body)
            .unwrap();

        assert_eq!(ops.len(), 4);
        assert!(matches!(ops[0], Op::JumpIfFalse(3)));
        assert!(matches!(ops[1], Op::Push(Value::Integer(10))));
        assert!(matches!(ops[2], Op::Jump(2)));
        assert!(matches!(ops[3], Op::Push(Value::Integer(20))));
    }

    #[test]
    fn test_compile_when_jumps() {
        let then_body = vec![Node::Literal(Value::Integer(10))];

        let ops = Compiler::new().compile_when_jumps(&then_body).unwrap();

        assert_eq!(ops.len(), 2);
        assert!(matches!(ops[0], Op::JumpIfFalse(2)));
        assert!(matches!(ops[1], Op::Push(Value::Integer(10))));
    }

    #[test]
    fn test_compile_while_jumps() {
        let cond = vec![Node::Dup, Node::Literal(Value::Integer(0)), Node::Gt];
        let body = vec![Node::Literal(Value::Integer(1)), Node::Sub];

        let ops = Compiler::new().compile_while_jumps(&cond, &body).unwrap();

        assert_eq!(ops.len(), 7);
        assert!(matches!(ops[0], Op::Dup));
        assert!(matches!(ops[1], Op::Push(Value::Integer(0))));
        assert!(matches!(ops[2], Op::Gt));
        assert!(matches!(ops[3], Op::JumpIfFalse(4)));
        assert!(matches!(ops[4], Op::Push(Value::Integer(1))));
        assert!(matches!(ops[5], Op::Sub));
        assert!(matches!(ops[6], Op::Jump(-7)));
    }

    #[test]
    fn test_compile_times_jumps_structure() {
        let body = vec![Node::Literal(Value::Integer(42)), Node::Drop];
        let ops = Compiler::new().compile_times_jumps(&body).unwrap();

        assert!(ops.len() > 4);
        assert!(matches!(ops[0], Op::Dup));
        assert!(matches!(ops.last(), Some(Op::Drop)));
    }

    // =========================================================================
    // Jump optimization integration tests (try_emit_*_jumps)
    // =========================================================================

    #[test]
    fn test_if_optimizes_to_jumps() {
        // true [ 10 ] [ 20 ] if
        let nodes = vec![
            Node::Literal(Value::Bool(true)),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(10))])),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(20))])),
            Node::If,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        // Should NOT have Op::If - should have jumps instead
        assert!(!ops.iter().any(|op| matches!(op, Op::If)));

        // Should have JumpIfFalse
        assert!(ops.iter().any(|op| matches!(op, Op::JumpIfFalse(_))));

        // Structure: Push(true), JumpIfFalse, Push(10), Jump, Push(20)
        assert!(matches!(ops[0], Op::Push(Value::Bool(true))));
        assert!(matches!(ops[1], Op::JumpIfFalse(_)));
    }

    #[test]
    fn test_if_falls_back_when_not_static() {
        // Just `if` with no quotations on compile-time stack
        let nodes = vec![Node::If];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        // Should have Op::If since we couldn't optimize
        assert!(matches!(ops[0], Op::If));
    }

    #[test]
    fn test_when_optimizes_to_jumps() {
        // true [ 10 ] when
        let nodes = vec![
            Node::Literal(Value::Bool(true)),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(10))])),
            Node::When,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        assert!(!ops.iter().any(|op| matches!(op, Op::When)));
        assert!(ops.iter().any(|op| matches!(op, Op::JumpIfFalse(_))));
    }

    #[test]
    fn test_when_falls_back_when_not_static() {
        let nodes = vec![Node::When];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        assert!(matches!(ops[0], Op::When));
    }

    #[test]
    fn test_times_optimizes_to_jumps() {
        // 5 [ 1 ] times
        let nodes = vec![
            Node::Literal(Value::Integer(5)),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(1))])),
            Node::Times,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        assert!(!ops.iter().any(|op| matches!(op, Op::Times)));
        assert!(ops.iter().any(|op| matches!(op, Op::JumpIfTrue(_))));
        // Should end with Drop (cleanup counter)
        assert!(matches!(ops.last(), Some(Op::Drop)));
    }

    #[test]
    fn test_times_falls_back_when_not_static() {
        let nodes = vec![Node::Times];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        assert!(matches!(ops[0], Op::Times));
    }

    #[test]
    fn test_nested_if_optimizes() {
        // true [ false [ 1 ] [ 2 ] if ] [ 3 ] if
        let inner_if = vec![
            Node::Literal(Value::Bool(false)),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(1))])),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(2))])),
            Node::If,
        ];

        let nodes = vec![
            Node::Literal(Value::Bool(true)),
            Node::Literal(Value::Quotation(inner_if)),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(3))])),
            Node::If,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        // Outer if should be optimized
        assert!(!ops.iter().any(|op| matches!(op, Op::If)));

        // Should have at least 2 JumpIfFalse (outer and inner)
        let jump_count = ops
            .iter()
            .filter(|op| matches!(op, Op::JumpIfFalse(_)))
            .count();
        assert!(jump_count >= 1); // At least outer; inner is in compiled quotation
    }
}

#[cfg(test)]
mod times_tests {
    use super::*;

    #[test]
    fn test_times_structure_has_correct_ops() {
        let body = vec![Node::Literal(Value::Integer(42))];
        let ops = Compiler::new().compile_times_jumps(&body).unwrap();

        // compile_times_jumps doesn't include the initial Push for counter
        // It starts from Dup
        assert!(matches!(ops[0], Op::Dup), "should start with Dup");
        assert!(
            matches!(ops[1], Op::Push(Value::Integer(0))),
            "should push threshold 0"
        );
        assert!(matches!(ops[2], Op::Le), "should compare with Le");
        assert!(
            matches!(ops[3], Op::JumpIfTrue(_)),
            "should conditionally exit"
        );
        assert!(matches!(ops[4], Op::ToAux), "should hide counter");
        assert!(
            matches!(ops[5], Op::Push(Value::Integer(42))),
            "body should be compiled"
        );
        assert!(matches!(ops[6], Op::FromAux), "should restore counter");
        assert!(matches!(ops[7], Op::Push(Value::Integer(1))));
        assert!(matches!(ops[8], Op::Sub));
        assert!(matches!(ops[9], Op::Jump(_)), "should jump back");
        assert!(matches!(ops[10], Op::Drop), "should end with Drop");
    }

    #[test]
    fn test_times_multi_op_body() {
        // Body with multiple operations
        let body = vec![Node::Dup, Node::Swap, Node::Drop];
        let ops = Compiler::new().compile_times_jumps(&body).unwrap();

        // Body starts after ToAux (position 4)
        assert!(matches!(ops[4], Op::ToAux));
        assert!(matches!(ops[5], Op::Dup));
        assert!(matches!(ops[6], Op::Swap));
        assert!(matches!(ops[7], Op::Drop));
        assert!(matches!(ops[8], Op::FromAux));
    }

    #[test]
    fn test_times_zero_iterations() {
        let body = vec![Node::Literal(Value::Integer(1))];
        let ops = Compiler::new().compile_times_jumps(&body).unwrap();

        let threshold = extract_push_threshold(&ops);
        assert_eq!(threshold, 0, "threshold should be 0 for <= comparison");
    }

    #[test]
    fn test_times_one_iteration() {
        let body = vec![Node::Literal(Value::Integer(1))];
        let ops = Compiler::new().compile_times_jumps(&body).unwrap();

        let threshold = extract_push_threshold(&ops);
        assert_eq!(threshold, 0);

        if let Op::Jump(offset) = ops[ops.len() - 2] {
            let jump_pos = (ops.len() - 2) as i32;
            let target = jump_pos + offset;
            assert_eq!(target, 0, "should jump back to start of loop");
        }
    }

    #[test]
    fn test_times_jump_offsets_consistent() {
        for body_size in 1..=5 {
            let body: Vec<Node> = (0..body_size)
                .map(|i| Node::Literal(Value::Integer(i as i64)))
                .collect();

            let ops = Compiler::new().compile_times_jumps(&body).unwrap();

            if let Op::JumpIfTrue(forward) = ops[3] {
                let target = 3 + forward;
                assert_eq!(
                    target as usize,
                    ops.len() - 1,
                    "forward jump should land on Drop for body_size={}",
                    body_size
                );
            }

            let back_jump_pos = ops.len() - 2;
            if let Op::Jump(backward) = &ops[back_jump_pos] {
                let target = back_jump_pos as i32 + backward;
                assert_eq!(
                    target, 0,
                    "backward jump should land on Dup for body_size={}",
                    body_size
                );
            }
        }
    }

    #[test]
    fn test_times_multi_instruction_body() {
        let body = vec![Node::Dup, Node::Swap, Node::Drop];
        let ops = Compiler::new().compile_times_jumps(&body).unwrap();

        println!("{:?}", ops);
        // [Dup, Push(Integer(0)), Le, JumpIfTrue(9), ToAux, Dup, Swap, Drop, FromAux, Push(Integer(1)), Sub, Jump(-11), Drop]

        assert!(matches!(ops[4], Op::ToAux));
        assert!(matches!(ops[5], Op::Dup));
        assert!(matches!(ops[6], Op::Swap));
    }

    fn extract_push_threshold(ops: &[Op]) -> i64 {
        // Threshold is at position 1 (after Dup)
        if let Op::Push(Value::Integer(n)) = &ops[1] {
            *n
        } else {
            panic!("expected Push(Integer) at position 1, got {:?}", ops[1]);
        }
    }
}

#[cfg(test)]
mod jump_optimization_tests {
    use super::*;

    // =========================================================================
    // If optimization tests
    // =========================================================================

    #[test]
    fn test_if_optimization_structure() {
        // true [ 10 ] [ 20 ] if
        let nodes = vec![
            Node::Literal(Value::Bool(true)),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(10))])),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(20))])),
            Node::If,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        // Expected: Push(true), JumpIfFalse(3), Push(10), Jump(2), Push(20)
        assert_eq!(ops.len(), 5);
        assert!(matches!(ops[0], Op::Push(Value::Bool(true))));
        assert!(matches!(ops[1], Op::JumpIfFalse(3))); // skip Push(10) + Jump
        assert!(matches!(ops[2], Op::Push(Value::Integer(10))));
        assert!(matches!(ops[3], Op::Jump(2))); // skip Push(20)
        assert!(matches!(ops[4], Op::Push(Value::Integer(20))));
    }

    #[test]
    fn test_if_optimization_with_multi_instruction_bodies() {
        // true [ 1 2 + ] [ 3 4 * ] if
        let nodes = vec![
            Node::Literal(Value::Bool(true)),
            Node::Literal(Value::Quotation(vec![
                Node::Literal(Value::Integer(1)),
                Node::Literal(Value::Integer(2)),
                Node::Add,
            ])),
            Node::Literal(Value::Quotation(vec![
                Node::Literal(Value::Integer(3)),
                Node::Literal(Value::Integer(4)),
                Node::Mul,
            ])),
            Node::If,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        // Push(true), JumpIfFalse(5), Push(1), Push(2), Add, Jump(4), Push(3), Push(4), Mul
        assert_eq!(ops.len(), 9);
        assert!(matches!(ops[1], Op::JumpIfFalse(5))); // skip 3 ops + Jump
        assert!(matches!(ops[5], Op::Jump(4))); // skip 3 ops
    }

    #[test]
    fn test_if_optimization_with_empty_then() {
        // true [ ] [ 20 ] if
        let nodes = vec![
            Node::Literal(Value::Bool(true)),
            Node::Literal(Value::Quotation(vec![])),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(20))])),
            Node::If,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        // Push(true), JumpIfFalse(2), Jump(2), Push(20)
        assert_eq!(ops.len(), 4);
        assert!(matches!(ops[1], Op::JumpIfFalse(2))); // skip nothing + Jump
        assert!(matches!(ops[2], Op::Jump(2))); // skip Push(20)
    }

    #[test]
    fn test_if_optimization_with_empty_else() {
        // true [ 10 ] [ ] if
        let nodes = vec![
            Node::Literal(Value::Bool(true)),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(10))])),
            Node::Literal(Value::Quotation(vec![])),
            Node::If,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        // Push(true), JumpIfFalse(3), Push(10), Jump(1)
        assert_eq!(ops.len(), 4);
        assert!(matches!(ops[1], Op::JumpIfFalse(3)));
        assert!(matches!(ops[3], Op::Jump(1))); // skip nothing
    }

    #[test]
    fn test_if_optimization_with_both_empty() {
        // true [ ] [ ] if
        let nodes = vec![
            Node::Literal(Value::Bool(true)),
            Node::Literal(Value::Quotation(vec![])),
            Node::Literal(Value::Quotation(vec![])),
            Node::If,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        // Push(true), JumpIfFalse(2), Jump(1)
        assert_eq!(ops.len(), 3);
    }

    #[test]
    fn test_if_no_optimization_only_one_quotation() {
        // [ 10 ] if  -- missing else quotation
        let nodes = vec![
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(10))])),
            Node::If,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        // Should fall back to Op::If
        assert!(ops.iter().any(|op| matches!(op, Op::If)));
    }

    #[test]
    fn test_if_no_optimization_non_quotation_values() {
        // 10 20 if  -- integers, not quotations
        let nodes = vec![
            Node::Literal(Value::Integer(10)),
            Node::Literal(Value::Integer(20)),
            Node::If,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        // Should fall back to Op::If
        assert!(matches!(ops[2], Op::If));
    }

    #[test]
    fn test_if_no_optimization_mixed_values() {
        // [ 10 ] 20 if  -- one quotation, one integer
        let nodes = vec![
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(10))])),
            Node::Literal(Value::Integer(20)),
            Node::If,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        assert!(ops.iter().any(|op| matches!(op, Op::If)));
    }

    // =========================================================================
    // When optimization tests
    // =========================================================================

    #[test]
    fn test_when_optimization_structure() {
        // true [ 42 ] when
        let nodes = vec![
            Node::Literal(Value::Bool(true)),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(42))])),
            Node::When,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        // Push(true), JumpIfFalse(2), Push(42)
        assert_eq!(ops.len(), 3);
        assert!(matches!(ops[0], Op::Push(Value::Bool(true))));
        assert!(matches!(ops[1], Op::JumpIfFalse(2)));
        assert!(matches!(ops[2], Op::Push(Value::Integer(42))));
    }

    #[test]
    fn test_when_optimization_multi_instruction_body() {
        // true [ 1 2 3 + + ] when
        let nodes = vec![
            Node::Literal(Value::Bool(true)),
            Node::Literal(Value::Quotation(vec![
                Node::Literal(Value::Integer(1)),
                Node::Literal(Value::Integer(2)),
                Node::Literal(Value::Integer(3)),
                Node::Add,
                Node::Add,
            ])),
            Node::When,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        // Push(true), JumpIfFalse(6), Push(1), Push(2), Push(3), Add, Add
        assert_eq!(ops.len(), 7);
        assert!(matches!(ops[1], Op::JumpIfFalse(6)));
    }

    #[test]
    fn test_when_optimization_empty_body() {
        // true [ ] when
        let nodes = vec![
            Node::Literal(Value::Bool(true)),
            Node::Literal(Value::Quotation(vec![])),
            Node::When,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        // Push(true), JumpIfFalse(1)
        assert_eq!(ops.len(), 2);
        assert!(matches!(ops[1], Op::JumpIfFalse(1)));
    }

    #[test]
    fn test_when_no_optimization_non_quotation() {
        // 42 when
        let nodes = vec![Node::Literal(Value::Integer(42)), Node::When];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        assert!(matches!(ops[1], Op::When));
    }

    #[test]
    fn test_when_no_optimization_empty_stack() {
        let nodes = vec![Node::When];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        assert!(matches!(ops[0], Op::When));
    }

    // =========================================================================
    // Times optimization tests
    // =========================================================================

    #[test]
    fn test_times_optimization_structure() {
        // 5 [ 1 ] times
        let nodes = vec![
            Node::Literal(Value::Integer(5)),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(1))])),
            Node::Times,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        // New structure with ToAux/FromAux:
        // 0: Push(5)         - counter
        // 1: Dup             - check
        // 2: Push(0)         - check
        // 3: Le              - check
        // 4: JumpIfTrue(...) - exit if counter <= 0
        // 5: ToAux           - hide counter
        // 6: Push(1)         - body
        // 7: FromAux         - restore counter
        // 8: Push(1)         - decrement
        // 9: Sub             - decrement
        // 10: Jump(...)      - loop back
        // 11: Drop           - cleanup

        assert!(matches!(ops[0], Op::Push(Value::Integer(5))));
        assert!(matches!(ops[1], Op::Dup));
        assert!(matches!(ops[2], Op::Push(Value::Integer(0))));
        assert!(matches!(ops[3], Op::Le));
        assert!(matches!(ops[4], Op::JumpIfTrue(_)));
        assert!(matches!(ops[5], Op::ToAux));
        // Body
        assert!(matches!(ops[6], Op::Push(Value::Integer(1))));
        // Restore counter
        assert!(matches!(ops[7], Op::FromAux));
        // Decrement
        assert!(matches!(ops[8], Op::Push(Value::Integer(1))));
        assert!(matches!(ops[9], Op::Sub));
        assert!(matches!(ops[10], Op::Jump(_)));
        assert!(matches!(ops[11], Op::Drop));
    }

    #[test]
    fn test_times_optimization_empty_body() {
        // 5 [ ] times
        let nodes = vec![
            Node::Literal(Value::Integer(5)),
            Node::Literal(Value::Quotation(vec![])),
            Node::Times,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        // Should still compile with empty body
        // Structure: Push(5), Dup, Push(0), Le, JumpIfTrue, ToAux, FromAux, Push(1), Sub, Jump, Drop
        assert!(matches!(ops[0], Op::Push(Value::Integer(5))));
        assert!(matches!(ops[1], Op::Dup));
        assert!(matches!(ops.last(), Some(Op::Drop)));

        // Verify ToAux and FromAux are present
        assert!(ops.iter().any(|op| matches!(op, Op::ToAux)));
        assert!(ops.iter().any(|op| matches!(op, Op::FromAux)));
    }

    #[test]
    fn test_times_no_optimization_non_quotation() {
        // 5 times (no quotation literal, falls back to Op::Times)
        let nodes = vec![Node::Literal(Value::Integer(5)), Node::Times];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        assert!(matches!(ops[1], Op::Times));
    }

    #[test]
    fn test_times_optimization_jump_targets() {
        // 3 [ dup drop ] times
        let nodes = vec![
            Node::Literal(Value::Integer(3)),
            Node::Literal(Value::Quotation(vec![Node::Dup, Node::Drop])),
            Node::Times,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        // Structure:
        // 0: Push(3)
        // 1: Dup
        // 2: Push(0)
        // 3: Le
        // 4: JumpIfTrue(8) -> position 12 (Drop)
        // 5: ToAux
        // 6: Dup           (body)
        // 7: Drop          (body)
        // 8: FromAux
        // 9: Push(1)
        // 10: Sub
        // 11: Jump(-10) -> position 1 (Dup)
        // 12: Drop

        // Find the JumpIfTrue and verify it targets the final Drop
        let exit_jump_pos = 4;
        if let Op::JumpIfTrue(offset) = ops[exit_jump_pos] {
            let target = exit_jump_pos as i32 + offset;
            assert_eq!(
                target as usize,
                ops.len() - 1,
                "exit jump should target final Drop"
            );
        } else {
            panic!("expected JumpIfTrue at position 4");
        }

        // Find the backward Jump and verify it targets position 1 (Dup)
        let back_jump_pos = ops.len() - 2; // Second to last
        if let Op::Jump(offset) = ops[back_jump_pos] {
            let target = back_jump_pos as i32 + offset;
            assert_eq!(target, 1, "backward jump should target Dup at position 1");
        } else {
            panic!("expected Jump at second-to-last position");
        }
    }

    // =========================================================================
    // Nested optimization tests
    // =========================================================================

    #[test]
    fn test_nested_if_in_then_branch() {
        // true [ false [ 1 ] [ 2 ] if ] [ 3 ] if
        let inner_if = vec![
            Node::Literal(Value::Bool(false)),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(1))])),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(2))])),
            Node::If,
        ];

        let nodes = vec![
            Node::Literal(Value::Bool(true)),
            Node::Literal(Value::Quotation(inner_if)),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(3))])),
            Node::If,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        // Outer should be optimized (no Op::If at top level)
        // Count JumpIfFalse - should have at least 1 for outer
        let jump_count = ops
            .iter()
            .filter(|op| matches!(op, Op::JumpIfFalse(_)))
            .count();
        assert!(jump_count >= 1);

        // The inner if is inside a compiled quotation, check it's there
        // Find the compiled quotation in the then branch
        let then_start = 2; // After Push(true), JumpIfFalse
        if let Op::Push(Value::CompiledQuotation(inner_ops)) = &ops[then_start] {
            // Inner should also be optimized
            assert!(!inner_ops.iter().any(|op| matches!(op, Op::If)));
            assert!(inner_ops.iter().any(|op| matches!(op, Op::JumpIfFalse(_))));
        }
    }

    #[test]
    fn test_nested_when_in_when() {
        // true [ true [ 42 ] when ] when
        let inner_when = vec![
            Node::Literal(Value::Bool(true)),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(42))])),
            Node::When,
        ];

        let nodes = vec![
            Node::Literal(Value::Bool(true)),
            Node::Literal(Value::Quotation(inner_when)),
            Node::When,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        // Outer when is optimized
        assert!(!ops.iter().any(|op| matches!(op, Op::When)));
    }

    #[test]
    fn test_times_inside_if() {
        // true [ 3 [ 1 ] times ] [ 0 ] if
        let times_body = vec![
            Node::Literal(Value::Integer(3)),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(1))])),
            Node::Times,
        ];

        let nodes = vec![
            Node::Literal(Value::Bool(true)),
            Node::Literal(Value::Quotation(times_body)),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(0))])),
            Node::If,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        // Outer if should be optimized
        assert!(!ops.iter().any(|op| matches!(op, Op::If)));
    }

    #[test]
    fn test_if_inside_times() {
        // 3 [ true [ 1 ] [ 2 ] if ] times
        let if_body = vec![
            Node::Literal(Value::Bool(true)),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(1))])),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(2))])),
            Node::If,
        ];

        let nodes = vec![
            Node::Literal(Value::Integer(3)),
            Node::Literal(Value::Quotation(if_body)),
            Node::Times,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        // Times should be optimized
        assert!(!ops.iter().any(|op| matches!(op, Op::Times)));
    }

    // =========================================================================
    // Edge cases and mixed scenarios
    // =========================================================================

    #[test]
    fn test_optimization_preserves_surrounding_ops() {
        // 1 2 + true [ 10 ] [ 20 ] if 3 *
        let nodes = vec![
            Node::Literal(Value::Integer(1)),
            Node::Literal(Value::Integer(2)),
            Node::Add,
            Node::Literal(Value::Bool(true)),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(10))])),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(20))])),
            Node::If,
            Node::Literal(Value::Integer(3)),
            Node::Mul,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        // Should start with arithmetic
        assert!(matches!(ops[0], Op::Push(Value::Integer(1))));
        assert!(matches!(ops[1], Op::Push(Value::Integer(2))));
        assert!(matches!(ops[2], Op::Add));

        // Should end with multiplication
        let len = ops.len();
        assert!(matches!(ops[len - 1], Op::Mul));
        assert!(matches!(ops[len - 2], Op::Push(Value::Integer(3))));
    }

    #[test]
    fn test_multiple_consecutive_ifs() {
        // true [ 1 ] [ 2 ] if  false [ 3 ] [ 4 ] if
        let nodes = vec![
            Node::Literal(Value::Bool(true)),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(1))])),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(2))])),
            Node::If,
            Node::Literal(Value::Bool(false)),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(3))])),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(4))])),
            Node::If,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        // Both should be optimized
        assert!(!ops.iter().any(|op| matches!(op, Op::If)));

        // Should have 2 JumpIfFalse
        let jump_count = ops
            .iter()
            .filter(|op| matches!(op, Op::JumpIfFalse(_)))
            .count();
        assert_eq!(jump_count, 2);
    }

    #[test]
    fn test_quotation_not_immediately_before_control() {
        // [ 10 ] dup if  -- quotation, then dup, then if
        let nodes = vec![
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(10))])),
            Node::Dup,
            Node::If,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        // Can't optimize because Dup is between quotation and if
        assert!(ops.iter().any(|op| matches!(op, Op::If)));
    }

    #[test]
    fn test_call_not_optimized() {
        // [ 42 ] call  -- call should never be optimized away
        let nodes = vec![
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(42))])),
            Node::Call,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        assert!(matches!(ops[1], Op::Call));
    }

    #[test]
    fn test_higher_order_ops_not_optimized() {
        // { 1 2 3 } [ 2 * ] map  -- map should remain as Op::Map
        let nodes = vec![
            Node::Literal(Value::List(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3),
            ])),
            Node::Literal(Value::Quotation(vec![
                Node::Literal(Value::Integer(2)),
                Node::Mul,
            ])),
            Node::Map,
        ];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        assert!(ops.iter().any(|op| matches!(op, Op::Map)));
    }

    // =========================================================================
    // Quotation with control flow inside
    // =========================================================================

    #[test]
    fn test_quotation_containing_optimized_if() {
        // [ true [ 1 ] [ 2 ] if ]
        let nodes = vec![Node::Literal(Value::Quotation(vec![
            Node::Literal(Value::Bool(true)),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(1))])),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(2))])),
            Node::If,
        ]))];

        let ops = Compiler::new().compile_nodes(&nodes).unwrap();

        // Should be one Push with a CompiledQuotation
        assert_eq!(ops.len(), 1);

        if let Op::Push(Value::CompiledQuotation(inner)) = &ops[0] {
            // Inner if should be optimized
            assert!(!inner.iter().any(|op| matches!(op, Op::If)));
            assert!(inner.iter().any(|op| matches!(op, Op::JumpIfFalse(_))));
        } else {
            panic!("expected CompiledQuotation");
        }
    }

    #[test]
    fn test_list_containing_quotation_with_control_flow() {
        // { [ true [ 1 ] [ 2 ] if ] }
        let quot_with_if = Value::Quotation(vec![
            Node::Literal(Value::Bool(true)),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(1))])),
            Node::Literal(Value::Quotation(vec![Node::Literal(Value::Integer(2))])),
            Node::If,
        ]);

        let list = Value::List(vec![quot_with_if]);

        let compiled = Compiler::new().compile_value(&list).unwrap();

        if let Value::List(items) = compiled {
            if let Value::CompiledQuotation(inner) = &items[0] {
                // Should be optimized
                assert!(!inner.iter().any(|op| matches!(op, Op::If)));
            } else {
                panic!("expected CompiledQuotation in list");
            }
        } else {
            panic!("expected List");
        }
    }
}
