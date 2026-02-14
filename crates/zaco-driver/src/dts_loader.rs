//! TypeScript .d.ts declaration file loader
//!
//! Extracts type declarations from .d.ts files for type checking.
//! This is a simplified parser that extracts function signatures,
//! interfaces, and type aliases.

use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub enum DtsDeclaration {
    Function {
        name: String,
        params: Vec<(String, String)>,
        return_type: String,
    },
    Interface {
        name: String,
        members: Vec<(String, String)>,
    },
    Variable {
        name: String,
        type_annotation: String,
    },
    TypeAlias {
        name: String,
        definition: String,
    },
}

pub struct DtsLoader;

impl DtsLoader {
    /// Load type declarations from a .d.ts file
    pub fn load_declarations(path: &Path) -> Result<Vec<DtsDeclaration>, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read .d.ts file {}: {}", path.display(), e))?;

        Self::parse_declarations(&content)
    }

    /// Parse declarations from .d.ts file content
    fn parse_declarations(content: &str) -> Result<Vec<DtsDeclaration>, String> {
        let mut declarations = Vec::new();

        // Simple line-by-line parsing
        // This is a minimal implementation - a full TypeScript parser would be more robust
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with("//") || line.starts_with("/*") {
                i += 1;
                continue;
            }

            // Match export patterns
            if line.starts_with("export ") {
                if let Some(decl) = Self::parse_export_line(line) {
                    declarations.push(decl);
                } else if line.starts_with("export function") || line.starts_with("export declare function") {
                    // Multi-line function - collect until we find the closing
                    let func_decl = Self::collect_function_declaration(&lines, &mut i)?;
                    if let Some(decl) = Self::parse_function_declaration(&func_decl) {
                        declarations.push(decl);
                    }
                } else if line.starts_with("export interface") || line.starts_with("export declare interface") {
                    // Multi-line interface - collect until closing brace
                    let interface_decl = Self::collect_interface_declaration(&lines, &mut i)?;
                    if let Some(decl) = Self::parse_interface_declaration(&interface_decl) {
                        declarations.push(decl);
                    }
                } else if line.starts_with("export type") {
                    // Type alias
                    let type_decl = Self::collect_type_declaration(&lines, &mut i)?;
                    if let Some(decl) = Self::parse_type_declaration(&type_decl) {
                        declarations.push(decl);
                    }
                }
            } else if line.starts_with("declare ") {
                // Handle declare statements (common in .d.ts files)
                if line.starts_with("declare function") {
                    let func_decl = Self::collect_function_declaration(&lines, &mut i)?;
                    if let Some(decl) = Self::parse_function_declaration(&func_decl) {
                        declarations.push(decl);
                    }
                } else if line.starts_with("declare interface") {
                    let interface_decl = Self::collect_interface_declaration(&lines, &mut i)?;
                    if let Some(decl) = Self::parse_interface_declaration(&interface_decl) {
                        declarations.push(decl);
                    }
                } else if line.starts_with("declare type") {
                    let type_decl = Self::collect_type_declaration(&lines, &mut i)?;
                    if let Some(decl) = Self::parse_type_declaration(&type_decl) {
                        declarations.push(decl);
                    }
                } else if line.starts_with("declare const") || line.starts_with("declare let") || line.starts_with("declare var") {
                    if let Some(decl) = Self::parse_variable_declaration(line) {
                        declarations.push(decl);
                    }
                }
            }

            i += 1;
        }

        Ok(declarations)
    }

    /// Parse a single-line export statement
    fn parse_export_line(line: &str) -> Option<DtsDeclaration> {
        // export const foo: string;
        if line.contains("const ") || line.contains("let ") || line.contains("var ") {
            Self::parse_variable_declaration(line)
        } else {
            None
        }
    }

    /// Collect a multi-line function declaration
    fn collect_function_declaration(lines: &[&str], i: &mut usize) -> Result<String, String> {
        let mut decl = String::new();
        let start = *i;

        while *i < lines.len() {
            let line = lines[*i].trim();
            decl.push_str(line);
            decl.push(' ');

            if line.contains(';') || line.ends_with(')') {
                break;
            }

            *i += 1;
        }

        if *i >= lines.len() && !decl.contains(';') {
            return Err(format!("Unterminated function declaration at line {}", start));
        }

        Ok(decl)
    }

    /// Collect a multi-line interface declaration
    fn collect_interface_declaration(lines: &[&str], i: &mut usize) -> Result<String, String> {
        let mut decl = String::new();
        let mut brace_count = 0;
        let start = *i;

        while *i < lines.len() {
            let line = lines[*i].trim();
            decl.push_str(line);
            decl.push('\n');

            for ch in line.chars() {
                if ch == '{' {
                    brace_count += 1;
                } else if ch == '}' {
                    brace_count -= 1;
                }
            }

            if brace_count == 0 && line.contains('{') {
                break;
            }

            *i += 1;
        }

        if brace_count != 0 {
            return Err(format!("Unmatched braces in interface at line {}", start));
        }

        Ok(decl)
    }

    /// Collect a type alias declaration
    fn collect_type_declaration(lines: &[&str], i: &mut usize) -> Result<String, String> {
        let mut decl = String::new();

        while *i < lines.len() {
            let line = lines[*i].trim();
            decl.push_str(line);
            decl.push(' ');

            if line.contains(';') {
                break;
            }

            *i += 1;
        }

        Ok(decl)
    }

    /// Parse function declaration
    fn parse_function_declaration(decl: &str) -> Option<DtsDeclaration> {
        // Extract function name and signature
        // Pattern: (export )?(declare )?function name(params): returnType
        let decl = decl.replace("export ", "").replace("declare ", "");

        let func_start = decl.find("function ")?;
        let after_func = &decl[func_start + 9..];

        let paren_start = after_func.find('(')?;
        let name = after_func[..paren_start].trim().to_string();

        let paren_end = after_func.rfind(')')?;
        let params_str = &after_func[paren_start + 1..paren_end];

        let return_type = if let Some(colon) = after_func[paren_end..].find(':') {
            let ret = &after_func[paren_end + colon + 1..];
            ret.trim().trim_end_matches(';').trim().to_string()
        } else {
            "void".to_string()
        };

        let params = Self::parse_params(params_str);

        Some(DtsDeclaration::Function {
            name,
            params,
            return_type,
        })
    }

    /// Parse interface declaration
    fn parse_interface_declaration(decl: &str) -> Option<DtsDeclaration> {
        // Extract interface name and members
        let decl = decl.replace("export ", "").replace("declare ", "");

        let interface_start = decl.find("interface ")?;
        let after_interface = &decl[interface_start + 10..];

        let brace_start = after_interface.find('{')?;
        let name = after_interface[..brace_start].trim().to_string();

        let brace_end = after_interface.rfind('}')?;
        let body = &after_interface[brace_start + 1..brace_end];

        let members = Self::parse_interface_members(body);

        Some(DtsDeclaration::Interface { name, members })
    }

    /// Parse type alias declaration
    fn parse_type_declaration(decl: &str) -> Option<DtsDeclaration> {
        let decl = decl.replace("export ", "").replace("declare ", "");

        let type_start = decl.find("type ")?;
        let after_type = &decl[type_start + 5..];

        let equals = after_type.find('=')?;
        let name = after_type[..equals].trim().to_string();

        let definition = after_type[equals + 1..]
            .trim()
            .trim_end_matches(';')
            .trim()
            .to_string();

        Some(DtsDeclaration::TypeAlias { name, definition })
    }

    /// Parse variable declaration
    fn parse_variable_declaration(decl: &str) -> Option<DtsDeclaration> {
        // export const foo: string;
        let decl = decl.replace("export ", "").replace("declare ", "");

        let var_keyword = if decl.contains("const ") {
            "const "
        } else if decl.contains("let ") {
            "let "
        } else {
            "var "
        };

        let var_start = decl.find(var_keyword)?;
        let after_var = &decl[var_start + var_keyword.len()..];

        let colon = after_var.find(':')?;
        let name = after_var[..colon].trim().to_string();

        let type_annotation = after_var[colon + 1..]
            .trim()
            .trim_end_matches(';')
            .trim()
            .to_string();

        Some(DtsDeclaration::Variable {
            name,
            type_annotation,
        })
    }

    /// Parse function parameters
    fn parse_params(params_str: &str) -> Vec<(String, String)> {
        if params_str.trim().is_empty() {
            return Vec::new();
        }

        params_str
            .split(',')
            .filter_map(|param| {
                let param = param.trim();
                if let Some(colon) = param.find(':') {
                    let name = param[..colon].trim().to_string();
                    let type_str = param[colon + 1..].trim().to_string();
                    Some((name, type_str))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Parse interface members
    fn parse_interface_members(body: &str) -> Vec<(String, String)> {
        body.lines()
            .filter_map(|line| {
                let line = line.trim().trim_end_matches(';').trim_end_matches(',');
                if line.is_empty() {
                    return None;
                }

                if let Some(colon) = line.find(':') {
                    let name = line[..colon].trim().to_string();
                    let type_str = line[colon + 1..].trim().to_string();
                    Some((name, type_str))
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_function_declaration() {
        let decl = "export function chunk<T>(array: T[], size: number): T[][];";
        let parsed = DtsLoader::parse_function_declaration(decl);

        assert!(parsed.is_some());
        if let Some(DtsDeclaration::Function {
            name,
            params,
            return_type,
        }) = parsed
        {
            assert_eq!(name, "chunk<T>");
            assert_eq!(params.len(), 2);
            assert_eq!(params[0].0, "array");
            assert_eq!(params[0].1, "T[]");
            assert_eq!(return_type, "T[][]");
        } else {
            panic!("Expected Function declaration");
        }
    }

    #[test]
    fn test_parse_interface() {
        let decl = r#"export interface User {
            id: number;
            name: string;
        }"#;

        let parsed = DtsLoader::parse_interface_declaration(decl);
        assert!(parsed.is_some());

        if let Some(DtsDeclaration::Interface { name, members }) = parsed {
            assert_eq!(name, "User");
            assert_eq!(members.len(), 2);
            assert_eq!(members[0].0, "id");
            assert_eq!(members[0].1, "number");
        }
    }

    #[test]
    fn test_parse_type_alias() {
        let decl = "export type ID = string | number;";
        let parsed = DtsLoader::parse_type_declaration(decl);

        assert!(parsed.is_some());
        if let Some(DtsDeclaration::TypeAlias { name, definition }) = parsed {
            assert_eq!(name, "ID");
            assert_eq!(definition, "string | number");
        }
    }

    #[test]
    fn test_parse_variable() {
        let decl = "export const VERSION: string;";
        let parsed = DtsLoader::parse_variable_declaration(decl);

        assert!(parsed.is_some());
        if let Some(DtsDeclaration::Variable {
            name,
            type_annotation,
        }) = parsed
        {
            assert_eq!(name, "VERSION");
            assert_eq!(type_annotation, "string");
        }
    }
}
