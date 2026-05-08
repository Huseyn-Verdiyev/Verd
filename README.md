# Verd

**Verd** is a statically-compiled, effect-transparent programming language written from scratch in Rust.

It transpiles to native Rust code and is compiled with `rustc -O3`, giving it performance comparable to C and Rust — while keeping a clean, minimal syntax with no semicolons and no hidden magic.

> *Built by a 15-year-old systems engineer as a fully standalone language project.*

---

## Why Verd?

Every language today makes a tradeoff: either you get **speed** (C, Rust) or you get **simplicity** (Python, JavaScript). Verd refuses this tradeoff.

| Feature | C | Python | Rust | **Verd** |
|---|:---:|:---:|:---:|:---:|
| Native speed | ✅ | ❌ | ✅ | ✅ |
| Simple syntax | ❌ | ✅ | ❌ | ✅ |
| No hidden GC | ✅ | ❌ | ✅ | ✅ |
| Effect transparency | ❌ | ❌ | ❌ | ✅ |
| No null crashes | ❌ | ❌ | ✅ | ✅ |

### Verd's Core Philosophy

**Effect Transparency** — In Verd, no function can silently mutate external state. If an `op` modifies an outer variable, it must declare it explicitly with `!flux(...)`. This eliminates the #1 cause of bugs in large codebases.

**No Null** — `null` does not exist. Verd uses `some(x) | none` with mandatory `match` handling. If you don't handle the `none` case, it's a compile-time error.

**Readable, Minimal Syntax** — No semicolons. No boilerplate. Keywords designed to express intent, not mimic C.

---

## Syntax

```verd
// Immutable variable
pin name = "Verd"

// Mutable variable
flux counter = 0

// Operation (function)
op greet(who) {
    yield "Hello, " + who + "!"
}

// Pipeline operator
"World" |> greet |> print

// Cycle (loop)
cycle counter < 5 {
    counter = counter + 1
}

// Optional values — no null
op find_user(id) {
    yield some("Huseyn")
}

find_user(1) match {
    some(name) -> print("Found:", name)
    none       -> print("Not found")
}

// Error handling
op divide(x, y) {
    rise "ZeroDivision: cannot divide by zero"
}

divide(10, 0) catch err {
    print("Caught:", err)
}
```

---

## Keyword Map

| Concept | Other Languages | Verd |
|---|---|---|
| Immutable variable | `let / const` | `pin` |
| Mutable variable | `var / mut` | `flux` |
| Function | `fn / def` | `op` |
| Loop | `while / for` | `cycle` |
| Return | `return` | `yield` |
| Throw error | `throw / raise` | `rise` |
| Catch error | `catch / except` | `catch` |
| Pipeline | (none) | `\|>` |
| Optional value | `Optional / None` | `some(x) \| none` |

---

## Architecture

Verd is a **Ahead-of-Time (AOT) transpiler** — it compiles to native binaries via Rust.

```
Source (.verd)
    ↓
  Lexer        → Tokenizes source into typed tokens
    ↓
  Parser       → Builds Abstract Syntax Tree (AST)
    ↓
  Rust Generator → Emits type-safe Rust source code
    ↓
  rustc -O3   → Compiles to native binary
    ↓
  ./program    → Runs at full native speed
```

---

## Getting Started

### Prerequisites
- [Rust](https://rustup.rs/) (for building Verd and compiling generated code)

### Build

```bash
git clone https://github.com/Huseyn-Verdiyev/Verd.git
cd Verd
cargo build --release
```

### Usage

```bash
# Interpret (fast dev cycle, no compilation step)
verd run hello.verd

# Compile to native binary
verd compile hello.verd hello

# Run the binary
./hello

# Inspect generated Rust code
verd emit-rust hello.verd

# Interactive REPL
verd
```

---

## Project Structure

```
src/
  main.rs          — CLI entry point (run / compile / emit-rust / REPL)
  token.rs         — Token definitions
  lexer.rs         — Character-by-character tokenizer
  ast.rs           — Abstract Syntax Tree node types
  parser.rs        — Recursive-descent parser
  evaluator.rs     — Tree-walking interpreter (for REPL / fast iteration)
  rust_generator.rs — AOT Rust code generator (for native compilation)
  c_generator.rs   — C code generator (experimental backend)
```

---

## Roadmap

- [x] Lexer (tokenizer)
- [x] Parser (recursive-descent, full AST)
- [x] Tree-walking interpreter + REPL
- [x] AOT Rust transpiler (native binary via rustc -O3)
- [x] `pin` / `flux` / `op` / `cycle` / `yield`
- [x] `some` / `none` / `match` (no-null system)
- [x] `rise` / `catch` (error handling)
- [x] `|>` pipeline operator
- [ ] Standard library (`use fs`, `use net`, `use math`)
- [ ] Static type inference
- [ ] Structs and records
- [ ] Closures
- [ ] Package manager (`verd add`)

---

## License

MIT — free to use, modify and distribute.

---

*Verd is a personal research project exploring language design and compiler construction from first principles.*
