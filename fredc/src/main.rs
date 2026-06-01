mod lexer;
mod parser;
mod ast;
mod codegen;
mod validator;

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::io::{self, Write, BufRead};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        run_repl();
        return;
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

fn run_repl() {
    println!("🔥 .fred REPL (type 'exit' to quit)");

    let stdin = io::stdin();
    let reader = stdin.lock();
    let mut lines = reader.lines();
    let mut statement_buffer = String::new();
    let mut executed_statements = String::new();
    let mut counter = 0;

    loop {
        print!("fred> ");
        io::stdout().flush().unwrap();

        if let Some(Ok(line)) = lines.next() {
            let trimmed = line.trim();

            // Check for exit command
            if trimmed == "exit" || trimmed == "quit" {
                println!("Goodbye!");
                break;
            }

            // Skip empty lines
            if trimmed.is_empty() {
                continue;
            }

            // Add to buffer
            statement_buffer.push_str(trimmed);
            statement_buffer.push('\n');

            // Check if statement looks complete (simple heuristic)
            if !is_incomplete(&statement_buffer) {
                counter += 1;
                // Combine all executed statements with the new one
                let full_code = format!("{}{}", executed_statements, statement_buffer);
                if execute_statement(&full_code, counter) {
                    // Only keep the statement if execution was successful
                    executed_statements.push_str(&statement_buffer);
                }
                statement_buffer.clear();
            } else {
                // Incomplete statement, show continuation prompt
                print!("     ");
                io::stdout().flush().unwrap();
            }
        } else {
            // EOF
            break;
        }
    }
}

fn is_incomplete(code: &str) -> bool {
    let mut open_braces = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for c in code.chars() {
        if escape_next {
            escape_next = false;
            continue;
        }

        match c {
            '\\' if in_string => escape_next = true,
            '"' | '\'' => in_string = !in_string,
            '{' if !in_string => open_braces += 1,
            '}' if !in_string => open_braces -= 1,
            _ => {}
        }
    }

    open_braces > 0 || code.trim().ends_with(',')
}

fn execute_statement(code: &str, counter: usize) -> bool {
    // Tokenize
    let tokens = match lexer::tokenize(code) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("✗ Lexer error: {}", e);
            return false;
        }
    };

    // Parse
    let ast = match parser::parse(tokens) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("✗ Parse error: {}", e);
            return false;
        }
    };

    // Validate
    let mut validator = validator::Validator::new();
    if let Err(errors) = validator.validate(&ast) {
        eprintln!("✗ Validation errors:");
        for err in errors {
            eprintln!("  {}", err);
        }
        return false;
    }

    // Generate C code
    let c_code = codegen::generate_c(&ast);

    // Write temporary C file
    let temp_name = format!("__repl_{}", counter);
    let c_file = format!("/tmp/{}.c", temp_name);
    if let Err(e) = fs::write(&c_file, &c_code) {
        eprintln!("✗ Error writing C file: {}", e);
        return false;
    }

    // Compile with gcc
    let output = Command::new("gcc")
        .args(&["-o", &temp_name, &c_file, "-lm"])
        .output();

    match output {
        Ok(out) => {
            if !out.status.success() {
                let err = String::from_utf8_lossy(&out.stderr);
                // Only show relevant error lines
                for line in err.lines() {
                    if line.contains("error:") {
                        eprintln!("✗ {}", line.split("error:").nth(1).unwrap_or("").trim());
                    }
                }
                return false;
            }
        }
        Err(e) => {
            eprintln!("✗ Error invoking GCC: {}", e);
            return false;
        }
    }

    // Run the executable
    let run_output = Command::new(format!("./{}", temp_name))
        .output();

    let mut success = true;
    match run_output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if !stdout.is_empty() {
                print!("{}", stdout);
            }
            if !out.status.success() {
                eprintln!("✗ Execution failed");
                success = false;
            }
        }
        Err(e) => {
            eprintln!("✗ Error running executable: {}", e);
            success = false;
        }
    }

    // Cleanup
    let _ = fs::remove_file(format!("./{}", temp_name));
    let _ = fs::remove_file(&c_file);

    success
}
