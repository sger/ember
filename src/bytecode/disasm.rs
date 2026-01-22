use crate::bytecode::{Op, ProgramBc};
use crate::lang::value::Value;

/// Print disassembly of a bytecode program
pub fn print_bc(bc: &ProgramBc) {
    for (ci, code) in bc.code.iter().enumerate() {
        println!("════════════════════════════════════════");
        println!(" Code Object [{}]", ci);
        println!(" {} instructions", code.ops.len());
        println!("════════════════════════════════════════");
        disassemble_ops(&code.ops);
        println!();
    }
}

/// Disassemble a slice of ops (useful for debugging quotations too)
pub fn disassemble_ops(ops: &[Op]) {
    // First pass: collect jump targets for annotation
    let jump_targets = collect_jump_targets(ops);

    for (ip, op) in ops.iter().enumerate() {
        // Print jump target marker if this is a destination
        if jump_targets.contains(&ip) {
            println!("      ┌──────────────────────────────────");
        }

        // Print IP
        print!("{:04} ", ip);

        // Print arrow if this is a jump target
        if jump_targets.contains(&ip) {
            print!("► ");
        } else {
            print!("  ");
        }

        // Print the instruction
        print_op(op, ip);
    }
}

/// Collect all jump target addresses
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

fn print_op(op: &Op, ip: usize) {
    match op {
        // Literals
        Op::Push(v) => println!("PUSH        {}", format_value(v)),

        // Stack operations
        Op::Dup => println!("DUP"),
        Op::Drop => println!("DROP"),
        Op::Swap => println!("SWAP"),
        Op::Over => println!("OVER"),
        Op::Rot => println!("ROT"),

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
        Op::If => println!("IF"),
        Op::When => println!("WHEN"),
        Op::Call => println!("CALL"),

        // Control flow - jumps
        Op::Jump(offset) => {
            let target = (ip as i32 + *offset) as usize;
            println!("JUMP        {:+} (→ {:04})", offset, target);
        }
        Op::JumpIfFalse(offset) => {
            let target = (ip as i32 + *offset) as usize;
            println!("JUMP_FALSE  {:+} (→ {:04})", offset, target);
        }
        Op::JumpIfTrue(offset) => {
            let target = (ip as i32 + *offset) as usize;
            println!("JUMP_TRUE   {:+} (→ {:04})", offset, target);
        }

        // Loops & higher-order
        Op::Times => println!("TIMES"),
        Op::Each => println!("EACH"),
        Op::Map => println!("MAP"),
        Op::Filter => println!("FILTER"),
        Op::Fold => println!("FOLD"),
        Op::Range => println!("RANGE"),

        // List operations
        Op::Len => println!("LEN"),
        Op::Head => println!("HEAD"),
        Op::Tail => println!("TAIL"),
        Op::Cons => println!("CONS"),
        Op::Concat => println!("CONCAT"),
        Op::StringConcat => println!("STR_CONCAT"),

        // I/O
        Op::Print => println!("PRINT"),
        Op::Emit => println!("EMIT"),
        Op::Read => println!("READ"),
        Op::Debug => println!("DEBUG"),

        // Stdlib
        Op::Min => println!("MIN"),
        Op::Max => println!("MAX"),
        Op::Pow => println!("POW"),
        Op::Sqrt => println!("SQRT"),
        Op::Nth => println!("NTH"),
        Op::Append => println!("APPEND"),
        Op::Sort => println!("SORT"),
        Op::Reverse => println!("REVERSE"),
        Op::Chars => println!("CHARS"),
        Op::Join => println!("JOIN"),
        Op::Split => println!("SPLIT"),
        Op::Upper => println!("UPPER"),
        Op::Lower => println!("LOWER"),
        Op::Trim => println!("TRIM"),
        Op::Clear => println!("CLEAR"),
        Op::Depth => println!("DEPTH"),
        Op::Type => println!("TYPE"),
        Op::ToString => println!("TO_STRING"),
        Op::ToInt => println!("TO_INT"),

        // Combinators
        Op::Dip => println!("DIP"),
        Op::Keep => println!("KEEP"),
        Op::Bi => println!("BI"),
        Op::Bi2 => println!("BI2"),
        Op::Tri => println!("TRI"),
        Op::Both => println!("BOTH"),
        Op::Compose => println!("COMPOSE"),
        Op::Curry => println!("CURRY"),
        Op::Apply => println!("APPLY"),

        // Word calls
        Op::CallWord(name) => println!("CALL_WORD   {:?}", name),
        Op::CallQualified { module, word } => {
            println!("CALL_QUAL   {:?}.{:?}", module, word)
        }

        // Return
        Op::Return => println!("RETURN"),
    }
}

/// Format a value for display
fn format_value(value: &Value) -> String {
    match value {
        Value::Integer(n) => format!("{}", n),
        Value::Float(f) => format!("{:?}", f),
        Value::String(s) => format!("{:?}", s),
        Value::Bool(b) => format!("{}", b),
        Value::List(items) => {
            let inner: Vec<String> = items.iter().map(|v| format_value(v)).collect();
            format!("{{ {} }}", inner.join(" "))
        }
        Value::Quotation(nodes) => {
            format!("[ <{} nodes> ]", nodes.len())
        }
        Value::CompiledQuotation(ops) => {
            format!("[ <{} ops> ]", ops.len())
        }
    }
}

/// Disassemble a compiled quotation (for debugging nested code)
pub fn disassemble_quotation(ops: &[Op], indent: usize) {
    let prefix = "  ".repeat(indent);
    let jump_targets = collect_jump_targets(ops);

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

        // For nested quotations, recursively disassemble
        if let Op::Push(Value::CompiledQuotation(inner_ops)) = op {
            println!("PUSH        [ <{} ops>:", inner_ops.len());
            disassemble_quotation(inner_ops, indent + 1);
            println!("{}          ]", prefix);
        } else {
            print_op(op, ip);
        }
    }
}

/// Full disassembly including nested quotations
pub fn print_bc_full(bc: &ProgramBc) {
    for (ci, code) in bc.code.iter().enumerate() {
        println!("════════════════════════════════════════");
        println!(" Code Object [{}]", ci);
        println!(" {} instructions", code.ops.len());
        println!("════════════════════════════════════════");
        disassemble_quotation(&code.ops, 0);
        println!();
    }
}

/// Return disassembly as a String (useful for testing/logging)
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

        output.push_str(&format_op(op, ip));
        output.push('\n');
    }

    output
}

/// Format a single op as a String
fn format_op(op: &Op, ip: usize) -> String {
    match op {
        Op::Push(v) => format!("PUSH        {}", format_value(v)),
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
        Op::CallWord(name) => format!("CALL_WORD   {:?}", name),
        Op::CallQualified { module, word } => format!("CALL_QUAL   {:?}.{:?}", module, word),
        Op::Return => "RETURN".to_string(),
        // Use debug format for simple ops
        _ => format!("{:?}", op).to_uppercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disassemble_simple() {
        let ops = vec![
            Op::Push(Value::Integer(5)),
            Op::Push(Value::Integer(3)),
            Op::Add,
            Op::Return,
        ];

        let output = disassemble_to_string(&ops);
        assert!(output.contains("PUSH"));
        assert!(output.contains("ADD"));
        assert!(output.contains("RETURN"));
    }

    #[test]
    fn test_disassemble_jumps_show_targets() {
        let ops = vec![
            Op::Push(Value::Bool(true)),
            Op::JumpIfFalse(3),
            Op::Push(Value::Integer(10)),
            Op::Jump(2),
            Op::Push(Value::Integer(20)),
            Op::Return,
        ];

        let output = disassemble_to_string(&ops);
        assert!(output.contains("→ 0004")); // JumpIfFalse target
        assert!(output.contains("→ 0006")); // Jump target
    }

    #[test]
    fn test_jump_targets_marked() {
        let ops = vec![
            Op::Push(Value::Integer(1)),
            Op::Jump(2),
            Op::Push(Value::Integer(2)),
            Op::Push(Value::Integer(3)), // jump target
            Op::Return,
        ];

        let output = disassemble_to_string(&ops);
        assert!(output.contains("►")); // target marker
    }

    #[test]
    fn test_format_value_list() {
        let list = Value::List(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);

        let formatted = format_value(&list);
        assert_eq!(formatted, "{ 1 2 3 }");
    }

    #[test]
    fn test_format_value_string() {
        let s = Value::String("hello".to_string());
        let formatted = format_value(&s);
        assert_eq!(formatted, "\"hello\"");
    }

    #[test]
    fn test_format_compiled_quotation() {
        let quot = Value::CompiledQuotation(vec![Op::Push(Value::Integer(1)), Op::Add]);

        let formatted = format_value(&quot);
        assert_eq!(formatted, "[ <2 ops> ]");
    }
}
