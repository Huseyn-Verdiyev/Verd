use std::collections::HashSet;
use std::path::Path;
use crate::ast::Expr;
use crate::lexer::Lexer;
use crate::parser::Parser;

/// The Resolver processes `use` statements before evaluation/compilation.
/// It handles two forms:
///   use "./other.verd"   — inline another Verd file
///   use std.fs           — load a built-in standard library module
pub struct Resolver {
    /// Tracks which files have already been loaded (prevents circular imports)
    loaded: HashSet<String>,
    /// Base directory for resolving relative paths
    base_dir: String,
}

impl Resolver {
    pub fn new(base_dir: &str) -> Self {
        Resolver {
            loaded: HashSet::new(),
            base_dir: base_dir.to_string(),
        }
    }

    /// Walk a parsed program, expand all `use` nodes, return a flat merged AST.
    pub fn resolve(&mut self, program: Vec<Expr>) -> Result<Vec<Expr>, String> {
        let mut output: Vec<Expr> = Vec::new();

        for expr in program {
            match &expr {
                Expr::Use { path } => {
                    let path = path.clone();
                    let expanded = self.resolve_use(&path)?;
                    output.extend(expanded);
                }
                _ => output.push(expr),
            }
        }

        Ok(output)
    }

    fn resolve_use(&mut self, path: &str) -> Result<Vec<Expr>, String> {
        // ── Standard library modules ─────────────────────────────────────────
        if path.starts_with("std.") || path == "std" {
            return Ok(self.load_stdlib(path));
        }

        // ── File import ───────────────────────────────────────────────────────
        let full_path = if path.starts_with("./") || path.starts_with("../") {
            format!("{}/{}", self.base_dir, &path[2..])
        } else {
            format!("{}/{}", self.base_dir, path)
        };

        // Normalize path
        let canonical = full_path.replace("\\", "/");

        // Guard: don't load the same file twice
        if self.loaded.contains(&canonical) {
            return Ok(vec![]);
        }
        self.loaded.insert(canonical.clone());

        // Read source
        let source = std::fs::read_to_string(&canonical)
            .map_err(|_| format!("[VERD ERROR] Cannot open imported file: {}", canonical))?;

        // Lex + parse
        let mut lexer = Lexer::new(&source);
        let tokens = lexer.tokenise();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse()?;

        // Recursively resolve any `use` inside the imported file
        self.resolve(ast)
    }

    /// Return synthetic AST nodes that declare std library ops.
    /// This injects built-in functions directly as OpDecl nodes
    /// so the evaluator and compiler both see them.
    fn load_stdlib(&self, module: &str) -> Vec<Expr> {
        // We return an empty Vec here — stdlib ops are handled as
        // built-ins inside the evaluator (eval_builtin_method) and
        // rust_generator (emit_stdlib_prelude). The `use std.*`
        // statement just sets a flag that the module is active.
        // This is a marker node that the evaluator checks.
        vec![Expr::Use { path: format!("__loaded__{}", module) }]
    }
}
