//! Expression parsing

use super::*;

impl Parser {
    pub(crate) fn parse_expression(&mut self) -> ParseResult<Node<Expr>> {
        self.parse_expression_with_precedence(0)
    }

    pub(crate) fn parse_expression_with_precedence(&mut self, min_precedence: u8) -> ParseResult<Node<Expr>> {
        let _start = self.current_token().span;
        let mut left = self.parse_prefix_expression()?;

        loop {
            let precedence = self.get_infix_precedence();

            if precedence == 0 || precedence < min_precedence {
                break;
            }

            left = self.parse_infix_expression(left)?;
        }

        Ok(left)
    }

    fn parse_prefix_expression(&mut self) -> ParseResult<Node<Expr>> {
        let start = self.current_token().span;

        let expr = match self.current_token().kind {
            // Unary operators
            TokenKind::Plus => {
                self.advance();
                let expr = Box::new(self.parse_expression_with_precedence(14)?);
                Expr::Unary {
                    op: UnaryOp::Plus,
                    expr,
                }
            }
            TokenKind::Minus => {
                self.advance();
                let expr = Box::new(self.parse_expression_with_precedence(14)?);
                Expr::Unary {
                    op: UnaryOp::Minus,
                    expr,
                }
            }
            TokenKind::Bang => {
                self.advance();
                let expr = Box::new(self.parse_expression_with_precedence(14)?);
                Expr::Unary {
                    op: UnaryOp::Not,
                    expr,
                }
            }
            TokenKind::Tilde => {
                self.advance();
                let expr = Box::new(self.parse_expression_with_precedence(14)?);
                Expr::Unary {
                    op: UnaryOp::BitNot,
                    expr,
                }
            }
            TokenKind::Typeof => {
                self.advance();
                let expr = Box::new(self.parse_expression_with_precedence(14)?);
                Expr::Unary {
                    op: UnaryOp::TypeOf,
                    expr,
                }
            }
            TokenKind::Void => {
                self.advance();
                let expr = Box::new(self.parse_expression_with_precedence(14)?);
                Expr::Unary {
                    op: UnaryOp::Void,
                    expr,
                }
            }
            _ if self.check(&TokenKind::Identifier) && self.current_token().value == "delete" => {
                self.advance();
                let expr = Box::new(self.parse_expression_with_precedence(14)?);
                Expr::Unary {
                    op: UnaryOp::Delete,
                    expr,
                }
            }
            TokenKind::PlusPlus => {
                self.advance();
                let expr = Box::new(self.parse_expression_with_precedence(14)?);
                Expr::Unary {
                    op: UnaryOp::PreIncrement,
                    expr,
                }
            }
            TokenKind::MinusMinus => {
                self.advance();
                let expr = Box::new(self.parse_expression_with_precedence(14)?);
                Expr::Unary {
                    op: UnaryOp::PreDecrement,
                    expr,
                }
            }
            TokenKind::Await => {
                self.advance();
                let expr = Box::new(self.parse_expression_with_precedence(14)?);
                Expr::Await(expr)
            }
            TokenKind::Yield => {
                self.advance();
                let delegate = if self.check(&TokenKind::Star) {
                    self.advance();
                    true
                } else {
                    false
                };
                // Check if there's an argument (not followed by semicolon, }, or comma)
                let argument = if !self.check(&TokenKind::Semicolon)
                    && !self.check(&TokenKind::RBrace)
                    && !self.check(&TokenKind::Comma)
                    && !self.check(&TokenKind::RParen)
                    && !self.is_at_end()
                {
                    Some(Box::new(self.parse_expression_with_precedence(2)?))
                } else {
                    None
                };
                Expr::Yield { argument, delegate }
            }
            TokenKind::Clone => {
                self.advance();
                let expr = Box::new(self.parse_expression_with_precedence(14)?);
                Expr::Clone(expr)
            }
            TokenKind::DotDotDot => {
                self.advance();
                let expr = Box::new(self.parse_expression_with_precedence(2)?);
                Expr::Spread(expr)
            }
            _ => return self.parse_primary_expression(),
        };

        let span = start.merge(&self.previous_token().span);
        Ok(Node::new(expr, span))
    }

    fn parse_infix_expression(&mut self, left: Node<Expr>) -> ParseResult<Node<Expr>> {
        let start = left.span;

        let expr = match self.current_token().kind {
            // Assignment operators
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
            | TokenKind::CaretEq => {
                let op = self.parse_assignment_operator()?;
                let right = Box::new(self.parse_expression_with_precedence(1)?);
                Expr::Assignment {
                    target: Box::new(left),
                    op,
                    value: right,
                }
            }

            // Ternary operator
            TokenKind::Question => {
                self.advance();
                let then_expr = Box::new(self.parse_expression()?);
                self.consume(TokenKind::Colon)?;
                let else_expr = Box::new(self.parse_expression_with_precedence(2)?);
                Expr::Ternary {
                    condition: Box::new(left),
                    then_expr,
                    else_expr,
                }
            }

            // Binary operators
            TokenKind::PipePipe
            | TokenKind::AmpAmp
            | TokenKind::Pipe
            | TokenKind::Caret
            | TokenKind::Amp
            | TokenKind::EqEq
            | TokenKind::BangEq
            | TokenKind::EqEqEq
            | TokenKind::BangEqEq
            | TokenKind::Lt
            | TokenKind::Gt
            | TokenKind::LtEq
            | TokenKind::GtEq
            | TokenKind::In
            | TokenKind::Instanceof
            | TokenKind::LtLt
            | TokenKind::GtGt
            | TokenKind::GtGtGt
            | TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Percent
            | TokenKind::StarStar
            | TokenKind::QuestionQuestion => {
                let precedence = self.get_infix_precedence();
                let op = self.parse_binary_operator()?;
                let right = Box::new(self.parse_expression_with_precedence(precedence + 1)?);
                Expr::Binary {
                    left: Box::new(left),
                    op,
                    right,
                }
            }

            // Member access
            TokenKind::Dot => {
                self.advance();
                let property = self.parse_identifier()?;
                Expr::Member {
                    object: Box::new(left),
                    property,
                    computed: false,
                }
            }

            // Optional chaining
            TokenKind::QuestionDot => {
                self.advance();
                // ?.( for optional call
                if self.check(&TokenKind::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                        if self.check(&TokenKind::DotDotDot) {
                            let spread_start = self.current_token().span;
                            self.advance();
                            let expr = self.parse_expression()?;
                            let spread_span = spread_start.merge(&expr.span);
                            args.push(Node::new(Expr::Spread(Box::new(expr)), spread_span));
                        } else {
                            args.push(self.parse_expression()?);
                        }
                        if !self.check(&TokenKind::RParen) {
                            self.consume(TokenKind::Comma)?;
                        }
                    }
                    self.consume(TokenKind::RParen)?;
                    Expr::OptionalCall {
                        callee: Box::new(left),
                        type_args: None,
                        args,
                    }
                }
                // ?.[ for optional index
                else if self.check(&TokenKind::LBracket) {
                    self.advance();
                    let index = Box::new(self.parse_expression()?);
                    self.consume(TokenKind::RBracket)?;
                    Expr::OptionalIndex {
                        object: Box::new(left),
                        index,
                    }
                }
                // ?.property for optional member
                else {
                    let property = self.parse_identifier()?;
                    Expr::OptionalMember {
                        object: Box::new(left),
                        property,
                    }
                }
            }

            // Index access
            TokenKind::LBracket => {
                self.advance();
                let index = Box::new(self.parse_expression()?);
                self.consume(TokenKind::RBracket)?;
                Expr::Index {
                    object: Box::new(left),
                    index,
                }
            }

            // Function call
            TokenKind::LParen => {
                self.advance();
                let mut args = Vec::new();

                while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                    if self.check(&TokenKind::DotDotDot) {
                        let spread_start = self.current_token().span;
                        self.advance();
                        let expr = self.parse_expression()?;
                        let spread_span = spread_start.merge(&expr.span);
                        args.push(Node::new(Expr::Spread(Box::new(expr)), spread_span));
                    } else {
                        args.push(self.parse_expression()?);
                    }
                    if !self.check(&TokenKind::RParen) {
                        self.consume(TokenKind::Comma)?;
                    }
                }

                self.consume(TokenKind::RParen)?;
                Expr::Call {
                    callee: Box::new(left),
                    type_args: None,
                    args,
                }
            }


            // Type cast
            TokenKind::As => {
                self.advance();
                let ty = Box::new(self.parse_type()?);
                Expr::TypeCast {
                    expr: Box::new(left),
                    ty,
                }
            }

            // Satisfies operator
            TokenKind::Satisfies => {
                self.advance();
                let ty = Box::new(self.parse_type()?);
                Expr::Satisfies {
                    expr: Box::new(left),
                    ty,
                }
            }

            // Postfix operators
            TokenKind::PlusPlus => {
                self.advance();
                Expr::Unary {
                    op: UnaryOp::PostIncrement,
                    expr: Box::new(left),
                }
            }

            TokenKind::MinusMinus => {
                self.advance();
                Expr::Unary {
                    op: UnaryOp::PostDecrement,
                    expr: Box::new(left),
                }
            }

            // Non-null assertion: expr!
            TokenKind::Bang => {
                self.advance();
                Expr::NonNullAssertion(Box::new(left))
            }

            _ => return Ok(left),
        };

        let span = start.merge(&self.previous_token().span);
        Ok(Node::new(expr, span))
    }

    pub(crate) fn parse_primary_expression(&mut self) -> ParseResult<Node<Expr>> {
        let start = self.current_token().span;

        let expr = match self.current_token().kind {
            // Literals
            TokenKind::NumberLiteral => {
                let value = self.advance().value.clone();
                let num = value.parse::<f64>().unwrap_or(0.0);
                Expr::Literal(Literal::Number(num))
            }
            TokenKind::StringLiteral => {
                let value = self.advance().value.clone();
                Expr::Literal(Literal::String(value))
            }
            TokenKind::True => {
                self.advance();
                Expr::Literal(Literal::Boolean(true))
            }
            TokenKind::False => {
                self.advance();
                Expr::Literal(Literal::Boolean(false))
            }
            TokenKind::Null => {
                self.advance();
                Expr::Literal(Literal::Null)
            }
            TokenKind::Undefined => {
                self.advance();
                Expr::Literal(Literal::Undefined)
            }

            // Template literal
            TokenKind::TemplateLiteral => {
                let value = self.advance().value.clone();
                // Simple template without expressions
                Expr::Template {
                    parts: vec![value],
                    exprs: vec![],
                }
            }

            // Identifiers
            TokenKind::Identifier => {
                let name = self.advance().value.clone();
                Expr::Ident(Ident::new(name))
            }

            // This
            TokenKind::This => {
                self.advance();
                Expr::This
            }

            // Super
            TokenKind::Super => {
                self.advance();
                Expr::Super
            }

            // Array literal
            TokenKind::LBracket => {
                self.advance();
                let mut elements = Vec::new();

                while !self.check(&TokenKind::RBracket) && !self.is_at_end() {
                    if self.check(&TokenKind::Comma) {
                        elements.push(None);
                        self.advance();
                    } else {
                        elements.push(Some(self.parse_expression()?));
                        if !self.check(&TokenKind::RBracket) {
                            self.consume(TokenKind::Comma)?;
                        }
                    }
                }

                self.consume(TokenKind::RBracket)?;
                Expr::Array(elements)
            }

            // Object literal
            TokenKind::LBrace => {
                self.advance();
                let mut properties = Vec::new();

                while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                    // Spread
                    if self.check(&TokenKind::DotDotDot) {
                        self.advance();
                        let expr = self.parse_expression()?;
                        properties.push(ObjectProperty::Spread(expr));
                    } else {
                        let key = self.parse_property_name()?;

                        // Method shorthand or property
                        if self.check(&TokenKind::LParen) {
                            let type_params = self.parse_type_parameters()?;
                            self.consume(TokenKind::LParen)?;
                            let params = self.parse_function_params()?;
                            self.consume(TokenKind::RParen)?;

                            let return_type = if self.check(&TokenKind::Colon) {
                                self.advance();
                                Some(Box::new(self.parse_type()?))
                            } else {
                                None
                            };

                            let body = self.parse_block_statement()?;

                            properties.push(ObjectProperty::Method {
                                key,
                                type_params,
                                params,
                                return_type,
                                body,
                            });
                        } else if self.check(&TokenKind::Colon) {
                            self.advance();
                            let value = self.parse_expression()?;
                            properties.push(ObjectProperty::Property {
                                key,
                                value,
                                shorthand: false,
                            });
                        } else {
                            // Shorthand property
                            if let PropertyName::Ident(ident) = &key {
                                let value_expr = Expr::Ident(ident.value.clone());
                                let value = Node::new(value_expr, ident.span);
                                properties.push(ObjectProperty::Property {
                                    key,
                                    value,
                                    shorthand: true,
                                });
                            } else {
                                return Err(self.error("Invalid property shorthand".to_string()));
                            }
                        }
                    }

                    if !self.check(&TokenKind::RBrace) {
                        self.consume(TokenKind::Comma)?;
                    }
                }

                self.consume(TokenKind::RBrace)?;
                Expr::Object(properties)
            }

            // Parenthesized expression or arrow function
            TokenKind::LParen => {
                return self.parse_paren_or_arrow();
            }

            // Function expression
            TokenKind::Function => {
                self.advance();

                let name = if self.check(&TokenKind::Identifier) {
                    Some(self.parse_identifier()?)
                } else {
                    None
                };

                let type_params = self.parse_type_parameters()?;
                self.consume(TokenKind::LParen)?;
                let params = self.parse_function_params()?;
                self.consume(TokenKind::RParen)?;

                let return_type = if self.check(&TokenKind::Colon) {
                    self.advance();
                    Some(Box::new(self.parse_type()?))
                } else {
                    None
                };

                let body = Box::new(self.parse_block_statement()?);

                Expr::Function {
                    name,
                    type_params,
                    params,
                    return_type,
                    body,
                    is_async: false,
                }
            }

            // New expression
            TokenKind::New => {
                self.advance();
                let callee = Box::new(self.parse_primary_expression()?);
                let type_args = self.parse_type_arguments()?;

                let args = if self.check(&TokenKind::LParen) {
                    self.advance();
                    let mut args = Vec::new();

                    while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                        if self.check(&TokenKind::DotDotDot) {
                            let spread_start = self.current_token().span;
                            self.advance();
                            let expr = self.parse_expression()?;
                            let spread_span = spread_start.merge(&expr.span);
                            args.push(Node::new(Expr::Spread(Box::new(expr)), spread_span));
                        } else {
                            args.push(self.parse_expression()?);
                        }
                        if !self.check(&TokenKind::RParen) {
                            self.consume(TokenKind::Comma)?;
                        }
                    }

                    self.consume(TokenKind::RParen)?;
                    args
                } else {
                    Vec::new()
                };

                Expr::New {
                    callee,
                    type_args,
                    args,
                }
            }

            // Async function expression or async arrow function
            TokenKind::Async => {
                self.advance();

                if self.check(&TokenKind::Function) {
                    // async function expression
                    self.advance();

                    let name = if self.check(&TokenKind::Identifier) {
                        Some(self.parse_identifier()?)
                    } else {
                        None
                    };

                    let type_params = self.parse_type_parameters()?;
                    self.consume(TokenKind::LParen)?;
                    let params = self.parse_function_params()?;
                    self.consume(TokenKind::RParen)?;

                    let return_type = if self.check(&TokenKind::Colon) {
                        self.advance();
                        Some(Box::new(self.parse_type()?))
                    } else {
                        None
                    };

                    let body = Box::new(self.parse_block_statement()?);

                    Expr::Function {
                        name,
                        type_params,
                        params,
                        return_type,
                        body,
                        is_async: true,
                    }
                } else {
                    // async arrow function
                    return self.parse_arrow_function(None, None, true);
                }
            }

            _ => {
                return Err(self.error(format!(
                    "Unexpected token in expression: {:?}",
                    self.current_token().kind
                )))
            }
        };

        let span = start.merge(&self.previous_token().span);
        Ok(Node::new(expr, span))
    }

    fn parse_paren_or_arrow(&mut self) -> ParseResult<Node<Expr>> {
        let start = self.current_token().span;
        self.consume(TokenKind::LParen)?;

        // Empty params arrow function
        if self.check(&TokenKind::RParen) {
            self.advance();
            return self.parse_arrow_function(Some(Vec::new()), None, false);
        }

        // Try to determine if this is an arrow function or parenthesized expression
        let checkpoint = self.current;

        // Try parsing as arrow function parameters
        if let Ok(params) = self.parse_function_params_for_arrow() {
            if self.check(&TokenKind::RParen) {
                self.advance();

                // Check for optional return type annotation before =>
                let return_type = if self.check(&TokenKind::Colon) {
                    self.advance();
                    Some(Box::new(self.parse_type()?))
                } else {
                    None
                };

                if self.check(&TokenKind::FatArrow) {
                    return self.parse_arrow_function(Some(params), return_type, false);
                }
            }
        }

        // Reset and parse as parenthesized expression
        self.current = checkpoint;

        let expr = self.parse_expression()?;
        self.consume(TokenKind::RParen)?;

        // Check if it's actually an arrow function
        if self.check(&TokenKind::FatArrow) {
            // Convert expression to parameter
            let param = self.expr_to_param(expr)?;
            return self.parse_arrow_function(Some(vec![param]), None, false);
        }

        let span = start.merge(&self.previous_token().span);
        Ok(Node::new(Expr::Paren(Box::new(expr)), span))
    }

    fn parse_arrow_function(
        &mut self,
        params: Option<Vec<Param>>,
        return_type: Option<Box<Node<Type>>>,
        _is_async: bool,
    ) -> ParseResult<Node<Expr>> {
        let start = self.current_token().span;

        let params = if let Some(p) = params {
            p
        } else {
            // Single parameter without parentheses
            let ident = self.parse_identifier()?;
            let pattern = Pattern::Ident {
                name: ident.clone(),
                type_annotation: None,
                ownership: None,
            };
            vec![Param {
                pattern: Node::new(pattern, ident.span),
                type_annotation: None,
                ownership: None,
                optional: false,
                is_rest: false,
            }]
        };

        self.consume(TokenKind::FatArrow)?;

        let body = if self.check(&TokenKind::LBrace) {
            ArrowBody::Block(Box::new(self.parse_block_statement()?))
        } else {
            ArrowBody::Expr(Box::new(self.parse_expression()?))
        };

        let span = start.merge(&self.previous_token().span);
        Ok(Node::new(
            Expr::Arrow {
                type_params: None,
                params,
                return_type,
                body,
            },
            span,
        ))
    }
}
