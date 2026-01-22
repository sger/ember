use crate::{
    bytecode::{CodeObject, Op, ProgramBc, compile_error::CompileError},
    lang::{node::Node, program::Program, use_item::UseItem, value::Value},
};

pub struct Compiler {
    program_bc: ProgramBc,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            program_bc: ProgramBc {
                code: vec![CodeObject::new()],
            },
        }
    }

    pub fn compile_program(mut self, program: &Program) -> Result<ProgramBc, CompileError> {
        let mut ops = self.compile_nodes(&program.main)?;
        ops.push(Op::Return);
        self.program_bc.code[0].ops = ops;
        Ok(self.program_bc)
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
                ops.push(Op::CallWord(name.clone()));
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

            // Catch-all for truly unhandled nodes
            _ => {
                return Err(CompileError::unhandled(node));
            }
        }

        Ok(())
    }

    fn compile_nodes(&mut self, nodes: &[Node]) -> Result<Vec<Op>, CompileError> {
        let mut ops = Vec::new();
        for node in nodes {
            self.compile_node(node, &mut ops)?;
        }
        Ok(ops)
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

    /// Try to optimize `times` using jumps.
    /// Expects stack to have: ... body-quot
    /// Returns true if optimization succeeded, false to fall back to Op::Times
    fn try_emit_times_jumps(&mut self, ops: &mut Vec<Op>) -> bool {
        if ops.is_empty() {
            return false;
        }

        let body_ops = match ops.last() {
            Some(Op::Push(Value::CompiledQuotation(body_ops))) => body_ops.clone(),
            _ => return false,
        };

        // Remove the Push op
        ops.pop();

        // Emit jump-based times:
        //   0: Dup
        //   1: Push(0)
        //   2: Le
        //   3: JumpIfTrue(body_len + footer_len + 1)  ; exit if n <= 0
        //   4..4+body_len: <body_ops>
        //   4+body_len: Push(1)
        //   5+body_len: Sub
        //   6+body_len: Jump(back to 0)
        //   7+body_len: Drop
        let body_len = body_ops.len() as i32;
        let check_len: i32 = 4;
        let footer_len: i32 = 3;

        ops.push(Op::Dup);
        ops.push(Op::Push(Value::Integer(0)));
        ops.push(Op::Le);
        ops.push(Op::JumpIfTrue(body_len + footer_len + 1));

        ops.extend(body_ops);

        ops.push(Op::Push(Value::Integer(1)));
        ops.push(Op::Sub);
        ops.push(Op::Jump(-(check_len + body_len + footer_len - 1)));

        ops.push(Op::Drop);

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

    #[allow(dead_code)]
    pub fn compile_times_jumps(&mut self, loop_body: &[Node]) -> Result<Vec<Op>, CompileError> {
        let body_ops = self.compile_nodes(loop_body)?;
        let body_len = body_ops.len() as i32;

        let check_len: i32 = 4;
        let footer_len: i32 = 3;

        let mut result = Vec::new();

        result.push(Op::Dup);
        result.push(Op::Push(Value::Integer(0)));
        result.push(Op::Le);
        result.push(Op::JumpIfTrue(body_len + footer_len + 1));

        result.extend(body_ops);

        result.push(Op::Push(Value::Integer(1)));
        result.push(Op::Sub);
        result.push(Op::Jump(-(check_len + body_len + footer_len - 1)));

        result.push(Op::Drop);

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
        assert!(
            matches!(ops[4], Op::Push(Value::Integer(42))),
            "body should be compiled"
        );
        assert!(matches!(ops[5], Op::Push(Value::Integer(1))));
        assert!(matches!(ops[6], Op::Sub));
        assert!(matches!(ops[7], Op::Jump(_)), "should jump back");
        assert!(matches!(ops[8], Op::Drop), "should end with Drop");
    }

    #[test]
    fn test_times_zero_iterations() {
        let body = vec![Node::Literal(Value::Integer(1))];
        let ops = Compiler::new().compile_times_jumps(&body).unwrap();

        let threshold = extract_push_threshold(&ops);
        assert_eq!(threshold, 0, "threshold should be 0 for correct semantics");

        if let Op::JumpIfTrue(offset) = ops[3] {
            let target = 3 + offset;
            assert_eq!(
                target as usize,
                ops.len() - 1,
                "should jump to final Drop on 0 iterations"
            );
        }
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

        assert!(matches!(ops[4], Op::Dup));
        assert!(matches!(ops[5], Op::Swap));
        assert!(matches!(ops[6], Op::Drop));
    }

    fn extract_push_threshold(ops: &[Op]) -> i64 {
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

        // Push(5), Dup, Push(0), Le, JumpIfTrue(...), Push(1), Push(1), Sub, Jump(...), Drop
        assert!(matches!(ops[0], Op::Push(Value::Integer(5))));
        assert!(matches!(ops[1], Op::Dup));
        assert!(matches!(ops[2], Op::Push(Value::Integer(0))));
        assert!(matches!(ops[3], Op::Le));
        assert!(matches!(ops[4], Op::JumpIfTrue(_)));
        // Body
        assert!(matches!(ops[5], Op::Push(Value::Integer(1))));
        // Footer
        assert!(matches!(ops[6], Op::Push(Value::Integer(1))));
        assert!(matches!(ops[7], Op::Sub));
        assert!(matches!(ops[8], Op::Jump(_)));
        assert!(matches!(ops[9], Op::Drop));
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
        // Push(5), Dup, Push(0), Le, JumpIfTrue(...), Push(1), Sub, Jump(...), Drop
        assert!(matches!(ops[0], Op::Push(Value::Integer(5))));
        assert!(matches!(ops[1], Op::Dup));
        assert!(matches!(ops.last(), Some(Op::Drop)));
    }

    #[test]
    fn test_times_no_optimization_non_quotation() {
        // 5 times
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

        // Find the JumpIfTrue and verify it targets the Drop
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
        let back_jump_pos = ops.len() - 2;
        if let Op::Jump(offset) = ops[back_jump_pos] {
            let target = back_jump_pos as i32 + offset;
            assert_eq!(target, 1, "backward jump should target Dup");
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
