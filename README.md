# Fredify: Lua+JS→C Compiler

**A compiled language built on Lua syntax.** Write `.fred` code (Lua-like syntax) or transpile JavaScript, get native executables. Static typing, zero interpreter overhead, full type safety at compile time.

No runtime. No garbage collector. Just C under the hood.

## Features

### Input Languages

All three are compiled to native executables with **no external tools** — no
Node, no Lua interpreter. Each non-native frontend is a hand-written Rust
transpiler that emits `.fred`, which then goes through the normal pipeline.

- **`.fred`**: The native language (Lua-like brace syntax). Full feature set.
- **JavaScript** (`.js`): Real modern-JS subset → `.fred` (`fredc/src/js.rs`).
  See [JavaScript support](#javascript-support).
- **Lua** (`.lua`): Real Lua subset (`function/end`, `then`, `local`, `..`,
  `~=`, `#`, `ipairs`, numeric `for`) → `.fred` (`fredc/src/lua.rs`).
  See [Lua support](#lua-support).
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
- `.trim()` - Strip leading/trailing whitespace
- `.char_at(i)` - One-char String at index
- `.replace(from, to)` - Replace all occurrences
- `string.find(str, pattern)` - Search
- `string.split(str, sep)` - Parse into array
- Concatenation with `+` operator

### Standard Libraries

**Math**: `Math.abs()`, `Math.sqrt()`, `Math.pow()`, `Math.max()`, `Math.min()`, `Math.floor()`, `Math.ceil()`, `Math.round()`, `Math.random()`

**OS**: `os.time()`, `os.exit(code)`, `os.getenv(name)`, `os.system(cmd)`, `os.sleep(ms)`

**IO**: `io.open(file, mode)`, `io.close(handle)`, `io.read(handle)`, `io.write(handle, data)`

**HTTP** (via curl): `http.get(url)`, `http.post(url, body)` — returns response body as `String`; `http.get_file(url, path)` — downloads to a file (returns 1/0). No JSON parser yet; use `string.find`/`.substring` or plain-text endpoints. See `examples/15_weather_http.fred`.

**Table**: `table.insert(arr, val)`, `table.remove(arr)`, `table.concat(arr, sep)`, `table.sort(arr)`

**Type Conversion**: `to_int()`, `to_float()`, `to_string()`, `to_int_str()`

**Keyboard Input**:
- `input_key()` — reads a single raw keypress (no Enter). Arrow keys → `1`/`2`/`3`/`4` (up/down/right/left), any other key → its ASCII code. See `examples/13_snake_game.fred`.
- `read_line()` — reads a full line from stdin (Enter required) as a `String`; pair with `to_int_str()` for numbers. See `examples/12_number_guessing_game.fred`.

Both are compiler builtins, so they work in `.fred`/`.lua`/`.js`.

### Control Flow
- `if/else` statements
- `while` loops
- `for` loops with `from...to` syntax
- `for-in` loops over arrays
- `break` statement (exit loops early)
- `switch/case/default` statements
- Ternary operator: `condition ? true_value : false_value`
- Array element assignment: `arr[i] = x`
- Compound assignment: `+=`, `-=`, `*=`, `/=` (variables and array elements)
- `nuke()` — `.fred`-only hard-crash kill switch (rejected in `.lua`/`.js`)
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
fred> exit
```

Line editing, history (↑/↓) and **Ctrl+L** (clear screen) come from `rustyline`.
The session is replayed-from-source each line, but only *new* output is printed.

REPL commands:

| Command | Does |
|---------|------|
| `:c`     | Show the C your current session transpiles to |
| `:ast`   | Show the `.fred` IR (AST) for the current session |
| `:cmode` | Switch to a raw **C** scratchpad (type C directly; `#`-lines become includes) |
| `:fred`  | Switch back to `.fred` mode |
| `:reset` | Purge all session state, back to a fresh `.fred` session |
| `:clear` | Clear the screen (or just press Ctrl+L) |
| `exit`   | Quit (or Ctrl+D) |

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
# Show the .fred a JavaScript file transpiles to
fred --to-fred mycode.js output

# Output C code only (don't compile with gcc)
fred --to-c source.fred output.c

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

See `examples/` directory for 15 working demonstrations:
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
11. `11_rock_paper_scissors.fred` - Interactive game: `input_key()` + random CPU
12. `12_number_guessing_game.fred` - Interactive game: `read_line()` numeric input
13. `13_snake_game.fred` - Interactive snake with real arrow-key/WASD input (`input_key()`)
14. `14_new_features.fred` - `arr[i]=`, compound assignment, string methods, `os.sleep`, `nuke()`
15. `15_weather_http.fred` - Networking: `http.get`/`http.post` + polling loop
16. `16_math.js` - **JavaScript**: recursion, GCD, primes, `.map`/`.reduce` → native
17. `17_rock_paper_scissors.js` - **JavaScript** port of #11 (arrow fns, `input_key()`)
18. `18_primes.lua` - **Lua**: real `function/end` syntax, primes, factorial, `^`
19. `19_array_stats.lua` - **Lua**: `ipairs`, `#`, `table.sort`/`table.concat`

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

**For JavaScript and Lua inputs**, a built-in transpiler (`fredc/src/js.rs` or
`fredc/src/lua.rs`) parses the source and emits `.fred` before the normal
pipeline begins. There is no Node.js or Lua interpreter dependency. Use
`fred --to-fred file.js out` (or `file.lua`) to see the generated `.fred`.

## JavaScript support

The JS frontend handles a practical modern subset:

- `let` / `const` / `var`, function declarations, **arrow functions** and
  function expressions (a `const f = (x) => ...` becomes a real fred `fn`)
- `if` / `else if` / `else`, `while`, C-style `for(;;)` (lowered to `while`),
  `for...of` (→ fred `for-in`), `return`, `break`
- Operators with JS→fred mapping: `===`/`!==` → `==`/`!=`, `&&`/`||` → `and`/`or`,
  `!`, ternary, `+ - * / %`, `+=`/`-=`/`*=`/`/=`, `++`/`--` (as statements)
- Template literals `` `${x}` ``, arrays, method/property calls
- `console.log` → `print`, `Math.*` pass through, `parseInt` → `to_int_str`,
  `.toUpperCase()`→`.uppercase()`, `.length` → `.len()`, plus `.map`/`.filter`/
  `.reduce`/`.push`/`.pop`/etc. which line up 1:1
- Compiler builtins like `input_key()` are callable directly

**Not supported** (errors): objects/dicts `{}`, classes, regex, `continue`,
destructuring, spread, async/generators. Arrays remain int-only (fred limit).

## Lua support

The Lua frontend (`fredc/src/lua.rs`) parses **real Lua**, not fred syntax:

- `local` / global vars, `function name() ... end`, `local function`,
  anonymous `function() ... end`
- `if`/`elseif`/`else`/`end`, `while ... do`, numeric `for i = a, b[, step]`,
  generic `for k, v in ipairs(t)` / `pairs(t)`, `repeat ... until`
- Operators with Lua→fred mapping: `..` (concat) → template string,
  `~=` → `!=`, `^` → `Math.pow`, `#x` → `x.len()`, `//` → `/`,
  `and`/`or`/`not` pass through
- `print` (multi-arg → spaced), `tostring` → `to_string`, `tonumber` →
  `to_int_str`, `math.*` → `Math.*`, `table.*`/`os.*` pass through,
  `s:upper()`/`string.upper(s)` → `.uppercase()`, `s:sub()` → `.substring()`
- Array-style table constructors `{1, 2, 3}` → fred arrays

**Not supported** (errors): key/value tables (dicts), metatables, multiple
assignment/return (`a, b = 1, 2`), varargs `...`, coroutines, `goto`,
modules/`require`. Arrays are int-only.

⚠️ **Indexing is passed through verbatim** — Lua is 1-based, fred/C is 0-based.
Prefer `ipairs` value-iteration over manual `t[i]` to keep results correct.

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
- **REPL**: [rustyline](https://github.com/kkawakam/rustyline) (line editing, history, Ctrl+L)
- **JS Transpiler**: hand-written in Rust (`fredc/src/js.rs`) — no external deps

### Running Tests

```bash
make test              # Tests examples/01-03
fred examples/*.fred   # Compile any example
```

### Building from Source

```bash
make check-deps        # Verify gcc, cargo installed
make install           # Build and install fred command
```

---

## Credits

- **Inspiration**: Built on Lua-like syntax, combined with static typing for zero-cost abstraction.
- The JavaScript frontend is a self-contained transpiler written for this project (`fredc/src/js.rs`); earlier versions used [CASTL](https://github.com/paulbernier/castl) by Paul Bernier, now removed.

## Documentation

- **[FREDLANG.md](FREDLANG.md)** — Complete language reference with syntax, types, libraries, and examples
- **[LIMITATIONS.md](LIMITATIONS.md)** — Detailed constraints, known issues, and workarounds

---

**Status**: v1 feature-complete. Production-ready for scripts, tools, and small systems.
