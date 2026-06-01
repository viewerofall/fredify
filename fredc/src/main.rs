mod lexer;
mod parser;
mod ast;
mod codegen;
mod validator;

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: fredc <file.fred> [output]");
        std::process::exit(1);
    }

    let input_file = &args[1];
    let output_name = args.get(2).cloned().unwrap_or_else(|| {
        Path::new(input_file)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    });

    // Read .fred file
    let source = match fs::read_to_string(input_file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading {}: {}", input_file, e);
            std::process::exit(1);
        }
    };

    // Tokenize
    let tokens = match lexer::tokenize(&source) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Lexer error: {}", e);
            std::process::exit(1);
        }
    };

    // Parse
    let ast = match parser::parse(tokens) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            std::process::exit(1);
        }
    };

    // Validate
    let mut validator = validator::Validator::new();
    if let Err(errors) = validator.validate(&ast) {
        eprintln!("Validation errors:");
        for err in errors {
            eprintln!("  ✗ {}", err);
        }
        std::process::exit(1);
    }

    // Generate C code
    let c_code = codegen::generate_c(&ast);

    // Write temporary C file
    let c_file = format!("/tmp/{}.c", output_name);
    if let Err(e) = fs::write(&c_file, &c_code) {
        eprintln!("Error writing C file: {}", e);
        std::process::exit(1);
    }

    println!("Generated: {}", c_file);

    // Compile with gcc
    let output = Command::new("gcc")
        .args(&["-o", &output_name, &c_file, "-lm"])
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                println!("✓ Compiled: {} → {}", input_file, output_name);
            } else {
                eprintln!("GCC compilation failed:");
                eprintln!("{}", String::from_utf8_lossy(&out.stderr));
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Error invoking GCC: {}", e);
            eprintln!("Make sure `gcc` is in your PATH");
            std::process::exit(1);
        }
    }
}
