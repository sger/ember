//! # Ember Abstract Syntax Tree
//!
//! This module defines the Abstract Syntax Tree (AST) for the Ember language.
//! The AST is produced by the parser and consumed by the interpreter or
//! bytecode compiler.
//!
//! ## Documentation conventions
//!
//! - Stack effects are written as `( before -- after )`.
//! - `{ ... }` denotes an Ember list literal.
//! - `[ ... ]` denotes an Ember quotation (anonymous function).

/// Runtime value in the Ember language.
///
/// Values are the only data that can exist on the Ember data stack.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// 64-bit signed integer.
    Integer(i64),

    /// 64-bit floating-point number.
    Float(f64),

    /// UTF-8 string value.
    String(String),

    /// Boolean value.
    Bool(bool),

    /// List literal value: `{ 1 2 3 }`.
    List(Vec<Value>),

    /// Quotation (anonymous function): `[ dup * ]`.
    ///
    /// Quotations are executable sequences of AST nodes and can be passed
    /// to higher-order combinators or executed via `Call`.
    Quotation(Vec<Node>),
}

impl std::fmt::Display for Value {
    /// Format a value using Ember surface syntax.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Integer(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::List(items) => {
                write!(f, "{{ ")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, " }}")
            }
            Value::Quotation(_) => write!(f, "[...]"),
        }
    }
}

/// Abstract Syntax Tree node for the Ember language.
///
/// Each `Node` represents a single executable or structural element
/// in an Ember program.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    // ───────────────────────────── Literals ─────────────────────────────
    /// Push a literal value onto the stack.
    ///
    /// Stack effect: `( -- x )`
    Literal(Value),

    // ─────────────────────────── Stack operations ───────────────────────
    /// Duplicate the top stack value.
    ///
    /// Stack effect: `( x -- x x )`
    Dup,

    /// Drop the top stack value.
    ///
    /// Stack effect: `( x -- )`
    Drop,

    /// Swap the top two stack values.
    ///
    /// Stack effect: `( a b -- b a )`
    Swap,

    /// Copy the second value to the top.
    ///
    /// Stack effect: `( a b -- a b a )`
    Over,

    /// Rotate the top three values.
    ///
    /// Stack effect: `( a b c -- b c a )`
    Rot,

    // ───────────────────────────── Arithmetic ───────────────────────────
    /// Add two numbers.
    ///
    /// Stack effect: `( a b -- a+b )`
    Add,

    /// Subtract two numbers.
    ///
    /// Stack effect: `( a b -- a-b )`
    Sub,

    /// Multiply two numbers.
    ///
    /// Stack effect: `( a b -- a*b )`
    Mul,

    /// Divide two numbers.
    ///
    /// Stack effect: `( a b -- a/b )`
    Div,

    /// Modulo operation.
    ///
    /// Stack effect: `( a b -- a%b )`
    Mod,

    /// Negate a number.
    ///
    /// Stack effect: `( x -- -x )`
    Neg,

    /// Absolute value.
    ///
    /// Stack effect: `( x -- |x| )`
    Abs,

    // ───────────────────────────── Comparison ───────────────────────────
    /// Equality comparison.
    ///
    /// Stack effect: `( a b -- bool )`
    Eq,

    /// Inequality comparison.
    ///
    /// Stack effect: `( a b -- bool )`
    NotEq,

    /// Less-than comparison.
    ///
    /// Stack effect: `( a b -- bool )`
    Lt,

    /// Greater-than comparison.
    ///
    /// Stack effect: `( a b -- bool )`
    Gt,

    /// Less-than or equal comparison.
    ///
    /// Stack effect: `( a b -- bool )`
    LtEq,

    /// Greater-than or equal comparison.
    ///
    /// Stack effect: `( a b -- bool )`
    GtEq,

    // ────────────────────────────── Logic ───────────────────────────────
    /// Logical AND.
    ///
    /// Stack effect: `( a b -- bool )`
    And,

    /// Logical OR.
    ///
    /// Stack effect: `( a b -- bool )`
    Or,

    /// Logical NOT.
    ///
    /// Stack effect: `( a -- bool )`
    Not,

    // ──────────────────────────── Control flow ──────────────────────────
    /// Conditional branching.
    ///
    /// Expected stack usage: `( cond [then] [else] -- ... )`
    If,

    /// Conditional execution.
    ///
    /// Expected stack usage: `( cond [body] -- ... )`
    When,

    /// Execute a quotation.
    ///
    /// Expected stack usage: `( [q] -- ... )`
    Call,

    // ───────────────────── Loops & higher-order combinators ─────────────
    /// Execute a quotation `n` times.
    ///
    /// Expected stack usage: `( n [body] -- ... )`
    Times,

    /// Apply a quotation to each element of a list.
    ///
    /// Expected stack usage: `( {xs} [f] -- )`
    Each,

    /// Map a quotation over a list.
    ///
    /// Expected stack usage: `( {xs} [f] -- {ys} )`
    Map,

    /// Filter a list using a predicate quotation.
    ///
    /// Expected stack usage: `( {xs} [pred] -- {xs'} )`
    Filter,

    /// Fold (reduce) a list with an accumulator.
    ///
    /// Expected stack usage: `( init {xs} [f] -- result )`
    Fold,

    /// Generate an integer range list.
    ///
    /// Expected stack usage: `( start end -- {range} )`
    Range,

    // ─────────────────────────── List operations ─────────────────────────
    /// Length of a list or string.
    ///
    /// Stack effect: `( x -- n )`
    Len,

    /// First element of a list.
    ///
    /// Stack effect: `( {x xs...} -- x )`
    Head,

    /// Tail of a list.
    ///
    /// Stack effect: `( {x xs...} -- {xs...} )`
    Tail,

    /// Prepend an element to a list.
    ///
    /// Stack effect: `( x {xs} -- {x xs} )`
    Cons,

    /// Concatenate two lists.
    ///
    /// Stack effect: `( {a} {b} -- {a+b} )`
    Concat,

    /// Concatenate two strings.
    ///
    /// Stack effect: `( "a" "b" -- "ab" )`
    StringConcat,

    // ─────────────────────────────── I/O ────────────────────────────────
    /// Print the top stack value.
    ///
    /// Stack effect: `( x -- )`
    Print,

    /// Emit a character.
    ///
    /// Stack effect: `( n -- )`
    Emit,

    /// Read input and push it onto the stack.
    ///
    /// Stack effect: `( -- x )`
    Read,

    /// Debug-print VM state.
    Debug,

    // ───────────────────────── Additional built-ins ─────────────────────
    /// Minimum of two numbers.
    Min,

    /// Maximum of two numbers.
    Max,

    /// Exponentiation.
    Pow,

    /// Square root.
    Sqrt,

    /// Nth element of a list.
    Nth,

    /// Append an element to a list.
    Append,

    /// Sort a list.
    Sort,

    /// Reverse a list.
    Reverse,

    /// Convert a string into a list of characters.
    Chars,

    /// Join a list into a string.
    Join,

    /// Split a string into a list.
    Split,

    /// Convert string to uppercase.
    Upper,

    /// Convert string to lowercase.
    Lower,

    /// Trim whitespace from a string.
    Trim,

    /// Clear the data stack.
    Clear,

    /// Push the current stack depth.
    Depth,

    /// Push the type of the top value.
    Type,

    /// Convert a value to string.
    ToString,

    /// Convert a value to integer.
    ToInt,

    // ───────────────────────── Word references ──────────────────────────
    /// Call a user-defined word.
    Word(String),

    /// Call a module-qualified word.
    QualifiedWord {
        /// Module name.
        module: String,
        /// Word name.
        word: String,
    },

    // ─────────────────────────── Definitions ────────────────────────────
    /// Define a new word.
    Def {
        /// Name of the word.
        name: String,
        /// Body of the word.
        body: Vec<Node>,
    },

    /// Declare a module.
    Module {
        /// Module name.
        name: String,
        /// Module definitions.
        definitions: Vec<Node>,
    },

    /// Import module items into scope.
    Use {
        /// Module name.
        module: String,
        /// Imported item(s).
        item: UseItem,
    },

    /// Import another Ember source file.
    Import(String),
}

/// Item selection in a `use` statement.
#[derive(Debug, Clone, PartialEq)]
pub enum UseItem {
    /// Import a single word.
    Single(String),
    /// Import all words from a module.
    All,
}

/// Parsed Ember program.
#[derive(Debug, Clone)]
pub struct Program {
    /// Top-level definitions.
    pub definitions: Vec<Node>,
    /// Main executable nodes.
    pub main: Vec<Node>,
}
