use super::node::Node;
use crate::bytecode::op::Op;
use serde::{Deserialize, Serialize};

/// Runtime value in the Ember language.
///
/// Values are the only data that can exist on the Ember data stack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

    CompiledQuotation(Vec<Op>),
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
            Value::CompiledQuotation(_) => write!(f, "[<compiled>]"),
        }
    }
}
