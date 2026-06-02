mod ast;
mod codegen;
mod js;
mod lexer;
mod lua;
mod parser;
mod validator;

use anyhow::{bail, Context, Result};
use clap::Parser;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Where to stop the pipeline. Maps to the --to-fred / --to-c flags.
#[derive(Clone, Copy, PartialEq)]
enum Stop {
    /// Emit the .fred IR (AST), or the transpiled .fred source for .js/.lua.
    Fred,
    /// Emit C source only.
    C,
    /// Full pipeline: C source compiled to a native executable via gcc.
    Exe,
}

/// Errors from a single file's compilation pipeline. Each stage maps to one
/// variant so the failing phase is unambiguous (thiserror for the library
/// layer; main wraps these with anyhow for top-level reporting).
#[derive(Debug, thiserror::Error)]
enum CompileError {
    #[error("error reading {path}: {source}")]
    Read { path: String, source: io::Error },
    #[error("error writing {path}: {source}")]
    Write { path: String, source: io::Error },
    #[error("{0}")]
    Transpile(String),
    #[error("lexer error: {0}")]
    Lex(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("validation failed:\n{}", .0.iter().map(|e| format!("  ✗ {e}")).collect::<Vec<_>>().join("\n"))]
    Validate(Vec<String>),
    #[error("gcc compilation failed:\n{0}")]
    Gcc(String),
    #[error("could not invoke gcc: {0} (is gcc on your PATH?)")]
    GccSpawn(io::Error),
}

const AFTER_HELP: &str = "\
EXAMPLES:
  fred examples/01_hello_world.fred        Compile a .fred file to ./01_hello_world
  fred examples/16_math.js out             JavaScript -> native binary named ./out
  fred --to-fred mycode.js                 See the .fred a JS file transpiles to
  fred --to-c source.fred                  Emit C source only (inspect codegen)
  fred -o ./bin src/                       Batch-compile every src/*.{fred,js,lua}
  fred                                     Drop into the interactive REPL

PIPELINE:  .fred source -> parser+validator -> AST -> codegen -> C -> gcc -> exe
INPUTS:    .fred (direct), .lua and .js (transpiled to .fred source first)";

/// .fred compiler — Lua+JS -> C static compiler.
#[derive(Parser)]
#[command(name = "fred", version, about, after_help = AFTER_HELP)]
struct Cli {
    /// Input .fred/.js/.lua file, or a directory to batch-compile. Omit for REPL.
    input: Option<String>,

    /// Output executable name (single-file mode only; defaults to the input stem).
    output: Option<String>,

    /// Output directory (creates it; required form for batch-compiling a directory).
    #[arg(short = 'o', long = "output")]
    output_dir: Option<String>,

    /// Stop at the .fred IR (AST), or transpiled .fred source for .js/.lua.
    #[arg(long = "to-fred", conflicts_with = "to_c")]
    to_fred: bool,

    /// Stop at generated C source (do not invoke gcc).
    #[arg(long = "to-c")]
    to_c: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let Some(input) = cli.input.as_deref() else {
        run_repl();
        return Ok(());
    };

    let stop = if cli.to_fred {
        Stop::Fred
    } else if cli.to_c {
        Stop::C
    } else {
        Stop::Exe
    };

    // Directory input → batch compile.
    if fs::metadata(input).map(|m| m.is_dir()).unwrap_or(false) {
        return compile_directory(input, cli.output_dir.as_deref(), stop);
    }

    // Single file. Output base path = [dir/]name (name defaults to input stem).
    let name = cli.output.clone().unwrap_or_else(|| stem(input));
    if let Some(dir) = &cli.output_dir {
        fs::create_dir_all(dir).with_context(|| format!("creating output dir {dir}"))?;
    }
    let out_base = match &cli.output_dir {
        Some(dir) => format!("{dir}/{name}"),
        None => name,
    };

    compile_one(input, &out_base, stop).map_err(|e| {
        // Match the historical exit-with-message behavior, via anyhow.
        anyhow::anyhow!("{e}")
    })?;
    Ok(())
}

fn compile_directory(dir: &str, output_dir: Option<&str>, stop: Stop) -> Result<()> {
    let output_dir = output_dir.unwrap_or(".");
    if output_dir != "." {
        fs::create_dir_all(output_dir)
            .with_context(|| format!("creating output dir {output_dir}"))?;
    }

    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("reading directory {dir}"))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && matches!(
                    p.extension().and_then(|e| e.to_str()),
                    Some("fred") | Some("lua") | Some("js")
                )
        })
        .collect();

    if files.is_empty() {
        bail!("no .fred, .lua, or .js files found in {dir}");
    }
    files.sort();
    println!("Compiling {} files to {}...", files.len(), output_dir);

    let mut ok = 0usize;
    for file in &files {
        let stem = file.file_stem().unwrap().to_string_lossy();
        let out_base = PathBuf::from(output_dir).join(stem.as_ref());
        match compile_one(&file.to_string_lossy(), &out_base.to_string_lossy(), stop) {
            Ok(()) => ok += 1,
            Err(e) => eprintln!("✗ {}: {e}", file.display()),
        }
    }

    println!("\n✓ Compiled {}/{} files", ok, files.len());
    if ok != files.len() {
        bail!("{} file(s) failed", files.len() - ok);
    }
    Ok(())
}

fn stem(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
        .to_string()
}

// Read a source file and, for .js/.lua inputs, transpile it to .fred source.
// .fred files are returned verbatim.
fn to_fred_source(path: &str, raw: String) -> Result<String, CompileError> {
    if path.ends_with(".js") {
        js::transpile(&raw)
            .map_err(|e| CompileError::Transpile(format!("JS transpile error in {path}: {e}")))
    } else if path.ends_with(".lua") {
        lua::transpile(&raw)
            .map_err(|e| CompileError::Transpile(format!("Lua transpile error in {path}: {e}")))
    } else {
        Ok(raw)
    }
}

fn write_file(path: &str, contents: &str) -> Result<(), CompileError> {
    fs::write(path, contents).map_err(|source| CompileError::Write {
        path: path.to_string(),
        source,
    })
}

/// Run one source file through the pipeline, writing output relative to
/// `out_base` (a path stem with no extension). `out_base` itself is the
/// executable path in full-compile mode.
fn compile_one(input: &str, out_base: &str, stop: Stop) -> Result<(), CompileError> {
    let raw = fs::read_to_string(input).map_err(|source| CompileError::Read {
        path: input.to_string(),
        source,
    })?;

    let transpiled = input.ends_with(".js") || input.ends_with(".lua");
    let source = to_fred_source(input, raw)?;

    // --to-fred on a .js/.lua file shows the transpiled .fred source.
    if stop == Stop::Fred && transpiled {
        let path = format!("{out_base}.fred");
        write_file(&path, &source)?;
        println!("✓ Generated: {path}");
        return Ok(());
    }

    let tokens = lexer::tokenize(&source).map_err(CompileError::Lex)?;
    let ast = parser::parse(tokens).map_err(CompileError::Parse)?;

    // --to-fred on a .fred file emits the AST (the .fred IR).
    if stop == Stop::Fred {
        let path = format!("{out_base}.fred");
        write_file(&path, &format!("{ast:#?}"))?;
        println!("✓ Generated: {path}");
        return Ok(());
    }

    let mut validator = validator::Validator::new();
    validator.set_allow_nuke(input.ends_with(".fred"));
    validator.validate(&ast).map_err(CompileError::Validate)?;

    let c_code = codegen::generate_c(&ast);

    if stop == Stop::C {
        let path = format!("{out_base}.c");
        write_file(&path, &c_code)?;
        println!("✓ Generated: {path}");
        return Ok(());
    }

    // Full compilation: stage C in /tmp (keyed by the output's file name so a
    // path like /tmp/foo doesn't become /tmp//tmp/foo.c), then gcc.
    let c_stem = Path::new(out_base)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| out_base.to_string());
    let c_file = format!("/tmp/{c_stem}.c");
    write_file(&c_file, &c_code)?;

    let output = Command::new("gcc")
        .args(["-o", out_base, &c_file, "-lm"])
        .output()
        .map_err(CompileError::GccSpawn)?;

    if !output.status.success() {
        return Err(CompileError::Gcc(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    println!("✓ Compiled: {input} → {out_base}");
    Ok(())
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
    let mut fred_src = String::new(); // accumulated .fred session source
    let mut c_body = String::new(); // accumulated raw-C statements (main body)
    let mut c_hdr = String::new(); // accumulated raw-C #include / top-level lines
    let mut prev_fred = String::new(); // last full stdout (fred mode) for delta printing
    let mut prev_c = String::new(); // last full stdout (raw-C mode)
    let mut buffer = String::new(); // multi-line statement accumulator
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
            .args(["-o", &bin_file, &c_file, "-lm"])
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
