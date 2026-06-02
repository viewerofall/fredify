# fredify roadmap

Captured so the next session starts cold without re-deriving anything.

## #1 big rock: real floats

Right now **all numbers are `int64_t`**. `Number(f64)` literals exist but every
`Math.*` returns `int64_t`, so `let y = 412.35` truncates and `Math.sqrt(2)`
returns `1`. This is the deepest correctness wart.

Needs a second numeric type threaded through:
- lexer already keeps `f64`; add a `Float` vs `Int` distinction in the AST/types
- type inference: a value is float if it has a fractional literal, comes from a
  float-returning fn, or mixes with a float
- codegen: emit `double` storage + `%g` printing; fix `Math.sqrt/pow/floor/...`
  to return `double` where appropriate
- mixed int/float arithmetic promotion rules

High effort (touches inference + codegen everywhere), high payoff.

## The text/data chain (do in order — each unlocks the next)

1. **String arrays** — arrays are int-only today. This is the single unlock for
   real text work: `string.split` returning words, lists of names, file lines.
2. **Dicts / key-value** — the other half. Pairs with string arrays.
3. **JSON parser** (C lib helper) — turns the HTTP layer from "fetch text" into
   "consume APIs". **Blocked** on 1 + 2; don't start it first.

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
