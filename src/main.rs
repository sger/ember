mod bytecode;
mod frontend;
mod lang;
mod runtime;

use std::{env, fs, path::Path};

use crate::bytecode::compile::Compiler;
use crate::runtime::vm_ast::VM;
use crate::runtime::vm_bc::VmBc;

use crate::bytecode::disasm::print_bc;
use crate::frontend::lexer::Lexer;
use crate::frontend::parser::Parser;
use crate::frontend::token_dumper::TokenDumper;

fn main() {
    let args: Vec<String> = env::args().collect();

    let tokens_only = args.contains(&"--tokens".to_string());
    let no_color = args.contains(&"--no-color".to_string());
    let pretty = args.contains(&"--pretty".to_string());
    let ast = args.contains(&"--ast".to_string());
    let ast_full = args.contains(&"--ast-full".to_string());
    let bytecode = args.contains(&"--bc".to_string()) || args.contains(&"--bytecode".to_string());

    // first non-flag argument is the filename
    let filename = args.iter().skip(1).find(|a| !a.starts_with('-'));

    match filename {
        Some(filename) => {
            ensure_extension(filename);
            match fs::read_to_string(filename) {
                Ok(source) => {
                    if tokens_only {
                        dump_tokens(&source, no_color, pretty);
                    } else {
                        run_program(&source, filename, bytecode, ast, ast_full);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read '{}': {}", filename, e);
                    std::process::exit(1);
                }
            }
        }
        None => {
            if args.len() == 1 {
                println!("demo mode");
            } else {
                print_usage();
            }
        }
    }
}

fn ensure_extension(filename: &str) {
    let path = Path::new(filename);
    if path.extension().and_then(|e| e.to_str()) != Some("em") {
        eprintln!("Error: expected a .em file, got {}", filename);
        std::process::exit(1);
    }
}

fn dump_tokens(source: &str, no_color: bool, pretty: bool) {
    let mut lexer = Lexer::new(source);

    match lexer.tokenize() {
        Ok(tokens) => {
            let mut dumper = TokenDumper::new();

            if no_color {
                dumper = dumper.no_color();
            }
            if pretty {
                dumper = dumper.pretty();
            }

            dumper.dump(&tokens);
        }
        Err(e) => {
            eprintln!("Lexer error: {}", e);
            std::process::exit(1);
        }
    }
}

fn print_usage() {
    println!("EMBER - Concatenative Functional Programming Language");
    println!();
    println!("Usage:");
    println!("  ember                     Run demo examples");
    println!("  ember <file.em>           Run a program");
    println!("  ember --repl, -i          Start interactive REPL");
    println!("  ember --tokens <file>     Show tokens only");
    println!("  ember --bc <file.em>      Run using bytecode VM (with stack checker)");
    println!("  ember --help, -h          Show this help");
}

fn run_program(source: &str, filename: &str, bytecode: bool, ast: bool, ast_full: bool) {
    let mut lexer = Lexer::new(source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Lexer error: {}", e);
            std::process::exit(1);
        }
    };

    // Parse
    let mut parser = Parser::new(tokens);
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            std::process::exit(1);
        }
    };

    // AST printing modes (do not depend on engine)
    if ast {
        println!("{:#?}", program);
        return;
    }

    // If you want --ast-full to be supported even in bc mode,
    // it needs to be done via the AST VM helper (or move printing elsewhere).
    if ast_full {
        let mut vm = VM::new();
        vm.set_current_dir(std::path::Path::new(filename));
        vm.print_ast_full(Some(Path::new(&filename)), &program);
        return;
    }

    if bytecode {
        run_program_bc(&program);
    } else {
        run_program_ast(&program, filename);
    }
}

fn run_program_ast(program: &crate::lang::program::Program, filename: &str) {
    let mut vm = VM::new();
    vm.set_current_dir(std::path::Path::new(filename));

    if let Err(e) = vm.load(program) {
        eprintln!("Runtime error: {}", e);
        std::process::exit(1);
    }

    if let Err(e) = vm.run(program) {
        eprintln!("Runtime error: {}", e);
        std::process::exit(1);
    }
}

fn run_program_bc(program: &crate::lang::program::Program) {
    let program_bytecode = match Compiler::new().compile_program(program) {
        Ok(program_bytecode) => program_bytecode,
        Err(e) => {
            eprintln!("Compile error: {}", e);
            std::process::exit(1);
        }
    };

    println!("=== BYTECODE PROGRAM ===");
    println!("{:#?}", program_bytecode);

    println!("=== BYTECODE PROGRAM ===");
    print_bc(&program_bytecode);

    let mut vm = VmBc::new();

    eprintln!("bytecode VM path not wired yet: need resolve+compile step");
    std::process::exit(1);
}
