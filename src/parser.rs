use crate::ast::{BinOp, Expr};
use crate::token::{SpannedToken, Token};

/// The Parser consumes a flat list of SpannedTokens produced by the Lexer
/// and builds a Vec<Expr> (the program's Abstract Syntax Tree).
pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        Parser { tokens, pos: 0 }
    }

    // ── Internal helpers ────────────────────────────────────────────────────

    fn current(&self) -> &Token {
        &self.tokens[self.pos].token
    }

    fn current_spanned(&self) -> &SpannedToken {
        &self.tokens[self.pos]
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos + 1).map(|t| &t.token).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> &Token {
        let t = &self.tokens[self.pos].token;
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn skip_newlines(&mut self) {
        while matches!(self.current(), Token::Newline) {
            self.advance();
        }
    }

    fn expect(&mut self, expected: &Token) -> Result<(), String> {
        if self.current() == expected {
            self.advance();
            Ok(())
        } else {
            let t = self.current_spanned();
            Err(format!(
                "[VERD ERROR] {}:{} — expected {:?}, got {:?}",
                t.line, t.col, expected, t.token
            ))
        }
    }

    fn expect_identifier(&mut self) -> Result<String, String> {
        match self.current().clone() {
            Token::Identifier(name) => {
                self.advance();
                Ok(name)
            }
            _ => {
                let t = self.current_spanned();
                Err(format!(
                    "[VERD ERROR] {}:{} — expected identifier, got {:?}",
                    t.line, t.col, t.token
                ))
            }
        }
    }

    // ── Public entry point ──────────────────────────────────────────────────

    pub fn parse(&mut self) -> Result<Vec<Expr>, String> {
        let mut program = Vec::new();
        self.skip_newlines();

        while !matches!(self.current(), Token::Eof) {
            let expr = self.parse_statement()?;
            program.push(expr);
            self.skip_newlines();
        }

        Ok(program)
    }

    // ── Statements ──────────────────────────────────────────────────────────

    fn parse_statement(&mut self) -> Result<Expr, String> {
        match self.current().clone() {
            Token::Pin   => self.parse_pin(),
            Token::Flux  => self.parse_flux(),
            Token::Op    => self.parse_op_decl(),
            Token::Cycle => self.parse_cycle(),
            Token::Yield => { self.advance(); let e = self.parse_expr()?; Ok(Expr::Yield(Box::new(e))) }
            Token::Rise  => { self.advance(); let e = self.parse_expr()?; Ok(Expr::Rise(Box::new(e))) }
            Token::Spawn => self.parse_spawn(),
            Token::Sync  => self.parse_sync(),
            _            => self.parse_expr_statement(),
        }
    }

    fn parse_pin(&mut self) -> Result<Expr, String> {
        self.advance(); // consume 'pin'
        let name = self.expect_identifier()?;
        // optional : type annotation (we skip the type for now)
        if matches!(self.current(), Token::Colon) {
            self.advance();
            self.advance(); // skip type name
        }
        self.expect(&Token::Assign)?;
        let value = self.parse_expr()?;
        Ok(Expr::Pin { name, value: Box::new(value) })
    }

    fn parse_flux(&mut self) -> Result<Expr, String> {
        self.advance(); // consume 'flux'
        let name = self.expect_identifier()?;
        if matches!(self.current(), Token::Colon) {
            self.advance();
            self.advance(); // skip type name
        }
        self.expect(&Token::Assign)?;
        let value = self.parse_expr()?;
        Ok(Expr::Flux { name, value: Box::new(value) })
    }

    fn parse_op_decl(&mut self) -> Result<Expr, String> {
        self.advance(); // consume 'op'
        let name = self.expect_identifier()?;

        // Parameters: (a, b, ...)
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        while !matches!(self.current(), Token::RParen | Token::Eof) {
            params.push(self.expect_identifier()?);
            if matches!(self.current(), Token::Comma) { self.advance(); }
        }
        self.expect(&Token::RParen)?;

        // Effect declarations: !flux(counter, other)
        let mut effects = Vec::new();
        while matches!(self.current(), Token::Bang) {
            self.advance(); // consume '!'
            // expect 'flux' keyword next
            if matches!(self.current(), Token::Flux) { self.advance(); }
            self.expect(&Token::LParen)?;
            while !matches!(self.current(), Token::RParen | Token::Eof) {
                effects.push(self.expect_identifier()?);
                if matches!(self.current(), Token::Comma) { self.advance(); }
            }
            self.expect(&Token::RParen)?;
        }

        // Optional -> return type annotation (skip)
        if matches!(self.current(), Token::Arrow) {
            self.advance();
            self.advance(); // skip type
        }

        // Body
        let body = self.parse_block()?;
        Ok(Expr::OpDecl { name, params, effects, body })
    }

    fn parse_cycle(&mut self) -> Result<Expr, String> {
        self.advance(); // consume 'cycle'
        let condition = self.parse_expr()?;
        let body = self.parse_block()?;
        Ok(Expr::Cycle { condition: Box::new(condition), body })
    }

    fn parse_spawn(&mut self) -> Result<Expr, String> {
        self.advance(); // consume 'spawn'
        let call = self.parse_expr()?;
        self.expect(&Token::Arrow)?;
        let handle = self.expect_identifier()?;
        Ok(Expr::Spawn { call: Box::new(call), handle })
    }

    fn parse_sync(&mut self) -> Result<Expr, String> {
        self.advance(); // consume 'sync'
        let handle = self.expect_identifier()?;
        Ok(Expr::Sync { handle })
    }

    fn parse_block(&mut self) -> Result<Vec<Expr>, String> {
        self.expect(&Token::LBrace)?;
        self.skip_newlines();
        let mut stmts = Vec::new();
        while !matches!(self.current(), Token::RBrace | Token::Eof) {
            stmts.push(self.parse_statement()?);
            self.skip_newlines();
        }
        self.expect(&Token::RBrace)?;
        Ok(stmts)
    }

    // ── Expression parsing ───────────────────────────────────────────────────

    fn parse_expr_statement(&mut self) -> Result<Expr, String> {
        let expr = self.parse_pipeline()?;

        // assignment: x = value
        if matches!(self.current(), Token::Assign) {
            if let Expr::Identifier(name) = &expr {
                let name = name.clone();
                self.advance(); // consume '='
                let value = self.parse_expr()?;
                return Ok(Expr::Assign { name, value: Box::new(value) });
            }
        }

        // catch: expr catch err { ... }
        if matches!(self.current(), Token::Catch) {
            self.advance();
            let err_var = self.expect_identifier()?;
            let handler = self.parse_block()?;
            return Ok(Expr::Catch {
                body: vec![expr],
                err_var,
                handler,
            });
        }

        // match: expr match { some(x) -> ..., none -> ... }
        if matches!(self.current(), Token::Match) {
            return self.parse_match(expr);
        }

        Ok(expr)
    }

    fn parse_pipeline(&mut self) -> Result<Expr, String> {
        let mut stages = vec![self.parse_expr()?];

        while matches!(self.current(), Token::Pipe) {
            self.advance(); // consume '|>'
            stages.push(self.parse_primary()?);
        }

        if stages.len() == 1 {
            Ok(stages.remove(0))
        } else {
            Ok(Expr::Pipeline { stages })
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_additive()?;

        loop {
            let op = match self.current() {
                Token::Eq     => BinOp::Eq,
                Token::NotEq  => BinOp::NotEq,
                Token::Lt     => BinOp::Lt,
                Token::Gt     => BinOp::Gt,
                Token::LtEq   => BinOp::LtEq,
                Token::GtEq   => BinOp::GtEq,
                _             => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            left = Expr::BinaryOp { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplicative()?;

        loop {
            let op = match self.current() {
                Token::Plus  => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _            => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::BinaryOp { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_question()?;

        loop {
            let op = match self.current() {
                Token::Star    => BinOp::Mul,
                Token::Slash   => BinOp::Div,
                Token::Percent => BinOp::Mod,
                _              => break,
            };
            self.advance();
            let right = self.parse_question()?;
            left = Expr::BinaryOp { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_question(&mut self) -> Result<Expr, String> {
        let expr = self.parse_primary()?;

        if matches!(self.current(), Token::Question) {
            self.advance();
            let body = self.parse_block()?;
            return Ok(Expr::Question { condition: Box::new(expr), body });
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.current().clone() {
            Token::Number(n) => { self.advance(); Ok(Expr::Number(n)) }
            Token::Text(s)   => { self.advance(); Ok(Expr::Text(s)) }
            Token::Bool(b)   => { self.advance(); Ok(Expr::Bool(b)) }
            Token::None      => { self.advance(); Ok(Expr::None) }

            Token::Some => {
                self.advance();
                self.expect(&Token::LParen)?;
                let inner = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                // Represent some(x) as a Call for now
                Ok(Expr::Call { name: "some".to_string(), args: vec![inner] })
            }

            Token::Identifier(name) => {
                self.advance();
                // Function call?
                if matches!(self.current(), Token::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    while !matches!(self.current(), Token::RParen | Token::Eof) {
                        args.push(self.parse_expr()?);
                        if matches!(self.current(), Token::Comma) { self.advance(); }
                    }
                    self.expect(&Token::RParen)?;
                    Ok(Expr::Call { name, args })
                } else {
                    Ok(Expr::Identifier(name))
                }
            }

            Token::LParen => {
                self.advance();
                let e = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(e)
            }

            other => {
                let t = self.current_spanned();
                Err(format!(
                    "[VERD ERROR] {}:{} — unexpected token {:?}",
                    t.line, t.col, other
                ))
            }
        }
    }

    fn parse_match(&mut self, subject: Expr) -> Result<Expr, String> {
        self.advance(); // consume 'match'
        self.expect(&Token::LBrace)?;
        self.skip_newlines();

        let mut some_branch = None;
        let mut none_branch = None;

        while !matches!(self.current(), Token::RBrace | Token::Eof) {
            match self.current().clone() {
                Token::Some => {
                    self.advance();
                    self.expect(&Token::LParen)?;
                    let bound = self.expect_identifier()?;
                    self.expect(&Token::RParen)?;
                    self.expect(&Token::Arrow)?;
                    // Single-expression branch OR block
                    let body = if matches!(self.current(), Token::LBrace) {
                        self.parse_block()?
                    } else {
                        vec![self.parse_expr()?]
                    };
                    some_branch = Some((bound, body));
                }
                Token::None => {
                    self.advance();
                    self.expect(&Token::Arrow)?;
                    let body = if matches!(self.current(), Token::LBrace) {
                        self.parse_block()?
                    } else {
                        vec![self.parse_expr()?]
                    };
                    none_branch = Some(body);
                }
                _ => { self.advance(); } // skip unexpected tokens
            }
            self.skip_newlines();
        }
        self.expect(&Token::RBrace)?;

        Ok(Expr::Match {
            subject: Box::new(subject),
            some_branch,
            none_branch,
        })
    }
}
