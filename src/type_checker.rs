/// Verd Type Inference — Phase 4
///
/// Walks the AST and infers concrete types for variables.
/// This lets the Rust generator emit `f64` / `String` / `bool`
/// instead of the generic `V` (VerdValue) enum where possible,
/// producing faster and cleaner compiled code.

use std::collections::HashMap;
use crate::ast::{BinOp, Expr};

/// The concrete type of a Verd expression, as far as we can infer.
#[derive(Debug, Clone, PartialEq)]
pub enum VType {
    Num,              // f64
    Txt,              // String
    Bool,             // bool
    Array(Box<VType>),// Vec<T>
    Map,              // HashMap<String, V>
    SomeOf(Box<VType>),// some(T)
    None,             // none
    Void,             // no value (statements)
    Unknown,          // cannot infer — fall back to V enum
}

impl VType {
    pub fn to_rust_type(&self) -> String {
        match self {
            VType::Num        => "f64".to_string(),
            VType::Txt        => "String".to_string(),
            VType::Bool       => "bool".to_string(),
            VType::Array(t)   => format!("Vec<{}>", t.to_rust_type()),
            VType::Map        => "std::collections::HashMap<String, V>".to_string(),
            VType::SomeOf(t)  => format!("Option<{}>", t.to_rust_type()),
            VType::None       => "Option<V>".to_string(),
            VType::Void       => "()".to_string(),
            VType::Unknown    => "V".to_string(),
        }
    }
}

/// Maps variable names → their inferred type.
pub type TypeEnv = HashMap<String, VType>;

/// The type checker / inferrer.
pub struct TypeChecker {
    /// Stack of scopes, innermost last.
    scopes: Vec<TypeEnv>,
}

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker { scopes: vec![HashMap::new()] }
    }

    fn push_scope(&mut self) { self.scopes.push(HashMap::new()); }
    fn pop_scope(&mut self)  { self.scopes.pop(); }

    fn declare(&mut self, name: &str, t: VType) {
        if let Some(s) = self.scopes.last_mut() {
            s.insert(name.to_string(), t);
        }
    }

    fn lookup(&self, name: &str) -> VType {
        for scope in self.scopes.iter().rev() {
            if let Some(t) = scope.get(name) {
                return t.clone();
            }
        }
        VType::Unknown
    }

    /// Infer the type of an expression.
    pub fn infer(&mut self, expr: &Expr) -> VType {
        match expr {
            // Literals
            Expr::Number(_) => VType::Num,
            Expr::Text(_)   => VType::Txt,
            Expr::Bool(_)   => VType::Bool,
            Expr::None      => VType::None,

            // Variable lookup
            Expr::Identifier(name) => self.lookup(name),

            // Declarations
            Expr::Pin { name, value } | Expr::Flux { name, value } => {
                let t = self.infer(value);
                self.declare(name, t);
                VType::Void
            }

            // Assignment
            Expr::Assign { name, value } => {
                let t = self.infer(value);
                self.declare(name, t);
                VType::Void
            }

            // Binary operations
            Expr::BinaryOp { op, left, right } => {
                let lt = self.infer(left);
                let rt = self.infer(right);
                self.infer_binop(op, &lt, &rt)
            }

            // Calls — infer return type from known built-ins
            Expr::Call { name, args } => {
                match name.as_str() {
                    "print" | "io.println" | "io.print" => VType::Void,
                    "some"  => {
                        let inner = args.first().map(|a| self.infer(a)).unwrap_or(VType::Unknown);
                        VType::SomeOf(Box::new(inner))
                    }
                    "fs.read" | "fs.lines" | "str.upper" | "str.lower"
                    | "str.trim" | "str.reverse" | "str.replace" | "str.repeat" => VType::Txt,
                    "fs.write" | "fs.delete" | "fs.exists" => VType::Bool,
                    "math.sqrt" | "math.pow" | "math.abs" | "math.floor"
                    | "math.ceil" | "math.round" | "math.min" | "math.max"
                    | "math.sin" | "math.cos" | "math.log" | "math.pi" | "math.e" => VType::Num,
                    "str.contains" | "str.starts_with" | "str.ends_with" => VType::Bool,
                    "str.split"  => VType::Array(Box::new(VType::Txt)),
                    "str.to_num" => VType::Num,
                    _ => VType::Unknown,
                }
            }

            // Array literal
            Expr::Array { elements } => {
                let elem_type = elements.first()
                    .map(|e| self.infer(e))
                    .unwrap_or(VType::Unknown);
                VType::Array(Box::new(elem_type))
            }

            // Map literal
            Expr::Map { .. } => VType::Map,

            // Index access
            Expr::Index { object, .. } => {
                match self.infer(object) {
                    VType::Array(t) => *t,
                    _               => VType::Unknown,
                }
            }

            // Field access
            Expr::Field { object, field } => {
                match (self.infer(object), field.as_str()) {
                    (VType::Array(_), "len") => VType::Num,
                    (VType::Txt,      "len") => VType::Num,
                    (VType::Map,      _)     => VType::Unknown,
                    _                        => VType::Unknown,
                }
            }

            // Method calls on known types
            Expr::MethodCall { object, method, .. } => {
                let obj_type = self.infer(object);
                match (obj_type, method.as_str()) {
                    (VType::Txt, "upper" | "lower" | "trim" | "reverse") => VType::Txt,
                    (VType::Txt, "split") => VType::Array(Box::new(VType::Txt)),
                    (VType::Txt, "contains" | "starts_with" | "ends_with") => VType::Bool,
                    (VType::Txt, "len") => VType::Num,
                    (VType::Array(_), "len") => VType::Num,
                    (VType::Array(t), "first" | "last" | "pop") => *t,
                    (VType::Array(_), "contains") => VType::Bool,
                    _ => VType::Unknown,
                }
            }

            // Block-ending statements
            Expr::Yield(inner) => self.infer(inner),
            Expr::Rise(_)      => VType::Void,
            Expr::Use { .. }   => VType::Void,
            Expr::OpDecl { .. }=> VType::Void,

            // Control flow — we don't infer through these yet
            _ => VType::Unknown,
        }
    }

    /// Type-check a full program, updating the environment as we go.
    /// Returns a map of all top-level names → inferred types.
    pub fn check_program(&mut self, program: &[Expr]) -> TypeEnv {
        for expr in program {
            self.infer(expr);
        }
        // Return the global (outermost) scope's type info
        self.scopes.first().cloned().unwrap_or_default()
    }

    fn infer_binop(&self, op: &BinOp, l: &VType, r: &VType) -> VType {
        match (op, l, r) {
            // Arithmetic on numbers → Num
            (BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod,
             VType::Num, VType::Num) => VType::Num,

            // String concat → Txt
            (BinOp::Add, VType::Txt, VType::Txt) => VType::Txt,

            // Comparisons → Bool
            (BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Gt
            | BinOp::LtEq | BinOp::GtEq, _, _) => VType::Bool,

            _ => VType::Unknown,
        }
    }
}
