use crate::lang::{node::Node, value::Value};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum CompileError {
    /// A node type that the compiler doesn't know how to handle
    UnhandledNode {
        node_type: String,
        hint: Option<String>,
    },
    /// A node that's valid but appeared in an invalid position
    InvalidPosition {
        node_type: String,
        name: Option<String>,
        reason: String,
        hint: Option<String>,
    },
    /// Internal compiler error (shouldn't happen in normal use)
    Internal(String),
}

impl CompileError {
    /// Create an error for an unhandled node type
    #[allow(dead_code)]
    pub fn unhandled(node: &Node) -> Self {
        CompileError::UnhandledNode {
            node_type: node_type_name(node).to_string(),
            hint: Some(
                "this may be a language feature not yet supported by the bytecode compiler"
                    .to_string(),
            ),
        }
    }

    /// Create an error for an unhandled node with custom hint
    #[allow(dead_code)]
    pub fn unhandled_with_hint(node: &Node, hint: impl Into<String>) -> Self {
        CompileError::UnhandledNode {
            node_type: node_type_name(node).to_string(),
            hint: Some(hint.into()),
        }
    }

    /// Create an error for a definition in runtime position
    pub fn def_in_runtime(name: &str) -> Self {
        CompileError::InvalidPosition {
            node_type: "def".to_string(),
            name: Some(name.to_string()),
            reason: "definitions cannot appear in runtime position".to_string(),
            hint: Some(
                "definitions must be at the top level, not inside quotations or expressions"
                    .to_string(),
            ),
        }
    }

    /// Create an error for a module in runtime position
    pub fn module_in_runtime(name: &str) -> Self {
        CompileError::InvalidPosition {
            node_type: "module".to_string(),
            name: Some(name.to_string()),
            reason: "modules cannot appear in runtime position".to_string(),
            hint: Some("modules must be declared at the top level".to_string()),
        }
    }

    /// Create an error for a use statement in runtime position
    pub fn use_in_runtime(module: &str, item: &str) -> Self {
        CompileError::InvalidPosition {
            node_type: "use".to_string(),
            name: Some(format!("{}.{}", module, item)),
            reason: "use statements cannot appear in runtime position".to_string(),
            hint: Some("use statements must be at the top level".to_string()),
        }
    }

    /// Create an error for an import in runtime position
    pub fn import_in_runtime(path: &str) -> Self {
        CompileError::InvalidPosition {
            node_type: "import".to_string(),
            name: Some(path.to_string()),
            reason: "imports cannot appear in runtime position".to_string(),
            hint: Some("imports must be at the top level".to_string()),
        }
    }

    /// Create an internal compiler error
    #[allow(dead_code)]
    pub fn internal(msg: impl Into<String>) -> Self {
        CompileError::Internal(msg.into())
    }

    /// Backward compatibility with existing code
    pub fn new(msg: impl Into<String>) -> Self {
        CompileError::Internal(msg.into())
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::UnhandledNode { node_type, hint } => {
                write!(f, "compile error: cannot compile '{}' node", node_type)?;
                if let Some(h) = hint {
                    write!(f, "\n  hint: {}", h)?;
                }
                Ok(())
            }
            CompileError::InvalidPosition {
                node_type,
                name,
                reason,
                hint,
            } => {
                write!(f, "compile error: ")?;
                match name {
                    Some(n) => write!(f, "{} '{}': {}", node_type, n, reason)?,
                    None => write!(f, "{}: {}", node_type, reason)?,
                }
                if let Some(h) = hint {
                    write!(f, "\n  hint: {}", h)?;
                }
                Ok(())
            }
            CompileError::Internal(msg) => {
                write!(f, "compile error: internal error: {}", msg)
            }
        }
    }
}

impl std::error::Error for CompileError {}

/// Extract a human-readable name for a node type
fn node_type_name(node: &Node) -> &'static str {
    match node {
        Node::Literal(v) => match v {
            Value::Integer(_) => "integer literal",
            Value::Float(_) => "float literal",
            Value::String(_) => "string literal",
            Value::Bool(_) => "bool literal",
            Value::List(_) => "list literal",
            Value::Quotation(_) => "quotation",
            Value::CompiledQuotation(_) => "compiled quotation",
        },
        Node::Dup => "dup",
        Node::Drop => "drop",
        Node::Swap => "swap",
        Node::Over => "over",
        Node::Rot => "rot",
        Node::Add => "+",
        Node::Sub => "-",
        Node::Mul => "*",
        Node::Div => "/",
        Node::Mod => "%",
        Node::Neg => "neg",
        Node::Abs => "abs",
        Node::Eq => "=",
        Node::NotEq => "!=",
        Node::Lt => "<",
        Node::Gt => ">",
        Node::LtEq => "<=",
        Node::GtEq => ">=",
        Node::And => "and",
        Node::Or => "or",
        Node::Not => "not",
        Node::If => "if",
        Node::When => "when",
        Node::Call => "call",
        Node::Times => "times",
        Node::Each => "each",
        Node::Map => "map",
        Node::Filter => "filter",
        Node::Fold => "fold",
        Node::Range => "range",
        Node::Len => "len",
        Node::Head => "head",
        Node::Tail => "tail",
        Node::Cons => "cons",
        Node::Concat => "concat",
        Node::StringConcat => "++",
        Node::Print => "print",
        Node::Emit => "emit",
        Node::Read => "read",
        Node::Debug => "debug",
        Node::Min => "min",
        Node::Max => "max",
        Node::Pow => "pow",
        Node::Sqrt => "sqrt",
        Node::Nth => "nth",
        Node::Append => "append",
        Node::Sort => "sort",
        Node::Reverse => "reverse",
        Node::Chars => "chars",
        Node::Join => "join",
        Node::Split => "split",
        Node::Upper => "upper",
        Node::Lower => "lower",
        Node::Trim => "trim",
        Node::Clear => "clear",
        Node::Depth => "depth",
        Node::Type => "type",
        Node::ToString => "to-string",
        Node::ToInt => "to-int",
        Node::Dip => "dip",
        Node::Keep => "keep",
        Node::Bi => "bi",
        Node::Bi2 => "bi2",
        Node::Tri => "tri",
        Node::Both => "both",
        Node::Compose => "compose",
        Node::Curry => "curry",
        Node::Apply => "apply",
        Node::Def { .. } => "def",
        Node::Module { .. } => "module",
        Node::Word(_) => "word",
        Node::QualifiedWord { .. } => "qualified word",
        Node::Use { .. } => "use",
        Node::Import(_) => "import",
        #[allow(unreachable_patterns)]
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unhandled_node_display() {
        let node = Node::Dup;
        let err = CompileError::unhandled(&node);

        let msg = err.to_string();
        assert!(msg.contains("cannot compile"));
        assert!(msg.contains("dup"));
        assert!(msg.contains("hint"));
    }

    #[test]
    fn test_unhandled_with_custom_hint() {
        let node = Node::Add;
        let err = CompileError::unhandled_with_hint(&node, "custom hint here");

        let msg = err.to_string();
        assert!(msg.contains("custom hint here"));
    }

    #[test]
    fn test_def_in_runtime_display() {
        let err = CompileError::def_in_runtime("my-word");

        let msg = err.to_string();
        assert!(msg.contains("def"));
        assert!(msg.contains("my-word"));
        assert!(msg.contains("runtime position"));
        assert!(msg.contains("hint"));
        assert!(msg.contains("top level"));
    }

    #[test]
    fn test_module_in_runtime_display() {
        let err = CompileError::module_in_runtime("my-module");

        let msg = err.to_string();
        assert!(msg.contains("module"));
        assert!(msg.contains("my-module"));
        assert!(msg.contains("runtime position"));
    }

    #[test]
    fn test_use_in_runtime_display() {
        let err = CompileError::use_in_runtime("math", "sqrt");

        let msg = err.to_string();
        assert!(msg.contains("use"));
        assert!(msg.contains("math.sqrt"));
        assert!(msg.contains("runtime position"));
    }

    #[test]
    fn test_import_in_runtime_display() {
        let err = CompileError::import_in_runtime("./lib/utils.ember");

        let msg = err.to_string();
        assert!(msg.contains("import"));
        assert!(msg.contains("./lib/utils.ember"));
        assert!(msg.contains("runtime position"));
    }

    #[test]
    fn test_internal_error_display() {
        let err = CompileError::internal("something went wrong");

        let msg = err.to_string();
        assert!(msg.contains("internal"));
        assert!(msg.contains("something went wrong"));
    }

    #[test]
    fn test_new_creates_internal_error() {
        let err = CompileError::new("legacy error");

        assert!(matches!(err, CompileError::Internal(_)));
        assert!(err.to_string().contains("legacy error"));
    }

    #[test]
    fn test_error_implements_std_error() {
        let err = CompileError::internal("test");
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_error_clone() {
        let err1 = CompileError::def_in_runtime("word");
        let err2 = err1.clone();

        assert_eq!(err1.to_string(), err2.to_string());
    }

    #[test]
    fn test_node_type_name_literals() {
        assert_eq!(
            node_type_name(&Node::Literal(Value::Integer(42))),
            "integer literal"
        );
        assert_eq!(
            node_type_name(&Node::Literal(Value::String("hi".to_string()))),
            "string literal"
        );
        assert_eq!(
            node_type_name(&Node::Literal(Value::Bool(true))),
            "bool literal"
        );
    }

    #[test]
    fn test_node_type_name_operators() {
        assert_eq!(node_type_name(&Node::Add), "+");
        assert_eq!(node_type_name(&Node::Eq), "=");
        assert_eq!(node_type_name(&Node::Lt), "<");
    }

    #[test]
    fn test_use_all_in_runtime_display() {
        let err = CompileError::use_in_runtime("math", "*");

        let msg = err.to_string();
        assert!(msg.contains("use"));
        assert!(msg.contains("math.*"));
        assert!(msg.contains("runtime position"));
    }
}
