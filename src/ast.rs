#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),
    List(Vec<Value>),
    Quotation(Vec<Node>),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Integer(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::String(n) => write!(f, "{}", n),
            Value::Bool(n) => write!(f, "{}", n),
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

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    // Literals
    Literal(Value),

    // Stack operations
    Dup,
    Drop,
    Swap,
    Over,
    Rot,

    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,
    Abs,

    // Comparison
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,

    // Logic
    And,
    Or,
    Not,

    // Control flow
    If,
    When,
    Call,

    // Loops & higher-order
    Times,
    Each,
    Map,
    Filter,
    Fold,
    Range,

    // List operations
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

    // Additional builtins
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

    // Word reference (call user-defined word)
    Word(String),

    // Qualified word reference: Module.word
    QualifiedWord {
        module: String,
        word: String,
    },

    // Definition
    Def {
        name: String,
        body: Vec<Node>,
    },

    // Module declaration
    Module {
        name: String,
        definitions: Vec<Node>,
    },

    // Use statement
    Use {
        module: String,
        item: UseItem,
    },

    // Import
    Import(String),
}

#[derive(Debug, Clone)]
pub struct Program {
    pub definitions: Vec<Node>,
    pub main: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UseItem {
    Single(String), // use Module.word
    All,            // use Module.*
}
