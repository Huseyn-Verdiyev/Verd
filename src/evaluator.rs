use std::collections::HashMap;
use crate::ast::{BinOp, Expr};

/// A runtime value in Verd.
#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    Text(String),
    Bool(bool),
    Some(Box<Value>),
    None,
    Void,

    // A declared op stored in the environment
    Op {
        params:  Vec<String>,
        effects: Vec<String>,
        body:    Vec<Expr>,
    },
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(n) => {
                if n.fract() == 0.0 { write!(f, "{}", *n as i64) }
                else                { write!(f, "{}", n) }
            }
            Value::Text(s)   => write!(f, "{}", s),
            Value::Bool(b)   => write!(f, "{}", b),
            Value::Some(v)   => write!(f, "some({})", v),
            Value::None      => write!(f, "none"),
            Value::Void      => Ok(()),
            Value::Op { .. } => write!(f, "<op>"),
        }
    }
}

/// A risen error (Verd's exception mechanism).
#[derive(Debug)]
pub struct Rise(pub String);

/// Signal types used for control flow inside blocks.
#[derive(Debug)]
enum Signal {
    Value(Value),
    Yield(Value),
    Rise(String),
}

/// The runtime environment: a stack of scopes.
/// Each scope is a HashMap of name → value.
pub struct Env {
    scopes: Vec<HashMap<String, Value>>,

    /// Which flux variables the CURRENT op is allowed to touch externally.
    /// Empty means "pure" — cannot read or write any outer flux.
    allowed_effects: Vec<String>,
}

impl Env {
    pub fn new() -> Self {
        Env {
            scopes: vec![HashMap::new()],
            allowed_effects: Vec::new(),
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    /// Declare a new variable in the current (innermost) scope.
    fn declare(&mut self, name: &str, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), value);
        }
    }

    /// Look up a variable, walking from inner to outer scopes.
    fn get(&self, name: &str) -> Option<&Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Some(v);
            }
        }
        None
    }

    /// Assign to an existing variable. Searches outward through scopes.
    /// Returns Err if the variable is not found or is an immutable pin.
    fn assign(&mut self, name: &str, value: Value) -> Result<(), String> {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), value);
                return Ok(());
            }
        }
        Err(format!("[VERD ERROR] '{}' is not defined.", name))
    }
}

/// The Evaluator walks the AST and produces runtime Values.
pub struct Evaluator {
    pub env: Env,
}

impl Evaluator {
    pub fn new() -> Self {
        Evaluator { env: Env::new() }
    }

    /// Evaluate a full program (list of top-level expressions).
    pub fn run(&mut self, program: Vec<Expr>) -> Result<(), String> {
        for expr in program {
            match self.eval(&expr)? {
                Signal::Rise(msg) => {
                    eprintln!("[VERD RISE] Unhandled error: {}", msg);
                    return Ok(());
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn eval(&mut self, expr: &Expr) -> Result<Signal, String> {
        match expr {
            // ── Literals ──────────────────────────────────────────────────
            Expr::Number(n) => Ok(Signal::Value(Value::Number(*n))),
            Expr::Text(s)   => Ok(Signal::Value(Value::Text(s.clone()))),
            Expr::Bool(b)   => Ok(Signal::Value(Value::Bool(*b))),
            Expr::None      => Ok(Signal::Value(Value::None)),

            // ── Variable access ──────────────────────────────────────────
            Expr::Identifier(name) => {
                match self.env.get(name) {
                    Some(v) => Ok(Signal::Value(v.clone())),
                    None    => Err(format!("[VERD ERROR] '{}' is not defined.", name)),
                }
            }

            // ── Declarations ─────────────────────────────────────────────
            Expr::Pin { name, value } | Expr::Flux { name, value } => {
                let v = self.eval_value(value)?;
                self.env.declare(name, v);
                Ok(Signal::Value(Value::Void))
            }

            // ── Assignment ───────────────────────────────────────────────
            Expr::Assign { name, value } => {
                let v = self.eval_value(value)?;
                self.env.assign(name, v)?;
                Ok(Signal::Value(Value::Void))
            }

            // ── Op declaration ───────────────────────────────────────────
            Expr::OpDecl { name, params, effects, body } => {
                self.env.declare(name, Value::Op {
                    params:  params.clone(),
                    effects: effects.clone(),
                    body:    body.clone(),
                });
                Ok(Signal::Value(Value::Void))
            }

            // ── Op call ──────────────────────────────────────────────────
            Expr::Call { name, args } => {
                // Built-in: print
                if name == "print" {
                    let parts: Result<Vec<_>, _> = args.iter()
                        .map(|a| self.eval_value(a))
                        .collect();
                    let output = parts?.iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(" ");
                    println!("{}", output);
                    return Ok(Signal::Value(Value::Void));
                }

                // Built-in: some(x)
                if name == "some" && args.len() == 1 {
                    let inner = self.eval_value(&args[0])?;
                    return Ok(Signal::Value(Value::Some(Box::new(inner))));
                }

                // User-defined op
                let op = match self.env.get(name).cloned() {
                    Some(Value::Op { params, effects, body }) => (params, effects, body),
                    Some(_) => return Err(format!("[VERD ERROR] '{}' is not an op.", name)),
                    None    => return Err(format!("[VERD ERROR] '{}' is not defined.", name)),
                };

                let (params, effects, body) = op;

                // Evaluate arguments in the CALLER's scope
                let arg_vals: Result<Vec<_>, _> = args.iter()
                    .map(|a| self.eval_value(a))
                    .collect();
                let arg_vals = arg_vals?;

                // Set up a new scope for the op's body
                self.env.push_scope();
                self.env.allowed_effects = effects;

                for (param, val) in params.iter().zip(arg_vals.into_iter()) {
                    self.env.declare(param, val);
                }

                let result = self.eval_block(&body);

                self.env.allowed_effects = Vec::new();
                self.env.pop_scope();

                match result? {
                    Signal::Yield(v) => Ok(Signal::Value(v)),
                    Signal::Rise(e)  => Ok(Signal::Rise(e)),
                    other            => Ok(other),
                }
            }

            // ── yield ────────────────────────────────────────────────────
            Expr::Yield(inner) => {
                let v = self.eval_value(inner)?;
                Ok(Signal::Yield(v))
            }

            // ── rise ─────────────────────────────────────────────────────
            Expr::Rise(inner) => {
                let v = self.eval_value(inner)?;
                Ok(Signal::Rise(v.to_string()))
            }

            // ── cycle ────────────────────────────────────────────────────
            Expr::Cycle { condition, body } => {
                loop {
                    let cond = self.eval_value(condition)?;
                    match cond {
                        Value::Bool(true) => {}
                        Value::Bool(false) => break,
                        Value::Number(n) if n != 0.0 => {}
                        _ => break,
                    }

                    match self.eval_block(body)? {
                        Signal::Yield(v) => return Ok(Signal::Yield(v)),
                        Signal::Rise(e)  => return Ok(Signal::Rise(e)),
                        _ => {}
                    }
                }
                Ok(Signal::Value(Value::Void))
            }

            // ── match ────────────────────────────────────────────────────
            Expr::Match { subject, some_branch, none_branch } => {
                let val = self.eval_value(subject)?;

                match val {
                    Value::Some(inner) => {
                        if let Some((bound, body)) = some_branch {
                            self.env.push_scope();
                            self.env.declare(bound, *inner);
                            let result = self.eval_block(body);
                            self.env.pop_scope();
                            result
                        } else {
                            Ok(Signal::Value(Value::Void))
                        }
                    }
                    Value::None => {
                        if let Some(body) = none_branch {
                            self.eval_block(body)
                        } else {
                            Ok(Signal::Value(Value::Void))
                        }
                    }
                    other => Err(format!(
                        "[VERD ERROR] 'match' expects some/none, got {:?}", other
                    )),
                }
            }

            // ── pipeline ─────────────────────────────────────────────────
            Expr::Pipeline { stages } => {
                // Each stage is either an Identifier (op name) or a Call.
                // The result of the previous stage is prepended to the args.
                let mut current = self.eval_value(&stages[0])?;

                for stage in &stages[1..] {
                    let call_name = match stage {
                        Expr::Identifier(name) => name.clone(),
                        Expr::Call { name, .. } => name.clone(),
                        _ => return Err("[VERD ERROR] Pipeline stage must be an op name.".to_string()),
                    };

                    // Built-in print shortcut
                    if call_name == "print" {
                        println!("{}", current);
                        current = Value::Void;
                        continue;
                    }

                    // Regular op call — inject current as first argument
                    let op = match self.env.get(&call_name).cloned() {
                        Some(Value::Op { params, effects, body }) => (params, effects, body),
                        _ => return Err(format!("[VERD ERROR] '{}' is not an op.", call_name)),
                    };
                    let (params, effects, body) = op;

                    self.env.push_scope();
                    self.env.allowed_effects = effects;
                    if let Some(first_param) = params.first() {
                        self.env.declare(first_param, current);
                    }
                    let result = self.eval_block(&body);
                    self.env.allowed_effects = Vec::new();
                    self.env.pop_scope();

                    current = match result? {
                        Signal::Yield(v) | Signal::Value(v) => v,
                        Signal::Rise(e) => return Ok(Signal::Rise(e)),
                    };
                }
                Ok(Signal::Value(current))
            }

            // ── catch ────────────────────────────────────────────────────
            Expr::Catch { body, err_var, handler } => {
                match self.eval_block(body)? {
                    Signal::Rise(msg) => {
                        self.env.push_scope();
                        self.env.declare(err_var, Value::Text(msg));
                        let result = self.eval_block(handler);
                        self.env.pop_scope();
                        result
                    }
                    other => Ok(other),
                }
            }

            // ── inline conditional (?): condition ? { body } ─────────────
            Expr::Question { condition, body } => {
                let cond = self.eval_value(condition)?;
                let is_truthy = match &cond {
                    Value::Bool(false) | Value::None | Value::Void => false,
                    Value::Number(n) if *n == 0.0 => false,
                    _ => true,
                };
                if is_truthy {
                    self.eval_block(body)
                } else {
                    Ok(Signal::Value(Value::Void))
                }
            }

            // ── Binary operations ────────────────────────────────────────
            Expr::BinaryOp { op, left, right } => {
                let l = self.eval_value(left)?;
                let r = self.eval_value(right)?;
                let result = self.apply_binop(op, l, r)?;
                Ok(Signal::Value(result))
            }

            Expr::Spawn { call, handle } => {
                // For now, spawn just evaluates immediately (no true threading yet)
                let val = self.eval_value(call)?;
                self.env.declare(handle, val);
                Ok(Signal::Value(Value::Void))
            }

            Expr::Sync { handle } => {
                // Just reads the handle variable
                match self.env.get(handle).cloned() {
                    Some(v) => Ok(Signal::Value(v)),
                    None    => Err(format!("[VERD ERROR] spawn handle '{}' not found.", handle)),
                }
            }
        }
    }

    fn eval_value(&mut self, expr: &Expr) -> Result<Value, String> {
        match self.eval(expr)? {
            Signal::Value(v) | Signal::Yield(v) => Ok(v),
            Signal::Rise(e) => Err(format!("[VERD RISE] {}", e)),
        }
    }

    fn eval_block(&mut self, stmts: &[Expr]) -> Result<Signal, String> {
        let mut last = Signal::Value(Value::Void);
        for stmt in stmts {
            last = self.eval(stmt)?;
            if matches!(last, Signal::Yield(_) | Signal::Rise(_)) {
                return Ok(last);
            }
        }
        Ok(last)
    }

    fn apply_binop(&self, op: &BinOp, l: Value, r: Value) -> Result<Value, String> {
        match (op, l, r) {
            // Arithmetic
            (BinOp::Add, Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
            (BinOp::Sub, Value::Number(a), Value::Number(b)) => Ok(Value::Number(a - b)),
            (BinOp::Mul, Value::Number(a), Value::Number(b)) => Ok(Value::Number(a * b)),
            (BinOp::Div, Value::Number(a), Value::Number(b)) => {
                if b == 0.0 { Err("[VERD ERROR] Division by zero.".to_string()) }
                else        { Ok(Value::Number(a / b)) }
            }
            (BinOp::Mod, Value::Number(a), Value::Number(b)) => Ok(Value::Number(a % b)),

            // String concatenation
            (BinOp::Add, Value::Text(a), Value::Text(b)) => Ok(Value::Text(a + &b)),

            // Comparisons
            (BinOp::Eq,    Value::Number(a), Value::Number(b)) => Ok(Value::Bool(a == b)),
            (BinOp::NotEq, Value::Number(a), Value::Number(b)) => Ok(Value::Bool(a != b)),
            (BinOp::Lt,    Value::Number(a), Value::Number(b)) => Ok(Value::Bool(a < b)),
            (BinOp::Gt,    Value::Number(a), Value::Number(b)) => Ok(Value::Bool(a > b)),
            (BinOp::LtEq,  Value::Number(a), Value::Number(b)) => Ok(Value::Bool(a <= b)),
            (BinOp::GtEq,  Value::Number(a), Value::Number(b)) => Ok(Value::Bool(a >= b)),

            (BinOp::Eq,    Value::Text(a), Value::Text(b)) => Ok(Value::Bool(a == b)),
            (BinOp::NotEq, Value::Text(a), Value::Text(b)) => Ok(Value::Bool(a != b)),

            (BinOp::Eq,    Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a == b)),

            (op, l, r) => Err(format!(
                "[VERD ERROR] Cannot apply {:?} to {:?} and {:?}", op, l, r
            )),
        }
    }
}
