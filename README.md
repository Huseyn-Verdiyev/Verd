<div align="center">
  <h1><code>Verd</code></h1>
  <p><strong>A systems programming language I built from scratch.</strong></p>
  <p>Native speed. No null. No hidden side-effects. Clean syntax.</p>
  <p><em>I started this at 15. Still going.</em></p>

  <img src="https://img.shields.io/badge/Status-Alpha-orange?style=for-the-badge" />
  <img src="https://img.shields.io/badge/Backend-AOT_Transpiler-blue?style=for-the-badge" />
  <img src="https://img.shields.io/badge/License-MIT-green?style=for-the-badge" />
</div>

<br/>

## Why I Built This

Every language I used made me pick a side.

Either I wrote **C** and got raw speed — but also memory leaks, segfaults, and pointer arithmetic that made my head hurt. Or I wrote **Python**, which I actually enjoyed, but it was slow and would randomly crash at runtime with a `NoneType` error. **Rust** was incredible but the learning curve was brutal for someone starting out.

I wanted something in between. Fast like C, safe like Rust, readable like Python. So I built it.

Verd compiles your code to native machine code via a Rust transpiler backend. You write clean, human-readable code — Verd handles the rest and hands you a binary.

---

## How Verd Compares

I'll be honest — this comparison isn't marketing. It's the actual reason I made these design choices.

### Speed

| Language | How it runs | Verd's take |
| :--- | :--- | :--- |
| Python | Interpreted (slow) | Verd compiles AOT via `rustc -O3`, ~50–100× faster |
| Java | JVM bytecode | No JVM warmup, no GC pauses, smaller binaries |
| C / C++ | Native | Same speed class — Verd compiles down to the same level |
| Rust | Native | Same speed, but Verd is dramatically simpler to write |

### Safety

| Problem | C/C++ | Python | Rust | Verd |
| :--- | :---: | :---: | :---: | :---: |
| Null crashes | ❌ | ❌ | ✅ | ✅ |
| Memory leaks | ❌ | (GC) | ✅ | ✅ |
| Silent side-effects | ❌ | ❌ | ❌ | ✅ (Effect system) |
| GC pauses | — | ❌ | ✅ | ✅ |

### Developer Experience

Rust is amazing, but writing it is hard. Verd gives you the same guarantees with a syntax you can actually read. No lifetime annotations, no borrow checker fights, no header files.

---

## What Makes Verd Different

### 1. No Null — Ever

`null` doesn't exist in Verd. Values are either `some(x)` or `none`, and the compiler forces you to handle both. You literally cannot get a null pointer crash.

```verd
op find_user(id) {
    id == 1 ? { yield some("Huseyn") }
    yield none
}

find_user(1) match {
    some(name) -> print("Found:", name)
    none       -> print("User doesn't exist.")
}
```

### 2. Effect Transparency — No Hidden Mutations

This one's my favorite feature. In any other language, a function can quietly change a global variable and you won't know until something breaks at 2am. In Verd, if an `op` touches an outer `flux` variable, it has to say so explicitly with `!flux(...)`. If it doesn't declare it, the compiler won't allow it.

```verd
flux counter = 0

op increment !flux(counter) {
    counter = counter + 1
}
```

### 3. Pipeline Operator

Instead of nesting function calls like `print(add_ten(double(5)))`, you push data forward:

```verd
5 |> double |> add_ten |> print
```

### 4. Immutable by Default

Every variable in Verd is immutable (`pin`) unless you explicitly say it can change (`flux`). This eliminates an entire class of bugs.

```verd
pin name = "Huseyn"    // locked, can't reassign
flux score = 0         // mutable
score = score + 10     // fine
```

### 5. Standard Library Built In

I also shipped a standard library so you can actually do things:

```verd
use std.fs
use std.math
use std.str

pin content = fs.read("data.txt")
print(math.sqrt(144))            // 12
print(str.upper("hello verd"))   // HELLO VERD
```

---

## Full Syntax Overview

```verd
// Variables
pin language = "Verd"      // immutable
flux version = 1           // mutable

// Operations (functions)
op greet(name) {
    yield "Hello, " + name + "!"
}

// Pipeline
"world" |> greet |> print

// Loops
flux i = 0
cycle i < 5 {
    print(i)
    i = i + 1
}

// Null safety
op fetch(id) {
    id == 0 ? { yield none }
    yield some("data")
}

fetch(1) match {
    some(val) -> print(val)
    none      -> print("empty")
}

// Error handling
op divide(a, b) {
    b == 0 ? { rise "Cannot divide by zero" }
    yield a / b
}

divide(10, 0) catch err {
    print("Caught:", err)
}

// Arrays and Maps
pin nums = [1, 2, 3, 4, 5]
print(nums[0])       // 1
print(nums.len)      // 5

pin user = { name: "Huseyn", age: 15 }
print(user.name)     // Huseyn

// Module imports
use "./utils.verd"
use std.math
```

---

## Keyword Reference

| Concept | Python / JS | Verd |
| :--- | :--- | :--- |
| Immutable variable | `const` / `val` | `pin` |
| Mutable variable | `let` / `var` | `flux` |
| Function | `def` / `function` | `op` |
| Return | `return` | `yield` |
| While loop | `while` | `cycle` |
| Throw error | `raise` / `throw` | `rise` |
| Catch error | `except` / `catch` | `catch` |
| Optional value | `Optional` / `null` | `some(x)` / `none` |
| Pipeline | (none) | `\|>` |
| Import | `import` / `require` | `use` |

---

## Getting Started

You'll need [Rust](https://rustup.rs/) installed since Verd compiles through `rustc`.

```bash
git clone https://github.com/Huseyn-Verdiyev/Verd.git
cd Verd
cargo build --release
```

### CLI Commands

```bash
# Run a Verd file directly (interpreter mode — fastest for dev)
verd run hello.verd

# Compile to a native binary (production mode, -O3)
verd compile hello.verd my_program

# Run the compiled binary
./my_program

# Check inferred types of all variables
verd check hello.verd

# See the generated Rust code
verd emit-rust hello.verd

# Open the interactive REPL
verd
```

---

## How the Compiler Works

I built a full compiler pipeline from scratch:

```
Source (.verd)
     ↓
  Lexer         tokenizes raw text into typed tokens
     ↓
  Parser        recursive-descent, builds a full AST
     ↓
  Resolver      expands `use` imports, loads stdlib markers
     ↓
  Type Checker  infers variable types (Num, Txt, Bool, Array...)
     ↓
  Rust Generator emits memory-safe Rust source code
     ↓
  rustc -O3     compiles to optimized native binary
     ↓
  ./program     runs at full native speed
```

---

## Project Structure

```
src/
  main.rs           CLI — run / compile / check / emit-rust / REPL
  lexer.rs          character-by-character tokenizer
  token.rs          token type definitions
  ast.rs            abstract syntax tree node types
  parser.rs         recursive-descent parser
  resolver.rs       module import resolver
  type_checker.rs   static type inference engine
  evaluator.rs      tree-walking interpreter (for REPL / fast dev)
  rust_generator.rs AOT Rust transpiler
  c_generator.rs    experimental C backend
```

---

## Roadmap

- [x] Lexer + Parser
- [x] Tree-walking interpreter + REPL
- [x] AOT Rust transpiler (native binaries via rustc)
- [x] `pin` / `flux` / `op` / `cycle` / `yield`
- [x] `some` / `none` / `match` (null-free system)
- [x] `rise` / `catch` (error handling)
- [x] `|>` pipeline operator
- [x] Arrays, Maps, index access, field access, method calls
- [x] Module system (`use "./file.verd"`)
- [x] Standard library (`fs`, `math`, `str`, `io`)
- [x] Type inference engine (`verd check`)
- [ ] Static type annotations (optional)
- [ ] Structs / records
- [ ] Closures
- [ ] Package manager (`verd add`)
- [ ] Self-hosting (Verd compiler written in Verd)

---

## License

MIT — do whatever you want with it.

---

<div align="center">
  <sub>Built with ❤️ by <strong>Huseyn Verdiyev</strong></sub>
</div>
