//! Statement parsing

use super::*;

impl Parser {
    pub(crate) fn parse_statement(&mut self) -> ParseResult<Node<Stmt>> {
        let start = self.current_token().span;

        let stmt = match self.current_token().kind {
            TokenKind::LBrace => Stmt::Block(self.parse_block_statement()?.value),
            TokenKind::If => self.parse_if_statement()?,
            TokenKind::For => self.parse_for_statement()?,
            TokenKind::While => self.parse_while_statement()?,
            TokenKind::Do => self.parse_do_while_statement()?,
            TokenKind::Switch => self.parse_switch_statement()?,
            TokenKind::Return => self.parse_return_statement()?,
            TokenKind::Break => self.parse_break_statement()?,
            TokenKind::Continue => self.parse_continue_statement()?,
            TokenKind::Throw => self.parse_throw_statement()?,
            TokenKind::Try => self.parse_try_statement()?,
            TokenKind::Debugger => {
                self.advance();
                self.consume_semicolon();
                Stmt::Debugger
            }
            TokenKind::Semicolon => {
                self.advance();
                Stmt::Empty
            }
            TokenKind::Const | TokenKind::Let | TokenKind::Var | TokenKind::Using => {
                let var_decl = self.parse_var_declaration()?;
                Stmt::VarDecl(var_decl)
            }
            _ if self.check(&TokenKind::Await) && self.peek_kind(1) == Some(&TokenKind::Using) => {
                // await using declaration
                self.advance(); // skip await
                self.advance(); // skip using
                let mut declarations = Vec::new();
                loop {
                    let pattern = self.parse_pattern()?;
                    let init = if self.check(&TokenKind::Eq) {
                        self.advance();
                        Some(self.parse_expression()?)
                    } else {
                        None
                    };
                    declarations.push(VarDeclarator { pattern, init });
                    if !self.check(&TokenKind::Comma) {
                        break;
                    }
                    self.advance();
                }
                self.consume_semicolon();
                Stmt::VarDecl(VarDecl {
                    kind: VarDeclKind::AwaitUsing,
                    declarations,
                })
            }
            _ => {
                // Check for labeled statement
                if self.check(&TokenKind::Identifier) && self.peek_kind(1) == Some(&TokenKind::Colon) {
                    let label = self.parse_identifier()?;
                    self.consume(TokenKind::Colon)?;
                    let stmt = Box::new(self.parse_statement()?);
                    Stmt::Labeled { label, stmt }
                } else {
                    let expr = self.parse_expression()?;
                    self.consume_semicolon();
                    Stmt::Expr(expr)
                }
            }
        };

        let span = start.merge(&self.previous_token().span);
        Ok(Node::new(stmt, span))
    }

    pub(crate) fn parse_block_statement(&mut self) -> ParseResult<Node<BlockStmt>> {
        let start = self.current_token().span;
        self.consume(TokenKind::LBrace)?;

        let mut stmts = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            stmts.push(self.parse_statement()?);
        }

        self.consume(TokenKind::RBrace)?;
        let span = start.merge(&self.previous_token().span);

        Ok(Node::new(BlockStmt { stmts }, span))
    }

    pub(crate) fn parse_var_declaration(&mut self) -> ParseResult<VarDecl> {
        let kind = match self.current_token().kind {
            TokenKind::Const => VarDeclKind::Const,
            TokenKind::Let => VarDeclKind::Let,
            TokenKind::Var => VarDeclKind::Var,
            TokenKind::Using => VarDeclKind::Using,
            _ => return Err(self.error("Expected var, let, const, or using".to_string())),
        };
        self.advance();

        let mut declarations = Vec::new();

        loop {
            let pattern = self.parse_pattern()?;
            let init = if self.check(&TokenKind::Eq) {
                self.advance();
                Some(self.parse_expression()?)
            } else {
                None
            };

            declarations.push(VarDeclarator { pattern, init });

            if !self.check(&TokenKind::Comma) {
                break;
            }
            self.advance();
        }

        self.consume_semicolon();

        Ok(VarDecl { kind, declarations })
    }

    fn parse_if_statement(&mut self) -> ParseResult<Stmt> {
        self.consume(TokenKind::If)?;
        self.consume(TokenKind::LParen)?;
        let condition = self.parse_expression()?;
        self.consume(TokenKind::RParen)?;

        let then_stmt = Box::new(self.parse_statement()?);

        let else_stmt = if self.check(&TokenKind::Else) {
            self.advance();
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };

        Ok(Stmt::If {
            condition,
            then_stmt,
            else_stmt,
        })
    }

    fn parse_for_statement(&mut self) -> ParseResult<Stmt> {
        self.consume(TokenKind::For)?;

        let is_await = if self.check(&TokenKind::Await) {
            self.advance();
            true
        } else {
            false
        };

        self.consume(TokenKind::LParen)?;

        // Check for for-in/for-of
        let checkpoint = self.current;

        // Try to parse as for-in/for-of
        if !self.check(&TokenKind::Semicolon) {
            let is_var_decl = matches!(
                self.current_token().kind,
                TokenKind::Const | TokenKind::Let | TokenKind::Var
            );

            if is_var_decl {
                let var_decl = self.parse_var_declaration_without_semicolon()?;

                if self.check(&TokenKind::In) {
                    self.advance();
                    let right = self.parse_expression()?;
                    self.consume(TokenKind::RParen)?;
                    let body = Box::new(self.parse_statement()?);

                    return Ok(Stmt::ForIn {
                        left: ForInLeft::VarDecl(var_decl),
                        right,
                        body,
                    });
                } else if self.check(&TokenKind::Of) {
                    self.advance();
                    let right = self.parse_expression()?;
                    self.consume(TokenKind::RParen)?;
                    let body = Box::new(self.parse_statement()?);

                    return Ok(Stmt::ForOf {
                        left: ForInLeft::VarDecl(var_decl),
                        right,
                        body,
                        is_await,
                    });
                }

                // Reset and parse as regular for loop
                self.current = checkpoint;
            } else {
                // Try pattern
                let pattern_result = self.parse_pattern();

                if let Ok(pat) = pattern_result {
                    if self.check(&TokenKind::In) {
                        self.advance();
                        let right = self.parse_expression()?;
                        self.consume(TokenKind::RParen)?;
                        let body = Box::new(self.parse_statement()?);

                        return Ok(Stmt::ForIn {
                            left: ForInLeft::Pattern(pat),
                            right,
                            body,
                        });
                    } else if self.check(&TokenKind::Of) {
                        self.advance();
                        let right = self.parse_expression()?;
                        self.consume(TokenKind::RParen)?;
                        let body = Box::new(self.parse_statement()?);

                        return Ok(Stmt::ForOf {
                            left: ForInLeft::Pattern(pat),
                            right,
                            body,
                            is_await,
                        });
                    }
                }

                // Reset
                self.current = checkpoint;
            }
        }

        // Regular for loop
        let init = if self.check(&TokenKind::Semicolon) {
            None
        } else if matches!(
            self.current_token().kind,
            TokenKind::Const | TokenKind::Let | TokenKind::Var
        ) {
            Some(ForInit::VarDecl(self.parse_var_declaration_without_semicolon()?))
        } else {
            Some(ForInit::Expr(self.parse_expression()?))
        };

        self.consume(TokenKind::Semicolon)?;

        let condition = if self.check(&TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expression()?)
        };

        self.consume(TokenKind::Semicolon)?;

        let update = if self.check(&TokenKind::RParen) {
            None
        } else {
            Some(self.parse_expression()?)
        };

        self.consume(TokenKind::RParen)?;
        let body = Box::new(self.parse_statement()?);

        Ok(Stmt::For {
            init,
            condition,
            update,
            body,
        })
    }

    fn parse_while_statement(&mut self) -> ParseResult<Stmt> {
        self.consume(TokenKind::While)?;
        self.consume(TokenKind::LParen)?;
        let condition = self.parse_expression()?;
        self.consume(TokenKind::RParen)?;
        let body = Box::new(self.parse_statement()?);

        Ok(Stmt::While { condition, body })
    }

    fn parse_do_while_statement(&mut self) -> ParseResult<Stmt> {
        self.consume(TokenKind::Do)?;
        let body = Box::new(self.parse_statement()?);
        self.consume(TokenKind::While)?;
        self.consume(TokenKind::LParen)?;
        let condition = self.parse_expression()?;
        self.consume(TokenKind::RParen)?;
        self.consume_semicolon();

        Ok(Stmt::DoWhile { body, condition })
    }

    fn parse_switch_statement(&mut self) -> ParseResult<Stmt> {
        self.consume(TokenKind::Switch)?;
        self.consume(TokenKind::LParen)?;
        let discriminant = self.parse_expression()?;
        self.consume(TokenKind::RParen)?;

        self.consume(TokenKind::LBrace)?;
        let mut cases = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            if self.check(&TokenKind::Case) {
                self.advance();
                let test = Some(self.parse_expression()?);
                self.consume(TokenKind::Colon)?;

                let mut consequent = Vec::new();
                while !self.check(&TokenKind::Case)
                    && !self.check(&TokenKind::Default)
                    && !self.check(&TokenKind::RBrace)
                {
                    consequent.push(self.parse_statement()?);
                }

                cases.push(SwitchCase { test, consequent });
            } else if self.check(&TokenKind::Default) {
                self.advance();
                self.consume(TokenKind::Colon)?;

                let mut consequent = Vec::new();
                while !self.check(&TokenKind::Case)
                    && !self.check(&TokenKind::Default)
                    && !self.check(&TokenKind::RBrace)
                {
                    consequent.push(self.parse_statement()?);
                }

                cases.push(SwitchCase {
                    test: None,
                    consequent,
                });
            } else {
                return Err(self.error("Expected case or default in switch statement".to_string()));
            }
        }

        self.consume(TokenKind::RBrace)?;

        Ok(Stmt::Switch {
            discriminant,
            cases,
        })
    }

    fn parse_return_statement(&mut self) -> ParseResult<Stmt> {
        self.consume(TokenKind::Return)?;

        let expr = if self.check(&TokenKind::Semicolon) || self.is_at_end() {
            None
        } else {
            Some(self.parse_expression()?)
        };

        self.consume_semicolon();
        Ok(Stmt::Return(expr))
    }

    fn parse_break_statement(&mut self) -> ParseResult<Stmt> {
        self.consume(TokenKind::Break)?;

        let label = if self.check(&TokenKind::Identifier) && !self.is_semicolon_ahead() {
            Some(self.parse_identifier()?)
        } else {
            None
        };

        self.consume_semicolon();
        Ok(Stmt::Break(label))
    }

    fn parse_continue_statement(&mut self) -> ParseResult<Stmt> {
        self.consume(TokenKind::Continue)?;

        let label = if self.check(&TokenKind::Identifier) && !self.is_semicolon_ahead() {
            Some(self.parse_identifier()?)
        } else {
            None
        };

        self.consume_semicolon();
        Ok(Stmt::Continue(label))
    }

    fn parse_throw_statement(&mut self) -> ParseResult<Stmt> {
        self.consume(TokenKind::Throw)?;
        let expr = self.parse_expression()?;
        self.consume_semicolon();
        Ok(Stmt::Throw(expr))
    }

    fn parse_try_statement(&mut self) -> ParseResult<Stmt> {
        self.consume(TokenKind::Try)?;
        let block = self.parse_block_statement()?;

        let catch = if self.check(&TokenKind::Catch) {
            self.advance();

            let param = if self.check(&TokenKind::LParen) {
                self.advance();
                let p = Some(self.parse_pattern()?);
                self.consume(TokenKind::RParen)?;
                p
            } else {
                None
            };

            let body = self.parse_block_statement()?;

            Some(CatchClause { param, body })
        } else {
            None
        };

        let finally = if self.check(&TokenKind::Finally) {
            self.advance();
            Some(self.parse_block_statement()?)
        } else {
            None
        };

        if catch.is_none() && finally.is_none() {
            return Err(self.error("Try statement must have catch or finally clause".to_string()));
        }

        Ok(Stmt::Try {
            block,
            catch,
            finally,
        })
    }

    // =========================================================================
}
