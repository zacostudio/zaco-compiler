//! Helper methods and utilities

use super::*;

impl Parser {
    pub(crate) fn parse_identifier(&mut self) -> ParseResult<Node<Ident>> {
        let token = self.consume(TokenKind::Identifier)?;
        Ok(Node::new(
            Ident::new(token.value.clone()),
            token.span,
        ))
    }

    pub(crate) fn parse_property_name(&mut self) -> ParseResult<PropertyName> {
        match self.current_token().kind {
            TokenKind::Identifier => {
                let ident = self.parse_identifier()?;
                Ok(PropertyName::Ident(ident))
            }
            TokenKind::StringLiteral => {
                let value = self.advance().value.clone();
                Ok(PropertyName::String(value))
            }
            TokenKind::NumberLiteral => {
                let value = self.advance().value.clone();
                let num = value.parse::<f64>().unwrap_or(0.0);
                Ok(PropertyName::Number(num))
            }
            TokenKind::LBracket => {
                self.advance();
                let expr = Box::new(self.parse_expression()?);
                self.consume(TokenKind::RBracket)?;
                Ok(PropertyName::Computed(expr))
            }
            _ => Err(self.error("Expected property name".to_string())),
        }
    }

    pub(crate) fn parse_function_params(&mut self) -> ParseResult<Vec<Param>> {
        let mut params = Vec::new();

        while !self.check(&TokenKind::RParen) && !self.is_at_end() {
            params.push(self.parse_function_param()?);
            if !self.check(&TokenKind::RParen) {
                self.consume(TokenKind::Comma)?;
            }
        }

        Ok(params)
    }

    pub(crate) fn parse_function_param(&mut self) -> ParseResult<Param> {
        let is_rest = if self.check(&TokenKind::DotDotDot) {
            self.advance();
            true
        } else {
            false
        };

        // Parse ownership annotation before the parameter name (e.g. `ref other: Point`)
        let ownership = self.parse_ownership_annotation()?;

        let pattern = self.parse_pattern()?;

        // Also check for ownership annotation after the pattern (alternative syntax)
        let ownership = if ownership.is_none() {
            self.parse_ownership_annotation()?
        } else {
            ownership
        };

        let type_annotation = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(Box::new(self.parse_type()?))
        } else {
            None
        };

        let optional = if self.check(&TokenKind::Question) {
            self.advance();
            true
        } else {
            false
        };

        Ok(Param {
            pattern,
            type_annotation,
            ownership,
            optional,
            is_rest,
        })
    }

    pub(crate) fn parse_function_params_for_arrow(&mut self) -> ParseResult<Vec<Param>> {
        self.parse_function_params()
    }

    pub(crate) fn parse_ownership_annotation(&mut self) -> ParseResult<Option<Ownership>> {
        let start = self.current_token().span;

        let kind = match self.current_token().kind {
            TokenKind::Owned => {
                self.advance();
                OwnershipKind::Owned
            }
            TokenKind::Ref => {
                self.advance();
                OwnershipKind::Ref
            }
            TokenKind::Mut => {
                self.advance();
                if self.check(&TokenKind::Ref) {
                    self.advance();
                    OwnershipKind::MutRef
                } else {
                    return Err(self.error("Expected 'ref' after 'mut'".to_string()));
                }
            }
            _ => return Ok(None),
        };

        let span = start.merge(&self.previous_token().span);
        Ok(Some(Ownership { kind, span }))
    }

    pub(crate) fn parse_binary_operator(&mut self) -> ParseResult<BinaryOp> {
        let op = match self.current_token().kind {
            TokenKind::Plus => BinaryOp::Add,
            TokenKind::Minus => BinaryOp::Sub,
            TokenKind::Star => BinaryOp::Mul,
            TokenKind::Slash => BinaryOp::Div,
            TokenKind::Percent => BinaryOp::Mod,
            TokenKind::StarStar => BinaryOp::Pow,
            TokenKind::EqEq => BinaryOp::Eq,
            TokenKind::BangEq => BinaryOp::NotEq,
            TokenKind::EqEqEq => BinaryOp::StrictEq,
            TokenKind::BangEqEq => BinaryOp::StrictNotEq,
            TokenKind::Lt => BinaryOp::Lt,
            TokenKind::LtEq => BinaryOp::LtEq,
            TokenKind::Gt => BinaryOp::Gt,
            TokenKind::GtEq => BinaryOp::GtEq,
            TokenKind::AmpAmp => BinaryOp::And,
            TokenKind::PipePipe => BinaryOp::Or,
            TokenKind::QuestionQuestion => BinaryOp::NullishCoalesce,
            TokenKind::Amp => BinaryOp::BitAnd,
            TokenKind::Pipe => BinaryOp::BitOr,
            TokenKind::Caret => BinaryOp::BitXor,
            TokenKind::LtLt => BinaryOp::LeftShift,
            TokenKind::GtGt => BinaryOp::RightShift,
            TokenKind::GtGtGt => BinaryOp::UnsignedRightShift,
            TokenKind::In => BinaryOp::In,
            TokenKind::Instanceof => BinaryOp::InstanceOf,
            _ => return Err(self.error("Expected binary operator".to_string())),
        };
        self.advance();
        Ok(op)
    }

    pub(crate) fn parse_assignment_operator(&mut self) -> ParseResult<AssignmentOp> {
        let op = match self.current_token().kind {
            TokenKind::Eq => AssignmentOp::Assign,
            TokenKind::PlusEq => AssignmentOp::AddAssign,
            TokenKind::MinusEq => AssignmentOp::SubAssign,
            TokenKind::StarEq => AssignmentOp::MulAssign,
            TokenKind::SlashEq => AssignmentOp::DivAssign,
            TokenKind::PercentEq => AssignmentOp::ModAssign,
            TokenKind::StarStarEq => AssignmentOp::PowAssign,
            TokenKind::AmpAmpEq => AssignmentOp::AndAssign,
            TokenKind::PipePipeEq => AssignmentOp::OrAssign,
            TokenKind::QuestionQuestionEq => AssignmentOp::NullishAssign,
            TokenKind::LtLtEq => AssignmentOp::LeftShiftAssign,
            TokenKind::GtGtEq => AssignmentOp::RightShiftAssign,
            TokenKind::GtGtGtEq => AssignmentOp::UnsignedRightShiftAssign,
            TokenKind::AmpEq => AssignmentOp::BitAndAssign,
            TokenKind::PipeEq => AssignmentOp::BitOrAssign,
            TokenKind::CaretEq => AssignmentOp::BitXorAssign,
            _ => return Err(self.error("Expected assignment operator".to_string())),
        };
        self.advance();
        Ok(op)
    }

    pub(crate) fn parse_var_declaration_without_semicolon(&mut self) -> ParseResult<VarDecl> {
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

        Ok(VarDecl { kind, declarations })
    }

    pub(crate) fn expr_to_param(&mut self, expr: Node<Expr>) -> ParseResult<Param> {
        let pattern = match expr.value {
            Expr::Ident(ident) => Pattern::Ident {
                name: Node::new(ident, expr.span),
                type_annotation: None,
                ownership: None,
            },
            _ => return Err(self.error("Invalid parameter expression".to_string())),
        };

        Ok(Param {
            pattern: Node::new(pattern, expr.span),
            type_annotation: None,
            ownership: None,
            optional: false,
            is_rest: false,
        })
    }

    // =========================================================================
    // Operator Precedence
    // =========================================================================

    pub(crate) fn get_infix_precedence(&self) -> u8 {
        match self.current_token().kind {
            TokenKind::Eq
            | TokenKind::PlusEq
            | TokenKind::MinusEq
            | TokenKind::StarEq
            | TokenKind::SlashEq
            | TokenKind::PercentEq
            | TokenKind::StarStarEq
            | TokenKind::AmpAmpEq
            | TokenKind::PipePipeEq
            | TokenKind::QuestionQuestionEq
            | TokenKind::LtLtEq
            | TokenKind::GtGtEq
            | TokenKind::GtGtGtEq
            | TokenKind::AmpEq
            | TokenKind::PipeEq
            | TokenKind::CaretEq => 1,
            TokenKind::Question => 2,
            TokenKind::QuestionQuestion => 3,
            TokenKind::PipePipe => 4,
            TokenKind::AmpAmp => 5,
            TokenKind::Pipe => 6,
            TokenKind::Caret => 7,
            TokenKind::Amp => 8,
            TokenKind::EqEq | TokenKind::BangEq | TokenKind::EqEqEq | TokenKind::BangEqEq => 9,
            TokenKind::Lt
            | TokenKind::Gt
            | TokenKind::LtEq
            | TokenKind::GtEq
            | TokenKind::In
            | TokenKind::Instanceof => 10,
            TokenKind::LtLt | TokenKind::GtGt | TokenKind::GtGtGt => 11,
            TokenKind::Plus | TokenKind::Minus => 12,
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => 13,
            TokenKind::StarStar => 14,
            TokenKind::As | TokenKind::Satisfies => 15,
            TokenKind::PlusPlus | TokenKind::MinusMinus | TokenKind::Bang => 16,
            TokenKind::Dot | TokenKind::QuestionDot | TokenKind::LBracket | TokenKind::LParen => 17,
            _ => 0,
        }
    }

    // =========================================================================
    // Utility Methods (Token Manipulation)
    // =========================================================================

    pub(crate) fn current_token(&self) -> &Token {
        &self.tokens[self.current.min(self.tokens.len() - 1)]
    }

    pub(crate) fn previous_token(&self) -> &Token {
        &self.tokens[(self.current.saturating_sub(1)).min(self.tokens.len() - 1)]
    }

    pub(crate) fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous_token()
    }

    pub(crate) fn check(&self, kind: &TokenKind) -> bool {
        !self.is_at_end() && &self.current_token().kind == kind
    }

    pub(crate) fn peek_kind(&self, offset: usize) -> Option<&TokenKind> {
        let index = self.current + offset;
        if index < self.tokens.len() {
            Some(&self.tokens[index].kind)
        } else {
            None
        }
    }

    pub(crate) fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len() || self.current_token().kind == TokenKind::Eof
    }

    pub(crate) fn consume(&mut self, kind: TokenKind) -> ParseResult<&Token> {
        if self.check(&kind) {
            Ok(self.advance())
        } else {
            Err(self.error(format!("Expected {:?}, found {:?}", kind, self.current_token().kind)))
        }
    }

    pub(crate) fn consume_semicolon(&mut self) {
        if self.check(&TokenKind::Semicolon) {
            self.advance();
        }
    }

    pub(crate) fn is_semicolon_ahead(&self) -> bool {
        self.check(&TokenKind::Semicolon) || self.is_at_end()
    }

    pub(crate) fn is_type_argument_start(&self) -> bool {
        // Heuristic: check if this looks like type arguments vs less-than operator
        if !self.check(&TokenKind::Lt) {
            return false;
        }

        // Look at the token after < to see if it looks like a type
        matches!(
            self.peek_kind(1),
            Some(TokenKind::Identifier)
                | Some(TokenKind::Void)
                | Some(TokenKind::Any)
                | Some(TokenKind::Never)
                | Some(TokenKind::Unknown)
                | Some(TokenKind::Null)
                | Some(TokenKind::Undefined)
                | Some(TokenKind::True)
                | Some(TokenKind::False)
                | Some(TokenKind::LParen)
                | Some(TokenKind::LBracket)
                | Some(TokenKind::LBrace)
                | Some(TokenKind::Typeof)
                | Some(TokenKind::Keyof)
                | Some(TokenKind::Infer)
                | Some(TokenKind::Readonly)
                | Some(TokenKind::StringLiteral)
                | Some(TokenKind::NumberLiteral)
                | Some(TokenKind::TemplateLiteral)
        )
    }

    pub(crate) fn error(&self, message: String) -> ParseError {
        ParseError {
            message,
            span: self.current_token().span,
        }
    }

    pub(crate) fn synchronize(&mut self) {
        self.advance();

        while !self.is_at_end() {
            if self.previous_token().kind == TokenKind::Semicolon {
                return;
            }

            match self.current_token().kind {
                TokenKind::Class
                | TokenKind::Function
                | TokenKind::Let
                | TokenKind::Const
                | TokenKind::Var
                | TokenKind::For
                | TokenKind::If
                | TokenKind::While
                | TokenKind::Return
                | TokenKind::Import
                | TokenKind::Export => return,
                _ => {}
            }

            self.advance();
        }
    }
}
