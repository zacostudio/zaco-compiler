//! Package.json parser
//!
//! Simple JSON parser for extracting package.json metadata.
//! Doesn't use serde_json to avoid adding dependencies.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct PackageJson {
    pub name: String,
    pub version: String,
    pub main: Option<String>,
    pub types: Option<String>,
    pub module: Option<String>,
    pub exports: Option<PackageExports>,
    pub dependencies: HashMap<String, String>,
    pub dev_dependencies: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct PackageExports {
    // Simplified representation for now
    pub default: Option<String>,
    pub types: Option<String>,
}

/// Parse a package.json file
pub fn parse_package_json(path: &Path) -> Result<PackageJson, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read package.json at {}: {}", path.display(), e))?;

    parse_package_json_str(&content)
}

/// Parse package.json from string content
fn parse_package_json_str(content: &str) -> Result<PackageJson, String> {
    let json = parse_json_object(content)?;

    let mut pkg = PackageJson::default();

    // Extract string fields
    if let Some(JsonValue::String(name)) = json.get("name") {
        pkg.name = name.clone();
    }

    if let Some(JsonValue::String(version)) = json.get("version") {
        pkg.version = version.clone();
    }

    if let Some(JsonValue::String(main)) = json.get("main") {
        pkg.main = Some(main.clone());
    }

    if let Some(JsonValue::String(types)) = json.get("types") {
        pkg.types = Some(types.clone());
    } else if let Some(JsonValue::String(typings)) = json.get("typings") {
        // "typings" is an alias for "types"
        pkg.types = Some(typings.clone());
    }

    if let Some(JsonValue::String(module)) = json.get("module") {
        pkg.module = Some(module.clone());
    }

    // Parse exports field (simplified)
    if let Some(JsonValue::Object(exports)) = json.get("exports") {
        pkg.exports = Some(parse_exports(exports));
    }

    // Parse dependencies
    if let Some(JsonValue::Object(deps)) = json.get("dependencies") {
        pkg.dependencies = extract_dependencies(deps);
    }

    // Parse devDependencies
    if let Some(JsonValue::Object(dev_deps)) = json.get("devDependencies") {
        pkg.dev_dependencies = extract_dependencies(dev_deps);
    }

    Ok(pkg)
}

fn parse_exports(exports: &HashMap<String, JsonValue>) -> PackageExports {
    let mut pkg_exports = PackageExports {
        default: None,
        types: None,
    };

    // Handle string exports: "exports": "./index.js"
    if let Some(JsonValue::String(default)) = exports.get(".") {
        pkg_exports.default = Some(default.clone());
    }

    // Handle object exports: "exports": { ".": { "default": "./index.js" } }
    if let Some(JsonValue::Object(dot_exports)) = exports.get(".") {
        if let Some(JsonValue::String(default)) = dot_exports.get("default") {
            pkg_exports.default = Some(default.clone());
        }
        if let Some(JsonValue::String(types)) = dot_exports.get("types") {
            pkg_exports.types = Some(types.clone());
        }
    }

    pkg_exports
}

fn extract_dependencies(deps: &HashMap<String, JsonValue>) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for (name, value) in deps {
        if let JsonValue::String(version) = value {
            result.insert(name.clone(), version.clone());
        }
    }
    result
}

// ============================================================================
// Minimal JSON parser
// ============================================================================

#[derive(Debug, Clone)]
enum JsonValue {
    String(String),
    Object(HashMap<String, JsonValue>),
    Array(Vec<JsonValue>),
    Number(f64),
    Bool(bool),
    Null,
}

struct JsonParser {
    chars: Vec<char>,
    pos: usize,
}

impl JsonParser {
    fn new(content: &str) -> Self {
        Self {
            chars: content.chars().collect(),
            pos: 0,
        }
    }

    fn parse(&mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();
        self.parse_value()
    }

    fn parse_value(&mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();

        if self.pos >= self.chars.len() {
            return Err("Unexpected end of input".to_string());
        }

        match self.chars[self.pos] {
            '{' => self.parse_object(),
            '[' => self.parse_array(),
            '"' => self.parse_string(),
            't' | 'f' => self.parse_bool(),
            'n' => self.parse_null(),
            '0'..='9' | '-' => self.parse_number(),
            _ => Err(format!("Unexpected character: {}", self.chars[self.pos])),
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, String> {
        let mut obj = HashMap::new();

        self.expect('{')?;
        self.skip_whitespace();

        // Empty object
        if self.pos < self.chars.len() && self.chars[self.pos] == '}' {
            self.pos += 1;
            return Ok(JsonValue::Object(obj));
        }

        loop {
            self.skip_whitespace();

            // Parse key (must be string)
            let key = if self.pos < self.chars.len() && self.chars[self.pos] == '"' {
                match self.parse_string()? {
                    JsonValue::String(s) => s,
                    _ => return Err("Expected string key".to_string()),
                }
            } else {
                return Err("Expected string key in object".to_string());
            };

            self.skip_whitespace();
            self.expect(':')?;

            // Parse value
            let value = self.parse_value()?;
            obj.insert(key, value);

            self.skip_whitespace();

            if self.pos >= self.chars.len() {
                return Err("Unexpected end in object".to_string());
            }

            match self.chars[self.pos] {
                ',' => {
                    self.pos += 1;
                    continue;
                }
                '}' => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(format!("Expected ',' or '}}' in object")),
            }
        }

        Ok(JsonValue::Object(obj))
    }

    fn parse_array(&mut self) -> Result<JsonValue, String> {
        let mut arr = Vec::new();

        self.expect('[')?;
        self.skip_whitespace();

        // Empty array
        if self.pos < self.chars.len() && self.chars[self.pos] == ']' {
            self.pos += 1;
            return Ok(JsonValue::Array(arr));
        }

        loop {
            let value = self.parse_value()?;
            arr.push(value);

            self.skip_whitespace();

            if self.pos >= self.chars.len() {
                return Err("Unexpected end in array".to_string());
            }

            match self.chars[self.pos] {
                ',' => {
                    self.pos += 1;
                    continue;
                }
                ']' => {
                    self.pos += 1;
                    break;
                }
                _ => return Err("Expected ',' or ']' in array".to_string()),
            }
        }

        Ok(JsonValue::Array(arr))
    }

    fn parse_string(&mut self) -> Result<JsonValue, String> {
        self.expect('"')?;

        let mut s = String::new();
        let mut escaped = false;

        while self.pos < self.chars.len() {
            let ch = self.chars[self.pos];

            if escaped {
                let unescaped = match ch {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '"' => '"',
                    '\\' => '\\',
                    '/' => '/',
                    _ => ch,
                };
                s.push(unescaped);
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                self.pos += 1;
                return Ok(JsonValue::String(s));
            } else {
                s.push(ch);
            }

            self.pos += 1;
        }

        Err("Unterminated string".to_string())
    }

    fn parse_bool(&mut self) -> Result<JsonValue, String> {
        if self.consume_literal("true") {
            Ok(JsonValue::Bool(true))
        } else if self.consume_literal("false") {
            Ok(JsonValue::Bool(false))
        } else {
            Err("Invalid boolean".to_string())
        }
    }

    fn parse_null(&mut self) -> Result<JsonValue, String> {
        if self.consume_literal("null") {
            Ok(JsonValue::Null)
        } else {
            Err("Invalid null".to_string())
        }
    }

    fn parse_number(&mut self) -> Result<JsonValue, String> {
        let mut num_str = String::new();

        // Optional leading minus
        if self.pos < self.chars.len() && self.chars[self.pos] == '-' {
            num_str.push('-');
            self.pos += 1;
        }

        // Integer digits
        while self.pos < self.chars.len() && self.chars[self.pos].is_ascii_digit() {
            num_str.push(self.chars[self.pos]);
            self.pos += 1;
        }

        // Optional decimal part
        if self.pos < self.chars.len() && self.chars[self.pos] == '.' {
            num_str.push('.');
            self.pos += 1;
            while self.pos < self.chars.len() && self.chars[self.pos].is_ascii_digit() {
                num_str.push(self.chars[self.pos]);
                self.pos += 1;
            }
        }

        num_str
            .parse::<f64>()
            .map(JsonValue::Number)
            .map_err(|_| format!("Invalid number: {}", num_str))
    }

    fn consume_literal(&mut self, literal: &str) -> bool {
        let chars: Vec<char> = literal.chars().collect();
        if self.pos + chars.len() > self.chars.len() {
            return false;
        }

        for (i, &ch) in chars.iter().enumerate() {
            if self.chars[self.pos + i] != ch {
                return false;
            }
        }

        self.pos += chars.len();
        true
    }

    fn expect(&mut self, ch: char) -> Result<(), String> {
        if self.pos >= self.chars.len() {
            return Err(format!("Expected '{}' but got EOF", ch));
        }

        if self.chars[self.pos] != ch {
            return Err(format!(
                "Expected '{}' but got '{}'",
                ch, self.chars[self.pos]
            ));
        }

        self.pos += 1;
        Ok(())
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.chars.len() {
            match self.chars[self.pos] {
                ' ' | '\t' | '\n' | '\r' => self.pos += 1,
                _ => break,
            }
        }
    }
}

fn parse_json_object(content: &str) -> Result<HashMap<String, JsonValue>, String> {
    let mut parser = JsonParser::new(content);
    match parser.parse()? {
        JsonValue::Object(obj) => Ok(obj),
        _ => Err("Expected JSON object".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_package_json() {
        let json = r#"{
            "name": "test-package",
            "version": "1.0.0",
            "main": "index.js",
            "types": "index.d.ts"
        }"#;

        let pkg = parse_package_json_str(json).unwrap();
        assert_eq!(pkg.name, "test-package");
        assert_eq!(pkg.version, "1.0.0");
        assert_eq!(pkg.main, Some("index.js".to_string()));
        assert_eq!(pkg.types, Some("index.d.ts".to_string()));
    }

    #[test]
    fn test_parse_dependencies() {
        let json = r#"{
            "name": "test",
            "version": "1.0.0",
            "dependencies": {
                "lodash": "^4.17.21",
                "express": "~4.18.2"
            },
            "devDependencies": {
                "typescript": "^5.0.0"
            }
        }"#;

        let pkg = parse_package_json_str(json).unwrap();
        assert_eq!(pkg.dependencies.len(), 2);
        assert_eq!(pkg.dependencies.get("lodash"), Some(&"^4.17.21".to_string()));
        assert_eq!(pkg.dev_dependencies.len(), 1);
    }

    #[test]
    fn test_parse_package_json_with_numbers() {
        let json = r#"{"name": "test", "version": "1.0.0", "config": {"port": 3000, "timeout": 30.5}}"#;
        let pkg = parse_package_json_str(json).unwrap();
        assert_eq!(pkg.name, "test");
        assert_eq!(pkg.version, "1.0.0");
    }

    #[test]
    fn test_parse_package_json_with_negative_numbers() {
        let json = r#"{"name": "test", "version": "1.0.0", "config": {"offset": -1, "scale": -0.5}}"#;
        let pkg = parse_package_json_str(json).unwrap();
        assert_eq!(pkg.name, "test");
        assert_eq!(pkg.version, "1.0.0");
    }

    #[test]
    fn test_parse_exports() {
        let json = r#"{
            "name": "test",
            "version": "1.0.0",
            "exports": {
                ".": {
                    "types": "./index.d.ts",
                    "default": "./index.js"
                }
            }
        }"#;

        let pkg = parse_package_json_str(json).unwrap();
        assert!(pkg.exports.is_some());
        let exports = pkg.exports.unwrap();
        assert_eq!(exports.types, Some("./index.d.ts".to_string()));
        assert_eq!(exports.default, Some("./index.js".to_string()));
    }
}
