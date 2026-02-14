//! Declaration parsing

use super::*;

impl Parser {
    pub(crate) fn parse_declaration(&mut self) -> ParseResult<Node<Decl>> {
        let start = self.current_token().span;

        // Parse decorators
        let decorators = self.parse_decorators()?;

        // Handle 'declare' modifier
        let is_declare = if self.check(&TokenKind::Declare) {
            self.advance();
            true
        } else {
            false
        };

        let decl = match self.current_token().kind {
            TokenKind::Function | TokenKind::Async => {
                let func_decl = self.parse_function_declaration(is_declare)?;
                Decl::Function(func_decl)
            }
            TokenKind::Class | TokenKind::Abstract => {
                let mut class_decl = self.parse_class_declaration(is_declare)?;
                class_decl.decorators = decorators;
                Decl::Class(class_decl)
            }
            TokenKind::Interface => {
                let interface_decl = self.parse_interface_declaration(is_declare)?;
                Decl::Interface(interface_decl)
            }
            TokenKind::Type => {
                let type_alias_decl = self.parse_type_alias_declaration(is_declare)?;
                Decl::TypeAlias(type_alias_decl)
            }
            TokenKind::Enum => {
                let enum_decl = self.parse_enum_declaration(is_declare)?;
                Decl::Enum(enum_decl)
            }
            TokenKind::Module | TokenKind::Namespace => {
                let module_decl = self.parse_module_declaration(is_declare)?;
                Decl::Module(module_decl)
            }
            TokenKind::Const | TokenKind::Let | TokenKind::Var => {
                let var_decl = self.parse_var_declaration()?;
                Decl::Var(var_decl)
            }
            _ => {
                return Err(self.error(format!(
                    "Expected declaration, found {:?}",
                    self.current_token().kind
                )))
            }
        };

        let span = start.merge(&self.previous_token().span);
        Ok(Node::new(decl, span))
    }

    pub(crate) fn parse_function_declaration(&mut self, is_declare: bool) -> ParseResult<FunctionDecl> {
        let is_async = if self.check(&TokenKind::Async) {
            self.advance();
            true
        } else {
            false
        };

        self.consume(TokenKind::Function)?;

        let is_generator = if self.check(&TokenKind::Star) {
            self.advance();
            true
        } else {
            false
        };

        let name = self.parse_identifier()?;
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

        let body = if is_declare || self.check(&TokenKind::Semicolon) {
            self.consume_semicolon();
            None
        } else {
            Some(self.parse_block_statement()?)
        };

        Ok(FunctionDecl {
            name,
            type_params,
            params,
            return_type,
            body,
            is_async,
            is_generator,
            is_declare,
        })
    }

    pub(crate) fn parse_class_declaration(&mut self, is_declare: bool) -> ParseResult<ClassDecl> {
        let is_abstract = if self.check(&TokenKind::Abstract) {
            self.advance();
            true
        } else {
            false
        };

        self.consume(TokenKind::Class)?;
        let name = self.parse_identifier()?;
        let type_params = self.parse_type_parameters()?;

        let extends = if self.check(&TokenKind::Extends) {
            self.advance();
            let base = Box::new(self.parse_primary_expression()?);
            let type_args = self.parse_type_arguments()?;
            Some(ClassExtends { base, type_args })
        } else {
            None
        };

        let mut implements = Vec::new();
        if self.check(&TokenKind::Implements) {
            self.advance();
            loop {
                implements.push(self.parse_type()?);
                if !self.check(&TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
        }

        self.consume(TokenKind::LBrace)?;
        let mut members = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            members.push(self.parse_class_member()?);
        }

        self.consume(TokenKind::RBrace)?;

        Ok(ClassDecl {
            name,
            type_params,
            extends,
            implements,
            members,
            is_abstract,
            is_declare,
            decorators: vec![],
        })
    }

    pub(crate) fn parse_class_member(&mut self) -> ParseResult<ClassMember> {
        // Parse decorators
        let decorators = self.parse_decorators()?;

        // Parse modifiers
        let mut access = AccessModifier::Public;
        let mut is_static = false;
        let mut is_readonly = false;
        let mut is_abstract = false;
        let mut is_override = false;

        loop {
            match self.current_token().kind {
                TokenKind::Public => {
                    self.advance();
                    access = AccessModifier::Public;
                }
                TokenKind::Private => {
                    self.advance();
                    access = AccessModifier::Private;
                }
                TokenKind::Protected => {
                    self.advance();
                    access = AccessModifier::Protected;
                }
                TokenKind::Static => {
                    self.advance();
                    is_static = true;
                }
                TokenKind::Readonly => {
                    self.advance();
                    is_readonly = true;
                }
                TokenKind::Abstract => {
                    self.advance();
                    is_abstract = true;
                }
                TokenKind::Override => {
                    self.advance();
                    is_override = true;
                }
                _ => break,
            }
        }

        // Constructor
        if self.check(&TokenKind::Identifier) && self.current_token().value == "constructor" {
            self.advance();
            self.consume(TokenKind::LParen)?;
            let params = self.parse_function_params()?;
            self.consume(TokenKind::RParen)?;

            let body = if self.check(&TokenKind::Semicolon) {
                self.advance();
                None
            } else {
                Some(self.parse_block_statement()?)
            };

            return Ok(ClassMember::Constructor {
                params,
                body,
                access,
            });
        }

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

            return Ok(ClassMember::IndexSignature {
                key_name,
                key_type,
                value_type,
                is_readonly,
            });
        }

        // Get/Set/Method/Property
        // Only treat `get`/`set` as accessor modifiers if next token is NOT `(`
        // `get propName()` → getter; `get()` → regular method named "get"
        let is_getter = self.check(&TokenKind::Identifier)
            && self.current_token().value == "get"
            && self.peek_kind(1) != Some(&TokenKind::LParen);
        let is_setter = self.check(&TokenKind::Identifier)
            && self.current_token().value == "set"
            && self.peek_kind(1) != Some(&TokenKind::LParen);

        if is_getter {
            self.advance();
        } else if is_setter {
            self.advance();
        }

        let name = self.parse_property_name()?;
        let is_optional = if self.check(&TokenKind::Question) {
            self.advance();
            true
        } else {
            false
        };

        // Getter
        if is_getter {
            self.consume(TokenKind::LParen)?;
            self.consume(TokenKind::RParen)?;

            let return_type = if self.check(&TokenKind::Colon) {
                self.advance();
                Some(Box::new(self.parse_type()?))
            } else {
                None
            };

            let body = if self.check(&TokenKind::Semicolon) {
                self.advance();
                None
            } else {
                Some(self.parse_block_statement()?)
            };

            return Ok(ClassMember::Getter {
                name,
                return_type,
                body,
                access,
                is_static,
                is_abstract,
            });
        }

        // Setter
        if is_setter {
            self.consume(TokenKind::LParen)?;
            let param = self.parse_function_param()?;
            self.consume(TokenKind::RParen)?;

            let body = if self.check(&TokenKind::Semicolon) {
                self.advance();
                None
            } else {
                Some(self.parse_block_statement()?)
            };

            return Ok(ClassMember::Setter {
                name,
                param,
                body,
                access,
                is_static,
                is_abstract,
            });
        }

        // Method or Property
        if self.check(&TokenKind::LParen) || self.check(&TokenKind::Lt) {
            // Method
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

            let body = if self.check(&TokenKind::Semicolon) {
                self.advance();
                None
            } else {
                Some(self.parse_block_statement()?)
            };

            Ok(ClassMember::Method {
                name,
                type_params,
                params,
                return_type,
                body,
                access,
                is_static,
                is_async: false,
                is_abstract,
                is_optional,
                is_override,
                decorators,
            })
        } else {
            // Property
            let ownership = self.parse_ownership_annotation()?;

            let type_annotation = if self.check(&TokenKind::Colon) {
                self.advance();
                Some(Box::new(self.parse_type()?))
            } else {
                None
            };

            let init = if self.check(&TokenKind::Eq) {
                self.advance();
                Some(self.parse_expression()?)
            } else {
                None
            };

            self.consume_semicolon();

            Ok(ClassMember::Property {
                name,
                type_annotation,
                ownership,
                init,
                access,
                is_static,
                is_readonly,
                is_abstract,
                is_optional,
                is_override,
                decorators: decorators.clone(),
            })
        }
    }

    pub(crate) fn parse_decorators(&mut self) -> ParseResult<Vec<Node<Expr>>> {
        let mut decorators = Vec::new();
        while self.check(&TokenKind::At) {
            let start = self.current_token().span;
            self.advance(); // skip @
            let expr = self.parse_expression_with_precedence(17)?; // high precedence, just member access + calls
            let span = start.merge(&expr.span);
            decorators.push(Node::new(expr.value, span));
        }
        Ok(decorators)
    }

    pub(crate) fn parse_interface_declaration(&mut self, is_declare: bool) -> ParseResult<InterfaceDecl> {
        self.consume(TokenKind::Interface)?;
        let name = self.parse_identifier()?;
        let type_params = self.parse_type_parameters()?;

        let mut extends = Vec::new();
        if self.check(&TokenKind::Extends) {
            self.advance();
            loop {
                extends.push(self.parse_type()?);
                if !self.check(&TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
        }

        self.consume(TokenKind::LBrace)?;
        let mut members = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            members.push(self.parse_object_type_member()?);
        }

        self.consume(TokenKind::RBrace)?;

        Ok(InterfaceDecl {
            name,
            type_params,
            extends,
            members,
            is_declare,
        })
    }

    pub(crate) fn parse_type_alias_declaration(&mut self, is_declare: bool) -> ParseResult<TypeAliasDecl> {
        self.consume(TokenKind::Type)?;
        let name = self.parse_identifier()?;
        let type_params = self.parse_type_parameters()?;
        self.consume(TokenKind::Eq)?;
        let ty = self.parse_type()?;
        self.consume_semicolon();

        Ok(TypeAliasDecl {
            name,
            type_params,
            ty,
            is_declare,
        })
    }

    pub(crate) fn parse_enum_declaration(&mut self, is_declare: bool) -> ParseResult<EnumDecl> {
        let is_const = if self.check(&TokenKind::Const) {
            self.advance();
            true
        } else {
            false
        };

        self.consume(TokenKind::Enum)?;
        let name = self.parse_identifier()?;
        self.consume(TokenKind::LBrace)?;

        let mut members = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let member_name = self.parse_identifier()?;
            let init = if self.check(&TokenKind::Eq) {
                self.advance();
                Some(self.parse_expression()?)
            } else {
                None
            };

            members.push(EnumMember {
                name: member_name,
                init,
            });

            if !self.check(&TokenKind::RBrace) {
                self.consume(TokenKind::Comma)?;
            }
        }

        self.consume(TokenKind::RBrace)?;

        Ok(EnumDecl {
            name,
            members,
            is_const,
            is_declare,
        })
    }

    pub(crate) fn parse_module_declaration(&mut self, is_declare: bool) -> ParseResult<ModuleDecl> {
        let is_namespace = self.check(&TokenKind::Namespace);
        if !is_namespace {
            self.consume(TokenKind::Module)?;
        } else {
            self.advance();
        }

        let name = if self.check(&TokenKind::StringLiteral) {
            ModuleName::String(self.advance().value.clone())
        } else {
            ModuleName::Ident(self.parse_identifier()?)
        };

        let body = if self.check(&TokenKind::Dot) {
            self.advance();
            let nested_start = self.current_token().span;
            let nested = self.parse_module_declaration(false)?;
            let span = nested_start.merge(&self.previous_token().span);
            ModuleBody::Namespace(Box::new(Node::new(nested, span)))
        } else {
            self.consume(TokenKind::LBrace)?;
            let mut items = Vec::new();

            while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                items.push(self.parse_module_item()?);
            }

            self.consume(TokenKind::RBrace)?;
            ModuleBody::Block(items)
        };

        Ok(ModuleDecl {
            name,
            body,
            is_declare,
        })
    }

    // =========================================================================
    // Statements
    // =========================================================================

}
