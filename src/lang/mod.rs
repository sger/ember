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

pub mod node;
pub mod program;
pub mod use_item;
pub mod value;
