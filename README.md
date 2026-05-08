<div align="center">
  <h1><code>Verd</code></h1>
  <p>
    <strong>A next-generation, effect-transparent systems language that compiles to native machine code.</strong>
  </p>
  <p>
    Python's simplicity. C's speed. Rust's safety. Zero compromises.
  </p>
  <p>
    <em>Built by <strong>Huseyn Verdiyev</strong>, a 15-year-old systems engineer, as a fully standalone compiler project.</em>
  </p>

  <!-- Badges -->
  <img src="https://img.shields.io/badge/Status-Alpha-orange?style=for-the-badge" alt="Status" />
  <img src="https://img.shields.io/badge/Architecture-AOT_Transpiler-blue?style=for-the-badge" alt="Architecture" />
  <img src="https://img.shields.io/badge/License-MIT-green?style=for-the-badge" alt="License" />
</div>

<br/>

## The Problem with Modern Languages

You usually have to pick your poison:
1. Write in **C/C++** and achieve maximum performance, but deal with memory leaks, segfaults, and unreadable pointer arithmetic.
2. Write in **Rust** and get speed with memory safety, but spend months fighting the Borrow Checker and waiting for slow compilation.
3. Write in **Python** and enjoy beautiful, rapid development, but sacrifice 99% of CPU performance and deal with runtime `NoneType` crashes.

## Enter Verd

**Verd** is a statically-compiled, ahead-of-time (AOT) systems programming language. It is designed from the ground up to eliminate these tradeoffs. 

Verd transpiles your human-readable code into memory-safe Rust under the hood, and compiles it via `rustc -O3` into a blazing-fast native binary. 

### Core Tenets

- **Zero-Hidden-Allocation:** No heavy, background Garbage Collector (GC) pausing your threads. 
- **Absolute Null-Safety:** The concept of `null` does not exist. Values are either guaranteed to exist or wrapped in `some/none`.
- **Strict Effect Transparency:** Silent side-effects are illegal. If a function mutates global or outer scope state, it must explicitly declare it with `!flux`.
- **Pipeline Architecture:** Data flows forward. `data |> process |> display` replaces deeply nested function calls.

---

## ⚡ Performance Matrix

| Metric | Python | C++ | Rust | Java (JVM) | **Verd** |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **Execution Speed** | Very Slow | **Max** | **Max** | Fast | **Max** |
| **Cognitive Load** | Low | High | Very High | Medium | **Low** |
| **Null Safety** | ❌ | ❌ | ✅ | ❌ | **✅** |
| **Memory Safety** | ✅ | ❌ | ✅ | ✅ | **✅** |
| **GC Pauses** | Yes | No | No | Yes | **No** |

---

## 📖 Syntax & Features

Verd's syntax is mathematically minimal. No semicolons, no curly brace hell, no boilerplate. 

### Variables & Mutability
By default, everything is immutable (`pin`). To mutate, you must explicitly declare a `flux`.
```rust
pin language = "Verd"
flux version = 1

version = version + 1
```

### The Pipeline Operator
Eliminate ugly nested function calls. Push data forward through the pipeline.
```rust
op double(n) { yield n * 2 }
op add_ten(n) { yield n + 10 }

// Instead of: print(add_ten(double(5)))
5 |> double |> add_ten |> print
```

### Absolute Null Safety
Verd forces you to handle missing data at compile time.
```rust
op fetch_user(id) {
    id == 1 ? { yield some("Huseyn") }
    yield none
}

fetch_user(1) match {
    some(name) -> print("Welcome,", name)
    none       -> print("User not found in DB.")
}
```

### Graceful Error Handling
No silent crashes. Catch explicitly or the compiler will panic safely.
```rust
op divide(a, b) {
    b == 0 ? { rise "Critical: Division by Zero" }
    yield a / b
}

divide(10, 0) catch err {
    print("Recovered from error:", err)
}
```

---

## 🚀 Getting Started

Verd is built on top of the Rust toolchain. Ensure you have [Rust](https://rustup.rs/) installed before proceeding.

### Installation

```bash
# Clone the compiler source
git clone https://github.com/Huseyn-Verdiyev/Verd.git
cd Verd

# Build the Verd compiler
cargo build --release
```

### Usage

The `verd` executable provides a complete toolchain: REPL, Interpreter, and AOT Compiler.

```bash
# 1. Interactive REPL (Fast prototyping)
verd

# 2. Interpret a file (No compilation overhead)
verd run main.verd

# 3. Compile to Native Binary (Production mode, rustc -O3)
verd compile main.verd output_binary

# Execute the resulting binary
./output_binary

# Debugging: See the generated transpiled Rust code
verd emit-rust main.verd
```

---

## 🏗️ Architecture

Verd is a true compiler pipeline, not just a script runner.

1. **Lexer:** Scans raw `.verd` text into a stream of typed Tokens.
2. **Parser:** Uses recursive-descent to construct a strict Abstract Syntax Tree (AST).
3. **Rust Transpiler:** Analyzes the AST and generates 100% memory-safe, zero-overhead Rust code.
4. **Native Compilation:** Invokes the host's `rustc` with `-O3` (Level 3 Optimization) to strip debug symbols and output a highly optimized `.exe` / ELF binary.

---

## 📜 License
Distributed under the **MIT License**. Free to use, modify, and distribute.

<br/>
<div align="center">
  <sub>Built with engineering precision by Huseyn Verdiyev.</sub>
</div>
