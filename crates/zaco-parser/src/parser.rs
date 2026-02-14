//! Core Parser struct and main parsing methods

use super::*;

/// Recursive descent parser for TypeScript/Zaco
pub struct Parser {
    pub(crate) tokens: Vec<Token>,
    pub(crate) current: usize,
}

impl Parser {
    /// Creates a new parser from a token stream
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    /// Parses a complete program
    pub fn parse_program(&mut self) -> Result<Program, Vec<ParseError>> {
        let start_span = self.current_token().span;
        let mut items = Vec::new();
        let mut errors = Vec::new();

        while !self.is_at_end() {
            match self.parse_module_item() {
                Ok(item) => items.push(item),
                Err(err) => {
                    errors.push(err);
                    self.synchronize();
                }
            }
        }

        if errors.is_empty() {
            let end_span = if items.is_empty() {
                start_span.clone()
            } else {
                items.last().unwrap().span
            };
            Ok(Program {
                items,
                span: start_span.merge(&end_span),
            })
        } else {
            Err(errors)
        }
    }

    // =========================================================================
    // Module Items
    // =========================================================================

    pub(crate) fn parse_module_item(&mut self) -> ParseResult<Node<ModuleItem>> {
        let start = self.current_token().span;

        let item = match self.current_token().kind {
            TokenKind::Import => {
                let import_decl = self.parse_import_decl()?;
                let _span = start.merge(&self.previous_token().span);
                ModuleItem::Import(import_decl)
            }
            TokenKind::Export => {
                let export_decl = self.parse_export_decl()?;
                let _span = start.merge(&self.previous_token().span);
                ModuleItem::Export(export_decl)
            }
            TokenKind::Declare
            | TokenKind::Interface
            | TokenKind::Type
            | TokenKind::Enum
            | TokenKind::Module
            | TokenKind::Namespace
            | TokenKind::Abstract => {
                let decl = self.parse_declaration()?;
                ModuleItem::Decl(decl)
            }
            TokenKind::Function | TokenKind::Async => {
                let decl = self.parse_declaration()?;
                ModuleItem::Decl(decl)
            }
            TokenKind::Class | TokenKind::At => {
                let decl = self.parse_declaration()?;
                ModuleItem::Decl(decl)
            }
            TokenKind::Const | TokenKind::Let | TokenKind::Var => {
                // Could be either declaration or statement
                let stmt = self.parse_statement()?;
                ModuleItem::Stmt(stmt)
            }
            _ => {
                let stmt = self.parse_statement()?;
                ModuleItem::Stmt(stmt)
            }
        };

        let span = start.merge(&self.previous_token().span);
        Ok(Node::new(item, span))
    }

    // =========================================================================
    // Import/Export
    // =========================================================================

    pub(crate) fn parse_import_decl(&mut self) -> ParseResult<ImportDecl> {
        self.consume(TokenKind::Import)?;

        // Check for type-only import
        let type_only = if self.check(&TokenKind::Type) && self.peek_kind(1) == Some(&TokenKind::LBrace) {
            self.advance();
            true
        } else {
            false
        };

        let mut specifiers = Vec::new();

        // import "module"
        if self.check(&TokenKind::StringLiteral) {
            let source = self.advance().value.clone();
            self.consume_semicolon();
            return Ok(ImportDecl {
                specifiers,
                source,
                type_only,
            });
        }

        // import defaultName from "module"
        if self.check(&TokenKind::Identifier) && self.peek_kind(1) != Some(&TokenKind::Comma) && self.peek_kind(1) != Some(&TokenKind::As) {
            let name = self.parse_identifier()?;
            specifiers.push(ImportSpecifier::Default(name));

            if self.check(&TokenKind::Comma) {
                self.advance();
            } else {
                self.consume(TokenKind::From)?;
                let source = self.consume(TokenKind::StringLiteral)?.value.clone();
                self.consume_semicolon();
                return Ok(ImportDecl {
                    specifiers,
                    source,
                    type_only,
                });
            }
        }

        // import * as name from "module"
        if self.check(&TokenKind::Star) {
            self.advance();
            self.consume(TokenKind::As)?;
            let name = self.parse_identifier()?;
            specifiers.push(ImportSpecifier::Namespace(name));
        }
        // import { a, b as c } from "module"
        else if self.check(&TokenKind::LBrace) {
            self.advance();

            while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                let spec_type_only = if self.check(&TokenKind::Type) {
                    self.advance();
                    true
                } else {
                    false
                };

                let imported = self.parse_identifier()?;
                let local = if self.check(&TokenKind::As) {
                    self.advance();
                    Some(self.parse_identifier()?)
                } else {
                    None
                };

                specifiers.push(ImportSpecifier::Named {
                    imported,
                    local,
                    type_only: spec_type_only,
                });

                if !self.check(&TokenKind::RBrace) {
                    self.consume(TokenKind::Comma)?;
                }
            }

            self.consume(TokenKind::RBrace)?;
        }

        self.consume(TokenKind::From)?;
        let source = self.consume(TokenKind::StringLiteral)?.value.clone();
        self.consume_semicolon();

        Ok(ImportDecl {
            specifiers,
            source,
            type_only,
        })
    }

    pub(crate) fn parse_export_decl(&mut self) -> ParseResult<ExportDecl> {
        self.consume(TokenKind::Export)?;

        // export type { ... }
        let type_only = if self.check(&TokenKind::Type) && self.peek_kind(1) == Some(&TokenKind::LBrace) {
            self.advance();
            true
        } else {
            false
        };

        // export default
        if self.check(&TokenKind::Default) {
            self.advance();

            // Check if it's a declaration or expression
            if self.check(&TokenKind::Function) || self.check(&TokenKind::Class) {
                let decl = self.parse_declaration()?;
                return Ok(ExportDecl::DefaultDecl(Box::new(decl)));
            } else {
                let expr = self.parse_expression()?;
                self.consume_semicolon();
                return Ok(ExportDecl::Default(expr));
            }
        }

        // export * from "module"
        if self.check(&TokenKind::Star) {
            self.advance();

            let as_name = if self.check(&TokenKind::As) {
                self.advance();
                Some(self.parse_identifier()?)
            } else {
                None
            };

            self.consume(TokenKind::From)?;
            let source = self.consume(TokenKind::StringLiteral)?.value.clone();
            self.consume_semicolon();

            return Ok(ExportDecl::All {
                source,
                as_name,
                type_only,
            });
        }

        // export { ... }
        if self.check(&TokenKind::LBrace) {
            self.advance();
            let mut specifiers = Vec::new();

            while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                let spec_type_only = if self.check(&TokenKind::Type) {
                    self.advance();
                    true
                } else {
                    false
                };

                let local = self.parse_identifier()?;
                let exported = if self.check(&TokenKind::As) {
                    self.advance();
                    Some(self.parse_identifier()?)
                } else {
                    None
                };

                specifiers.push(ExportSpecifier {
                    local,
                    exported,
                    type_only: spec_type_only,
                });

                if !self.check(&TokenKind::RBrace) {
                    self.consume(TokenKind::Comma)?;
                }
            }

            self.consume(TokenKind::RBrace)?;

            let source = if self.check(&TokenKind::From) {
                self.advance();
                Some(self.consume(TokenKind::StringLiteral)?.value.clone())
            } else {
                None
            };

            self.consume_semicolon();

            return Ok(ExportDecl::Named {
                specifiers,
                source,
                type_only,
            });
        }

        // export declaration
        let decl = self.parse_declaration()?;
        Ok(ExportDecl::Decl(Box::new(decl)))
    }
}
