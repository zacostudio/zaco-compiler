//! Pattern parsing

use super::*;

impl Parser {
    pub(crate) fn parse_pattern(&mut self) -> ParseResult<Node<Pattern>> {
        let start = self.current_token().span;

        let pattern = match self.current_token().kind {
            TokenKind::LBracket => self.parse_array_pattern()?,
            TokenKind::LBrace => self.parse_object_pattern()?,
            TokenKind::Identifier => {
                let name = self.parse_identifier()?;
                let ownership = self.parse_ownership_annotation()?;

                let type_annotation = if self.check(&TokenKind::Colon) {
                    self.advance();
                    Some(Box::new(self.parse_type()?))
                } else {
                    None
                };

                Pattern::Ident {
                    name,
                    type_annotation,
                    ownership,
                }
            }
            _ => {
                return Err(self.error(format!(
                    "Expected pattern, found {:?}",
                    self.current_token().kind
                )))
            }
        };

        let result = Node::new(pattern, start.merge(&self.previous_token().span));

        Ok(result)
    }

    /// Parse a pattern that may have a default value (used in destructuring contexts)
    pub(crate) fn parse_pattern_with_default(&mut self) -> ParseResult<Node<Pattern>> {
        let mut result = self.parse_pattern()?;

        // Assignment pattern (default value in destructuring)
        if self.check(&TokenKind::Eq) {
            self.advance();
            let default = Box::new(self.parse_expression()?);
            let span = result.span.merge(&default.span);
            result = Node::new(
                Pattern::Assignment {
                    pattern: Box::new(result),
                    default,
                },
                span,
            );
        }

        Ok(result)
    }

    pub(crate) fn parse_array_pattern(&mut self) -> ParseResult<Pattern> {
        self.consume(TokenKind::LBracket)?;
        let mut elements = Vec::new();
        let mut rest = None;

        while !self.check(&TokenKind::RBracket) && !self.is_at_end() {
            if self.check(&TokenKind::DotDotDot) {
                self.advance();
                rest = Some(Box::new(self.parse_pattern()?));
                break;
            } else if self.check(&TokenKind::Comma) {
                elements.push(None);
                self.advance();
            } else {
                elements.push(Some(self.parse_pattern_with_default()?));
                if !self.check(&TokenKind::RBracket) {
                    self.consume(TokenKind::Comma)?;
                }
            }
        }

        self.consume(TokenKind::RBracket)?;

        Ok(Pattern::Array { elements, rest })
    }

    pub(crate) fn parse_object_pattern(&mut self) -> ParseResult<Pattern> {
        self.consume(TokenKind::LBrace)?;
        let mut properties = Vec::new();
        let mut rest = None;

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            if self.check(&TokenKind::DotDotDot) {
                self.advance();
                rest = Some(Box::new(self.parse_pattern()?));
                break;
            }

            let key = self.parse_property_name()?;

            let (value, shorthand) = if self.check(&TokenKind::Colon) {
                self.advance();
                (self.parse_pattern()?, false)
            } else {
                // Shorthand
                if let PropertyName::Ident(ref ident) = key {
                    let pattern = Pattern::Ident {
                        name: ident.clone(),
                        type_annotation: None,
                        ownership: None,
                    };
                    (Node::new(pattern, ident.span), true)
                } else {
                    return Err(self.error("Invalid object pattern shorthand".to_string()));
                }
            };

            properties.push(ObjectPatternProperty {
                key,
                value,
                shorthand,
            });

            if !self.check(&TokenKind::RBrace) {
                self.consume(TokenKind::Comma)?;
            }
        }

        self.consume(TokenKind::RBrace)?;

        Ok(Pattern::Object { properties, rest })
    }

    // =========================================================================
    // Helper Parsers
    // =========================================================================

}
