/// All possible token types in the Verd language.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // === Literals ===
    Number(f64),        // 42, 3.14
    Text(String),       // "hello"
    Bool(bool),         // true, false

    // === Identifiers ===
    Identifier(String), // variable or op names

    // === Keywords ===
    Pin,                // pin  (immutable variable)
    Flux,               // flux (mutable variable)
    Op,                 // op   (function/operation)
    Cycle,              // cycle (loop)
    Yield,              // yield (return value)
    Rise,               // rise  (throw error)
    Use,                // use   (import module)
    Forge,              // forge (declare module)
    Spawn,              // spawn (start parallel task)
    Sync,               // sync  (wait for task)
    Match,              // match (pattern matching)
    Some,               // some  (optional value present)
    None,               // none  (no value)
    Catch,              // catch (handle a risen error)

    // === Operators ===
    Assign,             // =
    Plus,               // +
    Minus,              // -
    Star,               // *
    Slash,              // /
    Percent,            // %
    Eq,                 // ==
    NotEq,              // !=
    Lt,                 // <
    Gt,                 // >
    LtEq,               // <=
    GtEq,               // >=
    Bang,               // !  (effect declaration prefix)
    Question,           // ?  (inline conditional)
    Pipe,               // |>  (pipeline operator)
    Arrow,              // ->  (return type annotation)
    ColonColon,         // ::  (type annotation separator)
    Colon,              // :

    // === Delimiters ===
    LParen,             // (
    RParen,             // )
    LBrace,             // {
    RBrace,             // }
    LBracket,           // [
    RBracket,           // ]
    Comma,              // ,
    Dot,                // .  (field access / method call)
    Pipe2,              // |  (lambda param in .each |x| { })

    // === Special ===
    Newline,            // end of logical line
    Eof,                // end of file
}

/// A token with its location in the source file.
/// This is what the Lexer produces and the Parser consumes.
#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub line: usize,
    pub col: usize,
}
