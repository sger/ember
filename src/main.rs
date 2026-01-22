mod bytecode;
mod frontend;
mod lang;
mod runtime;

use std::{env, fs, path::Path};

use crate::bytecode::ProgramBc;
use crate::bytecode::compile::Compiler;
use crate::bytecode::disasm::print_bc;
use crate::frontend::lexer::Lexer;
use crate::frontend::parser::Parser;
use crate::frontend::token_dumper::TokenDumper;
use crate::runtime::vm_bc::VmBc;

fn main() {
    let args: Vec<String> = env::args().collect();

    let tokens_only = args.contains(&"--tokens".to_string());
    let no_color = args.contains(&"--no-color".to_string());
    let pretty = args.contains(&"--pretty".to_string());
    let ast = args.contains(&"--ast".to_string());
    let save_bc = args.contains(&"--save-bc".to_string());
    let disasm = args.contains(&"--disasm".to_string());

    let filename = args.iter().skip(1).find(|a| !a.starts_with('-'));

    match filename {
        Some(filename) => {
            let path = Path::new(filename);

            match path.extension().and_then(|e| e.to_str()) {
                Some("em") => {
                    if tokens_only {
                        let source = fs::read_to_string(filename).unwrap_or_else(|e| {
                            eprintln!("Failed to read '{}': {}", filename, e);
                            std::process::exit(1);
                        });
                        dump_tokens(&source, no_color, pretty);
                    } else {
                        run_from_source(path, ast, save_bc, disasm);
                    }
                }
                Some("ebc") => {
                    run_from_bytecode(path, disasm);
                }
                _ => {
                    eprintln!("Error: expected a .em or .ebc file, got {}", filename);
                    std::process::exit(1);
                }
            }
        }
        None => {
            if args.len() == 1 {
                println!("EMBER - Concatenative Functional Programming Language");
                println!("Use --help for usage information");
            } else {
                print_usage();
            }
        }
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
    println!("  ember <file.em>              Compile and run a program");
    println!("  ember <file.ebc>             Run pre-compiled bytecode");
    println!();
    println!("Options:");
    println!("  --save-bc                    Compile and save to .ebc file");
    println!("  --disasm                     Show bytecode disassembly");
    println!("  --ast                        Print AST and exit");
    println!("  --tokens                     Show tokens only");
    println!("  --no-color                   Disable colored output");
    println!("  --pretty                     Pretty-print tokens");
    println!("  --help, -h                   Show this help");
}

fn run_from_source(path: &Path, ast: bool, save_bc: bool, disasm: bool) {
    println!("Compiling {}...", path.display());

    let compiler = Compiler::new();
    let bytecode = match compiler.compile_from_file(path) {
        Ok(bc) => bc,
        Err(e) => {
            eprintln!("Compile error: {}", e);
            std::process::exit(1);
        }
    };

    println!("✓ Compiled {} words", bytecode.words.len());

    if ast {
        println!("\n{:#?}", bytecode);
        return;
    }

    if disasm {
        println!();
        print_bc(&bytecode);
        println!();
    }

    if save_bc {
        let output_path = path.with_extension("ebc");
        match save_bytecode(&bytecode, &output_path) {
            Ok(_) => println!("✓ Saved to {}", output_path.display()),
            Err(e) => {
                eprintln!("Warning: failed to save bytecode: {}", e);
            }
        }
    }

    println!("\nExecuting...\n");
    execute_bytecode(&bytecode);
}

fn run_from_bytecode(path: &Path, disasm: bool) {
    println!("Loading {}...", path.display());

    let bytecode = match load_bytecode(path) {
        Ok(bc) => bc,
        Err(e) => {
            eprintln!("Failed to load bytecode: {}", e);
            std::process::exit(1);
        }
    };

    println!("✓ Loaded {} words", bytecode.words.len());

    if disasm {
        println!();
        print_bc(&bytecode);
        println!();
    }

    println!("\nExecuting...\n");
    execute_bytecode(&bytecode);
}

fn execute_bytecode(bytecode: &ProgramBc) {
    let mut vm = VmBc::new();

    if let Err(e) = vm.run_compiled(bytecode) {
        eprintln!("\nRuntime error: {}", e);
        std::process::exit(1);
    }
}

// ============================================================================
// Bytecode serialization with postcard
// ============================================================================

fn save_bytecode(program: &ProgramBc, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Serialize with postcard
    let bytes =
        postcard::to_allocvec(program).map_err(|e| format!("Serialization failed: {}", e))?;

    // Write to file
    fs::write(path, &bytes)?;

    Ok(())
}

fn load_bytecode(path: &Path) -> Result<ProgramBc, Box<dyn std::error::Error>> {
    // Read file
    let bytes = fs::read(path)?;

    // Deserialize with postcard
    let program: ProgramBc =
        postcard::from_bytes(&bytes).map_err(|e| format!("Deserialization failed: {}", e))?;

    Ok(program)
}
