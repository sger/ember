use std::ops;

use crate::bytecode::Op;

#[derive(Debug)]
pub struct StackCheckError {
    pub message: String,
}

impl std::fmt::Display for StackCheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "stack-check error: {}", self.message)
    }
}

impl StackCheckError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Returns (pops, pushes) for an op, or None if effect is unknown/dynamic.
fn effect(op: &Op) -> Option<(i32, i32)> {
    use Op::*;
    Some(match op {
        Push(_) => (0, 1),

        Dup => (1, 2),
        Drop => (1, 0),
        Swap => (2, 2),
        Over => (2, 3),
        Rot => (3, 3),

        Add | Sub | Mul | Div | Mod => (2, 1),
        Neg | Abs => (1, 1),

        Eq | Ne | Lt | Gt | Le | Ge => (2, 1),

        And | Or => (2, 1),
        Not => (1, 1),

        // =================================================================
        // Phase 3: Jump instructions
        // =================================================================
        Jump(_) => (0, 0),
        JumpIfFalse(_) => (1, 0),
        JumpIfTrue(_) => (1, 0),

        // Control (quotation-based)
        If => (3, 0),
        When => (2, 0),
        Call => (1, 0),

        // Combinators
        Dip => (2, 0),     // ( a quot -- ... a ) - dynamic result
        Keep => (2, 0),    // ( a quot -- ... a ) - dynamic result
        Bi => (3, 0),      // ( a p q -- ... ) - dynamic
        Bi2 => (4, 0),     // ( a b p q -- ... ) - dynamic
        Tri => (4, 0),     // ( a p q r -- ... ) - dynamic
        Both => (3, 0),    // ( a b quot -- ... ) - dynamic
        Compose => (2, 1), // ( quot quot -- quot )
        Curry => (2, 1),   // ( value quot -- quot )
        Apply => (2, 0),   // ( list quot -- ... ) - dynamic

        // Loops & higher-order
        Times => (2, 0),
        Each => (2, 0),
        Map => (2, 1),
        Filter => (2, 1),
        Fold => (3, 1),
        Range => (2, 1),

        // List ops
        Len => (1, 1),
        Head => (1, 1),
        Tail => (1, 1),
        Cons => (2, 1),
        Concat => (2, 1),
        StringConcat => (2, 1),

        // I/O
        Print => (1, 0),
        Emit => (1, 0),
        Read => (0, 1),
        Debug => (1, 1),

        // Additional builtins
        Min | Max | Pow => (2, 1),
        Sqrt => (1, 1),
        Nth => (2, 1),
        Append => (2, 1),
        Sort | Reverse => (1, 1),
        Chars => (1, 1),
        Join => (2, 1),
        Split => (2, 1),
        Upper | Lower | Trim => (1, 1),
        Clear => (0, 0), // Actually clears stack, but can't express that
        Depth => (0, 1),
        Type => (1, 2),
        ToString => (1, 1),
        ToInt => (1, 1),

        // Aux stack ops - from main stack perspective:
        // ToAux pops 1 from main, pushes 0 to main (moves to aux)
        // FromAux pops 0 from main, pushes 1 to main (moves from aux)
        ToAux => (1, 0),
        FromAux => (0, 1),

        Return => (0, 0),

        // Unknown effect - can't statically analyze
        CallWord(_) => return None,
        CallQualified { .. } => return None,
    })
}

/// Check stack effects with a given initial stack height.
///
/// NOTE: This is a simple linear scan that doesn't follow jump targets.
/// For jump-based code, this provides basic validation but not full
/// control-flow analysis. A complete solution would need to:
/// 1. Build a control flow graph
/// 2. Track stack heights at each basic block entry
/// 3. Verify heights match at join points
pub fn check_ops_with_initial(ops: &[Op], initial_height: i32) -> Result<(), StackCheckError> {
    let mut h: i32 = initial_height;

    for (ip, op) in ops.iter().enumerate() {
        match effect(op) {
            Some((pops, pushes)) => {
                h -= pops;
                if h < 0 {
                    return Err(StackCheckError::new(format!(
                        "stack underflow at ip={}, op={:?}, needed {} items",
                        ip, op, pops
                    )));
                }
                h += pushes;
            }
            None => {
                // Unknown effect (e.g., user-defined call). From here, we can't
                // soundly reason about stack height, so stop checking.
                return Ok(());
            }
        }
    }

    Ok(())
}

/// Check stack effects starting from empty stack.
pub fn check_ops(ops: &[Op]) -> Result<(), StackCheckError> {
    check_ops_with_initial(ops, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::value::Value;

    #[test]
    fn test_simple_ops() {
        let ops = vec![
            Op::Push(Value::Integer(1)),
            Op::Push(Value::Integer(2)),
            Op::Add,
        ];
        assert!(check_ops(&ops).is_ok());
    }

    #[test]
    fn test_underflow() {
        let ops = vec![Op::Add];
        let result = check_ops(&ops);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("underflow"));
    }

    #[test]
    fn test_jump_no_stack_effect() {
        let ops = vec![
            Op::Push(Value::Integer(1)),
            Op::Jump(2),
            Op::Push(Value::Integer(2)),
            Op::Push(Value::Integer(3)),
        ];
        assert!(check_ops(&ops).is_ok());
    }

    #[test]
    fn test_jump_if_false_pops_one() {
        let ops = vec![
            Op::Push(Value::Bool(true)),
            Op::JumpIfFalse(2),
            Op::Push(Value::Integer(1)),
        ];
        assert!(check_ops(&ops).is_ok());
    }

    #[test]
    fn test_jump_if_false_underflow() {
        let ops = vec![Op::JumpIfFalse(2)];
        assert!(check_ops(&ops).is_err());
    }

    #[test]
    fn test_jump_if_true_pops_one() {
        let ops = vec![
            Op::Push(Value::Bool(false)),
            Op::JumpIfTrue(2),
            Op::Push(Value::Integer(1)),
        ];
        assert!(check_ops(&ops).is_ok());
    }

    #[test]
    fn test_if_with_jumps_pattern() {
        let ops = vec![
            Op::Push(Value::Bool(true)),
            Op::JumpIfFalse(3),
            Op::Push(Value::Integer(10)),
            Op::Jump(2),
            Op::Push(Value::Integer(20)),
        ];
        assert!(check_ops(&ops).is_ok());
    }

    #[test]
    fn test_combinators_stack_effects() {
        // Compose: takes 2 quotations, produces 1
        let ops = vec![
            Op::Push(Value::CompiledQuotation(vec![])),
            Op::Push(Value::CompiledQuotation(vec![])),
            Op::Compose,
        ];
        assert!(check_ops(&ops).is_ok());

        // Curry: takes value + quotation, produces quotation
        let ops = vec![
            Op::Push(Value::Integer(1)),
            Op::Push(Value::CompiledQuotation(vec![])),
            Op::Curry,
        ];
        assert!(check_ops(&ops).is_ok());
    }

    #[test]
    fn test_dip_underflow() {
        // Dip needs 2 items (value and quotation)
        let ops = vec![Op::Push(Value::CompiledQuotation(vec![])), Op::Dip];
        assert!(check_ops(&ops).is_err());
    }

    #[test]
    fn test_bi_needs_three() {
        // Bi needs value + 2 quotations
        let ops = vec![
            Op::Push(Value::Integer(1)),
            Op::Push(Value::CompiledQuotation(vec![])),
            Op::Bi, // Missing second quotation
        ];
        assert!(check_ops(&ops).is_err());
    }

    #[test]
    fn test_call_word_stops_analysis() {
        // After CallWord, we can't know the stack effect
        let ops = vec![
            Op::Push(Value::Integer(1)),
            Op::CallWord("unknown".to_string()),
            Op::Add, // This might underflow, but we can't know
        ];
        // Should return Ok because we stop analyzing at CallWord
        assert!(check_ops(&ops).is_ok());
    }
}
