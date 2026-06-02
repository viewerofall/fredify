# Fredify: Lua+JS→C Compiler

**A compiled language built on Lua syntax.** Write `.fred` code (Lua-like syntax) or transpile JavaScript, get native executables. Static typing, zero interpreter overhead, full type safety at compile time.

No runtime. No garbage collector. Just C under the hood.

## Features

### Input Languages
- **JavaScript**: Transpiled to Lua via CASTL, then to C
- **Lua**: Direct parsing and compilation to C
- Unified intermediate representation (`.fred`)

### Type System
- Static typing with inference
- `int64_t` (integers)
- `String` (immutable strings with concatenation)
- `Array` (dynamic arrays with bounds checking)
- `FileHandle` (for I/O operations)

### Array Operations
- `.map(closure)` - Transform elements
- `.filter(closure)` - Conditional filtering
- `.reduce(closure, init)` - Fold/aggregate
- `.slice(start, end)` - Extract subarray
- `.join(sep)` - Convert to string
- `.includes(val)` - Check membership
- `.len()`, `.push(val)`, `.pop()`

### String Methods
- `.length()` - Get length
- `.uppercase()`, `.lowercase()` - Case conversion
- `.substring(start, end)` - Extract substring
- `string.find(str, pattern)` - Search
- `string.split(str, sep)` - Parse into array
- Concatenation with `+` operator

### Standard Libraries

**Math**: `Math.abs()`, `Math.sqrt()`, `Math.pow()`, `Math.max()`, `Math.min()`, `Math.floor()`, `Math.ceil()`, `Math.round()`, `Math.random()`

**OS**: `os.time()`, `os.exit(code)`, `os.getenv(name)`, `os.system(cmd)`

**IO**: `io.open(file, mode)`, `io.close(handle)`, `io.read(handle)`, `io.write(handle, data)`

**Table**: `table.insert(arr, val)`, `table.remove(arr)`, `table.concat(arr, sep)`, `table.sort(arr)`

**Type Conversion**: `to_int()`, `to_float()`, `to_string()`, `to_int_str()`

### Control Flow
- `if/else` statements
- `while` loops
- `for` loops with `from...to` syntax
- `for-in` loops over arrays
- `break` statement (exit loops early)
- `switch/case/default` statements
- Ternary operator: `condition ? true_value : false_value`
- Function definitions with closures
- Lexical scoping with closure capture

### String Features
- **Template strings** with `\`...\`` syntax
- **Interpolation** with `${expression}` in templates
- Automatic type conversion in templates

## Quick Start

### Install
```bash
cd fredify
make install
export PATH=~/.local/bin:$PATH
```

### Compile and run
```bash
fred examples/01_hello_world.fred
./01_hello_world
# Output: Hello, World!
```

### Interactive REPL
```bash
fred
fred> let x = 42
fred> print(x)
42
fred> for i in [1, 2, 3] { print(i) }
1
2
3
fred> clear
fred> exit
```

### Batch compile a project
```bash
fred -o ./bin src/
ls ./bin/
# All executables ready
```

## Usage

### Compile a file
```bash
fred <file.fred> [output_name]
```

### Batch compilation
Compile an entire directory:
```bash
fred -o ./bin src/
# Compiles all .fred/.lua/.js files in src/ → ./bin/
```

### New Language Features
```bash
# Break statement for early loop exit
loop i from 1 to 10 {
    if (i == 5) { break }
    print(i)
}

# Ternary operator for conditional expressions
let status = 200
let msg = (status == 200) ? "OK" : "Error"
print(msg)
```

### Intermediate outputs
Stop compilation at any stage for debugging:
```bash
# Output .fred IR (Abstract Syntax Tree)
fred --to-fred source.lua output.fred

# Output C code only (don't compile with gcc)
fred --to-c source.fred output.c

# Output Lua from JavaScript (CASTL transpile only)
fred --to-lua mycode.js output.lua

# Full compilation to executable (default)
fred source.fred
```

### Interactive REPL
```bash
fred
# Drops into interactive shell
fred> let x = 5
fred> print(x)
5
fred> exit
```

### Examples

See `examples/` directory for 13 working demonstrations:
1. `01_hello_world.fred` - Basic output
2. `02_arrays_and_closures.fred` - Functional programming
3. `03_strings.fred` - String manipulation
4. `04_math_library.fred` - Math operations
5. `05_file_io.fred` - File reading/writing
6. `06_table_operations.fred` - Array operations
7. `07_advanced_features.fred` - Complex features combined
8. `08_for_in_loops.fred` - For-in iteration
9. `09_switch_statements.fred` - Switch/case control flow
10. `10_string_interpolation.fred` - Template strings with `${}`
11. `11_rock_paper_scissors.fred` - Interactive game with break + ternary
12. `12_number_guessing_game.fred` - Multi-round game with random numbers
13. `13_snake_game.fred` - Animated game with AI pathfinding

## Architecture

```
Source File (.fred, .lua, .js)
    ↓
Lexer (tokenization)
    ↓
Parser (builds AST)
    ↓
Validator (type checking, safety)
    ↓
Code Generator (Rust)
    ↓
C Code (readable, debuggable)
    ↓
GCC Compilation
    ↓
Native Executable (x86-64)
```

**For JavaScript inputs**, an additional transpilation step handles the JS→Lua conversion using [CASTL](https://github.com/paulbernier/castl) before the normal pipeline begins.

The entire pipeline (tokenize → parse → validate → codegen → gcc) happens for each REPL interaction, enabling interactive development with full type safety.

## Safety

The compiler validates:
- **Undefined variables** - All variables must be declared before use
- **Unknown methods** - Library methods are checked at compile-time
- **Type consistency** - Operations validated for type correctness
- **Bounds checking** - C code includes runtime bounds checks for arrays

## Limitations

**Core constraints:**
- No dynamic typing (static types inferred at compile time)
- Arrays are homogeneous (only int64_t elements)
- No closures as first-class values (can't store/return them)
- No mutual recursion (functions not forward-declared)

**Intentional design choices:**
- No metatables or general key-value dicts
- No coroutines or async features
- No module system or imports (single-file only)
- No regex or advanced string operations

**See [LIMITATIONS.md](LIMITATIONS.md) for detailed constraints and workarounds.**

## Building

```bash
cd fredc
cargo build --release
```

The compiler binary is at `fredc/target/release/fredc`.

## Generated C Code

The compiler generates readable, optimized C code with:
- Zero runtime interpreter overhead
- Direct native execution
- Minimal memory footprint
- GCC compilation (compatible with clang, LLVM)

## Compilation Speed

The compiler is fast:
- Tokenize + Parse: <10ms for most programs
- Validate: <5ms
- Codegen: <20ms
- GCC: 100-500ms (depending on code size)

**Total**: Typical program compiles in 200-600ms, making the REPL feel snappy.

## Development

Built with:
- **Lexer/Parser**: Rust (hand-written)
- **Codegen**: Rust → C
- **Backend**: GCC (native compilation)
- **JS Transpiler**: [CASTL](https://github.com/paulbernier/castl) by Paul Bernier

### Running Tests

```bash
make test              # Tests examples/01-03
fred examples/*.fred   # Compile any example
```

### Building from Source

```bash
make check-deps        # Verify gcc, node, cargo installed
make install           # Build and install fred command
```

---

## Credits

- **CASTL**: JavaScript→Lua transpilation powered by [CASTL](https://github.com/paulbernier/castl) — an excellent, permissive transpiler. Major thanks to Paul Bernier and contributors.
- **Inspiration**: Built on Lua syntax and semantics, combined with static typing for zero-cost abstraction.

## Documentation

- **[FREDLANG.md](FREDLANG.md)** — Complete language reference with syntax, types, libraries, and examples
- **[LIMITATIONS.md](LIMITATIONS.md)** — Detailed constraints, known issues, and workarounds

---

**Status**: v1 feature-complete. Production-ready for scripts, tools, and small systems.
