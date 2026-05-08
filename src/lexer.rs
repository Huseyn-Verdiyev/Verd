use crate::token::{SpannedToken, Token};

/// The Lexer reads raw Verd source code character by character
/// and produces a flat list of SpannedTokens for the Parser to consume.
pub struct Lexer {
    source: Vec<char>,
    pos: usize,     // current character index
    line: usize,    // current line number (1-indexed)
    col: usize,     // current column number (1-indexed)
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    // ── Internal helpers ────────────────────────────────────────────────────

    fn current(&self) -> Option<char> {
        self.source.get(self.pos).copied()
    }

    fn peek(&self) -> Option<char> {
        self.source.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.current();
        if let Some(c) = ch {
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        ch
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.current() {
                // Skip spaces and tabs (but NOT newlines — they are tokens)
                Some(' ') | Some('\t') | Some('\r') => { self.advance(); }

                // Single-line comment: // ...
                Some('/') if self.peek() == Some('/') => {
                    while self.current().map(|c| c != '\n').unwrap_or(false) {
                        self.advance();
                    }
                }

                _ => break,
            }
        }
    }

    fn make_token(&self, token: Token, line: usize, col: usize) -> SpannedToken {
        SpannedToken { token, line, col }
    }

    // ── Scanning helpers ────────────────────────────────────────────────────

    fn scan_number(&mut self, start_line: usize, start_col: usize) -> SpannedToken {
        let mut s = String::new();
        let mut has_dot = false;

        while let Some(c) = self.current() {
            if c.is_ascii_digit() {
                s.push(c);
                self.advance();
            } else if c == '.' && !has_dot && self.peek().map(|p| p.is_ascii_digit()).unwrap_or(false) {
                has_dot = true;
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }

        let value: f64 = s.parse().unwrap_or(0.0);
        self.make_token(Token::Number(value), start_line, start_col)
    }

    fn scan_text(&mut self, start_line: usize, start_col: usize) -> SpannedToken {
        self.advance(); // consume opening "
        let mut s = String::new();

        loop {
            match self.current() {
                Some('"') => { self.advance(); break; }
                Some('\\') => {
                    self.advance();
                    match self.advance() {
                        Some('n')  => s.push('\n'),
                        Some('t')  => s.push('\t'),
                        Some('"')  => s.push('"'),
                        Some('\\') => s.push('\\'),
                        _ => {}
                    }
                }
                Some(c) => { s.push(c); self.advance(); }
                None => break, // unterminated string — parser will report error
            }
        }

        self.make_token(Token::Text(s), start_line, start_col)
    }

    fn scan_identifier_or_keyword(&mut self, start_line: usize, start_col: usize) -> SpannedToken {
        let mut s = String::new();

        while let Some(c) = self.current() {
            if c.is_alphanumeric() || c == '_' {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }

        let token = match s.as_str() {
            "pin"   => Token::Pin,
            "flux"  => Token::Flux,
            "op"    => Token::Op,
            "cycle" => Token::Cycle,
            "yield" => Token::Yield,
            "rise"  => Token::Rise,
            "use"   => Token::Use,
            "forge" => Token::Forge,
            "spawn" => Token::Spawn,
            "sync"  => Token::Sync,
            "match" => Token::Match,
            "some"  => Token::Some,
            "none"  => Token::None,
            "catch" => Token::Catch,
            "true"  => Token::Bool(true),
            "false" => Token::Bool(false),
            _       => Token::Identifier(s),
        };

        self.make_token(token, start_line, start_col)
    }

    // ── Main tokenise function ───────────────────────────────────────────────

    /// Tokenise the entire source and return a Vec of SpannedTokens.
    /// The last token is always Token::Eof.
    pub fn tokenise(&mut self) -> Vec<SpannedToken> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace_and_comments();

            let line = self.line;
            let col  = self.col;

            let ch = match self.current() {
                Some(c) => c,
                None => {
                    tokens.push(self.make_token(Token::Eof, line, col));
                    break;
                }
            };

            let tok = match ch {
                '\n' => { self.advance(); self.make_token(Token::Newline, line, col) }

                '0'..='9' => self.scan_number(line, col),
                '"'        => self.scan_text(line, col),
                c if c.is_alphabetic() || c == '_' => self.scan_identifier_or_keyword(line, col),

                '(' => { self.advance(); self.make_token(Token::LParen,  line, col) }
                ')' => { self.advance(); self.make_token(Token::RParen,  line, col) }
                '{' => { self.advance(); self.make_token(Token::LBrace,  line, col) }
                '}' => { self.advance(); self.make_token(Token::RBrace,  line, col) }
                ',' => { self.advance(); self.make_token(Token::Comma,   line, col) }
                '+' => { self.advance(); self.make_token(Token::Plus,    line, col) }
                '-' => {
                    self.advance();
                    if self.current() == Some('>') {
                        self.advance();
                        self.make_token(Token::Arrow, line, col)
                    } else {
                        self.make_token(Token::Minus, line, col)
                    }
                }
                '*' => { self.advance(); self.make_token(Token::Star,    line, col) }
                '%' => { self.advance(); self.make_token(Token::Percent, line, col) }
                '?' => { self.advance(); self.make_token(Token::Question,line, col) }

                '/' => { self.advance(); self.make_token(Token::Slash, line, col) }

                '|' => {
                    self.advance();
                    if self.current() == Some('>') {
                        self.advance();
                        self.make_token(Token::Pipe, line, col)
                    } else {
                        // bare '|' — not defined yet, skip
                        continue;
                    }
                }

                ':' => {
                    self.advance();
                    if self.current() == Some(':') {
                        self.advance();
                        self.make_token(Token::ColonColon, line, col)
                    } else {
                        self.make_token(Token::Colon, line, col)
                    }
                }

                '=' => {
                    self.advance();
                    if self.current() == Some('=') {
                        self.advance();
                        self.make_token(Token::Eq, line, col)
                    } else {
                        self.make_token(Token::Assign, line, col)
                    }
                }

                '!' => {
                    self.advance();
                    if self.current() == Some('=') {
                        self.advance();
                        self.make_token(Token::NotEq, line, col)
                    } else {
                        self.make_token(Token::Bang, line, col)
                    }
                }

                '<' => {
                    self.advance();
                    if self.current() == Some('=') {
                        self.advance();
                        self.make_token(Token::LtEq, line, col)
                    } else {
                        self.make_token(Token::Lt, line, col)
                    }
                }

                '>' => {
                    self.advance();
                    if self.current() == Some('=') {
                        self.advance();
                        self.make_token(Token::GtEq, line, col)
                    } else {
                        self.make_token(Token::Gt, line, col)
                    }
                }

                // Unknown character — skip silently for now
                _ => { self.advance(); continue; }
            };

            tokens.push(tok);
        }

        tokens
    }
}
