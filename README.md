# Fredify: Lua+JS→C Compiler

A production-grade compiler that translates Lua and JavaScript into statically-typed C code with zero runtime overhead.

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
- `switch/case/default` statements
- Function definitions with closures
- Lexical scoping with closure capture

### String Features
- **Template strings** with `\`...\`` syntax
- **Interpolation** with `${expression}` in templates
- Automatic type conversion in templates

## Quick Start

```bash
./fredc/target/release/fredc examples/01_hello_world.fred
./01_hello_world
# Output: Hello, World!
```

## Usage

### Compile a file
```bash
fredc <file.fred> [output_name]
```

### Interactive REPL
```bash
fredc
# Drops into interactive shell
fred> let x = 5
fred> print(x)
5
fred> exit
```

### Examples

See `examples/` directory for 10 working demonstrations:
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

## Architecture

```
JavaScript/Lua Source
    ↓ [CASTL transpiler for JS]
Lua AST
    ↓ [Parser builds AST]
Abstract Syntax Tree
    ↓ [Validator checks safety]
Validated AST
    ↓ [Rust codegen]
Clean C code with:
    - Array struct (int64_t*, len, cap)
    - String struct (char*, len)
    - Closure forward declarations
    - Type-safe dispatch
    ↓ [GCC with -lm]
Native executable (x86-64)
```

## Safety

The compiler validates:
- **Undefined variables** - All variables must be declared before use
- **Unknown methods** - Library methods are checked at compile-time
- **Type consistency** - Operations validated for type correctness
- **Bounds checking** - C code includes runtime bounds checks for arrays

## Limitations

- No dynamic typing (static types inferred at compile time)
- No metatables or Lua tables as general key-value stores
- No coroutines or complex Lua features
- Arrays are homogeneous (only int64_t elements)

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

---

**Status**: Feature-complete and production-ready.
