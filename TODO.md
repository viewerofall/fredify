# fredify TODO — deferred / out-of-scope

Parked here so the floats + boxed-objects work could land clean. Roughly ordered
by value. Nothing here is started; `ROADMAP.md` has the deeper design notes.

## Done this round (for context)
- ✅ Real floats (`double`) threaded through `.fred`/`.js`/`.lua` — literals,
  arithmetic promotion, `Math.sqrt`/`pow`/`fabs`, `%g` print, `${}`/`..` interp.
- ✅ Objects / string-keyed dicts via a boxed `Value` + `Dict` runtime: `{k: v}`
  literals, `o.field` read/write, `o["key"]`, nested objects, dict-returning fns,
  runtime-dispatched arithmetic/compare/print on fields. JS `{}` and Lua
  `{k=v}` frontends emit these.

## High value — the gaps you'll hit first
- **Function parameter type inference.** Params are always `int64_t` today, so
  `f("hi")`, `f(3.5)`, or `f(someObject)` truncate or fail to compile. This is the
  most-felt limitation now that objects/floats exist. Needs: infer each param's
  type from call sites (or body usage) in the codegen pre-pass, then emit the
  right C param type. Until then: only pass integers to user-defined functions.
- **String & heterogeneous arrays** (the other half of "boxed containers").
  Arrays are still `int64_t`-only. Making `Array.data` a `Value*` touches every
  array helper (push/pop/get/set/slice/join/includes/sort) AND the map/filter/
  reduce closure codegen (closures take/return `int64_t` and do native math on
  elements — they'd need unbox/rebox). Once done: `["a","b"]`, `[1,"x",3.5]`,
  arrays-of-objects, objects-containing-arrays all work.

## Container follow-ups (need string arrays first)
- Store arrays inside dicts and dicts inside arrays (blocked on Value-arrays).
- Lua mixed array+dict tables `{1, 2, x=3}` (currently errors; pick a split).
- Object iteration: `for k, v in pairs(obj)`, `dict.keys()`, `dict.has(k)`,
  `dict.len()`, deleting keys.
- Print a whole object/dict (currently shows `[object]`) — pretty/JSON form.
- JSON parse/stringify (ROADMAP: blocked on string-arrays + dicts; now half-unblocked).

## Language semantics to decide
- **Division.** fred keeps `int / int == int` (C semantics). JS and Lua both treat
  `/` as always-float. Right now JS/Lua `10/3` → `3`, not `3.333`. Either lower
  `/` to float in those frontends, or document the divergence loudly.
- **Float function params** — same root cause as param inference above.
- Lua **1-based indexing** is passed through verbatim (`t[1]` → fred `t[1]`).
  Generic-for/`ipairs` sidesteps it; documented, not auto-rewritten.

## Native `.fred` ergonomics (each small, no type-system risk)
- `else if` chaining in the `.fred`/`.lua` parser (JS already lowers it).
- `continue` statement (have `break`); also unblocks JS/Lua `continue`.
- `assert(cond, msg)` / `panic(msg)` — real error paths besides `nuke()`.
- `os.exec(cmd) -> String`, `io.read_all(handle) -> String`.
- `Math.random_range(lo, hi)`, float `Math.random()`, `Math.sin/cos/tan/log/exp`,
  a `Math.PI` constant, `string.format(fmt, ...)`.

## Big / probably-not
- **Full dynamic typing** (variables change type at runtime). Considered and
  deliberately rejected: it boxes every scalar/loop counter and guts the native
  model. Boxed containers cover the real use cases without the perf hit.
- try/catch / pcall (the saga that gave us `nuke()`), classes/metatables, regex,
  async/coroutines, modules/`require`, destructuring, spread, varargs.

## Frontend gaps
- JS: classes, regex, `continue`, destructuring, spread, async/generators,
  labelled loops, multiple declarators in one `let a = 1, b = 2`.
- Lua: metatables, multiple assignment/return (`a, b = 1, 2`), varargs `...`,
  `goto`, modules/`require`, `string.format`/`gsub`/patterns, coroutines.
