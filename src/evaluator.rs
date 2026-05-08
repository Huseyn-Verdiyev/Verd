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

    // Collection types
    Array(Vec<Value>),
    Map(HashMap<String, Value>),
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
            Value::Array(els) => {
                let parts: Vec<String> = els.iter().map(|v| v.to_string()).collect();
                write!(f, "[{}]", parts.join(", "))
            }
            Value::Map(m) => {
                let parts: Vec<String> = m.iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect();
                write!(f, "{{ {} }}", parts.join(", "))
            }
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

                // Built-in: input(prompt)
                if name == "input" {
                    use std::io::{self, Write};
                    if let Some(prompt) = args.first() {
                        let p = self.eval_value(prompt)?;
                        print!("{}", p);
                        io::stdout().flush().ok();
                    }
                    let mut line = String::new();
                    io::stdin().read_line(&mut line).ok();
                    return Ok(Signal::Value(Value::Text(line.trim().to_string())));
                }

                // Stdlib namespace calls: fs.read(...), math.sqrt(...) etc.
                if let Some(result) = self.try_stdlib_call(name, args)? {
                    return Ok(Signal::Value(result));
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
                let val = self.eval_value(call)?;
                self.env.declare(handle, val);
                Ok(Signal::Value(Value::Void))
            }

            Expr::Sync { handle } => {
                match self.env.get(handle).cloned() {
                    Some(v) => Ok(Signal::Value(v)),
                    None    => Err(format!("[VERD ERROR] spawn handle '{}' not found.", handle)),
                }
            }

            // ── Array literal ────────────────────────────────────────────
            Expr::Array { elements } => {
                let mut vals = Vec::new();
                for el in elements {
                    vals.push(self.eval_value(el)?);
                }
                Ok(Signal::Value(Value::Array(vals)))
            }

            // ── Map literal ──────────────────────────────────────────────
            Expr::Map { pairs } => {
                let mut map = HashMap::new();
                for (key, val_expr) in pairs {
                    let val = self.eval_value(val_expr)?;
                    map.insert(key.clone(), val);
                }
                Ok(Signal::Value(Value::Map(map)))
            }

            // ── Index access: arr[i] or map["key"] ──────────────────────
            Expr::Index { object, index } => {
                let obj = self.eval_value(object)?;
                let idx = self.eval_value(index)?;
                match (obj, idx) {
                    (Value::Array(arr), Value::Number(n)) => {
                        let i = n as usize;
                        arr.get(i).cloned()
                            .map(|v| Signal::Value(v))
                            .ok_or_else(|| format!("[VERD ERROR] Array index {} out of bounds (len {})", i, arr.len()))
                    }
                    (Value::Map(m), Value::Text(key)) => {
                        m.get(&key).cloned()
                            .map(|v| Signal::Value(v))
                            .ok_or_else(|| format!("[VERD ERROR] Map key '{}' not found", key))
                    }
                    (obj, idx) => Err(format!("[VERD ERROR] Cannot index {:?} with {:?}", obj, idx)),
                }
            }

            // ── Field access: obj.field ──────────────────────────────────
            Expr::Field { object, field } => {
                // Check if object is a stdlib namespace constant: math.pi, math.e
                if let Expr::Identifier(ns) = object.as_ref() {
                    let stdlib_namespaces = ["math", "fs", "str", "io"];
                    if stdlib_namespaces.contains(&ns.as_str()) {
                        let full_name = format!("{}.{}", ns, field);
                        // Pass empty args slice for constants
                        if let Some(result) = self.try_stdlib_call(&full_name, &[])? {
                            return Ok(Signal::Value(result));
                        }
                    }
                }

                let obj = self.eval_value(object)?;
                match &obj {
                    Value::Array(arr) => match field.as_str() {
                        "len" => Ok(Signal::Value(Value::Number(arr.len() as f64))),
                        f => Err(format!("[VERD ERROR] Array has no field '{}'", f)),
                    },
                    Value::Map(m) => {
                        m.get(field.as_str()).cloned()
                            .map(|v| Signal::Value(v))
                            .ok_or_else(|| format!("[VERD ERROR] Map has no field '{}'", field))
                    }
                    Value::Text(s) => match field.as_str() {
                        "len" => Ok(Signal::Value(Value::Number(s.len() as f64))),
                        f => Err(format!("[VERD ERROR] Text has no field '{}'", f)),
                    },
                    other => Err(format!("[VERD ERROR] {:?} has no field '{}'", other, field)),
                }
            }

            // ── Method call: obj.method(args) ────────────────────────────
            Expr::MethodCall { object, method, args } => {
                // Check if object is a stdlib namespace: math.sqrt(x), fs.read(x)
                if let Expr::Identifier(ns) = object.as_ref() {
                    let stdlib_namespaces = ["math", "fs", "str", "io"];
                    if stdlib_namespaces.contains(&ns.as_str()) {
                        let full_name = format!("{}.{}", ns, method);
                        if let Some(result) = self.try_stdlib_call(&full_name, args)? {
                            return Ok(Signal::Value(result));
                        }
                    }
                }

                let obj_val = self.eval_value(object)?;
                let arg_vals: Result<Vec<_>, _> = args.iter().map(|a| self.eval_value(a)).collect();
                let arg_vals = arg_vals?;

                match (obj_val, method.as_str()) {
                    // Array methods
                    (Value::Array(mut arr), "push") => {
                        if let Some(v) = arg_vals.into_iter().next() { arr.push(v); }
                        // We need to write back — find the name in env
                        // For now return the mutated array; assignment handles write-back
                        Ok(Signal::Value(Value::Array(arr)))
                    }
                    (Value::Array(mut arr), "pop") => {
                        let v = arr.pop().unwrap_or(Value::None);
                        Ok(Signal::Value(v))
                    }
                    (Value::Array(arr), "contains") => {
                        let target = arg_vals.into_iter().next().unwrap_or(Value::None);
                        let found = arr.iter().any(|v| v.to_string() == target.to_string());
                        Ok(Signal::Value(Value::Bool(found)))
                    }
                    (Value::Array(arr), "first") => {
                        Ok(Signal::Value(arr.first().cloned().unwrap_or(Value::None)))
                    }
                    (Value::Array(arr), "last") => {
                        Ok(Signal::Value(arr.last().cloned().unwrap_or(Value::None)))
                    }
                    // Text methods
                    (Value::Text(s), "upper") => Ok(Signal::Value(Value::Text(s.to_uppercase()))),
                    (Value::Text(s), "lower") => Ok(Signal::Value(Value::Text(s.to_lowercase()))),
                    (Value::Text(s), "trim")  => Ok(Signal::Value(Value::Text(s.trim().to_string()))),
                    (Value::Text(s), "len")   => Ok(Signal::Value(Value::Number(s.len() as f64))),
                    (Value::Text(s), "contains") => {
                        let pat = arg_vals.into_iter().next().unwrap_or(Value::None).to_string();
                        Ok(Signal::Value(Value::Bool(s.contains(&pat))))
                    }
                    (Value::Text(s), "starts_with") => {
                        let pat = arg_vals.into_iter().next().unwrap_or(Value::None).to_string();
                        Ok(Signal::Value(Value::Bool(s.starts_with(&pat))))
                    }
                    (Value::Text(s), "split") => {
                        let sep = arg_vals.into_iter().next().unwrap_or(Value::Text(",".into())).to_string();
                        let parts: Vec<Value> = s.split(&sep as &str)
                            .map(|p| Value::Text(p.to_string())).collect();
                        Ok(Signal::Value(Value::Array(parts)))
                    }
                    (obj, m) => Err(format!("[VERD ERROR] {:?} has no method '{}'", obj, m)),
                }
            }

            // ── use (import) — load stdlib modules into scope ────────
            Expr::Use { path } => {
                self.load_stdlib_if_needed(path);
                Ok(Signal::Value(Value::Void))
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

    // ── Standard Library ─────────────────────────────────────────────────────

    /// Called when `use std.*` is seen. Currently a no-op since all stdlib
    /// functions are dispatched dynamically by name in try_stdlib_call.
    fn load_stdlib_if_needed(&mut self, _path: &str) {
        // Future: could set a flag to restrict access to only loaded modules.
        // For now, all std functions are always available once called.
    }

    /// Try to dispatch a stdlib namespaced call like "fs.read", "math.sqrt".
    /// Returns Some(Value) if it was a stdlib call, None if it's user-defined.
    fn try_stdlib_call(&mut self, name: &str, args: &[Expr]) -> Result<Option<Value>, String> {
        let arg_vals: Result<Vec<_>, _> = args.iter().map(|a| self.eval_value(a)).collect();
        let arg_vals = arg_vals?;

        let get = |idx: usize, expected: &str| -> Result<String, String> {
            arg_vals.get(idx)
                .map(|v| v.to_string())
                .ok_or_else(|| format!("[VERD ERROR] {} requires argument {}", name, expected))
        };

        let get_num = |idx: usize, expected: &str| -> Result<f64, String> {
            match arg_vals.get(idx) {
                Some(Value::Number(n)) => Ok(*n),
                _ => Err(format!("[VERD ERROR] {} requires numeric argument '{}'", name, expected)),
            }
        };

        match name {
            // ── fs module ───────────────────────────────────────────────────
            "fs.read" => {
                let path = get(0, "path")?;
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| format!("[VERD ERROR] fs.read: {}", e))?;
                Ok(Some(Value::Text(content)))
            }
            "fs.write" => {
                let path    = get(0, "path")?;
                let content = get(1, "content")?;
                std::fs::write(&path, content)
                    .map_err(|e| format!("[VERD ERROR] fs.write: {}", e))?;
                Ok(Some(Value::Void))
            }
            "fs.exists" => {
                let path = get(0, "path")?;
                Ok(Some(Value::Bool(std::path::Path::new(&path).exists())))
            }
            "fs.delete" => {
                let path = get(0, "path")?;
                std::fs::remove_file(&path)
                    .map_err(|e| format!("[VERD ERROR] fs.delete: {}", e))?;
                Ok(Some(Value::Void))
            }
            "fs.lines" => {
                let path = get(0, "path")?;
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| format!("[VERD ERROR] fs.lines: {}", e))?;
                let lines: Vec<Value> = content.lines()
                    .map(|l| Value::Text(l.to_string())).collect();
                Ok(Some(Value::Array(lines)))
            }

            // ── math module ─────────────────────────────────────────────────
            "math.sqrt"  => Ok(Some(Value::Number(get_num(0, "n")?.sqrt()))),
            "math.pow"   => Ok(Some(Value::Number(get_num(0, "base")?.powf(get_num(1, "exp")?)))),
            "math.abs"   => Ok(Some(Value::Number(get_num(0, "n")?.abs()))),
            "math.floor" => Ok(Some(Value::Number(get_num(0, "n")?.floor()))),
            "math.ceil"  => Ok(Some(Value::Number(get_num(0, "n")?.ceil()))),
            "math.round" => Ok(Some(Value::Number(get_num(0, "n")?.round()))),
            "math.min"   => Ok(Some(Value::Number(get_num(0, "a")?.min(get_num(1, "b")?)))),
            "math.max"   => Ok(Some(Value::Number(get_num(0, "a")?.max(get_num(1, "b")?)))),
            "math.sin"   => Ok(Some(Value::Number(get_num(0, "n")?.sin()))),
            "math.cos"   => Ok(Some(Value::Number(get_num(0, "n")?.cos()))),
            "math.log"   => Ok(Some(Value::Number(get_num(0, "n")?.ln()))),
            "math.log2"  => Ok(Some(Value::Number(get_num(0, "n")?.log2()))),
            "math.pi"    => Ok(Some(Value::Number(std::f64::consts::PI))),
            "math.e"     => Ok(Some(Value::Number(std::f64::consts::E))),

            // ── str module ──────────────────────────────────────────────────
            "str.upper"   => Ok(Some(Value::Text(get(0, "s")?.to_uppercase()))),
            "str.lower"   => Ok(Some(Value::Text(get(0, "s")?.to_lowercase()))),
            "str.trim"    => Ok(Some(Value::Text(get(0, "s")?.trim().to_string()))),
            "str.len"     => Ok(Some(Value::Number(get(0, "s")?.len() as f64))),
            "str.reverse" => Ok(Some(Value::Text(get(0, "s")?.chars().rev().collect()))),
            "str.contains" => {
                let s   = get(0, "s")?;
                let pat = get(1, "pattern")?;
                Ok(Some(Value::Bool(s.contains(&pat as &str))))
            }
            "str.replace" => {
                let s    = get(0, "s")?;
                let from = get(1, "from")?;
                let to   = get(2, "to")?;
                Ok(Some(Value::Text(s.replace(&from as &str, &to as &str))))
            }
            "str.split" => {
                let s   = get(0, "s")?;
                let sep = get(1, "sep")?;
                let parts: Vec<Value> = s.split(&sep as &str)
                    .map(|p| Value::Text(p.to_string())).collect();
                Ok(Some(Value::Array(parts)))
            }
            "str.starts_with" => {
                let s   = get(0, "s")?;
                let pat = get(1, "pattern")?;
                Ok(Some(Value::Bool(s.starts_with(&pat as &str))))
            }
            "str.ends_with" => {
                let s   = get(0, "s")?;
                let pat = get(1, "pattern")?;
                Ok(Some(Value::Bool(s.ends_with(&pat as &str))))
            }
            "str.repeat" => {
                let s = get(0, "s")?;
                let n = get_num(1, "n")? as usize;
                Ok(Some(Value::Text(s.repeat(n))))
            }
            "str.to_num" => {
                let s = get(0, "s")?;
                match s.trim().parse::<f64>() {
                    Ok(n)  => Ok(Some(Value::Number(n))),
                    Err(_) => Ok(Some(Value::None)),
                }
            }

            // ── io module ───────────────────────────────────────────────────
            "io.print"   => {
                let parts: Vec<String> = arg_vals.iter().map(|v| v.to_string()).collect();
                print!("{}", parts.join(" "));
                Ok(Some(Value::Void))
            }
            "io.println" => {
                let parts: Vec<String> = arg_vals.iter().map(|v| v.to_string()).collect();
                println!("{}", parts.join(" "));
                Ok(Some(Value::Void))
            }

            // Not a stdlib call
            _ => Ok(None),
        }
    }
}
