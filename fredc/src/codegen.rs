use crate::ast::{Expr, Stmt};
use std::collections::HashMap;

pub fn generate_c(stmts: &[Stmt]) -> String {
    let mut gen = CGen::new();

    // Add header comment about compilation
    gen.output.push_str("/* Generated C code from .fred compiler\n");
    gen.output.push_str(" * Compile with: gcc -o output thisfile.c -lm\n");
    gen.output.push_str(" * The -lm flag links libmath (required for Math.* functions)\n");
    gen.output.push_str(" */\n\n");

    // Generate array struct and helper functions
    gen.emit_array_structs();
    gen.emit_array_helpers();

    // Separate function definitions from other statements
    let mut fn_defs = Vec::new();
    let mut other_stmts = Vec::new();

    for stmt in stmts {
        match stmt {
            Stmt::FnDef { .. } => fn_defs.push(stmt),
            _ => other_stmts.push(stmt),
        }
    }

    // Pre-pass: infer each function's return type so calls + signatures agree.
    // Iterate twice so functions that call later-defined functions still resolve.
    for _ in 0..2 {
        for stmt in &fn_defs {
            if let Stmt::FnDef { name, params, body } = stmt {
                let ret = gen.infer_fn_return_type(params, body);
                gen.fn_types.insert(name.clone(), ret);
            }
        }
    }

    // Generate function definitions at top level
    for stmt in fn_defs {
        gen.gen_stmt(stmt);
    }

    // First pass: count closures
    for stmt in &other_stmts {
        gen.scan_stmt_for_closures(stmt);
    }

    // Generate forward declarations for closures
    for i in 0..gen.closure_counter {
        gen.output.push_str(&format!("int64_t closure_map_{}(int64_t x);\n", i));
        gen.output.push_str(&format!("int64_t closure_filter_{}(int64_t x);\n", i));
        gen.output.push_str(&format!("int64_t closure_reduce_{}(int64_t acc, int64_t x);\n", i));
    }
    gen.output.push_str("\n");

    // Reset closure counter for actual generation
    gen.closure_counter = 0;
    gen.closures.clear();

    // Generate main function with other statements
    gen.output.push_str("int main() {\n");
    gen.output.push_str("  srand((unsigned)time(NULL));\n");
    gen.indent = 1;
    for stmt in other_stmts {
        gen.gen_stmt(stmt);
    }
    gen.output.push_str("  return 0;\n}\n");

    // Emit closures at end
    gen.output.push_str(&gen.closures);

    gen.output
}

struct CGen {
    output: String,
    closures: String,
    indent: usize,
    closure_counter: usize,
    var_types: HashMap<String, String>,
    fn_types: HashMap<String, String>,
}

impl CGen {
    fn new() -> Self {
        CGen {
            output: String::from("#include <stdio.h>\n#include <stdint.h>\n#include <stdlib.h>\n#include <string.h>\n#include <termios.h>\n#include <unistd.h>\n\n"),
            closures: String::new(),
            indent: 0,
            closure_counter: 0,
            var_types: HashMap::new(),
            fn_types: HashMap::new(),
        }
    }

    // Infer a function's C return type by scanning its return statements.
    // Returns "String", "Array", or "int64_t".
    fn infer_fn_return_type(&self, params: &[String], body: &[Stmt]) -> String {
        // Treat params as int64_t for the purpose of this scan (they always are).
        let mut probe = CGen {
            output: String::new(),
            closures: String::new(),
            indent: 0,
            closure_counter: 0,
            var_types: HashMap::new(),
            fn_types: self.fn_types.clone(),
        };
        for p in params {
            probe.var_types.insert(p.clone(), "int64_t".to_string());
        }
        probe.scan_returns_for_type(body)
    }

    fn scan_returns_for_type(&mut self, body: &[Stmt]) -> String {
        for stmt in body {
            // Track locals so return of a local var resolves correctly.
            if let Stmt::Let { name, value: Some(v) } = stmt {
                if self.expr_returns_string(v) {
                    self.var_types.insert(name.clone(), "String".to_string());
                } else if self.expr_returns_array(v) {
                    self.var_types.insert(name.clone(), "Array".to_string());
                } else {
                    self.var_types.insert(name.clone(), "int64_t".to_string());
                }
            }
            let found = match stmt {
                Stmt::Return(Some(e)) => {
                    if self.expr_returns_string(e) {
                        Some("String".to_string())
                    } else if self.expr_returns_array(e) {
                        Some("Array".to_string())
                    } else {
                        None
                    }
                }
                Stmt::If { then_body, else_body, .. } => {
                    let t = self.scan_returns_for_type(then_body);
                    if t != "int64_t" {
                        Some(t)
                    } else if let Some(eb) = else_body {
                        let e = self.scan_returns_for_type(eb);
                        if e != "int64_t" { Some(e) } else { None }
                    } else {
                        None
                    }
                }
                Stmt::While { body, .. } | Stmt::Loop { body, .. } | Stmt::ForIn { body, .. } => {
                    let t = self.scan_returns_for_type(body);
                    if t != "int64_t" { Some(t) } else { None }
                }
                _ => None,
            };
            if let Some(ty) = found {
                return ty;
            }
        }
        "int64_t".to_string()
    }

    fn emit_array_structs(&mut self) {
        self.output.push_str("typedef struct {\n");
        self.output.push_str("  int64_t* data;\n");
        self.output.push_str("  int64_t len;\n");
        self.output.push_str("  int64_t cap;\n");
        self.output.push_str("} Array;\n\n");
        self.output.push_str("typedef struct {\n");
        self.output.push_str("  char* data;\n");
        self.output.push_str("  int64_t len;\n");
        self.output.push_str("} String;\n\n");
    }

    fn emit_array_helpers(&mut self) {
        // array_new
        self.output.push_str("Array array_new() {\n");
        self.output.push_str("  Array arr;\n");
        self.output.push_str("  arr.data = malloc(sizeof(int64_t) * 10);\n");
        self.output.push_str("  arr.len = 0;\n");
        self.output.push_str("  arr.cap = 10;\n");
        self.output.push_str("  return arr;\n");
        self.output.push_str("}\n\n");

        // array_push
        self.output.push_str("void array_push(Array* arr, int64_t val) {\n");
        self.output.push_str("  if (arr->len >= arr->cap) {\n");
        self.output.push_str("    arr->cap *= 2;\n");
        self.output.push_str("    arr->data = realloc(arr->data, sizeof(int64_t) * arr->cap);\n");
        self.output.push_str("  }\n");
        self.output.push_str("  arr->data[arr->len++] = val;\n");
        self.output.push_str("}\n\n");

        // array_pop
        self.output.push_str("int64_t array_pop(Array* arr) {\n");
        self.output.push_str("  if (arr->len > 0) return arr->data[--arr->len];\n");
        self.output.push_str("  return 0;\n");
        self.output.push_str("}\n\n");

        // array_len
        self.output.push_str("int64_t array_len(Array* arr) {\n");
        self.output.push_str("  return arr->len;\n");
        self.output.push_str("}\n\n");

        // array_get
        self.output.push_str("int64_t array_get(Array* arr, int64_t idx) {\n");
        self.output.push_str("  if (idx >= 0 && idx < arr->len) return arr->data[idx];\n");
        self.output.push_str("  return 0;\n");
        self.output.push_str("}\n\n");

        // array_set
        self.output.push_str("void array_set(Array* arr, int64_t idx, int64_t val) {\n");
        self.output.push_str("  if (idx >= 0 && idx < arr->len) arr->data[idx] = val;\n");
        self.output.push_str("}\n\n");

        // string_new_literal
        self.output.push_str("String string_new_literal(const char* s) {\n");
        self.output.push_str("  String str;\n");
        self.output.push_str("  str.len = strlen(s);\n");
        self.output.push_str("  str.data = malloc(str.len + 1);\n");
        self.output.push_str("  strcpy(str.data, s);\n");
        self.output.push_str("  return str;\n");
        self.output.push_str("}\n\n");

        // string_concat
        self.output.push_str("String string_concat(String a, String b) {\n");
        self.output.push_str("  String result;\n");
        self.output.push_str("  result.len = a.len + b.len;\n");
        self.output.push_str("  result.data = malloc(result.len + 1);\n");
        self.output.push_str("  strcpy(result.data, a.data);\n");
        self.output.push_str("  strcat(result.data, b.data);\n");
        self.output.push_str("  return result;\n");
        self.output.push_str("}\n\n");

        // Math functions
        self.output.push_str("#include <math.h>\n");
        self.output.push_str("int64_t math_abs(int64_t x) { return (x < 0) ? -x : x; }\n");
        self.output.push_str("int64_t math_floor(double x) { return (int64_t)floor(x); }\n");
        self.output.push_str("int64_t math_ceil(double x) { return (int64_t)ceil(x); }\n");
        self.output.push_str("int64_t math_round(double x) { return (int64_t)round(x); }\n");
        self.output.push_str("int64_t math_sqrt(double x) { return (int64_t)sqrt(x); }\n");
        self.output.push_str("int64_t math_pow(double x, double y) { return (int64_t)pow(x, y); }\n");
        self.output.push_str("int64_t math_max(int64_t a, int64_t b) { return (a > b) ? a : b; }\n");
        self.output.push_str("int64_t math_min(int64_t a, int64_t b) { return (a < b) ? a : b; }\n");
        self.output.push_str("int64_t math_random() { return rand() % 1000000; }\n\n");

        // Raw keyboard input: returns 1=up 2=down 3=right 4=left for arrow keys,
        // otherwise the raw ASCII code of the key pressed (e.g. 'q'=113, 'w'=119).
        self.output.push_str("int64_t input_key() {\n");
        self.output.push_str("  struct termios old, raw;\n");
        self.output.push_str("  if (tcgetattr(STDIN_FILENO, &old) != 0) return getchar();\n");
        self.output.push_str("  raw = old;\n");
        self.output.push_str("  raw.c_lflag &= ~(ICANON | ECHO);\n");
        self.output.push_str("  raw.c_cc[VMIN] = 1; raw.c_cc[VTIME] = 0;\n");
        self.output.push_str("  tcsetattr(STDIN_FILENO, TCSANOW, &raw);\n");
        self.output.push_str("  int c = getchar();\n");
        self.output.push_str("  int64_t key = c;\n");
        self.output.push_str("  if (c == 27) {\n");
        self.output.push_str("    if (getchar() == '[') {\n");
        self.output.push_str("      switch (getchar()) {\n");
        self.output.push_str("        case 'A': key = 1; break;\n");
        self.output.push_str("        case 'B': key = 2; break;\n");
        self.output.push_str("        case 'C': key = 3; break;\n");
        self.output.push_str("        case 'D': key = 4; break;\n");
        self.output.push_str("      }\n");
        self.output.push_str("    }\n");
        self.output.push_str("  }\n");
        self.output.push_str("  tcsetattr(STDIN_FILENO, TCSANOW, &old);\n");
        self.output.push_str("  return key;\n");
        self.output.push_str("}\n\n");

        // Read a full line from stdin (newline stripped) as a String.
        self.output.push_str("String read_line() {\n");
        self.output.push_str("  char buf[1024];\n");
        self.output.push_str("  if (!fgets(buf, sizeof(buf), stdin)) { String s; s.len = 0; s.data = malloc(1); s.data[0] = '\\0'; return s; }\n");
        self.output.push_str("  size_t n = strlen(buf);\n");
        self.output.push_str("  if (n > 0 && buf[n-1] == '\\n') buf[n-1] = '\\0';\n");
        self.output.push_str("  return string_new_literal(buf);\n");
        self.output.push_str("}\n\n");

        // Type conversion functions
        self.output.push_str("int64_t to_int(double x) { return (int64_t)x; }\n");
        self.output.push_str("double to_float(int64_t x) { return (double)x; }\n");
        self.output.push_str("String to_string(int64_t x) { String s; char buf[64]; snprintf(buf, sizeof(buf), \"%ld\", x); s = string_new_literal(buf); return s; }\n");
        self.output.push_str("int64_t to_int_str(String s) { return strtoll(s.data, NULL, 10); }\n");
        self.output.push_str("int64_t string_length(String s) { return s.len; }\n\n");

        // String methods
        self.output.push_str("String string_uppercase(String s) {\n");
        self.output.push_str("  String result;\n");
        self.output.push_str("  result.len = s.len;\n");
        self.output.push_str("  result.data = malloc(s.len + 1);\n");
        self.output.push_str("  for (int64_t i = 0; i < s.len; i++) {\n");
        self.output.push_str("    result.data[i] = (s.data[i] >= 'a' && s.data[i] <= 'z') ? s.data[i] - 32 : s.data[i];\n");
        self.output.push_str("  }\n");
        self.output.push_str("  result.data[s.len] = '\\0';\n");
        self.output.push_str("  return result;\n");
        self.output.push_str("}\n\n");

        self.output.push_str("String string_lowercase(String s) {\n");
        self.output.push_str("  String result;\n");
        self.output.push_str("  result.len = s.len;\n");
        self.output.push_str("  result.data = malloc(s.len + 1);\n");
        self.output.push_str("  for (int64_t i = 0; i < s.len; i++) {\n");
        self.output.push_str("    result.data[i] = (s.data[i] >= 'A' && s.data[i] <= 'Z') ? s.data[i] + 32 : s.data[i];\n");
        self.output.push_str("  }\n");
        self.output.push_str("  result.data[s.len] = '\\0';\n");
        self.output.push_str("  return result;\n");
        self.output.push_str("}\n\n");

        self.output.push_str("String string_substring(String s, int64_t start, int64_t end) {\n");
        self.output.push_str("  String result;\n");
        self.output.push_str("  if (start < 0) start = 0;\n");
        self.output.push_str("  if (end > s.len) end = s.len;\n");
        self.output.push_str("  if (start >= end || start >= s.len) { result.len = 0; result.data = malloc(1); result.data[0] = '\\0'; return result; }\n");
        self.output.push_str("  result.len = end - start;\n");
        self.output.push_str("  result.data = malloc(result.len + 1);\n");
        self.output.push_str("  for (int64_t i = 0; i < result.len; i++) result.data[i] = s.data[start + i];\n");
        self.output.push_str("  result.data[result.len] = '\\0';\n");
        self.output.push_str("  return result;\n");
        self.output.push_str("}\n\n");

        // Trim leading/trailing ASCII whitespace
        self.output.push_str("String string_trim(String s) {\n");
        self.output.push_str("  int64_t start = 0; int64_t end = s.len;\n");
        self.output.push_str("  while (start < end && (s.data[start]==' '||s.data[start]=='\\t'||s.data[start]=='\\n'||s.data[start]=='\\r')) start++;\n");
        self.output.push_str("  while (end > start && (s.data[end-1]==' '||s.data[end-1]=='\\t'||s.data[end-1]=='\\n'||s.data[end-1]=='\\r')) end--;\n");
        self.output.push_str("  return string_substring(s, start, end);\n");
        self.output.push_str("}\n\n");

        // Return a one-char String at index (empty if out of range)
        self.output.push_str("String string_char_at(String s, int64_t idx) {\n");
        self.output.push_str("  if (idx < 0 || idx >= s.len) { String e; e.len=0; e.data=malloc(1); e.data[0]='\\0'; return e; }\n");
        self.output.push_str("  return string_substring(s, idx, idx + 1);\n");
        self.output.push_str("}\n\n");

        // Replace all occurrences of `from` with `to`
        self.output.push_str("String string_replace(String s, String from, String to) {\n");
        self.output.push_str("  if (from.len == 0) return s;\n");
        self.output.push_str("  char* out = malloc(s.len * (to.len > 1 ? to.len : 1) + s.len + 1);\n");
        self.output.push_str("  int64_t oi = 0; int64_t i = 0;\n");
        self.output.push_str("  while (i < s.len) {\n");
        self.output.push_str("    if (i + from.len <= s.len && strncmp(s.data + i, from.data, from.len) == 0) {\n");
        self.output.push_str("      memcpy(out + oi, to.data, to.len); oi += to.len; i += from.len;\n");
        self.output.push_str("    } else { out[oi++] = s.data[i++]; }\n");
        self.output.push_str("  }\n");
        self.output.push_str("  out[oi] = '\\0';\n");
        self.output.push_str("  String r = string_new_literal(out); free(out); return r;\n");
        self.output.push_str("}\n\n");

        // Table operations
        self.output.push_str("void table_insert(Array* t, int64_t val) { array_push(t, val); }\n");
        self.output.push_str("int64_t table_remove(Array* t) { return array_pop(t); }\n");
        self.output.push_str("String table_concat(Array* t, String sep) {\n");
        self.output.push_str("  if (t->len == 0) { String s; s.len = 0; s.data = malloc(1); s.data[0] = '\\0'; return s; }\n");
        self.output.push_str("  String result; result.len = 0; result.data = malloc(1024); result.data[0] = '\\0';\n");
        self.output.push_str("  for (int64_t i = 0; i < t->len; i++) {\n");
        self.output.push_str("    char buf[64]; snprintf(buf, 64, \"%ld\", t->data[i]);\n");
        self.output.push_str("    strcat(result.data, buf);\n");
        self.output.push_str("    if (i < t->len - 1) strcat(result.data, sep.data);\n");
        self.output.push_str("  }\n");
        self.output.push_str("  result.len = strlen(result.data);\n");
        self.output.push_str("  return result;\n");
        self.output.push_str("}\n\n");

        self.output.push_str("void table_sort(Array* t) {\n");
        self.output.push_str("  for (int64_t i = 0; i < t->len - 1; i++) {\n");
        self.output.push_str("    for (int64_t j = 0; j < t->len - i - 1; j++) {\n");
        self.output.push_str("      if (t->data[j] > t->data[j+1]) {\n");
        self.output.push_str("        int64_t temp = t->data[j]; t->data[j] = t->data[j+1]; t->data[j+1] = temp;\n");
        self.output.push_str("      }\n");
        self.output.push_str("    }\n");
        self.output.push_str("  }\n");
        self.output.push_str("}\n\n");

        // OS operations
        self.output.push_str("#include <time.h>\n");
        self.output.push_str("int64_t os_time() { return (int64_t)time(NULL); }\n");
        self.output.push_str("void os_exit(int64_t code) { exit((int)code); }\n");
        self.output.push_str("String os_getenv(String name) {\n");
        self.output.push_str("  const char* val = getenv(name.data);\n");
        self.output.push_str("  if (val) return string_new_literal(val);\n");
        self.output.push_str("  String s; s.len = 0; s.data = malloc(1); s.data[0] = '\\0'; return s;\n");
        self.output.push_str("}\n");
        self.output.push_str("int64_t os_system(String cmd) { return system(cmd.data); }\n");
        self.output.push_str("void os_sleep(int64_t ms) { usleep((useconds_t)(ms * 1000)); }\n\n");

        // nuke: .fred-only hard crash (SIGABRT). Gated in the validator.
        self.output.push_str("int64_t fred_nuke() { fprintf(stderr, \"\\n*** FRED NUKE DETONATED ***\\n\"); abort(); return 0; }\n\n");

        // IO operations
        self.output.push_str("typedef void* FileHandle;\n");
        self.output.push_str("FileHandle io_open(String filename, String mode) { return (FileHandle)fopen(filename.data, mode.data); }\n");
        self.output.push_str("void io_close(FileHandle f) { if (f) fclose((FILE*)f); }\n");
        self.output.push_str("String io_read(FileHandle f) {\n");
        self.output.push_str("  char buf[1024]; fgets(buf, sizeof(buf), (FILE*)f); return string_new_literal(buf);\n");
        self.output.push_str("}\n");
        self.output.push_str("void io_write(FileHandle f, String s) { fputs(s.data, (FILE*)f); }\n\n");

        // String operations
        self.output.push_str("int64_t string_find(String s, String pattern) {\n");
        self.output.push_str("  char* pos = strstr(s.data, pattern.data);\n");
        self.output.push_str("  if (pos) return pos - s.data;\n");
        self.output.push_str("  return -1;\n");
        self.output.push_str("}\n\n");

        self.output.push_str("Array string_split(String s, String sep) {\n");
        self.output.push_str("  Array result = array_new();\n");
        self.output.push_str("  if (sep.len == 0) return result;\n");
        self.output.push_str("  char* copy = malloc(s.len + 1);\n");
        self.output.push_str("  strcpy(copy, s.data);\n");
        self.output.push_str("  char* token = strtok(copy, sep.data);\n");
        self.output.push_str("  while (token) {\n");
        self.output.push_str("    String str = string_new_literal(token);\n");
        self.output.push_str("    array_push(&result, (int64_t)(uintptr_t)str.data);\n");
        self.output.push_str("    token = strtok(NULL, sep.data);\n");
        self.output.push_str("  }\n");
        self.output.push_str("  free(copy);\n");
        self.output.push_str("  return result;\n");
        self.output.push_str("}\n\n");

        // Array operations
        self.output.push_str("Array array_slice(Array* arr, int64_t start, int64_t end) {\n");
        self.output.push_str("  Array result = array_new();\n");
        self.output.push_str("  if (start < 0) start = 0;\n");
        self.output.push_str("  if (end > arr->len) end = arr->len;\n");
        self.output.push_str("  if (start >= end || start >= arr->len) return result;\n");
        self.output.push_str("  for (int64_t i = start; i < end; i++) {\n");
        self.output.push_str("    array_push(&result, arr->data[i]);\n");
        self.output.push_str("  }\n");
        self.output.push_str("  return result;\n");
        self.output.push_str("}\n\n");

        self.output.push_str("String array_join(Array* arr, String sep) {\n");
        self.output.push_str("  if (arr->len == 0) { String s; s.len = 0; s.data = malloc(1); s.data[0] = '\\0'; return s; }\n");
        self.output.push_str("  String result; result.len = 0; result.data = malloc(4096); result.data[0] = '\\0';\n");
        self.output.push_str("  for (int64_t i = 0; i < arr->len; i++) {\n");
        self.output.push_str("    char buf[64]; snprintf(buf, 64, \"%ld\", arr->data[i]);\n");
        self.output.push_str("    strcat(result.data, buf);\n");
        self.output.push_str("    if (i < arr->len - 1) strcat(result.data, sep.data);\n");
        self.output.push_str("  }\n");
        self.output.push_str("  result.len = strlen(result.data);\n");
        self.output.push_str("  return result;\n");
        self.output.push_str("}\n\n");

        self.output.push_str("int64_t array_includes(Array* arr, int64_t val) {\n");
        self.output.push_str("  for (int64_t i = 0; i < arr->len; i++) {\n");
        self.output.push_str("    if (arr->data[i] == val) return 1;\n");
        self.output.push_str("  }\n");
        self.output.push_str("  return 0;\n");
        self.output.push_str("}\n\n");
    }

    fn sanitize_name(&self, name: &str) -> String {
        let reserved = ["double", "int", "float", "char", "void", "return", "if", "else", "while", "for"];
        if reserved.contains(&name) {
            format!("{}_", name)
        } else {
            name.to_string()
        }
    }

    fn indent_str(&self) -> String {
        "  ".repeat(self.indent)
    }

    fn emit(&mut self, code: &str) {
        self.output.push_str(&self.indent_str());
        self.output.push_str(code);
        self.output.push('\n');
    }

    fn gen_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::FnDef { name, params, body } => {
                let safe_name = self.sanitize_name(name);
                let params_str = if params.is_empty() {
                    "void".to_string()
                } else {
                    params
                        .iter()
                        .map(|p| format!("int64_t {}", p))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                let ret_type = self.fn_types.get(name).cloned().unwrap_or_else(|| "int64_t".to_string());
                // Params are int64_t; register them so the body generates correctly.
                let saved_types = self.var_types.clone();
                for p in params {
                    self.var_types.insert(p.clone(), "int64_t".to_string());
                }
                self.output.push_str(&format!("{} {}({}) {{\n", ret_type, safe_name, params_str));
                self.indent += 1;
                for stmt in body {
                    self.gen_stmt(stmt);
                }
                self.indent -= 1;
                self.output.push_str("}\n\n");
                self.var_types = saved_types;
            }
            Stmt::Let { name, value } => {
                if let Some(val) = value {
                    let is_array = self.expr_returns_array(val);
                    let is_string = self.expr_returns_string(val);
                    let is_file_handle = if let Expr::MethodCall { obj, method, .. } = val {
                        self.expr_to_string(obj) == "io" && method == "open"
                    } else {
                        false
                    };
                    let expr = self.gen_expr(val);
                    if is_array {
                        self.var_types.insert(name.clone(), "Array".to_string());
                        self.emit(&format!("Array {} = {};", name, expr));
                    } else if is_string {
                        self.var_types.insert(name.clone(), "String".to_string());
                        self.emit(&format!("String {} = {};", name, expr));
                    } else if is_file_handle {
                        self.var_types.insert(name.clone(), "FileHandle".to_string());
                        self.emit(&format!("FileHandle {} = {};", name, expr));
                    } else {
                        self.var_types.insert(name.clone(), "int64_t".to_string());
                        self.emit(&format!("int64_t {} = {};", name, expr));
                    }
                } else {
                    self.var_types.insert(name.clone(), "int64_t".to_string());
                    self.emit(&format!("int64_t {} = 0;", name));
                }
            }
            Stmt::Assign { target, value } => {
                let expr = self.gen_expr(value);
                self.emit(&format!("{} = {};", target, expr));
            }
            Stmt::AssignIndex { obj, index, value } => {
                let obj_str = self.gen_expr(obj);
                let idx_str = self.gen_expr(index);
                let val_str = self.gen_expr(value);
                self.emit(&format!("array_set(&{}, {}, {});", obj_str, idx_str, val_str));
            }
            Stmt::If {
                cond,
                then_body,
                else_body,
            } => {
                let cond_str = self.gen_expr(cond);
                self.emit(&format!("if ({}) {{", cond_str));
                self.indent += 1;
                for stmt in then_body {
                    self.gen_stmt(stmt);
                }
                self.indent -= 1;
                if let Some(else_stmts) = else_body {
                    self.emit("} else {");
                    self.indent += 1;
                    for stmt in else_stmts {
                        self.gen_stmt(stmt);
                    }
                    self.indent -= 1;
                }
                self.emit("}");
            }
            Stmt::While { cond, body } => {
                let cond_str = self.gen_expr(cond);
                self.emit(&format!("while ({}) {{", cond_str));
                self.indent += 1;
                for stmt in body {
                    self.gen_stmt(stmt);
                }
                self.indent -= 1;
                self.emit("}");
            }
            Stmt::Loop {
                var,
                from,
                to,
                step,
                body,
            } => {
                let from_str = self.gen_expr(from);
                let to_str = self.gen_expr(to);
                let step_val = step.as_ref().map(|s| self.gen_expr(s)).unwrap_or_else(|| "1".to_string());
                self.emit(&format!("int64_t {} = {};", var, from_str));
                self.emit(&format!("while ({} <= {}) {{", var, to_str));
                self.indent += 1;
                for stmt in body {
                    self.gen_stmt(stmt);
                }
                self.emit(&format!("{} += {};", var, step_val));
                self.indent -= 1;
                self.emit("}");
            }
            Stmt::Return(expr) => {
                if let Some(e) = expr {
                    let expr_str = self.gen_expr(e);
                    self.emit(&format!("return {};", expr_str));
                } else {
                    self.emit("return 0;");
                }
            }
            Stmt::Break => {
                self.emit("break;");
            }
            Stmt::Expr(expr) => {
                let expr_str = self.gen_expr(expr);
                self.emit(&format!("(void){};", expr_str));
            }
            Stmt::ForIn { var, iter, body } => {
                let iter_expr = self.gen_expr(iter);
                let iter_var = format!("__iter_{}", var);
                self.emit(&format!("Array {} = {};", iter_var, iter_expr));
                self.emit(&format!("for (int64_t __i_{} = 0; __i_{} < {}.len; __i_{}++) {{", var, var, iter_var, var));
                self.indent += 1;
                self.emit(&format!("int64_t {} = {}.data[__i_{}];", var, iter_var, var));
                for stmt in body {
                    self.gen_stmt(stmt);
                }
                self.indent -= 1;
                self.emit("}");
            }
            Stmt::Switch { expr, cases } => {
                let expr_str = self.gen_expr(expr);
                self.emit(&format!("switch ({}) {{", expr_str));
                self.indent += 1;
                for (case_expr, body) in cases {
                    if let Some(ce) = case_expr {
                        let case_str = self.gen_expr(ce);
                        self.emit(&format!("case {}:", case_str));
                    } else {
                        self.emit("default:");
                    }
                    self.indent += 1;
                    for stmt in body {
                        self.gen_stmt(stmt);
                    }
                    self.emit("break;");
                    self.indent -= 1;
                }
                self.indent -= 1;
                self.emit("}");
            }
        }
    }

    fn gen_expr(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Number(n) => {
                if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            Expr::String(s) => {
                let escaped = s.replace("\\", "\\\\")
                    .replace("\"", "\\\"")
                    .replace("\n", "\\n")
                    .replace("\r", "\\r")
                    .replace("\t", "\\t");
                format!("string_new_literal(\"{}\")", escaped)
            }
            Expr::Bool(b) => if *b { "1" } else { "0" }.to_string(),
            Expr::Nil => "0".to_string(),
            Expr::Id(name) => name.clone(),
            Expr::BinOp { left, op, right } => {
                let l = self.gen_expr(left);
                let r = self.gen_expr(right);
                if op == "+" {
                    let is_left_string = self.expr_returns_string(left);
                    let is_right_string = self.expr_returns_string(right);
                    if is_left_string || is_right_string {
                        return format!("string_concat({}, {})", l, r);
                    }
                }
                let op_str = match op.as_str() {
                    "==" => "==",
                    "!=" => "!=",
                    "<" => "<",
                    "<=" => "<=",
                    ">" => ">",
                    ">=" => ">=",
                    "and" => "&&",
                    "or" => "||",
                    _ => op,
                };
                format!("({} {} {})", l, op_str, r)
            }
            Expr::UnOp { op, expr } => {
                let e = self.gen_expr(expr);
                match op.as_str() {
                    "!" => format!("(!{})", e),
                    "-" => format!("(-{})", e),
                    _ => format!("{}({})", op, e),
                }
            }
            Expr::Call { func, args } => {
                let f = self.gen_expr(func);
                let args_str = args
                    .iter()
                    .map(|a| self.gen_expr(a))
                    .collect::<Vec<_>>()
                    .join(", ");

                // Special handling for built-in functions
                if let Expr::Id(name) = func.as_ref() {
                    if name == "print" {
                        if args.is_empty() {
                            return "printf(\"%s\", \"\")".to_string();
                        }
                        let mut format_str = String::new();
                        let mut arg_values = Vec::new();
                        for arg in args {
                            let is_string = self.expr_returns_string(arg);
                            if is_string {
                                format_str.push_str("%s");
                                arg_values.push(format!("{}.data", self.gen_expr(arg)));
                            } else {
                                format_str.push_str("%ld");
                                arg_values.push(self.gen_expr(arg));
                            }
                        }
                        format_str.push_str("\\n");
                        let args_str = arg_values.join(", ");
                        return format!("printf(\"{}\", {})", format_str, args_str);
                    }
                    // nuke: .fred-only hard crash
                    if name == "nuke" {
                        return "fred_nuke()".to_string();
                    }
                    // Type conversion functions
                    match name.as_str() {
                        "to_int" | "to_float" | "to_int_str" | "string_length" => {
                            return format!("{}({})", name, args_str);
                        }
                        _ => {}
                    }
                }

                format!("{}({})", f, args_str)
            }
            Expr::MethodCall { obj, method, args } => {
                let obj_str = self.expr_to_string(obj);
                let obj_type = if matches!(obj.as_ref(), Expr::String(_)) {
                    "String".to_string()
                } else if let Expr::MethodCall { method: m, .. } = obj.as_ref() {
                    // Check if the inner method call returns a string
                    if matches!(m.as_str(), "uppercase" | "lowercase" | "substring" | "trim" | "char_at" | "replace") {
                        "String".to_string()
                    } else {
                        "Array".to_string()
                    }
                } else {
                    self.var_types.get(&obj_str).cloned().unwrap_or_else(|| "int64_t".to_string())
                };
                let obj_expr = self.gen_expr(obj);

                // Math library handling
                if obj_str == "Math" {
                    let args_str = args.iter().map(|a| self.gen_expr(a)).collect::<Vec<_>>().join(", ");
                    return match method.as_str() {
                        "abs" => format!("math_abs({})", args_str),
                        "floor" => format!("math_floor({})", args_str),
                        "ceil" => format!("math_ceil({})", args_str),
                        "round" => format!("math_round({})", args_str),
                        "sqrt" => format!("math_sqrt({})", args_str),
                        "pow" => format!("math_pow({})", args_str),
                        "max" => format!("math_max({})", args_str),
                        "min" => format!("math_min({})", args_str),
                        "random" => "math_random()".to_string(),
                        _ => "0".to_string(),
                    };
                }

                // Table library handling
                if obj_str == "table" {
                    let args_str = args.iter().map(|a| self.gen_expr(a)).collect::<Vec<_>>().join(", ");
                    return match method.as_str() {
                        "insert" => {
                            if args.len() >= 2 {
                                let arr = self.gen_expr(&args[0]);
                                let val = self.gen_expr(&args[1]);
                                format!("(table_insert(&({}), {}), 0)", arr, val)
                            } else {
                                "0".to_string()
                            }
                        }
                        "remove" => format!("table_remove(&({}))", self.gen_expr(&args[0])),
                        "concat" => {
                            if args.len() >= 2 {
                                let arr = self.gen_expr(&args[0]);
                                let sep = self.gen_expr(&args[1]);
                                format!("table_concat(&({}), {})", arr, sep)
                            } else {
                                "0".to_string()
                            }
                        }
                        "sort" => {
                            let arr = self.gen_expr(&args[0]);
                            format!("(table_sort(&({})), 0)", arr)
                        }
                        _ => "0".to_string(),
                    };
                }

                // OS library handling
                if obj_str == "os" {
                    return match method.as_str() {
                        "time" => "os_time()".to_string(),
                        "exit" => {
                            let code = self.gen_expr(&args[0]);
                            format!("(os_exit({}), 0)", code)
                        }
                        "getenv" => {
                            let name = self.gen_expr(&args[0]);
                            format!("os_getenv({})", name)
                        }
                        "system" => {
                            let cmd = self.gen_expr(&args[0]);
                            format!("os_system({})", cmd)
                        }
                        "sleep" => {
                            let ms = self.gen_expr(&args[0]);
                            format!("(os_sleep({}), 0)", ms)
                        }
                        _ => "0".to_string(),
                    };
                }

                // IO library handling
                if obj_str == "io" {
                    return match method.as_str() {
                        "open" => {
                            let filename = self.gen_expr(&args[0]);
                            let mode = self.gen_expr(&args[1]);
                            format!("io_open({}, {})", filename, mode)
                        }
                        "close" => {
                            let handle = self.gen_expr(&args[0]);
                            format!("(io_close({}), 0)", handle)
                        }
                        "read" => {
                            let handle = self.gen_expr(&args[0]);
                            format!("io_read({})", handle)
                        }
                        "write" => {
                            let handle = self.gen_expr(&args[0]);
                            let data = self.gen_expr(&args[1]);
                            format!("(io_write({}, {}), 0)", handle, data)
                        }
                        _ => "0".to_string(),
                    };
                }

                // String methods handling
                if obj_type == "String" {
                    return match method.as_str() {
                        "length" => format!("string_length({})", obj_expr),
                        "uppercase" => format!("string_uppercase({})", obj_expr),
                        "lowercase" => format!("string_lowercase({})", obj_expr),
                        "substring" => {
                            if args.len() == 2 {
                                let start = self.gen_expr(&args[0]);
                                let end = self.gen_expr(&args[1]);
                                format!("string_substring({}, {}, {})", obj_expr, start, end)
                            } else {
                                "0".to_string()
                            }
                        }
                        "trim" => format!("string_trim({})", obj_expr),
                        "char_at" => {
                            if args.len() == 1 {
                                let idx = self.gen_expr(&args[0]);
                                format!("string_char_at({}, {})", obj_expr, idx)
                            } else {
                                "0".to_string()
                            }
                        }
                        "replace" => {
                            if args.len() == 2 {
                                let from = self.gen_expr(&args[0]);
                                let to = self.gen_expr(&args[1]);
                                format!("string_replace({}, {}, {})", obj_expr, from, to)
                            } else {
                                "0".to_string()
                            }
                        }
                        _ => "0".to_string(),
                    };
                }

                // String library handling
                if obj_str == "string" {
                    return match method.as_str() {
                        "find" => {
                            let s = self.gen_expr(&args[0]);
                            let pattern = self.gen_expr(&args[1]);
                            format!("string_find({}, {})", s, pattern)
                        }
                        "split" => {
                            let s = self.gen_expr(&args[0]);
                            let sep = self.gen_expr(&args[1]);
                            format!("string_split({}, {})", s, sep)
                        }
                        _ => "0".to_string(),
                    };
                }

                match method.as_str() {
                    "len" => {
                        if obj_type == "Array" {
                            format!("array_len(&{})", obj_expr)
                        } else {
                            format!("0")
                        }
                    }
                    "push" => {
                        if args.len() == 1 && obj_type == "Array" {
                            let arg = self.gen_expr(&args[0]);
                            format!("(array_push(&{}, {}), 0)", obj_expr, arg)
                        } else {
                            format!("0")
                        }
                    }
                    "pop" => {
                        if obj_type == "Array" {
                            format!("array_pop(&{})", obj_expr)
                        } else {
                            format!("0")
                        }
                    }
                    "map" => {
                        if args.len() == 1 && obj_type == "Array" {
                            let closure = &args[0];
                            if let Expr::Closure { params, body } = closure {
                                self.gen_map_closure(&obj_expr, params, body)
                            } else {
                                format!("0")
                            }
                        } else {
                            format!("0")
                        }
                    }
                    "filter" => {
                        if args.len() == 1 && obj_type == "Array" {
                            let closure = &args[0];
                            if let Expr::Closure { params, body } = closure {
                                self.gen_filter_closure(&obj_expr, params, body)
                            } else {
                                format!("0")
                            }
                        } else {
                            format!("0")
                        }
                    }
                    "reduce" => {
                        if args.len() >= 2 && obj_type == "Array" {
                            let closure = &args[0];
                            let init = self.gen_expr(&args[1]);
                            if let Expr::Closure { params, body } = closure {
                                self.gen_reduce_closure(&obj_expr, params, body, &init)
                            } else {
                                format!("0")
                            }
                        } else {
                            format!("0")
                        }
                    }
                    "slice" => {
                        if args.len() == 2 && obj_type == "Array" {
                            let start = self.gen_expr(&args[0]);
                            let end = self.gen_expr(&args[1]);
                            format!("array_slice(&{}, {}, {})", obj_expr, start, end)
                        } else {
                            format!("0")
                        }
                    }
                    "join" => {
                        if args.len() == 1 && obj_type == "Array" {
                            let sep = self.gen_expr(&args[0]);
                            format!("array_join(&{}, {})", obj_expr, sep)
                        } else {
                            format!("0")
                        }
                    }
                    "includes" => {
                        if args.len() == 1 && obj_type == "Array" {
                            let val = self.gen_expr(&args[0]);
                            format!("array_includes(&{}, {})", obj_expr, val)
                        } else {
                            format!("0")
                        }
                    }
                    _ => format!("0"),
                }
            }
            Expr::Index { obj, index } => {
                let o = self.gen_expr(obj);
                let i = self.gen_expr(index);
                format!("array_get(&{}, {})", o, i)
            }
            Expr::Field { obj, field } => {
                let o = self.gen_expr(obj);
                format!("{}.{}", o, field)
            }
            Expr::Array(elements) => {
                let mut code = "({ ".to_string();
                code.push_str("Array __arr = array_new(); ");
                for elem in elements {
                    let e = self.gen_expr(elem);
                    code.push_str(&format!("array_push(&__arr, {}); ", e));
                }
                code.push_str("__arr; })");
                code
            }
            Expr::Object(_fields) => "0".to_string(), // Stub for now
            Expr::Closure { .. } => "0".to_string(), // Closures handled in method calls
            Expr::TemplateString(nodes) => {
                let mut parts = Vec::new();
                for node in nodes {
                    match node {
                        crate::ast::TemplateStringNode::Text(t) => {
                            let escaped = t.replace("\\", "\\\\")
                                .replace("\"", "\\\"")
                                .replace("\n", "\\n")
                                .replace("\r", "\\r")
                                .replace("\t", "\\t");
                            parts.push(format!("string_new_literal(\"{}\")", escaped));
                        }
                        crate::ast::TemplateStringNode::Expr(e) => {
                            let expr_code = self.gen_expr(e);
                            if self.expr_returns_string(e) {
                                parts.push(expr_code);
                            } else {
                                parts.push(format!("to_string({})", expr_code));
                            }
                        }
                    }
                }
                if parts.is_empty() {
                    "string_new_literal(\"\")".to_string()
                } else {
                    parts.iter().cloned().reduce(|a, b| format!("string_concat({}, {})", a, b)).unwrap()
                }
            }
            Expr::Ternary { cond, then_expr, else_expr } => {
                let cond_str = self.gen_expr(cond);
                let then_str = self.gen_expr(then_expr);
                let else_str = self.gen_expr(else_expr);
                format!("(({}) ? ({}) : ({}))", cond_str, then_str, else_str)
            }
        }
    }

    fn expr_to_string(&self, expr: &Expr) -> String {
        match expr {
            Expr::Id(name) => name.clone(),
            _ => String::new(),
        }
    }

    fn expr_returns_string(&self, expr: &Expr) -> bool {
        match expr {
            Expr::String(_) => true,
            Expr::TemplateString(_) => true,
            Expr::Id(name) => self.var_types.get(name).map(|t| t == "String").unwrap_or(false),
            Expr::BinOp { left, op, right } => {
                if op == "+" {
                    self.expr_returns_string(left) || self.expr_returns_string(right)
                } else {
                    false
                }
            }
            Expr::MethodCall { obj, method, .. } => {
                let obj_str = self.expr_to_string(obj);
                match obj_str.as_str() {
                    "os" => matches!(method.as_str(), "getenv"),
                    "io" => matches!(method.as_str(), "read"),
                    "table" => matches!(method.as_str(), "concat"),
                    "string" => matches!(method.as_str(), "split") == false && matches!(method.as_str(), "find") == false,
                    _ => matches!(method.as_str(), "uppercase" | "lowercase" | "substring" | "join" | "trim" | "char_at" | "replace"),
                }
            }
            Expr::Call { func, .. } => {
                if let Expr::Id(name) = func.as_ref() {
                    name == "to_string"
                        || name == "read_line"
                        || self.fn_types.get(name).map(|t| t == "String").unwrap_or(false)
                } else {
                    false
                }
            }
            Expr::Ternary { then_expr, else_expr, .. } => {
                self.expr_returns_string(then_expr) || self.expr_returns_string(else_expr)
            }
            _ => false,
        }
    }

    fn expr_returns_array(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Array(_) => true,
            Expr::Id(name) => self.var_types.get(name).map(|t| t == "Array").unwrap_or(false),
            Expr::Call { func, .. } => {
                if let Expr::Id(name) = func.as_ref() {
                    self.fn_types.get(name).map(|t| t == "Array").unwrap_or(false)
                } else {
                    false
                }
            }
            Expr::MethodCall { obj, method, .. } => {
                let obj_str = self.expr_to_string(obj);
                match obj_str.as_str() {
                    "string" => matches!(method.as_str(), "split"),
                    _ => matches!(method.as_str(), "map" | "filter" | "slice"),
                }
            }
            _ => false,
        }
    }

    fn scan_for_closures(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.scan_stmt_for_closures(stmt);
        }
    }

    fn scan_stmt_for_closures(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { value: Some(val), .. } => self.scan_expr_for_closures(val),
            Stmt::Assign { value, .. } => self.scan_expr_for_closures(value),
            Stmt::AssignIndex { obj, index, value } => {
                self.scan_expr_for_closures(obj);
                self.scan_expr_for_closures(index);
                self.scan_expr_for_closures(value);
            }
            Stmt::If { cond, then_body, else_body, .. } => {
                self.scan_expr_for_closures(cond);
                for s in then_body {
                    self.scan_stmt_for_closures(s);
                }
                if let Some(els) = else_body {
                    for s in els {
                        self.scan_stmt_for_closures(s);
                    }
                }
            }
            Stmt::While { cond, body } => {
                self.scan_expr_for_closures(cond);
                for s in body {
                    self.scan_stmt_for_closures(s);
                }
            }
            Stmt::Loop { from, to, step, body, .. } => {
                self.scan_expr_for_closures(from);
                self.scan_expr_for_closures(to);
                if let Some(s) = step {
                    self.scan_expr_for_closures(s);
                }
                for stmt in body {
                    self.scan_stmt_for_closures(stmt);
                }
            }
            Stmt::Return(Some(expr)) => self.scan_expr_for_closures(expr),
            Stmt::Expr(expr) => self.scan_expr_for_closures(expr),
            _ => {}
        }
    }

    fn scan_expr_for_closures(&mut self, expr: &Expr) {
        match expr {
            Expr::MethodCall { args, .. } => {
                for arg in args {
                    if matches!(arg, Expr::Closure { .. }) {
                        self.closure_counter += 1;
                    }
                }
            }
            Expr::Call { args, .. } => {
                for arg in args {
                    self.scan_expr_for_closures(arg);
                }
            }
            Expr::BinOp { left, right, .. } => {
                self.scan_expr_for_closures(left);
                self.scan_expr_for_closures(right);
            }
            Expr::UnOp { expr, .. } => self.scan_expr_for_closures(expr),
            Expr::Index { obj, index } => {
                self.scan_expr_for_closures(obj);
                self.scan_expr_for_closures(index);
            }
            Expr::Field { obj, .. } => self.scan_expr_for_closures(obj),
            Expr::Array(elements) => {
                for elem in elements {
                    self.scan_expr_for_closures(elem);
                }
            }
            Expr::Ternary { cond, then_expr, else_expr } => {
                self.scan_expr_for_closures(cond);
                self.scan_expr_for_closures(then_expr);
                self.scan_expr_for_closures(else_expr);
            }
            _ => {}
        }
    }

    fn gen_map_closure(&mut self, arr: &str, params: &[String], body: &[Stmt]) -> String {
        let closure_id = self.closure_counter;
        self.closure_counter += 1;

        let param_name = params.get(0).map(|p| p.as_str()).unwrap_or("x");

        // Generate closure function to closures field
        self.closures.push_str(&format!("int64_t closure_map_{}(int64_t {}) {{\n", closure_id, param_name));
        for stmt in body {
            if let Stmt::Return(Some(expr)) = stmt {
                let expr_str = self.gen_expr(expr);
                self.closures.push_str(&format!("  return {};\n", expr_str));
            }
        }
        self.closures.push_str("  return 0;\n}\n\n");

        // Generate map loop
        format!(
            "({{ Array __result = array_new(); for (int64_t __i = 0; __i < array_len(&{}); __i++) {{ array_push(&__result, closure_map_{}(array_get(&{}, __i))); }} __result; }})",
            arr, closure_id, arr
        )
    }

    fn gen_filter_closure(&mut self, arr: &str, params: &[String], body: &[Stmt]) -> String {
        let closure_id = self.closure_counter;
        self.closure_counter += 1;

        let param_name = params.get(0).map(|p| p.as_str()).unwrap_or("x");

        // Generate closure function to closures field
        self.closures.push_str(&format!("int64_t closure_filter_{}(int64_t {}) {{\n", closure_id, param_name));
        for stmt in body {
            if let Stmt::Return(Some(expr)) = stmt {
                let expr_str = self.gen_expr(expr);
                self.closures.push_str(&format!("  return {};\n", expr_str));
            }
        }
        self.closures.push_str("  return 0;\n}\n\n");

        // Generate filter loop
        format!(
            "({{ Array __result = array_new(); for (int64_t __i = 0; __i < array_len(&{}); __i++) {{ int64_t __v = array_get(&{}, __i); if (closure_filter_{}(__v)) array_push(&__result, __v); }} __result; }})",
            arr, arr, closure_id
        )
    }

    fn gen_reduce_closure(&mut self, arr: &str, params: &[String], body: &[Stmt], init: &str) -> String {
        let closure_id = self.closure_counter;
        self.closure_counter += 1;

        let acc_param = params.get(0).map(|p| p.as_str()).unwrap_or("acc");
        let val_param = params.get(1).map(|p| p.as_str()).unwrap_or("x");

        // Generate closure function to closures field
        self.closures.push_str(&format!("int64_t closure_reduce_{}(int64_t {}, int64_t {}) {{\n", closure_id, acc_param, val_param));
        for stmt in body {
            if let Stmt::Return(Some(expr)) = stmt {
                let expr_str = self.gen_expr(expr);
                self.closures.push_str(&format!("  return {};\n", expr_str));
            }
        }
        self.closures.push_str("  return 0;\n}\n\n");

        // Generate reduce loop
        format!(
            "({{ int64_t __acc = {}; for (int64_t __i = 0; __i < array_len(&{}); __i++) {{ __acc = closure_reduce_{}(__acc, array_get(&{}, __i)); }} __acc; }})",
            init, arr, closure_id, arr
        )
    }
}
