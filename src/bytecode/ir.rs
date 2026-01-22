//! Bytecode IR (intermediate representation) types.
//!
//! This module defines the structural container for compiled bytecode programs.
//! The bytecode compiler lowers a parsed program (AST) into a [`ProgramBc`],
//! and the bytecode VM executes its [`Op`] instruction streams.
//!
//! # Design notes
//! - A program consists of one or more [`CodeObject`]s.
//! - `code[0]` is reserved for the entry point ("main").
//! - Additional code objects can represent compiled word bodies, functions,
//!   or compiled quotations.

use crate::{bytecode::Op, frontend::lexer::Span};

/// A compiled bytecode program.
///
/// A [`ProgramBc`] is the output of the bytecode compiler and the input to the
/// bytecode VM. It is a collection of [`CodeObject`]s, where each code object
/// contains a linear sequence of [`Op`] instructions.
///
/// # Invariants
/// - `code[0]` is always the entry point (main).
#[derive(Debug, Clone)]
pub struct ProgramBc {
    /// Collection of code objects (instruction streams).
    ///
    /// Convention:
    /// - `code[0]` is always `main`.
    pub code: Vec<CodeObject>,
}

/// A single compiled instruction stream.
///
/// A [`CodeObject`] represents executable bytecode. It is typically used for:
/// - the program entry point (`main`)
/// - compiled word bodies
/// - compiled quotations (if you choose to store them as code objects)
///
/// The VM executes the `ops` vector sequentially using an instruction pointer.
/// Control flow is encoded using jump-like [`Op`] variants.
#[derive(Debug, Clone)]
pub struct CodeObject {
    /// Linear bytecode instructions executed by the VM.
    pub ops: Vec<Op>,
}

impl CodeObject {
    /// Create an empty code object.
    ///
    /// The bytecode compiler generally emits instructions into a fresh
    /// [`CodeObject`] and later appends a terminator such as `Op::Return`
    /// (depending on your instruction set design).
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }
}
