mod lexer;
mod parser;
mod ast;
mod codegen;
mod validator;
mod js;
mod lua;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::io::{self, Write};

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
                if input_path.is_empty() {
                    input_path = &args[i];
                    // Only set output_name from next arg if no -o was given
                    if output_dir.is_none() && i + 1 < args.len() && !args[i + 1].starts_with('-') {
                        output_name = Some(args[i + 1].clone());
                        i += 2;
                        continue;
                    }
                }
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

    // Read file
    let raw = match fs::read_to_string(input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading {}: {}", input_path, e);
            std::process::exit(1);
        }
    };

    // JavaScript and Lua inputs are transpiled to .fred source first (no node).
    let transpiled = input_path.ends_with(".js") || input_path.ends_with(".lua");
    let source = match to_fred_source(input_path, raw) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    // --to-fred on a .js/.lua file shows the transpiled .fred source.
    if stop_at == "fred" && transpiled {
        let fred_file = format!("{}.fred", output_name);
        if let Err(e) = fs::write(&fred_file, &source) {
            eprintln!("Error writing .fred file: {}", e);
            std::process::exit(1);
        }
        println!("✓ Generated: {}", fred_file);
        return;
    }

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
    validator.set_allow_nuke(input_path.ends_with(".fred"));
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

    // Full compilation: C file in /tmp, then gcc. Use only the file name for
    // the temp path so an output like `/tmp/foo` doesn't become `/tmp//tmp/foo.c`.
    let c_stem = Path::new(&output_name)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| output_name.clone());
    let c_file = format!("/tmp/{}.c", c_stem);
    if let Err(e) = fs::write(&c_file, &c_code) {
        eprintln!("Error writing C file: {}", e);
        std::process::exit(1);
    }

    println!("Generated: {}", c_file);

    // Determine output executable path
    let exe_path = match &output_dir {
        Some(dir) => format!("{}/{}", dir, output_name),
        None => output_name.clone(),
    };

    // Create output directory if needed
    if let Some(dir) = &output_dir {
        if let Err(e) = fs::create_dir_all(dir) {
            eprintln!("Error creating output directory {}: {}", dir, e);
            std::process::exit(1);
        }
    }

    // Compile with gcc
    let output = Command::new("gcc")
        .args(&["-o", &exe_path, &c_file, "-lm"])
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                println!("✓ Compiled: {} → {}", input_path, exe_path);
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

// Read a source file and, for .js/.lua inputs, transpile it to .fred source.
// .fred files are returned verbatim.
fn to_fred_source(path: &str, raw: String) -> Result<String, String> {
    if path.ends_with(".js") {
        js::transpile(&raw).map_err(|e| format!("JS transpile error in {}: {}", path, e))
    } else if path.ends_with(".lua") {
        lua::transpile(&raw).map_err(|e| format!("Lua transpile error in {}: {}", path, e))
    } else {
        Ok(raw)
    }
}

fn compile_single_file(input_file: &str, output_name: &str, stop_at: &str) -> bool {
    // Read file
    let raw = match fs::read_to_string(input_file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("✗ Error reading {}: {}", input_file, e);
            return false;
        }
    };

    // .js/.lua are transpiled to .fred source first.
    let transpiled = input_file.ends_with(".js") || input_file.ends_with(".lua");
    let source = match to_fred_source(input_file, raw) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("✗ {}", e);
            return false;
        }
    };

    // --to-fred on a .js/.lua file shows the transpiled .fred source.
    if stop_at == "fred" && transpiled {
        let fred_file = format!("{}.fred", output_name);
        if let Err(e) = fs::write(&fred_file, &source) {
            eprintln!("✗ Error writing .fred file: {}", e);
            return false;
        }
        println!("  ✓ {}", fred_file);
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
    validator.set_allow_nuke(input_file.ends_with(".fred"));
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

#[derive(PartialEq)]
enum ReplMode {
    Fred,
    RawC,
}

fn run_repl() {
    use rustyline::error::ReadlineError;
    use rustyline::DefaultEditor;

    println!("🔥 .fred REPL — type ':help' for commands, 'exit' to quit");

    let mut rl = match DefaultEditor::new() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("✗ Could not start line editor: {}", e);
            return;
        }
    };

    let mut mode = ReplMode::Fred;
    let mut fred_src = String::new();   // accumulated .fred session source
    let mut c_body = String::new();     // accumulated raw-C statements (main body)
    let mut c_hdr = String::new();      // accumulated raw-C #include / top-level lines
    let mut prev_fred = String::new();  // last full stdout (fred mode) for delta printing
    let mut prev_c = String::new();     // last full stdout (raw-C mode)
    let mut buffer = String::new();     // multi-line statement accumulator
    let mut counter = 0;

    loop {
        let prompt = if !buffer.is_empty() {
            "....  "
        } else if mode == ReplMode::RawC {
            "c> "
        } else {
            "fred> "
        };

        match rl.readline(prompt) {
            Ok(line) => {
                let _ = rl.add_history_entry(line.as_str());
                let trimmed = line.trim();

                // Commands are only recognized between statements (empty buffer).
                if buffer.is_empty() {
                    match trimmed {
                        "exit" | "quit" => {
                            println!("Goodbye!");
                            break;
                        }
                        ":clear" => {
                            let _ = rl.clear_screen();
                            continue;
                        }
                        ":reset" => {
                            fred_src.clear();
                            c_body.clear();
                            c_hdr.clear();
                            prev_fred.clear();
                            prev_c.clear();
                            buffer.clear();
                            mode = ReplMode::Fred;
                            println!("→ session reset (.fred mode, no state).");
                            continue;
                        }
                        ":help" => {
                            print_repl_help();
                            continue;
                        }
                        ":c" => {
                            // Show the C that the current .fred session transpiles to.
                            if fred_src.trim().is_empty() {
                                println!("(no .fred statements yet)");
                            } else if let Ok(c) = build_fred_c(&fred_src) {
                                print!("{}", c);
                            }
                            continue;
                        }
                        ":ast" => {
                            // Show the .fred IR (AST) for the current session.
                            if fred_src.trim().is_empty() {
                                println!("(no .fred statements yet)");
                            } else if let Ok(tokens) = lexer::tokenize(&fred_src) {
                                match parser::parse(tokens) {
                                    Ok(ast) => println!("{:#?}", ast),
                                    Err(e) => eprintln!("✗ Parse error: {}", e),
                                }
                            }
                            continue;
                        }
                        ":cmode" => {
                            mode = ReplMode::RawC;
                            println!("→ raw C mode. Lines compile as C; '#'-lines become includes. ':fred' to go back.");
                            continue;
                        }
                        ":fred" => {
                            mode = ReplMode::Fred;
                            println!("→ .fred mode.");
                            continue;
                        }
                        "" => continue,
                        _ => {}
                    }
                }

                buffer.push_str(&line);
                buffer.push('\n');

                if is_incomplete(&buffer) {
                    continue; // keep accumulating; prompt switches to "...."
                }

                counter += 1;
                match mode {
                    ReplMode::Fred => {
                        let full = format!("{}{}", fred_src, buffer);
                        if let Ok(c) = build_fred_c(&full) {
                            if let Some(out) = compile_and_run(&c, counter) {
                                print_delta(&prev_fred, &out);
                                prev_fred = out;
                                fred_src = full;
                            }
                        }
                    }
                    ReplMode::RawC => {
                        let (mut new_hdr, mut new_body) = (String::new(), String::new());
                        for l in buffer.lines() {
                            if l.trim_start().starts_with('#') {
                                new_hdr.push_str(l);
                                new_hdr.push('\n');
                            } else {
                                new_body.push_str(l);
                                new_body.push('\n');
                            }
                        }
                        let full_hdr = format!("{}{}", c_hdr, new_hdr);
                        let full_body = format!("{}{}", c_body, new_body);
                        let c = build_raw_c(&full_hdr, &full_body);
                        if let Some(out) = compile_and_run(&c, counter) {
                            print_delta(&prev_c, &out);
                            prev_c = out;
                            c_hdr = full_hdr;
                            c_body = full_body;
                        }
                    }
                }
                buffer.clear();
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl+C: abandon the in-progress statement, stay in the REPL.
                buffer.clear();
                continue;
            }
            Err(ReadlineError::Eof) => break, // Ctrl+D
            Err(e) => {
                eprintln!("✗ Input error: {}", e);
                break;
            }
        }
    }
}

fn print_repl_help() {
    println!("REPL commands:");
    println!("  :help     show this help");
    println!("  :c        show the C your current .fred session transpiles to");
    println!("  :ast      show the .fred IR (AST) for the current session");
    println!("  :cmode    switch to raw C mode (type C directly)");
    println!("  :fred     switch back to .fred mode");
    println!("  :reset    purge all session state, back to fresh .fred mode");
    println!("  :clear    clear the screen (or just press Ctrl+L)");
    println!("  exit      quit (or Ctrl+D)");
}

// Print only the part of `full` not already shown in `prev`. The session is
// re-run from scratch each line, so earlier output is a prefix of the new
// output — we slice it off so old prints don't repeat.
fn print_delta(prev: &str, full: &str) {
    if let Some(rest) = full.strip_prefix(prev) {
        print!("{}", rest);
    } else {
        // Replay diverged (e.g. randomness/time) — show the whole thing.
        print!("{}", full);
    }
    io::stdout().flush().unwrap();
}

// Run the full .fred session source through the pipeline, returning the C
// source. Prints stage errors and returns Err so the caller drops the line.
fn build_fred_c(src: &str) -> Result<String, ()> {
    let tokens = match lexer::tokenize(src) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("✗ Lexer error: {}", e);
            return Err(());
        }
    };
    let ast = match parser::parse(tokens) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("✗ Parse error: {}", e);
            return Err(());
        }
    };
    let mut validator = validator::Validator::new();
    validator.set_allow_nuke(true); // REPL is .fred
    if let Err(errors) = validator.validate(&ast) {
        eprintln!("✗ Validation errors:");
        for err in errors {
            eprintln!("  {}", err);
        }
        return Err(());
    }
    Ok(codegen::generate_c(&ast))
}

// Wrap accumulated raw-C lines into a compilable program.
fn build_raw_c(hdr: &str, body: &str) -> String {
    format!(
        "#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n#include <math.h>\n{}\nint main(void) {{\n{}\nreturn 0;\n}}\n",
        hdr, body
    )
}

// Compile a C string with gcc and run it. Returns the program's stdout on a
// successful compile+run (None if compilation or execution failed). Errors and
// the program's stderr are surfaced directly. Temp files live in /tmp and are
// cleaned up afterward.
fn compile_and_run(c_code: &str, counter: usize) -> Option<String> {
    let temp_name = format!("__repl_{}", counter);
    let c_file = format!("/tmp/{}.c", temp_name);
    let bin_file = format!("/tmp/{}", temp_name);

    if let Err(e) = fs::write(&c_file, c_code) {
        eprintln!("✗ Error writing C file: {}", e);
        return None;
    }

    let result = (|| {
        let output = Command::new("gcc")
            .args(&["-o", &bin_file, &c_file, "-lm"])
            .output();
        match output {
            Ok(out) => {
                if !out.status.success() {
                    let err = String::from_utf8_lossy(&out.stderr);
                    for line in err.lines() {
                        if line.contains("error:") {
                            eprintln!("✗ {}", line.split("error:").nth(1).unwrap_or("").trim());
                        }
                    }
                    return None;
                }
            }
            Err(e) => {
                eprintln!("✗ Error invoking GCC: {}", e);
                return None;
            }
        }

        match Command::new(&bin_file).output() {
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                if !stderr.is_empty() {
                    eprint!("{}", stderr);
                }
                if !out.status.success() {
                    use std::os::unix::process::ExitStatusExt;
                    if out.status.signal() == Some(6) {
                        eprintln!("💥 nuke() detonated — the program crashed (the REPL survives).");
                    } else {
                        eprintln!("✗ Execution failed");
                    }
                    return None;
                }
                Some(String::from_utf8_lossy(&out.stdout).into_owned())
            }
            Err(e) => {
                eprintln!("✗ Error running executable: {}", e);
                None
            }
        }
    })();

    let _ = fs::remove_file(&bin_file);
    let _ = fs::remove_file(&c_file);
    result
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

fn print_help() {
    println!("🔥 .fred Compiler — Lua+JS→C static compiler\n");
    println!("USAGE:");
    println!("  fred                              Drop into interactive REPL");
    println!("  fred <file.fred|file.lua|file.js> [output]  Compile to executable");
    println!("  fred -o <dir> <input_dir>        Batch compile directory");
    println!("  fred --to-fred <file.js>          Show the .fred a JS file transpiles to");
    println!("  fred --to-fred <file>             Output .fred IR (AST) for .fred/.lua");
    println!("  fred --to-c <file>                Output C code only\n");

    println!("EXAMPLES:");
    println!("  fred examples/01_hello_world.fred");
    println!("  fred examples/16_math.js out      # JavaScript → native binary");
    println!("  fred --to-fred mycode.js out      # see the transpiled .fred");
    println!("  fred --to-c source.fred output.c");
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

