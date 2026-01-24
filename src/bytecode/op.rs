use crate::lang::value::Value;
use serde::{Deserialize, Serialize};

// =============================================================================
// OP - Bytecode instructions
// =============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Op {
    // literals
    Push(Value),

    // stack ops
    Dup,
    Drop,
    Swap,
    Over,
    Rot,

    // arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,
    Abs,

    // comparison
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,

    // logic
    And,
    Or,
    Not,

    // ==========================================================================
    // Control flow - quotation-based (kept for dynamic quotations)
    // ==========================================================================
    If,   // ( cond then-quot else-quot -- result )
    When, // ( cond then-quot -- )
    Call, // ( quot -- result )

    // ==========================================================================
    // Phase 3: Jump instructions for flat control flow
    // ==========================================================================
    /// Unconditional relative jump. Offset is added to current ip.
    /// Jump(1) skips next instruction, Jump(0) is a no-op, Jump(-1) loops forever.
    Jump(i32),

    /// Pop bool from stack, jump if false. If true, continue to next instruction.
    JumpIfFalse(i32),

    /// Pop bool from stack, jump if true. If false, continue to next instruction.
    JumpIfTrue(i32),
    Return,

    // loops & higher-order (still quotation-based for now)
    Times,
    Each,
    Map,
    Filter,
    Fold,
    Range,

    // list ops
    Len,
    Head,
    Tail,
    Cons,
    Concat,
    StringConcat,

    // I/O
    Print,
    Emit,
    Read,
    Debug,

    // stdlib
    Min,
    Max,
    Pow,
    Sqrt,
    Nth,
    Append,
    Sort,
    Reverse,
    Chars,
    Join,
    Split,
    Upper,
    Lower,
    Trim,
    Clear,
    Depth,
    Type,
    ToString,
    ToInt,

    Dip,
    Keep,
    Bi,
    Bi2,
    Tri,
    Both,
    Compose,
    Curry,
    Apply,

    // User-defined word calls
    CallWord(String),
    CallQualified {
        module: String,
        word: String,
    },

    // ==========================================================================
    // Auxiliary stack operations (for internal use by compiler)
    // ==========================================================================
    /// Move top of main stack to auxiliary stack
    ToAux,
    /// Move top of auxiliary stack to main stack
    FromAux,
}
