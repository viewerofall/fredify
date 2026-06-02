# Fredify Limitations and Known Issues

This document details what .fred cannot do, why, and workarounds when available.

## Design Limitations

These are intentional restrictions that define .fred's scope and enable its simplicity.

### 1. **Arrays are Homogeneous (int64_t only)**

**What doesn't work:**
```fred
let arr = [1, "two", 3]           // ✗ Type mismatch
let nested = [[1, 2], [3, 4]]     // ✗ Array of arrays
let mixed = [1, 2.5, true]        // ✗ Mixed types
```

**Why:**
Arrays compile to C's `int64_t*`. Storing heterogeneous types would require runtime type tags, defeating .fred's zero-overhead goal.

**Workaround:**
- Use separate arrays for each type: `let nums = [1, 2, 3]; let strs = ["a", "b", "c"]`
- Use indices into arrays as proxies: `let names = [...]; let ages = [...]; // indices correspond`
- For structs: Not currently supported; use multiple parallel arrays

---

### 2. **Closures are Not First-Class Values**

**What doesn't work:**
```fred
let add = fn(x) { return fn(y) { return x + y } }  // ✗ Can't return closure
let f = add(5)
print(f(3))

let funcs = [fn(x) { return x }, fn(x) { return x * 2 }]  // ✗ Array of closures
```

**Why:**
Closures are inlined as C functions at compile time. You can't store or return them because they're not values—they're code generation patterns.

**Workaround:**
- Capture variables in outer scope: `let x = 5; let f = fn(y) { return x + y }`
- Use .map/.filter/.reduce inline instead of storing closures
- Pass closures only as direct arguments to .map/.filter/.reduce

---

### 3. **Mutual Recursion Not Supported**

**What doesn't work:**
```fred
fn is_even(n) {
  if (n == 0) { return 1 }
  return is_odd(n - 1)  // ✗ is_odd not declared yet
}
fn is_odd(n) {
  if (n == 0) { return 0 }
  return is_even(n - 1)
}
```

**Why:**
Functions are compiled in order. The second function's declaration happens after the first uses it.

**Workaround:**
- Rewrite with single recursion: `fn is_even(n) { return (n % 2) == 0 }`
- Use a helper parameter: `fn check(n, is_checking_even) { ... }`
- Declare both functions before calling either

---

### 4. **No Dynamic Typing or Runtime Type Checking**

**What doesn't work:**
```fred
let x = 5
if (type(x) == "number") { print(x) }  // ✗ type() doesn't exist
```

**Why:**
.fred is statically typed. Types are fully resolved at compile time; no runtime type information exists.

**Workaround:**
- Keep track of types yourself with variable naming: `let count = 5; let name = "Alice"`
- Use separate variables if you need dual handling: `let num_data = 42; let str_data = "answer"`

---

### 5. **Arrays Cannot Contain Strings**

**What doesn't work:**
```fred
let words = ["hello", "world"]  // ✗ Validation error
```

**Why:**
Arrays store only `int64_t`. Strings are `String` structs; mixing them would require heterogeneous storage.

**Workaround:**
- Use multiple arrays in parallel
- Store indices and maintain a separate string array
- If you need a list of strings, consider file-based storage or alternative data structures

---

### 6. **No Key-Value Dictionaries/Tables**

**What doesn't work:**
```fred
let config = { name = "app", version = 1 }  // ✗ Not supported
print(config.name)
```

**Why:**
.fred only supports arrays (int64_t lists) and strings. General hash tables would add interpreter overhead.

**Workaround:**
- Use parallel arrays: `let keys = ["name", "version"]; let values = ["app", 1]`
- Use the `table.*` library for array operations only (insert, remove, concat, sort)
- For small datasets, use multiple variables: `let app_name = "app"; let app_version = 1`

---

## Feature Limitations

These features don't exist or are incomplete.

### 7. **No Coroutines or async/await**

Not planned for .fred's scope. Use simple loops and conditionals instead.

### 8. **No Metatables**

Lua metatables aren't supported. This is intentional—.fred is simpler than Lua.

### 9. **No Variable Redeclaration**

**What doesn't work:**
```fred
let x = 5
let x = 10  // ✗ x already declared
```

**Why:**
In C, you can't declare the same variable twice. Prevents accidental shadowing.

**Fix:**
- Use assignment instead: `let x = 5; x = 10`
- Use different variable names in different scopes

---

### 10. **Limited String Operations**

**What doesn't work:**
```fred
let s = "hello"
print(s[0])              // ✗ No bracket indexing (use s.char_at(0))
print(s.chars())         // ✗ Not supported
```

**Available string methods:**
- `.length()` - Get length
- `.uppercase()` / `.lowercase()` - Case conversion
- `.substring(start, end)` - Extract substring
- `.trim()` - Strip leading/trailing whitespace
- `.char_at(i)` - One-char String at index
- `.replace(from, to)` - Replace all occurrences
- `string.find(str, pattern)` - Find substring
- `string.split(str, sep)` - Split by separator
- `+` operator - Concatenation

**Workaround:**
- For character access: use `s.char_at(i)` (bracket `s[i]` indexing is not supported)
- For complex parsing: Use Lua or JavaScript and compile to .fred

---

### 11. **No Regex Support**

Regular expressions aren't available. Use `string.find()` and `string.split()` for basic patterns.

---

### 12. **No File Globbing or Directory Listing**

The `io` and `os` libraries don't support wildcards or directory operations.

**Available:**
- `io.open(path, mode)` - Open specific file
- `os.system(cmd)` - Run shell command (can glob via shell)

**Workaround:**
```fred
os.system("ls /path/to/*.txt > /tmp/files.txt")
let file = io.open("/tmp/files.txt", "r")
let contents = io.read(file)
```

---

### 13. **No Lazy Evaluation or Generators**

.fred evaluates all code eagerly. No lazy lists or generators.

---

### 14. **No Module System or Imports**

All code must be in a single file. No `require()` or `import`.

**Workaround:**
- Use shell script to concatenate files before compiling
- Compile separate tools that communicate via files or pipes

---

## Known Bugs and Issues

### Integer Overflow Wraps Silently

```fred
let x = 9223372036854775807  // Max int64_t
print(x + 1)  // Wraps to -9223372036854775808, no error
```

**Impact:** Low (expected behavior in C)
**Workaround:** Check ranges before operations if needed

---

### No Line Numbers in Error Messages

Error messages don't include line numbers, making large files harder to debug.

**Workaround:**
- Keep files small
- Use `fred --to-c` to inspect generated C code
- Search error messages in source manually

---

### Closure Variable Capture May Have Scoping Issues

**Potential issue:**
```fred
let funcs = []
loop i from 0 to 3 {
  let f = fn() { return i }
  // f captures i, but i changes
}
```

**Current behavior:** May not work as expected if closures outlive their loop scope.

**Workaround:**
- Don't rely on closure capture across loop iterations
- Pass values as function arguments instead

---

### Closure Forward Declaration Bug

**Issue:**
When multiple closures are used in the same scope (e.g., multiple `.map()` or `.filter()` calls), the generated C code tries to use closure function declarations before they're defined:

```fred
let evens = arr.filter(fn(x) { return x % 2 == 0 })
let doubled = arr.map(fn(x) { return x * 2 })  // Error on compilation
```

Generated C has:
```c
// closure_filter_0 called here but not yet declared
Array evens = ({ ... closure_filter_0(...) ... });
// declaration comes later
int64_t closure_filter_0(int64_t x) { return x % 2 == 0; }
```

**Workaround:**
- Use a single combined operation: `let result = arr.filter(...).map(...)`
- Or split into intermediate variables with **only one closure per line**
- Avoid multiple filters/maps in the same code block

**Fix needed:**
Add a closure declaration phase that scans for all closures before code generation, then emit forward declarations.

---

### GCC Errors Not User-Friendly

Type mismatches are caught by GCC, not .fred, resulting in C compiler errors:

```
/tmp/test.c:215:31: error: incompatible type for argument 2 of 'string_concat'
```

**Workaround:**
- Use `fred --to-c` to see generated code
- Remember: strings are `String`, numbers are `int64_t`
- Type inference catches most issues at validation stage

---

## What Works Well

- **Recursion** (single-function, up to ~100+ levels)
- **Large arrays** (1000+ elements)
- **String operations** (very long strings, 10k+ chars)
- **Closures with variable capture** (in direct use, not as values)
- **Chained array operations** (.filter().map().reduce())
- **Deep nesting** (many if/loop/switch levels)
- **Integer arithmetic** (with overflow wrapping)
- **File I/O** (reading/writing specific files)
- **Template strings** (with expression interpolation)

---

## Design Philosophy

These limitations exist because .fred has a specific goal: **compiled scripts with zero interpreter overhead, static typing, and simplicity.**

Features like dynamic typing, metatables, and arbitrary data structures would add complexity and runtime overhead. Instead, .fred optimizes for:
- **Fast compilation** (200-600ms typical)
- **No runtime** (pure C execution)
- **Predictable performance** (no GC, no interpretation)
- **Readable generated C code** (for debugging)

If you need:
- **General-purpose scripting** → Use Lua or Python
- **Embeddable bytecode** → Use Lua or Wasm
- **Dynamic features** → Use JavaScript
- **Systems programming** → Use C or Rust

If you need **fast, predictable, compiled scripts with static types**, .fred is for you.

---

## Reporting Issues

Found something not on this list? Open an issue describing:
1. What you tried
2. What you expected
3. What happened instead
4. The generated C code (via `fred --to-c`)
