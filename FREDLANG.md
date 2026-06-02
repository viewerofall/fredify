# The .fred Language

## What is .fred?

`.fred` is **not** "Lua + C together". It's a **statically-typed intermediate representation** that compiles directly to C. You write in `.fred` syntax (which resembles Lua), the compiler validates and infers types, then generates pure C code.

Think of it like this:

```
Your .fred code
    ↓ (parser)
Abstract Syntax Tree
    ↓ (validator + type inference)
Typed AST
    ↓ (codegen)
C code with structs, arrays, strings
    ↓ (gcc -lm)
Native executable
```

`.fred` itself is **not a runtime language**—there's no interpreter, no bytecode, no garbage collector. The `.fred` compiler is just a **transpiler** written in Rust that turns your code into C.

---

## Basic Syntax

### Variables and Types

```fred
let x = 42              // int64_t
let s = "hello"         // String struct
let arr = [1, 2, 3]     // Array struct
let b = true            // int64_t (0 or 1)
```

Type inference is automatic. You don't declare types—the compiler figures them out.

### Functions

```fred
fn add(a, b) {
    return a + b
}

fn greet(name) {
    print(`Hello ${name}!`)
}
```

Functions are top-level. They compile to C functions returning `int64_t` (unless they return strings/arrays, which are detected at compile time).

### Control Flow

#### if/else
```fred
if (x > 10) {
    print("big")
} else {
    print("small")
}
```

#### while
```fred
while (x < 100) {
    x = x + 1
}
```

#### for (numeric range)
```fred
loop i from 1 to 10 {
    print(i)
}

loop i from 1 to 100, 2 {  // step by 2
    print(i)
}
```

#### for-in (array iteration)
```fred
let nums = [10, 20, 30]
for n in nums {
    print(n)
}
```

#### switch/case
```fred
switch (code) {
    case 200:
        print("OK")
    case 404:
        print("Not Found")
    default:
        print("Error")
}
```

#### break (early loop exit)
```fred
loop i from 1 to 10 {
    if (i == 5) {
        break
    }
    print(i)
}
// Output: 1 2 3 4
```

#### Ternary operator
```fred
let x = 15
let category = (x < 10) ? "small" : (x < 20) ? "medium" : "large"
print(category)  // "medium"

let status = 200
let msg = (status == 200) ? "OK" : "Error"
print(msg)
```

---

## Type System

### Primitives
- **int64_t**: Numbers (integers and floats coerce to int64_t)
- **String**: Immutable strings with `.length()`, `.uppercase()`, `.lowercase()`, `.substring()`
- **Array**: Dynamic arrays (only store int64_t values)
- **FileHandle**: Opaque pointer for I/O

### Type Inference Rules

```fred
let x = 5 + 3           // int64_t
let s = "a" + "b"       // String (+ on strings = concat)
let arr = [1, 2, 3]     // Array
let str = x + 1         // int64_t (arithmetic)

// Error: can't mix types in operations
// let bad = "text" + 42  ← Validator catches this
```

### Arrays

Arrays **only** store `int64_t`. Strings cannot be in arrays (validation error).

```fred
let arr = [1, 2, 3]

print(arr[0])           // 1
arr[1] = 99             // set element

// Array methods
arr.push(4)
arr.pop()
arr.len()

// Higher-order (with closures)
let doubled = arr.map(fn(x) { return x * 2 })
let evens = arr.filter(fn(x) { return x % 2 == 0 })
let sum = arr.reduce(fn(acc, x) { return acc + x }, 0)
```

### Strings

```fred
let s = "hello"

s.length()
s.uppercase()
s.lowercase()
s.substring(0, 3)       // "hel"

string.find(s, "ll")    // position
string.split(s, "l")    // returns array of positions (limitation)

// Concatenation
let msg = "hello" + " " + "world"

// Template strings (interpolation)
let name = "Claude"
let age = 42
print(`${name} is ${age} years old`)
```

---

## Libraries

### Math
```fred
Math.abs(-5)
Math.sqrt(16)
Math.pow(2, 10)
Math.floor(3.7)
Math.ceil(3.2)
Math.round(3.5)
Math.max(a, b)
Math.min(a, b)
Math.random()
```

### OS
```fred
os.time()               // current timestamp
os.exit(0)              // exit program
os.getenv("HOME")       // env var
os.system("ls -la")     // shell command
```

### IO (file operations)
```fred
let file = io.open("data.txt", "r")
let content = io.read(file)
io.write(file, "data")
io.close(file)
```

### Table (array utilities)
```fred
table.insert(arr, 42)
table.remove(arr)
table.concat(arr, "-")  // join with separator
table.sort(arr)
```

### Type Conversion
```fred
to_int(3.14)            // 3
to_float(5)             // 5.0
to_string(42)           // "42"
to_int_str("123")       // 123
```

---

## Closures

Closures capture their lexical environment and compile to separate C functions.

```fred
fn make_adder(n) {
    fn adder(x) {
        return x + n
    }
    return adder
}

let add5 = make_adder(5)
print(add5(10))  // 15
```

Used extensively with `map`, `filter`, `reduce`:

```fred
let nums = [1, 2, 3, 4, 5]
let result = nums.map(fn(x) {
    return x * x
})
// result: [1, 4, 9, 16, 25]
```

---

## String Interpolation

Template strings use backticks with `${}` for expressions:

```fred
let x = 5
let y = 10
print(`${x} + ${y} = ${x + y}`)
// Output: 5 + 10 = 15

// Works with variables
let name = "World"
print(`Hello ${name}!`)

// Works with method calls
let s = "hello"
print(`Uppercase: ${s.uppercase()}`)

// Automatic type conversion
let num = 42
print(`The answer is ${num}`)
```

---

## Compilation Process

### 1. Lexing
`.fred` source → tokens (keywords, identifiers, operators, strings)

### 2. Parsing
Tokens → Abstract Syntax Tree (AST)

### 3. Validation
- Check undefined variables
- Check library method existence
- Validate closures
- Prevent string arrays

### 4. Type Inference
- Track variable types through assignments
- Detect string vs numeric operations
- Mark return types for functions

### 5. Code Generation
- Emit C struct definitions (Array, String)
- Emit helper functions (array_push, string_concat, etc.)
- Generate function bodies
- Inline closures as C functions

### 6. GCC Compilation
```bash
gcc -o output /tmp/generated.c -lm
```

The `-lm` flag links libmath for `sqrt()`, `pow()`, etc.

---

## Design Philosophy

### What .fred Is
- **Static**: All types known at compile time
- **Simple**: No metatables, no coroutines, no dynamic features
- **Direct**: Compiles to readable C, not bytecode
- **Fast**: Zero interpreter overhead, native execution

### What .fred Is NOT
- Not a Lua clone (syntax borrowed, semantics different)
- Not a C wrapper (it IS C generation)
- Not dynamic (no runtime type checking)
- Not general-purpose (designed for specific use cases: scripts, tools, games)

### The Catch
- Arrays are homogeneous (int64_t only)
- No general tables/dicts (only arrays)
- No complex OOP features
- Limited string manipulation (no regex)
- Type errors caught at compile time, not runtime

---

## Examples

### Example 1: Simple Script
```fred
let count = 10
let sum = 0

loop i from 1 to count {
    sum = sum + i
}

print(sum)  // 55
```

Compiles to:
```c
int main() {
    int64_t count = 10;
    int64_t sum = 0;
    int64_t i = 1;
    while (i <= count) {
        sum = sum + i;
        i += 1;
    }
    printf("%ld\n", sum);
    return 0;
}
```

### Example 2: Array Processing
```fred
let data = [3, 1, 4, 1, 5, 9]
let doubled = data.map(fn(x) { return x * 2 })
let sum = doubled.reduce(fn(acc, x) { return acc + x }, 0)
print(sum)  // 46
```

### Example 3: String Interpolation
```fred
fn factorial(n) {
    if (n <= 1) {
        return 1
    }
    return n * factorial(n - 1)
}

let num = 5
print(`${num}! = ${factorial(num)}`)
// Output: 5! = 120
```

---

## Limitations & Workarounds

| Issue | Workaround |
|-------|-----------|
| No string arrays | Use indices separately: `idx1 = 0; idx2 = 1; ...` |
| No dicts/tables | Use parallel arrays (arr1 keys, arr2 values) |
| No closures in chains | Use intermediate variables: `let x = arr.map(...); let y = x.filter(...)` |
| Limited string ops | Use `string.split()` for parsing, `string.find()` for search |

---

## Summary

**.fred is:**
- A **compiled language** (not interpreted)
- **Lua-like syntax** (familiar to Lua/JS programmers)
- **Statically typed** (type checking at compile time)
- **Direct C generation** (zero interpreter overhead)
- **Designed for scripts and tools** (not general-purpose)

You write `.fred` code, the compiler validates it, generates C, and `gcc` produces a native executable. No runtime, no interpreter, no garbage collector.
