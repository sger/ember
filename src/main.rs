mod ast;
mod lexer;
mod parser;
mod parser_error;
mod runtime_error;
mod token;
mod token_dumper;
mod vm;

use lexer::Lexer;
use parser::Parser;
use std::{env, fs, path::Path};
use token_dumper::TokenDumper;

use crate::vm::vm::VM;
use crate::vm::vm_bc::VmBc;

fn main() {
    let args: Vec<String> = env::args().collect();

    let tokens_only = args.contains(&"--tokens".to_string());
    let no_color = args.contains(&"--no-color".to_string());
    let pretty = args.contains(&"--pretty".to_string());
    let ast = args.contains(&"--ast".to_string());
    let ast_full = args.contains(&"--ast-full".to_string());

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
                        run_program(&source, filename, ast, ast_full);
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

fn run_program(source: &str, filename: &str, ast: bool, ast_full: bool) {
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

    let mut vm = VM::new();
    vm.set_current_dir(std::path::Path::new(filename));

    // Print full ast
    if ast_full {
        vm.print_ast_full(Some(Path::new(&filename)), &program);
        return;
    }

    // Print ast only
    if ast {
        println!("{:#?}", program);
        return;
    }

    if let Err(e) = vm.load(&program) {
        eprintln!("Runtime error: {}", e);
        std::process::exit(1);
    }

    if let Err(e) = vm.run(&program) {
        eprintln!("Runtime error: {}", e);
        std::process::exit(1);
    }
}
