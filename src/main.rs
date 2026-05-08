mod token;
mod lexer;
mod ast;
mod parser;
mod evaluator;

use lexer::Lexer;
use parser::Parser;
use evaluator::Evaluator;
use std::io::{self, Write};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        // verd run <file.verd>
        Some("run") => {
            let path = args.get(2).unwrap_or_else(|| {
                eprintln!("[VERD ERROR] No file specified. Usage: verd run <file.verd>");
                std::process::exit(1);
            });

            let source = std::fs::read_to_string(path).unwrap_or_else(|_| {
                eprintln!("[VERD ERROR] Could not read file: {}", path);
                std::process::exit(1);
            });

            run_source(&source);
        }

        // verd (no args) → REPL
        _ => {
            repl();
        }
    }
}

fn run_source(source: &str) {
    // 1. Lex
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenise();

    // 2. Parse
    let mut parser = Parser::new(tokens);
    let ast = match parser.parse() {
        Ok(nodes) => nodes,
        Err(e) => { eprintln!("{}", e); return; }
    };

    // 3. Evaluate
    let mut evaluator = Evaluator::new();
    if let Err(e) = evaluator.run(ast) {
        eprintln!("{}", e);
    }
}

fn repl() {
    println!("┌─────────────────────────────────┐");
    println!("│  Verd 0.1.0  — Interactive REPL  │");
    println!("│  Type 'exit' to quit.             │");
    println!("└─────────────────────────────────┘");
    println!();

    let mut evaluator = Evaluator::new();

    loop {
        print!("verd> ");
        io::stdout().flush().unwrap();

        let mut line = String::new();
        if io::stdin().read_line(&mut line).is_err() { break; }

        let trimmed = line.trim();
        if trimmed == "exit" || trimmed == "quit" { break; }
        if trimmed.is_empty() { continue; }

        // Lex → Parse → Eval each line, sharing the same evaluator
        // so variables persist across REPL entries.
        let mut lexer = Lexer::new(trimmed);
        let tokens = lexer.tokenise();

        let mut parser = Parser::new(tokens);
        let ast = match parser.parse() {
            Ok(nodes) => nodes,
            Err(e) => { eprintln!("{}", e); continue; }
        };

        if let Err(e) = evaluator.run(ast) {
            eprintln!("{}", e);
        }
    }

    println!("Goodbye!");
}
