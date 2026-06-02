mod lexer;
mod parser;
mod ast;
mod codegen;
mod validator;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::io::{self, Write, BufRead};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        run_repl();
        return;
    }

    // Check for help flags
    if args.len() == 2 && (args[1] == "--help" || args[1] == "-h" || args[1] == "help") {
        print_help();
        return;
    }

    let mut input_path = "";
    let mut output_dir = None;
    let mut output_name = None;
    let mut stop_at = "exe"; // "lua", "fred", "c", or "exe"

    // Parse args
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--to-lua" => stop_at = "lua",
            "--to-fred" => stop_at = "fred",
            "--to-c" => stop_at = "c",
            "-o" | "--output" => {
                if i + 1 < args.len() {
                    output_dir = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: -o requires a directory\n");
                    print_help();
                    std::process::exit(1);
                }
                continue;
            }
            _ if !args[i].starts_with('-') => {
                input_path = &args[i];
                if output_dir.is_none() && i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    output_name = Some(args[i + 1].clone());
                }
                break;
            }
            _ => {}
        }
        i += 1;
    }

    if input_path.is_empty() {
        eprintln!("Error: No input file or directory specified\n");
        print_help();
        std::process::exit(1);
    }

    // Check if input is a directory
    if let Ok(metadata) = fs::metadata(input_path) {
        if metadata.is_dir() {
            compile_directory(input_path, output_dir.as_deref(), stop_at);
            return;
        }
    }

    // Single file compilation
    let output_name = output_name.unwrap_or_else(|| {
        Path::new(input_path)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    });

    // For .lua files with --to-lua, just output as-is
    if stop_at == "lua" && input_path.ends_with(".lua") {
        let content = match fs::read_to_string(input_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error reading {}: {}", input_path, e);
                std::process::exit(1);
            }
        };
        let lua_file = format!("{}.lua", output_name);
        if let Err(e) = fs::write(&lua_file, &content) {
            eprintln!("Error writing {}: {}", lua_file, e);
            std::process::exit(1);
        }
        println!("✓ Generated: {}", lua_file);
        return;
    }

    // Read file
    let source = match fs::read_to_string(input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading {}: {}", input_path, e);
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

    // Early exit: output AST as .fred
    if stop_at == "fred" {
        let fred_file = format!("{}.fred", output_name);
        if let Err(e) = fs::write(&fred_file, format!("{:#?}", ast)) {
            eprintln!("Error writing .fred file: {}", e);
            std::process::exit(1);
        }
        println!("✓ Generated: {}", fred_file);
        return;
    }

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

    // Early exit: output C code
    if stop_at == "c" {
        let c_file = format!("{}.c", output_name);
        if let Err(e) = fs::write(&c_file, &c_code) {
            eprintln!("Error writing C file: {}", e);
            std::process::exit(1);
        }
        println!("✓ Generated: {}", c_file);
        return;
    }

    // Full compilation: C file in /tmp, then gcc
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
                println!("✓ Compiled: {} → {}", input_path, output_name);
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

fn compile_directory(dir: &str, output_dir: Option<&str>, stop_at: &str) {
    let output_dir = output_dir.unwrap_or(".");

    // Create output directory if specified
    if output_dir != "." {
        if let Err(e) = fs::create_dir_all(output_dir) {
            eprintln!("Error creating output directory {}: {}", output_dir, e);
            std::process::exit(1);
        }
    }

    // Find all .fred, .lua, .js files
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy();
                    if ext_str == "fred" || ext_str == "lua" || ext_str == "js" {
                        files.push(path);
                    }
                }
            }
        }
    }

    if files.is_empty() {
        eprintln!("No .fred, .lua, or .js files found in {}", dir);
        return;
    }

    files.sort();
    println!("Compiling {} files to {}...", files.len(), output_dir);

    let mut success_count = 0;
    let mut fail_count = 0;

    for file_path in files {
        let file_str = file_path.to_string_lossy();
        let file_name = file_path.file_stem().unwrap().to_string_lossy();
        let output = PathBuf::from(output_dir).join(file_name.as_ref());
        let output_str = output.to_string_lossy();

        // Compile each file
        if compile_single_file(&file_str, &output_str, stop_at) {
            success_count += 1;
        } else {
            fail_count += 1;
        }
    }

    println!("\n✓ Compiled {}/{} files", success_count, success_count + fail_count);
    if fail_count > 0 {
        std::process::exit(1);
    }
}

fn compile_single_file(input_file: &str, output_name: &str, stop_at: &str) -> bool {
    // Read file
    let source = match fs::read_to_string(input_file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("✗ Error reading {}: {}", input_file, e);
            return false;
        }
    };

    // Early exit: for .lua files with --to-lua, just output as-is
    if stop_at == "lua" && input_file.ends_with(".lua") {
        let lua_file = format!("{}.lua", output_name);
        if let Err(e) = fs::write(&lua_file, &source) {
            eprintln!("✗ Error writing {}: {}", lua_file, e);
            return false;
        }
        println!("  ✓ {}", lua_file);
        return true;
    }

    // Tokenize
    let tokens = match lexer::tokenize(&source) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("✗ Lexer error in {}: {}", input_file, e);
            return false;
        }
    };

    // Parse
    let ast = match parser::parse(tokens) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("✗ Parse error in {}: {}", input_file, e);
            return false;
        }
    };

    // Early exit: output AST as .fred
    if stop_at == "fred" {
        let fred_file = format!("{}.fred", output_name);
        if let Err(e) = fs::write(&fred_file, format!("{:#?}", ast)) {
            eprintln!("✗ Error writing .fred file: {}", e);
            return false;
        }
        println!("  ✓ {}", fred_file);
        return true;
    }

    // Validate
    let mut validator = validator::Validator::new();
    if let Err(errors) = validator.validate(&ast) {
        eprintln!("✗ Validation errors in {}:", input_file);
        for err in errors {
            eprintln!("    {}", err);
        }
        return false;
    }

    // Generate C code
    let c_code = codegen::generate_c(&ast);

    // Early exit: output C code
    if stop_at == "c" {
        let c_file = format!("{}.c", output_name);
        if let Err(e) = fs::write(&c_file, &c_code) {
            eprintln!("✗ Error writing C file: {}", e);
            return false;
        }
        println!("  ✓ {}", c_file);
        return true;
    }

    // Full compilation: C file in /tmp, then gcc
    let c_file = format!("/tmp/{}.c", Path::new(output_name).file_name().unwrap().to_string_lossy());
    if let Err(e) = fs::write(&c_file, &c_code) {
        eprintln!("✗ Error writing C file: {}", e);
        return false;
    }

    // Compile with gcc
    let output = Command::new("gcc")
        .args(&["-o", output_name, &c_file, "-lm"])
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                println!("  ✓ {}", output_name);
                true
            } else {
                eprintln!("✗ GCC failed for {}:", input_file);
                let stderr = String::from_utf8_lossy(&out.stderr);
                for line in stderr.lines().filter(|l| l.contains("error:")) {
                    eprintln!("    {}", line);
                }
                false
            }
        }
        Err(e) => {
            eprintln!("✗ Error invoking GCC: {}", e);
            false
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

            // Check for clear command
            if trimmed == "clear" {
                print!("\x1b[2J\x1b[H");
                io::stdout().flush().unwrap();
                continue;
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

fn print_help() {
    println!("🔥 .fred Compiler — Lua+JS→C static compiler\n");
    println!("USAGE:");
    println!("  fred                              Drop into interactive REPL");
    println!("  fred <file.fred|file.lua|file.js> [output]  Compile to executable");
    println!("  fred -o <dir> <input_dir>        Batch compile directory");
    println!("  fred --to-lua <file.js>           Output Lua only (JS transpile)");
    println!("  fred --to-fred <file>             Output .fred IR (AST)");
    println!("  fred --to-c <file>                Output C code only\n");

    println!("EXAMPLES:");
    println!("  fred examples/01_hello_world.fred");
    println!("  fred --to-lua source.js output.lua");
    println!("  fred --to-c source.fred output.c");
    println!("  fred --to-fred mycode.lua");
    println!("  fred mycode.js myapp");
    println!("  fred -o ./bin src/                Compile all src/ → ./bin/");
    println!("  fred                              # REPL mode\n");

    println!("COMPILATION PIPELINE:");
    println!("  .fred source");
    println!("      ↓ parser + validator");
    println!("  Abstract Syntax Tree");
    println!("      ↓ codegen");
    println!("  C code");
    println!("      ↓ gcc");
    println!("  executable\n");

    println!("FLAGS:");
    println!("  -o, --output <dir>  Output directory for batch compilation");
    println!("  --to-lua            Stop after JS→Lua transpile (CASTL)");
    println!("  --to-fred           Stop at AST (for debugging/inspection)");
    println!("  --to-c              Stop at C code (inspect generated code)");
    println!("  --help, -h          Show this help\n");

    println!("SUPPORTED INPUTS:");
    println!("  .fred files   Direct compilation (recommended)");
    println!("  .lua files    Parsed as .fred syntax");
    println!("  .js files     Transpiled to .lua via CASTL, then compiled\n");

    println!("FEATURES:");
    println!("  • Static typing with inference");
    println!("  • for-in loops, switch/case statements");
    println!("  • String interpolation with backticks and ${{expr}}");
    println!("  • Arrays, closures, type-safe operations");
    println!("  • Standard libraries: Math, OS, IO, Table, type conversion\n");

    println!("REPL COMMANDS (in interactive mode):");
    println!("  let x = 42                Statement execution");
    println!("  print(x)                  Variable inspection");
    println!("  clear                     Clear screen");
    println!("  exit, quit                Exit REPL\n");

    println!("Learn more: check FREDLANG.md in the project root");
}

