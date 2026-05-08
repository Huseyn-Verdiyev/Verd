mod token;
mod lexer;
mod ast;
mod parser;
mod evaluator;
mod c_generator;
mod rust_generator;

use lexer::Lexer;
use parser::Parser;
use evaluator::Evaluator;
use c_generator::CGenerator;
use rust_generator::RustGenerator;
use std::io::{self, Write};
use std::process::Command;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        // verd run <file.verd>   — interpret (fast dev cycle)
        Some("run") => {
            let path = get_path(&args, 2);
            let source = read_file(path);
            interpret_source(&source);
        }

        // verd compile <file.verd> [output]  — compile to native binary via Rust
        Some("compile") => {
            let path = get_path(&args, 2);
            let source = read_file(path);
            let default_out = path.replace(".verd", "");
            let out_name = args.get(3).map(|s| s.as_str()).unwrap_or(&default_out);
            compile_source(&source, path, out_name);
        }

        // verd emit-rust <file.verd>  — print generated Rust code
        Some("emit-rust") => {
            let path = get_path(&args, 2);
            let source = read_file(path);
            println!("{}", generate_rust(&source));
        }

        // verd emit-c <file.verd>  — print generated C code
        Some("emit-c") => {
            let path = get_path(&args, 2);
            let source = read_file(path);
            println!("{}", generate_c(&source));
        }

        // verd (no args) → REPL
        _ => {
            repl();
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn get_path<'a>(args: &'a [String], idx: usize) -> &'a str {
    args.get(idx).unwrap_or_else(|| {
        eprintln!("[VERD ERROR] No file specified.");
        std::process::exit(1);
    })
}

fn read_file(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|_| {
        eprintln!("[VERD ERROR] Could not read file: {}", path);
        std::process::exit(1);
    })
}

fn parse_source(source: &str) -> Vec<ast::Expr> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenise();
    let mut parser = Parser::new(tokens);
    parser.parse().unwrap_or_else(|e| {
        eprintln!("{}", e);
        std::process::exit(1);
    })
}

fn generate_c(source: &str) -> String {
    let ast = parse_source(source);
    let mut codegen = CGenerator::new();
    codegen.generate(&ast)
}

fn generate_rust(source: &str) -> String {
    let ast = parse_source(source);
    let mut codegen = RustGenerator::new();
    codegen.generate(&ast)
}

// ── Interpret (Tree-Walking) ──────────────────────────────────────────────────

fn interpret_source(source: &str) {
    let ast = parse_source(source);
    let mut ev = Evaluator::new();
    if let Err(e) = ev.run(ast) {
        eprintln!("{}", e);
    }
}

// ── Compile (Verd → Rust → Native Binary) ────────────────────────────────────

fn compile_source(source: &str, input_path: &str, out_name: &str) {
    let rust_code = generate_rust(source);

    // Write generated Rust to a temp file
    let rs_path = format!("{}.rs", input_path);
    std::fs::write(&rs_path, &rust_code).unwrap_or_else(|_| {
        eprintln!("[VERD ERROR] Could not write generated Rust file.");
        std::process::exit(1);
    });

    println!("[verd] Transpiled → {}", rs_path);

    // Compile with rustc (always available since we're built with Rust!)
    let status = Command::new("rustc")
        .args([
            &rs_path,
            "--crate-name", "verd_out",
            "-o", out_name,
            "--edition", "2021",
            "-C", "opt-level=3",   // Maximum optimization — Rust speed!
            "-C", "debuginfo=0",
        ])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("[verd] Compiled  → {} (rustc, -O3)", out_name);
            println!("[verd] Done! Run with: ./{}", out_name);
        }
        Ok(_) => {
            eprintln!("[VERD ERROR] Compilation failed. Generated Rust is at: {}", rs_path);
        }
        Err(e) => {
            eprintln!("[VERD ERROR] Could not run rustc: {}", e);
        }
    }
}

// ── REPL ─────────────────────────────────────────────────────────────────────

fn repl() {
    println!("┌──────────────────────────────────────────────┐");
    println!("│  Verd 0.1.0  — Interactive REPL               │");
    println!("│  'emit-rust <expr>'  to see generated Rust    │");
    println!("│  'exit' to quit                               │");
    println!("└──────────────────────────────────────────────┘");
    println!();

    let mut ev = Evaluator::new();

    loop {
        print!("verd> ");
        io::stdout().flush().unwrap();

        let mut line = String::new();
        if io::stdin().read_line(&mut line).is_err() { break; }

        let trimmed = line.trim();
        if trimmed == "exit" || trimmed == "quit" { break; }
        if trimmed.is_empty() { continue; }

        if let Some(rest) = trimmed.strip_prefix("emit-rust ") {
            println!("{}", generate_rust(rest));
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("emit-c ") {
            println!("{}", generate_c(rest));
            continue;
        }

        let mut lexer = Lexer::new(trimmed);
        let tokens = lexer.tokenise();
        let mut parser = Parser::new(tokens);
        let ast = match parser.parse() {
            Ok(n) => n,
            Err(e) => { eprintln!("{}", e); continue; }
        };
        if let Err(e) = ev.run(ast) {
            eprintln!("{}", e);
        }
    }

    println!("Goodbye!");
}
