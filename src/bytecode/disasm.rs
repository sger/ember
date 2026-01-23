use crate::bytecode::{Op, ProgramBc};
use crate::lang::value::Value;
use std::collections::HashMap;

/// Print disassembly of a bytecode program
pub fn print_bc(bc: &ProgramBc) {
    println!("=== BYTECODE PROGRAM ===\n");

    // Print main code
    for (ci, code) in bc.code.iter().enumerate() {
        let label = if ci == 0 {
            "main".to_string()
        } else {
            format!("code[{}]", ci)
        };
        print_code_object(&label, &code.ops, 0);
    }

    // Print word definitions (sorted alphabetically)
    let mut words: Vec<_> = bc.words.iter().collect();
    words.sort_by_key(|(name, _)| *name);

    for (name, ops) in words {
        print_code_object(name, ops, 0);
    }
}

/// Print a single code object with optional indentation
fn print_code_object(name: &str, ops: &[Op], indent: usize) {
    let prefix = "  ".repeat(indent);

    println!("{}════════════════════════════════════════", prefix);
    println!("{} {}", prefix, name);
    println!("{} {} instructions", prefix, ops.len());
    println!("{}════════════════════════════════════════", prefix);
    disassemble_ops(ops, indent);
    println!();
}

/// Disassemble a slice of ops with indentation support
pub fn disassemble_ops(ops: &[Op], indent: usize) {
    let jump_targets = collect_jump_targets(ops);
    let prefix = "  ".repeat(indent);

    for (ip, op) in ops.iter().enumerate() {
        if jump_targets.contains(&ip) {
            println!("{}      ┌──────────────────────────────────", prefix);
        }

        print!("{}{:04} ", prefix, ip);

        if jump_targets.contains(&ip) {
            print!("► ");
        } else {
            print!("  ");
        }

        print_op(op, ip, indent);
    }
}

fn collect_jump_targets(ops: &[Op]) -> Vec<usize> {
    let mut targets = Vec::new();

    for (ip, op) in ops.iter().enumerate() {
        let offset = match op {
            Op::Jump(offset) => Some(*offset),
            Op::JumpIfFalse(offset) => Some(*offset),
            Op::JumpIfTrue(offset) => Some(*offset),
            _ => None,
        };

        if let Some(offset) = offset {
            let target = (ip as i32 + offset) as usize;
            if !targets.contains(&target) {
                targets.push(target);
            }
        }
    }

    targets
}

fn print_op(op: &Op, ip: usize, indent: usize) {
    let prefix = "  ".repeat(indent);

    match op {
        // Literals - with special handling for quotations
        Op::Push(v) => match v {
            Value::CompiledQuotation(inner_ops) => {
                println!("PUSH        [");
                print_inline_quotation(inner_ops, indent + 1);
                println!("{}          ]", prefix);
            }
            Value::List(items) if contains_quotation(items) => {
                println!("PUSH        {{");
                print_list_items(items, indent + 1);
                println!("{}          }}", prefix);
            }
            _ => println!("PUSH        {}", format_value(v)),
        },

        // Stack operations
        Op::Dup => println!("DUP"),
        Op::Drop => println!("DROP"),
        Op::Swap => println!("SWAP"),
        Op::Over => println!("OVER"),
        Op::Rot => println!("ROT"),

        // Auxiliary stack operations
        Op::ToAux => println!("TO_AUX      ; ( a -- ) R:( -- a )"),
        Op::FromAux => println!("FROM_AUX    ; ( -- a ) R:( a -- )"),

        // Arithmetic
        Op::Add => println!("ADD"),
        Op::Sub => println!("SUB"),
        Op::Mul => println!("MUL"),
        Op::Div => println!("DIV"),
        Op::Mod => println!("MOD"),
        Op::Neg => println!("NEG"),
        Op::Abs => println!("ABS"),

        // Comparison
        Op::Eq => println!("EQ"),
        Op::Ne => println!("NE"),
        Op::Lt => println!("LT"),
        Op::Gt => println!("GT"),
        Op::Le => println!("LE"),
        Op::Ge => println!("GE"),

        // Logic
        Op::And => println!("AND"),
        Op::Or => println!("OR"),
        Op::Not => println!("NOT"),

        // Control flow - quotation based
        Op::If => println!("IF          ; ( cond then else -- result )"),
        Op::When => println!("WHEN        ; ( cond then -- )"),
        Op::Call => println!("CALL        ; ( quot -- result )"),

        // Control flow - jumps
        Op::Jump(offset) => {
            let target = (ip as i32 + *offset) as usize;
            let direction = if *offset < 0 { "↑" } else { "↓" };
            println!("JUMP        {:+} {} (→ {:04})", offset, direction, target);
        }
        Op::JumpIfFalse(offset) => {
            let target = (ip as i32 + *offset) as usize;
            let direction = if *offset < 0 { "↑" } else { "↓" };
            println!("JUMP_FALSE  {:+} {} (→ {:04})", offset, direction, target);
        }
        Op::JumpIfTrue(offset) => {
            let target = (ip as i32 + *offset) as usize;
            let direction = if *offset < 0 { "↑" } else { "↓" };
            println!("JUMP_TRUE   {:+} {} (→ {:04})", offset, direction, target);
        }

        // Loops & higher-order
        Op::Times => println!("TIMES       ; ( n quot -- )"),
        Op::Each => println!("EACH        ; ( list quot -- )"),
        Op::Map => println!("MAP         ; ( list quot -- list )"),
        Op::Filter => println!("FILTER      ; ( list quot -- list )"),
        Op::Fold => println!("FOLD        ; ( list init quot -- result )"),
        Op::Range => println!("RANGE       ; ( start end -- list )"),

        // List operations
        Op::Len => println!("LEN         ; ( list -- n )"),
        Op::Head => println!("HEAD        ; ( list -- item )"),
        Op::Tail => println!("TAIL        ; ( list -- list )"),
        Op::Cons => println!("CONS        ; ( item list -- list )"),
        Op::Concat => println!("CONCAT      ; ( list list -- list )"),
        Op::StringConcat => println!("STR_CONCAT  ; ( str str -- str )"),

        // I/O
        Op::Print => println!("PRINT       ; ( value -- )"),
        Op::Emit => println!("EMIT        ; ( char -- )"),
        Op::Read => println!("READ        ; ( -- str )"),
        Op::Debug => println!("DEBUG       ; ( value -- value )"),

        // Stdlib
        Op::Min => println!("MIN         ; ( a b -- min )"),
        Op::Max => println!("MAX         ; ( a b -- max )"),
        Op::Pow => println!("POW         ; ( base exp -- result )"),
        Op::Sqrt => println!("SQRT        ; ( n -- sqrt )"),
        Op::Nth => println!("NTH         ; ( list n -- item )"),
        Op::Append => println!("APPEND      ; ( list item -- list )"),
        Op::Sort => println!("SORT        ; ( list -- list )"),
        Op::Reverse => println!("REVERSE     ; ( list -- list )"),
        Op::Chars => println!("CHARS       ; ( str -- list )"),
        Op::Join => println!("JOIN        ; ( list sep -- str )"),
        Op::Split => println!("SPLIT       ; ( str sep -- list )"),
        Op::Upper => println!("UPPER       ; ( str -- str )"),
        Op::Lower => println!("LOWER       ; ( str -- str )"),
        Op::Trim => println!("TRIM        ; ( str -- str )"),
        Op::Clear => println!("CLEAR       ; ( ... -- )"),
        Op::Depth => println!("DEPTH       ; ( -- n )"),
        Op::Type => println!("TYPE        ; ( value -- str )"),
        Op::ToString => println!("TO_STRING   ; ( value -- str )"),
        Op::ToInt => println!("TO_INT      ; ( str -- int )"),

        // Combinators
        Op::Dip => println!("DIP         ; ( a quot -- a )"),
        Op::Keep => println!("KEEP        ; ( a quot -- a result )"),
        Op::Bi => println!("BI          ; ( a p q -- p(a) q(a) )"),
        Op::Bi2 => println!("BI2         ; ( a b p q -- p(a,b) q(a,b) )"),
        Op::Tri => println!("TRI         ; ( a p q r -- p(a) q(a) r(a) )"),
        Op::Both => println!("BOTH        ; ( a b quot -- quot(a) quot(b) )"),
        Op::Compose => println!("COMPOSE     ; ( quot quot -- quot )"),
        Op::Curry => println!("CURRY       ; ( value quot -- quot )"),
        Op::Apply => println!("APPLY       ; ( list quot -- result )"),

        // Word calls
        Op::CallWord(name) => println!("CALL_WORD   \"{}\"", name),
        Op::CallQualified { module, word } => {
            println!("CALL_QUAL   \"{}.{}\"", module, word)
        }

        // Return
        Op::Return => println!("RETURN"),
    }
}

/// Print inline quotation contents
fn print_inline_quotation(ops: &[Op], indent: usize) {
    let prefix = "  ".repeat(indent);
    let jump_targets = collect_jump_targets(ops);

    for (ip, op) in ops.iter().enumerate() {
        if jump_targets.contains(&ip) {
            println!("{}    ┌────────────────────────────", prefix);
        }

        print!("{}  {:04} ", prefix, ip);

        if jump_targets.contains(&ip) {
            print!("► ");
        } else {
            print!("  ");
        }

        print_op(op, ip, indent);
    }
}

/// Check if a list contains any quotations
fn contains_quotation(items: &[Value]) -> bool {
    items
        .iter()
        .any(|v| matches!(v, Value::Quotation(_) | Value::CompiledQuotation(_)))
}

/// Print list items with quotation expansion
fn print_list_items(items: &[Value], indent: usize) {
    let prefix = "  ".repeat(indent);

    for (i, item) in items.iter().enumerate() {
        match item {
            Value::CompiledQuotation(ops) => {
                println!("{}  [{}]: [", prefix, i);
                print_inline_quotation(ops, indent + 1);
                println!("{}        ]", prefix);
            }
            Value::Quotation(nodes) => {
                println!("{}  [{}]: [ <{} nodes> ]", prefix, i, nodes.len());
            }
            _ => {
                println!("{}  [{}]: {}", prefix, i, format_value(item));
            }
        }
    }
}

fn format_value(value: &Value) -> String {
    match value {
        Value::Integer(n) => format!("{}", n),
        Value::Float(f) => format!("{:?}", f),
        Value::String(s) => format!("{:?}", s),
        Value::Bool(b) => format!("{}", b),
        Value::List(items) => {
            if items.is_empty() {
                "{ }".to_string()
            } else if contains_quotation(items) {
                format!("{{ <{} items with quotations> }}", items.len())
            } else {
                let inner: Vec<String> = items.iter().map(format_value).collect();
                format!("{{ {} }}", inner.join(" "))
            }
        }
        Value::Quotation(nodes) => {
            format!("[ <{} nodes> ]", nodes.len())
        }
        Value::CompiledQuotation(ops) => {
            format!("[ <{} ops> ]", ops.len())
        }
    }
}

// =============================================================================
// Compact mode (optional - for less verbose output)
// =============================================================================

/// Print compact disassembly (no stack comments)
pub fn print_bc_compact(bc: &ProgramBc) {
    println!("=== BYTECODE (compact) ===\n");

    for (ci, code) in bc.code.iter().enumerate() {
        let label = if ci == 0 { "main" } else { "code" };
        println!("-- {} [{}] ({} ops) --", label, ci, code.ops.len());
        for (ip, op) in code.ops.iter().enumerate() {
            println!("  {:04}  {:?}", ip, op);
        }
        println!();
    }

    for (name, ops) in &bc.words {
        println!("-- {} ({} ops) --", name, ops.len());
        for (ip, op) in ops.iter().enumerate() {
            println!("  {:04}  {:?}", ip, op);
        }
        println!();
    }
}

// =============================================================================
// String output (for testing/logging)
// =============================================================================

/// Return disassembly as a String
pub fn disassemble_to_string(ops: &[Op]) -> String {
    let mut output = String::new();
    let jump_targets = collect_jump_targets(ops);

    for (ip, op) in ops.iter().enumerate() {
        if jump_targets.contains(&ip) {
            output.push_str("      ┌──────────────────────────────────\n");
        }

        output.push_str(&format!("{:04} ", ip));

        if jump_targets.contains(&ip) {
            output.push_str("► ");
        } else {
            output.push_str("  ");
        }

        output.push_str(&format_op_string(op, ip));
        output.push('\n');
    }

    output
}

fn format_op_string(op: &Op, ip: usize) -> String {
    match op {
        Op::Push(v) => format!("PUSH        {}", format_value(v)),
        Op::ToAux => "TO_AUX".to_string(),
        Op::FromAux => "FROM_AUX".to_string(),
        Op::Jump(offset) => {
            let target = (ip as i32 + *offset) as usize;
            format!("JUMP        {:+} (→ {:04})", offset, target)
        }
        Op::JumpIfFalse(offset) => {
            let target = (ip as i32 + *offset) as usize;
            format!("JUMP_FALSE  {:+} (→ {:04})", offset, target)
        }
        Op::JumpIfTrue(offset) => {
            let target = (ip as i32 + *offset) as usize;
            format!("JUMP_TRUE   {:+} (→ {:04})", offset, target)
        }
        Op::CallWord(name) => format!("CALL_WORD   \"{}\"", name),
        Op::CallQualified { module, word } => format!("CALL_QUAL   \"{}.{}\"", module, word),
        Op::Return => "RETURN".to_string(),
        other => format!("{:?}", other).to_uppercase(),
    }
}

// =============================================================================
// Statistics
// =============================================================================

/// Print bytecode statistics
pub fn print_bc_stats(bc: &ProgramBc) {
    println!("=== BYTECODE STATISTICS ===\n");

    let main_ops: usize = bc.code.iter().map(|c| c.ops.len()).sum();
    let word_ops: usize = bc.words.values().map(|ops| ops.len()).sum();
    let total_ops = main_ops + word_ops;

    println!("Code objects:     {}", bc.code.len());
    println!("Word definitions: {}", bc.words.len());
    println!();
    println!("Instructions:");
    println!("  main:           {}", main_ops);
    println!("  words:          {}", word_ops);
    println!("  total:          {}", total_ops);
    println!();

    // Count op types
    let mut op_counts: HashMap<&str, usize> = HashMap::new();

    for code in &bc.code {
        count_ops(&code.ops, &mut op_counts);
    }
    for ops in bc.words.values() {
        count_ops(ops, &mut op_counts);
    }

    println!("Op frequency:");
    let mut counts: Vec<_> = op_counts.iter().collect();
    counts.sort_by(|a, b| b.1.cmp(a.1));

    for (op, count) in counts.iter().take(10) {
        let pct = (**count as f64 / total_ops as f64) * 100.0;
        println!("  {:<14} {:>4} ({:>5.1}%)", op, count, pct);
    }
}

fn count_ops<'a>(ops: &'a [Op], counts: &mut HashMap<&'a str, usize>) {
    for op in ops {
        let name = op_name(op);
        *counts.entry(name).or_insert(0) += 1;

        // Count nested quotations
        if let Op::Push(Value::CompiledQuotation(inner)) = op {
            count_ops(inner, counts);
        }
    }
}

fn op_name(op: &Op) -> &'static str {
    match op {
        Op::Push(_) => "PUSH",
        Op::Dup => "DUP",
        Op::Drop => "DROP",
        Op::Swap => "SWAP",
        Op::Over => "OVER",
        Op::Rot => "ROT",
        Op::ToAux => "TO_AUX",
        Op::FromAux => "FROM_AUX",
        Op::Add => "ADD",
        Op::Sub => "SUB",
        Op::Mul => "MUL",
        Op::Div => "DIV",
        Op::Mod => "MOD",
        Op::Neg => "NEG",
        Op::Abs => "ABS",
        Op::Eq => "EQ",
        Op::Ne => "NE",
        Op::Lt => "LT",
        Op::Gt => "GT",
        Op::Le => "LE",
        Op::Ge => "GE",
        Op::And => "AND",
        Op::Or => "OR",
        Op::Not => "NOT",
        Op::If => "IF",
        Op::When => "WHEN",
        Op::Call => "CALL",
        Op::Jump(_) => "JUMP",
        Op::JumpIfFalse(_) => "JUMP_FALSE",
        Op::JumpIfTrue(_) => "JUMP_TRUE",
        Op::Times => "TIMES",
        Op::Each => "EACH",
        Op::Map => "MAP",
        Op::Filter => "FILTER",
        Op::Fold => "FOLD",
        Op::Range => "RANGE",
        Op::Len => "LEN",
        Op::Head => "HEAD",
        Op::Tail => "TAIL",
        Op::Cons => "CONS",
        Op::Concat => "CONCAT",
        Op::StringConcat => "STR_CONCAT",
        Op::Print => "PRINT",
        Op::Emit => "EMIT",
        Op::Read => "READ",
        Op::Debug => "DEBUG",
        Op::Min => "MIN",
        Op::Max => "MAX",
        Op::Pow => "POW",
        Op::Sqrt => "SQRT",
        Op::Nth => "NTH",
        Op::Append => "APPEND",
        Op::Sort => "SORT",
        Op::Reverse => "REVERSE",
        Op::Chars => "CHARS",
        Op::Join => "JOIN",
        Op::Split => "SPLIT",
        Op::Upper => "UPPER",
        Op::Lower => "LOWER",
        Op::Trim => "TRIM",
        Op::Clear => "CLEAR",
        Op::Depth => "DEPTH",
        Op::Type => "TYPE",
        Op::ToString => "TO_STRING",
        Op::ToInt => "TO_INT",
        Op::Dip => "DIP",
        Op::Keep => "KEEP",
        Op::Bi => "BI",
        Op::Bi2 => "BI2",
        Op::Tri => "TRI",
        Op::Both => "BOTH",
        Op::Compose => "COMPOSE",
        Op::Curry => "CURRY",
        Op::Apply => "APPLY",
        Op::CallWord(_) => "CALL_WORD",
        Op::CallQualified { .. } => "CALL_QUAL",
        Op::Return => "RETURN",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disassemble_with_quotation() {
        let ops = vec![
            Op::Push(Value::Integer(5)),
            Op::Push(Value::CompiledQuotation(vec![
                Op::Push(Value::Integer(1)),
                Op::Add,
            ])),
            Op::Call,
            Op::Return,
        ];

        // Should not panic and should show nested quotation
        let output = disassemble_to_string(&ops);
        assert!(output.contains("PUSH"));
    }

    #[test]
    fn test_jump_direction_arrows() {
        let ops = vec![
            Op::Jump(-3), // backward
            Op::Jump(3),  // forward
        ];

        // Verify it compiles - visual check for arrows
        disassemble_ops(&ops, 0);
    }

    #[test]
    fn test_format_empty_list() {
        let list = Value::List(vec![]);
        assert_eq!(format_value(&list), "{ }");
    }

    #[test]
    fn test_format_list_with_quotation() {
        let list = Value::List(vec![
            Value::Integer(1),
            Value::CompiledQuotation(vec![Op::Add]),
        ]);

        let formatted = format_value(&list);
        assert!(formatted.contains("quotations"));
    }

    #[test]
    fn test_op_counts() {
        let ops = vec![
            Op::Push(Value::Integer(1)),
            Op::Push(Value::Integer(2)),
            Op::Add,
            Op::Push(Value::Integer(3)),
            Op::Mul,
        ];

        let mut counts = HashMap::new();
        count_ops(&ops, &mut counts);

        assert_eq!(counts.get("PUSH"), Some(&3));
        assert_eq!(counts.get("ADD"), Some(&1));
        assert_eq!(counts.get("MUL"), Some(&1));
    }

    #[test]
    fn test_nested_quotation_counting() {
        let ops = vec![Op::Push(Value::CompiledQuotation(vec![
            Op::Push(Value::Integer(1)),
            Op::Push(Value::Integer(2)),
            Op::Add,
        ]))];

        let mut counts = HashMap::new();
        count_ops(&ops, &mut counts);

        // 1 outer PUSH + 2 inner PUSH = 3 total
        assert_eq!(counts.get("PUSH"), Some(&3));
        assert_eq!(counts.get("ADD"), Some(&1));
    }

    #[test]
    fn test_aux_stack_ops_disassemble() {
        let ops = vec![
            Op::Push(Value::Integer(5)),
            Op::ToAux,
            Op::Push(Value::Integer(10)),
            Op::FromAux,
            Op::Add,
        ];

        let output = disassemble_to_string(&ops);
        assert!(output.contains("TO_AUX"));
        assert!(output.contains("FROM_AUX"));
    }
}
