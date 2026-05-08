/// Abstract Syntax Tree nodes for the Verd language.
/// Every piece of Verd code maps to one of these variants.

#[derive(Debug, Clone)]
pub enum Expr {
    // === Literals ===
    Number(f64),
    Text(String),
    Bool(bool),
    None,

    // === Variable reference ===
    Identifier(String),

    // === Variable declaration ===
    // pin x = <value>   (immutable)
    // flux x = <value>  (mutable)
    Pin  { name: String, value: Box<Expr> },
    Flux { name: String, value: Box<Expr> },

    // === Assignment ===
    // x = <value>
    Assign { name: String, value: Box<Expr> },

    // === Binary operations ===
    // a + b, a == b, a < b, etc.
    BinaryOp {
        op: BinOp,
        left:  Box<Expr>,
        right: Box<Expr>,
    },

    // === Op (function) declaration ===
    // op add(a, b) { ... }
    OpDecl {
        name:    String,
        params:  Vec<String>,
        effects: Vec<String>,   // names from !flux(x, y)
        body:    Vec<Expr>,
    },

    // === Op call ===
    // add(1, 2)  or  print("hello")
    Call {
        name: String,
        args: Vec<Expr>,
    },

    // === yield (return) ===
    Yield(Box<Expr>),

    // === rise (throw) ===
    Rise(Box<Expr>),

    // === cycle (while loop) ===
    Cycle {
        condition: Box<Expr>,
        body:      Vec<Expr>,
    },

    // === match expression ===
    // find_user(1) match { some(x) -> ..., none -> ... }
    Match {
        subject: Box<Expr>,
        some_branch: Option<(String, Vec<Expr>)>,  // (bound_name, body)
        none_branch: Option<Vec<Expr>>,
    },

    // === Pipeline ===
    // "hello" |> to_upper |> print
    Pipeline {
        stages: Vec<Expr>,
    },

    // === catch ===
    // expr catch err { ... }
    Catch {
        body:    Vec<Expr>,
        err_var: String,
        handler: Vec<Expr>,
    },

    // === Inline conditional ===
    // condition ? { ... }
    Question {
        condition: Box<Expr>,
        body:      Vec<Expr>,
    },

    // === Spawn / Sync ===
    Spawn { call: Box<Expr>, handle: String },
    Sync  { handle: String },

    // === Array literal ===
    // [1, 2, 3]  or  ["a", "b"]
    Array { elements: Vec<Expr> },

    // === Map literal ===
    // { name: "Huseyn", age: 15 }
    Map { pairs: Vec<(String, Expr)> },

    // === Index access ===
    // arr[0]  or  map["key"]
    Index { object: Box<Expr>, index: Box<Expr> },

    // === Field access ===
    // arr.len  or  user.name
    Field { object: Box<Expr>, field: String },

    // === Method call ===
    // arr.push(x)  or  str.upper()
    MethodCall { object: Box<Expr>, method: String, args: Vec<Expr> },

    // === Use (import) ===
    // use "./math.verd"  or  use std.fs
    Use { path: String },
}

/// Binary operator kinds.
#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, NotEq, Lt, Gt, LtEq, GtEq,
}
