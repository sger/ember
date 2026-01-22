use crate::bytecode::{Op, ProgramBc};

pub fn print_bc(bc: &ProgramBc) {
    for (ci, code) in bc.code.iter().enumerate() {
        println!("== code[{}] ==", ci);

        for (ip, op) in code.ops.iter().enumerate() {
            print!("{:04}   ", ip);
            print_op(op);
        }

        println!();
    }
}

fn print_op(op: &Op) {
    match op {
        Op::Push(v) => println!("Push {:?}", v),
        Op::Return => println!("Return"),
        _ => println!("{:?}", op),
    }
}
