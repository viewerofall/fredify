# fredify roadmap

Captured so the next session starts cold without re-deriving anything.

## ✅ DONE: real floats

Numbers used to be `int64_t`-only. Now there's a `Float` token/AST node and a
`double` type threaded through inference + codegen: float literals, arithmetic
promotion (any float operand → float result), `Math.sqrt`/`pow`/`fabs` return
`double`, `%g` printing, and `${}`/`..` interpolation via `to_string_f`. Works in
`.fred`, `.js`, and `.lua`. (Division still follows C: `int/int == int` — see
TODO.md for the JS/Lua always-float question.)

## ✅ DONE (half): objects / dicts via boxed Values

A tagged `Value` union + string-keyed `Dict` runtime. Scalars stay native/fast;
only object fields are boxed and dispatch on the tag at runtime. Supports `{k:v}`
literals, `o.field` read/write, `o["key"]`, nested objects, dict-returning
functions, and runtime arithmetic/compare/print on fields. JS `{}` and Lua
`{k=v}` frontends emit these.

## The text/data chain (do in order — each unlocks the next)

1. **String / heterogeneous arrays** — arrays are still int-only. Make the Array
   element type the boxed `Value` (touches every array helper + map/filter/reduce
   closures). Unlocks `string.split` word lists, arrays-of-objects, mixed arrays.
2. ✅ **Dicts / key-value** — done (see above). Arrays-in-dicts still blocked on #1.
3. **JSON parser** (C lib helper) — half-unblocked; still wants string-arrays (#1).

## Also now on the critical path
- **Function parameter type inference** — params are `int64_t`, so strings/floats/
  objects can't be passed to user functions yet. Highest-value next step. See TODO.md.

## Tier 1 ergonomics (each a few hours, self-contained, no type-system risk)

- `else if` chaining in the `.fred`/`.lua` parser (JS frontend already lowers it
  to nested `else { if }`, but native `.fred` still can't write it)
- `continue` statement (we have `break`); also unblocks JS `continue`
- `os.exec(cmd) -> String` — generalize the existing `http_run` popen helper to
  capture any command's stdout
- `io.read_all(handle) -> String` — `io.read` only grabs one 1024-byte `fgets`
- `assert(cond, msg)` + `panic(msg)` — real error paths besides `nuke()`
- `Math.random_range(lo, hi)`
- `string.format(fmt, ...)`

## JS frontend (`fredc/src/js.rs`) — known gaps

Currently a solid modern-JS subset. Not yet supported:
- objects/dicts `{}` (blocked on fred dicts), classes, regex
- `continue` (blocked on fred `continue`)
- destructuring, spread, async/generators, labelled loops
- multiple declarators in one `let a = 1, b = 2` (split them)
- `.length` always maps to `.len()` (correct for arrays; strings should use
  `.length()` — revisit once type info is available at transpile time)

## Lua frontend (`fredc/src/lua.rs`) — known gaps

Real Lua subset now compiles. Not yet supported:
- key/value tables / dicts (blocked on fred dicts), metatables
- multiple assignment/return (`a, b = 1, 2`), varargs `...`
- coroutines, `goto`, modules/`require`
- `string.format`/`gsub`/patterns (no regex in fred)
- **1-based indexing is passed through** — Lua `t[1]` becomes fred `t[1]`
  (the *second* element). Generic-for/`ipairs` sidesteps it; revisit if we
  want true Lua index semantics (would mean rewriting `t[i]` → `t[i-1]`,
  which is unsafe without type/intent info). Documented loudly for now.

## Notes

- Both `.js` and `.lua` are now real, self-contained frontends (no Node, no Lua
  interpreter) — the original goal: compile JS/Lua to native without big tools.
