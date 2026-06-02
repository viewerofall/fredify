# fredify roadmap

Single source of truth. Check boxes as things land. Captured so a cold session
starts without re-deriving anything. (TODO.md was merged into this file.)

## ✅ Done

- [x] **Real floats.** `Float` token/AST node + `double` threaded through inference
  + codegen: float literals, arithmetic promotion (any float operand → float),
  `Math.sqrt`/`pow`/`fabs` return `double`, `%g` printing, `${}`/`..` interp via
  `to_string_f`. Works in `.fred`, `.js`, `.lua`.
- [x] **Objects / string-keyed dicts (boxed Values).** Tagged `Value` union +
  string-keyed `Dict` runtime. Scalars stay native/fast; only object fields are
  boxed and dispatch on tag at runtime. `{k:v}` literals, `o.field` read/write,
  `o["key"]`, nested objects, dict-returning fns, runtime arithmetic/compare/print
  on fields. JS `{}` and Lua `{k=v}` frontends emit these.

## The text/data chain — do in order, each unlocks the next

- [ ] **Function parameter type inference.** *(highest value — do first)* Params are
  always `int64_t` today, so `f("hi")`, `f(3.5)`, `f(someObject)` truncate or fail.
  Needs: infer each param's type from call sites (or body usage) in the codegen
  pre-pass, then emit the right C param type. Until then: only pass integers to
  user-defined functions. Also fixes "float function params".
- [ ] **String & heterogeneous arrays.** Arrays are still `int64_t`-only. Make
  `Array.data` a `Value*` — touches every array helper (push/pop/get/set/slice/
  join/includes/sort) AND map/filter/reduce closure codegen (closures take/return
  `int64_t` and do native math — need unbox/rebox). Unlocks `["a","b"]`,
  `[1,"x",3.5]`, arrays-of-objects, objects-containing-arrays.
- [ ] **JSON parse/stringify** (C lib helper). Half-unblocked by dicts; still wants
  string-arrays above.

## Container follow-ups (need string arrays first)

- [ ] Arrays inside dicts / dicts inside arrays.
- [ ] Lua mixed array+dict tables `{1, 2, x=3}` (currently errors; pick a split).
- [ ] Object iteration: `for k, v in pairs(obj)`, `dict.keys()`, `dict.has(k)`,
  `dict.len()`, key deletion.
- [ ] Print a whole object/dict (currently `[object]`) — pretty/JSON form.

## Language semantics to decide

- [ ] **Division.** fred keeps `int / int == int` (C). JS/Lua treat `/` as
  always-float, so JS/Lua `10/3` → `3` not `3.333` right now. Either lower `/` to
  float in those frontends, or document the divergence loudly.
- [ ] Lua **1-based indexing** passed through verbatim (`t[1]` → fred `t[1]`).
  Generic-for/`ipairs` sidesteps it; documented, not auto-rewritten (unsafe without
  intent info).

## Tier 1 ergonomics — each small, self-contained, no type-system risk

- [ ] `else if` chaining in the `.fred`/`.lua` parser (JS already lowers it).
- [ ] `continue` statement (we have `break`); also unblocks JS/Lua `continue`.
- [ ] `os.exec(cmd) -> String` — generalize the `http_run` popen helper to capture
  any command's stdout.
- [ ] `io.read_all(handle) -> String` — `io.read` only grabs one 1024-byte `fgets`.
- [ ] `assert(cond, msg)` / `panic(msg)` — real error paths besides `nuke()`.
- [ ] `Math.random_range(lo, hi)`, float `Math.random()`, `Math.sin/cos/tan/log/exp`,
  `Math.PI` constant, `string.format(fmt, ...)`.

## Optimization / speed (do after the data chain; profile before + after)

- [ ] Don't emit the full `Value`/`Dict` runtime when a program uses no objects —
  gate `emit_value_helpers()` on actual usage to shrink output + gcc time.
- [ ] `dict_get` is linear over keys — fine for small records, but interning keys or
  a tiny hash would help dict-heavy code. Measure first.
- [ ] REPL replays the *entire* session every line (tokenize→codegen→gcc→exec). Cache
  the compiled prefix / incremental-compile instead of full rebuild per line.
- [ ] Array growth strategy + avoid redundant `to_string` allocs in interpolation.
- [ ] Pass `-O2` (and try `-march=native`) for `make`-built example binaries; confirm
  it's already on for installed builds.

## Big / probably-not

- [ ] **Full dynamic typing** (variables change type at runtime). Considered and
  deliberately rejected: boxes every scalar/loop counter, guts the native model.
  Boxed containers cover the real use cases without the perf hit.
- [ ] try/catch / pcall (the saga that gave us `nuke()`), classes/metatables, regex,
  async/coroutines, modules/`require`, destructuring, spread, varargs.

## Frontend gaps

- **JS** (`fredc/src/js.rs`): classes, regex, `continue`, destructuring, spread,
  async/generators, labelled loops, multiple declarators in one `let a = 1, b = 2`.
  `.length` always maps to `.len()` (right for arrays; strings should be `.length()`
  once transpile-time type info exists).
- **Lua** (`fredc/src/lua.rs`): metatables, multiple assignment/return (`a, b = 1, 2`),
  varargs `...`, `goto`, modules/`require`, `string.format`/`gsub`/patterns,
  coroutines.

## Notes

- Both `.js` and `.lua` are real self-contained frontends (no Node, no Lua interp) —
  the original goal: compile JS/Lua to native without big tools.
