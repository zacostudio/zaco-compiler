//! Type annotation parsing

use super::*;

impl Parser {
    pub(crate) fn parse_type(&mut self) -> ParseResult<Node<Type>> {
        self.parse_union_type()
    }

    pub(crate) fn parse_union_type(&mut self) -> ParseResult<Node<Type>> {
        let start = self.current_token().span;
        let mut types = vec![self.parse_intersection_type()?];

        while self.check(&TokenKind::Pipe) {
            self.advance();
            types.push(self.parse_intersection_type()?);
        }

        if types.len() == 1 {
            Ok(types.into_iter().next().unwrap())
        } else {
            let span = start.merge(&self.previous_token().span);
            Ok(Node::new(Type::Union(types), span))
        }
    }

    pub(crate) fn parse_intersection_type(&mut self) -> ParseResult<Node<Type>> {
        let start = self.current_token().span;
        let mut types = vec![self.parse_primary_type()?];

        while self.check(&TokenKind::Amp) {
            self.advance();
            types.push(self.parse_primary_type()?);
        }

        if types.len() == 1 {
            Ok(types.into_iter().next().unwrap())
        } else {
            let span = start.merge(&self.previous_token().span);
            Ok(Node::new(Type::Intersection(types), span))
        }
    }

    pub(crate) fn parse_primary_type(&mut self) -> ParseResult<Node<Type>> {
        let start = self.current_token().span;

        // Check for ownership prefix
        let ownership = self.parse_ownership_annotation()?;

        let mut ty = self.parse_base_type()?;

        // Apply ownership if present
        if let Some(ownership) = ownership {
            let span = start.merge(&self.previous_token().span);
            ty = Node::new(
                Type::WithOwnership {
                    base: Box::new(ty),
                    ownership,
                },
                span,
            );
        }

        // Array suffix and indexed access types
        while self.check(&TokenKind::LBracket) {
            if self.peek_kind(1) == Some(&TokenKind::RBracket) {
                // Array type: T[]
                self.advance();
                self.advance();
                let span = start.merge(&self.previous_token().span);
                ty = Node::new(Type::Array(Box::new(ty)), span);
            } else {
                // Indexed access type: T[K]
                self.advance();
                let index_type = Box::new(self.parse_type()?);
                self.consume(TokenKind::RBracket)?;
                let span = start.merge(&self.previous_token().span);
                ty = Node::new(Type::IndexedAccess {
                    object_type: Box::new(ty),
                    index_type,
                }, span);
            }
        }

        // Conditional type: T extends U ? X : Y
        if self.check(&TokenKind::Extends) {
            self.advance();
            let extends_type = Box::new(self.parse_primary_type()?);
            self.consume(TokenKind::Question)?;
            let true_type = Box::new(self.parse_type()?);
            self.consume(TokenKind::Colon)?;
            let false_type = Box::new(self.parse_type()?);
            let span = start.merge(&self.previous_token().span);
            ty = Node::new(Type::Conditional {
                check_type: Box::new(ty),
                extends_type,
                true_type,
                false_type,
            }, span);
        }

        Ok(ty)
    }

    pub(crate) fn parse_base_type(&mut self) -> ParseResult<Node<Type>> {
        let start = self.current_token().span;

        let ty = match self.current_token().kind {
            // Primitive types
            TokenKind::Identifier => {
                let name = self.current_token().value.clone();
                let primitive = match name.as_str() {
                    "number" => Some(PrimitiveType::Number),
                    "string" => Some(PrimitiveType::String),
                    "boolean" => Some(PrimitiveType::Boolean),
                    "void" => Some(PrimitiveType::Void),
                    "null" => Some(PrimitiveType::Null),
                    "undefined" => Some(PrimitiveType::Undefined),
                    "any" => Some(PrimitiveType::Any),
                    "never" => Some(PrimitiveType::Never),
                    "unknown" => Some(PrimitiveType::Unknown),
                    _ => None,
                };

                if let Some(prim) = primitive {
                    self.advance();
                    Type::Primitive(prim)
                } else {
                    let name = self.parse_identifier()?;
                    let type_args = self.parse_type_arguments()?;
                    Type::TypeRef { name, type_args }
                }
            }

            // Function type
            TokenKind::LParen => {
                self.advance();
                let mut params = Vec::new();

                while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                    let _is_rest = if self.check(&TokenKind::DotDotDot) {
                        self.advance();
                        true
                    } else {
                        false
                    };

                    let name = if self.check(&TokenKind::Identifier) && self.peek_kind(1) == Some(&TokenKind::Colon) {
                        let n = self.parse_identifier()?;
                        self.consume(TokenKind::Colon)?;
                        Some(n)
                    } else {
                        None
                    };

                    let ownership = self.parse_ownership_annotation()?;
                    let ty = self.parse_type()?;

                    let optional = if self.check(&TokenKind::Question) {
                        self.advance();
                        true
                    } else {
                        false
                    };

                    params.push(FunctionTypeParam {
                        name,
                        ty,
                        optional,
                        ownership,
                    });

                    if !self.check(&TokenKind::RParen) {
                        self.consume(TokenKind::Comma)?;
                    }
                }

                self.consume(TokenKind::RParen)?;
                self.consume(TokenKind::FatArrow)?;
                let return_type = Box::new(self.parse_type()?);

                Type::Function(FunctionType {
                    type_params: None,
                    params,
                    return_type,
                })
            }

            // Object type or mapped type
            TokenKind::LBrace => {
                self.advance();

                // Check for mapped type: { [K in ...]: V } or { +/-readonly [K in ...]: V }
                // First check if it starts with +/- readonly or [
                let is_mapped = self.check(&TokenKind::LBracket)
                    || (self.check(&TokenKind::Plus) && self.peek_kind(1) == Some(&TokenKind::Readonly))
                    || (self.check(&TokenKind::Minus) && self.peek_kind(1) == Some(&TokenKind::Readonly))
                    || self.check(&TokenKind::Readonly);

                if is_mapped {
                    let saved = self.current;

                    // Check for optional +/- readonly BEFORE [
                    let readonly = if self.check(&TokenKind::Plus) {
                        self.advance();
                        if self.check(&TokenKind::Readonly) {
                            self.advance();
                            Some(MappedModifier::Add)
                        } else {
                            // Not a mapped type, restore position
                            self.current = saved;
                            let mut members = Vec::new();
                            while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                                members.push(self.parse_object_type_member()?);
                            }
                            self.consume(TokenKind::RBrace)?;
                            return Ok(Node::new(Type::Object(ObjectType { members }), start.merge(&self.previous_token().span)));
                        }
                    } else if self.check(&TokenKind::Minus) {
                        self.advance();
                        if self.check(&TokenKind::Readonly) {
                            self.advance();
                            Some(MappedModifier::Remove)
                        } else {
                            self.current = saved;
                            let mut members = Vec::new();
                            while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                                members.push(self.parse_object_type_member()?);
                            }
                            self.consume(TokenKind::RBrace)?;
                            return Ok(Node::new(Type::Object(ObjectType { members }), start.merge(&self.previous_token().span)));
                        }
                    } else if self.check(&TokenKind::Readonly) {
                        self.advance();
                        Some(MappedModifier::Present)
                    } else {
                        None
                    };

                    // Now we should see [
                    if !self.check(&TokenKind::LBracket) {
                        // Not a mapped type after all, restore and parse as object
                        self.current = saved;
                        let mut members = Vec::new();
                        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                            members.push(self.parse_object_type_member()?);
                        }
                        self.consume(TokenKind::RBrace)?;
                        return Ok(Node::new(Type::Object(ObjectType { members }), start.merge(&self.previous_token().span)));
                    }

                    self.advance(); // skip [

                    if self.check(&TokenKind::Identifier) && self.peek_kind(1) == Some(&TokenKind::In) {
                        // This is a mapped type
                        let type_param = self.parse_identifier()?;
                        self.consume(TokenKind::In)?;
                        let constraint = Box::new(self.parse_type()?);

                        // Optional 'as' for key remapping
                        let name_type = if self.check(&TokenKind::As) {
                            self.advance();
                            Some(Box::new(self.parse_type()?))
                        } else {
                            None
                        };

                        self.consume(TokenKind::RBracket)?;

                        // Optional +/- ?
                        let optional = if self.check(&TokenKind::Question) {
                            self.advance();
                            Some(MappedModifier::Present)
                        } else if self.check(&TokenKind::Plus) {
                            self.advance();
                            self.consume(TokenKind::Question)?;
                            Some(MappedModifier::Add)
                        } else if self.check(&TokenKind::Minus) {
                            self.advance();
                            self.consume(TokenKind::Question)?;
                            Some(MappedModifier::Remove)
                        } else {
                            None
                        };

                        self.consume(TokenKind::Colon)?;
                        let value_type = Box::new(self.parse_type()?);

                        // Optional semicolon
                        if self.check(&TokenKind::Semicolon) {
                            self.advance();
                        }

                        self.consume(TokenKind::RBrace)?;
                        Type::Mapped {
                            type_param,
                            constraint,
                            name_type,
                            value_type,
                            readonly,
                            optional,
                        }
                    } else {
                        // Not a mapped type, restore and parse as object
                        self.current = saved;
                        let mut members = Vec::new();
                        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                            members.push(self.parse_object_type_member()?);
                        }
                        self.consume(TokenKind::RBrace)?;
                        Type::Object(ObjectType { members })
                    }
                } else {
                    // Regular object type
                    let mut members = Vec::new();
                    while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                        members.push(self.parse_object_type_member()?);
                    }
                    self.consume(TokenKind::RBrace)?;
                    Type::Object(ObjectType { members })
                }
            }

            // Tuple type
            TokenKind::LBracket => {
                self.advance();
                let mut types = Vec::new();

                while !self.check(&TokenKind::RBracket) && !self.is_at_end() {
                    types.push(self.parse_type()?);
                    if !self.check(&TokenKind::RBracket) {
                        self.consume(TokenKind::Comma)?;
                    }
                }

                self.consume(TokenKind::RBracket)?;
                Type::Tuple(types)
            }

            // Keyword types that the lexer tokenizes as keywords, not identifiers
            TokenKind::Void => {
                self.advance();
                Type::Primitive(PrimitiveType::Void)
            }
            TokenKind::Null => {
                self.advance();
                Type::Primitive(PrimitiveType::Null)
            }
            TokenKind::Undefined => {
                self.advance();
                Type::Primitive(PrimitiveType::Undefined)
            }
            TokenKind::Any => {
                self.advance();
                Type::Primitive(PrimitiveType::Any)
            }
            TokenKind::Never => {
                self.advance();
                Type::Primitive(PrimitiveType::Never)
            }
            TokenKind::Unknown => {
                self.advance();
                Type::Primitive(PrimitiveType::Unknown)
            }

            // Literal types
            TokenKind::StringLiteral => {
                let value = self.advance().value.clone();
                Type::Literal(LiteralType::String(value))
            }
            TokenKind::NumberLiteral => {
                let value = self.advance().value.clone();
                let num = value.parse::<f64>().unwrap_or(0.0);
                Type::Literal(LiteralType::Number(num))
            }
            TokenKind::TemplateLiteral => {
                let value = self.advance().value.clone();
                Type::TemplateLiteral {
                    parts: vec![value],
                    types: vec![],
                }
            }
            TokenKind::True => {
                self.advance();
                Type::Literal(LiteralType::Boolean(true))
            }
            TokenKind::False => {
                self.advance();
                Type::Literal(LiteralType::Boolean(false))
            }

            // keyof type: keyof T
            TokenKind::Keyof => {
                self.advance();
                let ty = Box::new(self.parse_primary_type()?);
                Type::Keyof(ty)
            }

            // typeof in type position: typeof someVar
            TokenKind::Typeof => {
                self.advance();
                let name = self.parse_identifier()?;
                let type_args = self.parse_type_arguments()?;
                let inner = Box::new(Node::new(Type::TypeRef { name, type_args }, start.merge(&self.previous_token().span)));
                Type::TypeofType(inner)
            }

            // infer type: infer T
            TokenKind::Infer => {
                self.advance();
                let name = self.parse_identifier()?;
                Type::Infer(name)
            }

            // import type: import("module").Type
            TokenKind::Import => {
                self.advance();
                self.consume(TokenKind::LParen)?;
                let argument = self.consume(TokenKind::StringLiteral)?.value.clone();
                self.consume(TokenKind::RParen)?;
                let qualifier = if self.check(&TokenKind::Dot) {
                    self.advance();
                    let name = self.parse_identifier()?;
                    let type_args = self.parse_type_arguments()?;
                    Some(Box::new(Node::new(Type::TypeRef { name, type_args }, start.merge(&self.previous_token().span))))
                } else {
                    None
                };
                let type_args = self.parse_type_arguments()?;
                Type::ImportType {
                    argument,
                    qualifier,
                    type_args,
                }
            }

            _ => {
                return Err(self.error(format!(
                    "Expected type, found {:?}",
                    self.current_token().kind
                )))
            }
        };

        let span = start.merge(&self.previous_token().span);
        Ok(Node::new(ty, span))
    }

    pub(crate) fn parse_object_type_member(&mut self) -> ParseResult<ObjectTypeMember> {
        // Index signature
        if self.check(&TokenKind::LBracket) {
            self.advance();
            let key_name = self.parse_identifier()?;
            self.consume(TokenKind::Colon)?;
            let key_type = self.parse_type()?;
            self.consume(TokenKind::RBracket)?;
            self.consume(TokenKind::Colon)?;
            let value_type = self.parse_type()?;
            self.consume_semicolon();

            return Ok(ObjectTypeMember::IndexSignature {
                key_name,
                key_type,
                value_type,
            });
        }

        // Call signature
        if self.check(&TokenKind::LParen) {
            let type_params = self.parse_type_parameters()?;
            self.consume(TokenKind::LParen)?;
            let params = self.parse_function_type_params()?;
            self.consume(TokenKind::RParen)?;
            self.consume(TokenKind::Colon)?;
            let return_type = self.parse_type()?;
            self.consume_semicolon();

            return Ok(ObjectTypeMember::CallSignature {
                type_params,
                params,
                return_type,
            });
        }

        let readonly = if self.check(&TokenKind::Readonly) {
            self.advance();
            true
        } else {
            false
        };

        let name = self.parse_property_name()?;
        let optional = if self.check(&TokenKind::Question) {
            self.advance();
            true
        } else {
            false
        };

        // Method signature
        if self.check(&TokenKind::LParen) || self.check(&TokenKind::Lt) {
            let type_params = self.parse_type_parameters()?;
            self.consume(TokenKind::LParen)?;
            let params = self.parse_function_type_params()?;
            self.consume(TokenKind::RParen)?;
            self.consume(TokenKind::Colon)?;
            let return_type = self.parse_type()?;
            self.consume_semicolon();

            Ok(ObjectTypeMember::Method {
                name,
                type_params,
                params,
                return_type,
                optional,
            })
        } else {
            // Property signature
            self.consume(TokenKind::Colon)?;
            let ty = self.parse_type()?;
            self.consume_semicolon();

            Ok(ObjectTypeMember::Property {
                name,
                ty,
                optional,
                readonly,
            })
        }
    }

    pub(crate) fn parse_function_type_params(&mut self) -> ParseResult<Vec<FunctionTypeParam>> {
        let mut params = Vec::new();

        while !self.check(&TokenKind::RParen) && !self.is_at_end() {
            let name = if self.check(&TokenKind::Identifier) && self.peek_kind(1) == Some(&TokenKind::Colon) {
                let n = self.parse_identifier()?;
                self.consume(TokenKind::Colon)?;
                Some(n)
            } else {
                None
            };

            let ownership = self.parse_ownership_annotation()?;
            let ty = self.parse_type()?;

            let optional = if self.check(&TokenKind::Question) {
                self.advance();
                true
            } else {
                false
            };

            params.push(FunctionTypeParam {
                name,
                ty,
                optional,
                ownership,
            });

            if !self.check(&TokenKind::RParen) {
                self.consume(TokenKind::Comma)?;
            }
        }

        Ok(params)
    }

    pub(crate) fn parse_type_parameters(&mut self) -> ParseResult<Option<Vec<TypeParam>>> {
        if !self.check(&TokenKind::Lt) {
            return Ok(None);
        }

        self.advance();
        let mut params = Vec::new();

        while !self.check(&TokenKind::Gt) && !self.is_at_end() {
            let name = self.parse_identifier()?;

            let constraint = if self.check(&TokenKind::Extends) {
                self.advance();
                Some(Box::new(self.parse_type()?))
            } else {
                None
            };

            let default = if self.check(&TokenKind::Eq) {
                self.advance();
                Some(Box::new(self.parse_type()?))
            } else {
                None
            };

            params.push(TypeParam {
                name,
                constraint,
                default,
            });

            if !self.check(&TokenKind::Gt) {
                self.consume(TokenKind::Comma)?;
            }
        }

        self.consume(TokenKind::Gt)?;
        Ok(Some(params))
    }

    pub(crate) fn parse_type_arguments(&mut self) -> ParseResult<Option<Vec<Node<Type>>>> {
        if !self.check(&TokenKind::Lt) || !self.is_type_argument_start() {
            return Ok(None);
        }

        self.advance();
        let mut args = Vec::new();

        while !self.check(&TokenKind::Gt) && !self.is_at_end() {
            args.push(self.parse_type()?);
            if !self.check(&TokenKind::Gt) {
                self.consume(TokenKind::Comma)?;
            }
        }

        self.consume(TokenKind::Gt)?;
        Ok(Some(args))
    }

    // =========================================================================
    // Patterns
    // =========================================================================

}
