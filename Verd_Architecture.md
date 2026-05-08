# Verd Programming Language — Architecture & Design

**Verd** is a statically-reasoned, effect-transparent scripting language. It is built from scratch in Rust with zero external dependencies. It is not a clone of Python, C, or any other existing language. Every design decision exists to solve a real, documented problem in modern programming.

---

## Problems Verd Solves

| Problem | Other Languages | Verd |
|---|---|---|
| Null crashes | Everywhere | No `null`. `some / none` model. |
| Silent failures | Often uncaught | `rise` is mandatory |
| Hidden side effects | Invisible | `!flux` declaration required |
| Dependency hell | npm/pip chaos | Built-in primitives only |
| Unreadable errors | `NullPointerException` | Precise, line-level hints |

---

## Core Philosophy: "Strict Effect System"

In Verd, **no `op` (function) can touch the outside world without explicitly declaring it.** If an op reads or modifies an external `flux` variable, it must be declared in the signature. If it doesn't declare it, the evaluator blocks access entirely.

This eliminates the #1 cause of bugs in large codebases: invisible state mutation.

---

## Keyword Map (No Python/C clones)

| Concept | Traditional | Verd |
|---|---|---|
| Immutable variable | `let / const` | `pin` |
| Mutable variable | `var / mut` | `flux` |
| Function | `fn / def` | `op` |
| Loop | `while / for` | `cycle` |
| Return | `return` | `yield` |
| Error throw | `throw / raise` | `rise` |
| Module import | `import / use` | `use` |
| Module declare | `mod / module` | `forge` |
| Parallel task | `thread / goroutine` | `spawn` |
| Sync wait | `await / join` | `sync` |

---

## Type System: "Flow Types"

Verd does not use `int`, `str`, `float`. Types describe the *nature* of data, not its binary representation.

| Type | Meaning |
|---|---|
| `text` | A sequence of characters |
| `count` | A whole number |
| `rate` | A decimal number |
| `flag` | True or false |
| `list(T)` | A collection of T |
| `map(K, V)` | A key-value store |
| `some(T) \| none` | An optional value (no null!) |

---

## Syntax Reference

```verd
// Immutable and mutable variables
pin MAX = 100
flux counter = 0

// Types are optional but encouraged
pin name : text = "Verd"

// Operations (functions)
op add(a, b) {
    yield a + b
}

// Operations with side effects MUST declare them
op increment() !flux(counter) {
    counter = counter + 1
}

// Loops
cycle counter < MAX {
    increment()
}

// Optional values — null does not exist
op find_user(id) -> some(text) | none {
    id == 1 ? yield some("Huseyn")
    yield none
}

// You MUST handle both cases (enforced by evaluator)
find_user(1) match {
    some(name) -> print(name)
    none       -> print("Not found")
}

// Error handling
op divide(a, b) {
    b == 0 ? rise "ZeroDivision: cannot divide by zero"
    yield a / b
}

divide(10, 0) catch err {
    print("Error: ", err)
}

// Pipeline operator — data flows left to right
"hello" |> to_upper |> print

// Parallel execution
spawn heavy_task(data) -> handle
sync handle
```

---

## Architecture: How Verd Executes Code

Verd is a **Tree-Walking Interpreter** written in Rust.

```
Source Code (.verd)
      ↓
  [LEXER]       → Splits text into Tokens
      ↓
  [PARSER]      → Builds Abstract Syntax Tree (AST) from Tokens
      ↓
  [EVALUATOR]   → Walks the AST, executes nodes, enforces Effect System
      ↓
   Result / Error
```

### Execution Stages

1. **Lexer** — Reads raw source character by character. Emits typed Tokens: `Pin`, `Flux`, `Op`, `Cycle`, `Identifier`, `Number`, `Text`, `Plus`, `Pipe`, `Bang`, etc.
2. **Parser** — Consumes the Token stream. Validates grammar. Builds an AST where each node represents one operation (Assignment, Call, BinaryOp, MatchExpr, etc.).
3. **Evaluator** — Walks the AST recursively. Maintains an **Environment** (scope stack). Enforces that ops without `!flux(...)` declarations cannot access external flux variables.

---

## Error Format

Every error includes: file name, line number, column, the problematic code, an arrow pointing to the exact issue, and a human-readable hint.

```
[VERD ERROR] main.verd, line 8, col 12
  find_user(42)
  ^^^^^^^^^^^^^
  → 'find_user' yields 'some(text) | none'
    but the 'none' case is not handled.
  Hint: Use 'match' to handle both outcomes.
```

---

## Build Plan

- `[ ]` **Step 1:** Token enum + Lexer (character-by-character scanner)
- `[ ]` **Step 2:** Parser (Token stream → AST nodes)
- `[ ]` **Step 3:** Evaluator + Environment (scope, variables, effect checking)
- `[ ]` **Step 4:** Error reporter with line/col hints
- `[ ]` **Step 5:** REPL (`verd` command — type code, see results instantly)
- `[ ]` **Step 6:** File runner (`verd run main.verd`)
