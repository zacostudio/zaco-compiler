//! Built-in module type registry
//!
//! This module defines the type signatures for built-in modules like fs, path, process, os,
//! and global objects like Math, JSON, console.

use std::collections::HashMap;
use crate::types::Type;

/// Registry of built-in module types
pub struct BuiltinRegistry {
    modules: HashMap<String, HashMap<String, Type>>,
}

impl BuiltinRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            modules: HashMap::new(),
        };
        registry.register_all_builtins();
        registry
    }

    /// Check if a module is a known built-in module
    pub fn is_builtin_module(&self, module_name: &str) -> bool {
        self.modules.contains_key(module_name)
    }

    /// Get the type of an exported symbol from a built-in module
    pub fn get_export_type(&self, module_name: &str, symbol: &str) -> Option<&Type> {
        self.modules.get(module_name)?.get(symbol)
    }

    /// Get all exports from a module
    pub fn get_module_exports(&self, module_name: &str) -> Option<&HashMap<String, Type>> {
        self.modules.get(module_name)
    }

    fn register_module(&mut self, name: &str, exports: HashMap<String, Type>) {
        self.modules.insert(name.to_string(), exports);
    }

    fn register_all_builtins(&mut self) {
        self.register_fs_module();
        self.register_path_module();
        self.register_process_module();
        self.register_os_module();
        self.register_http_module();
        self.register_events_module();
    }

    fn register_fs_module(&mut self) {
        let mut exports = HashMap::new();

        // readFileSync(path: string, encoding: string) => string
        exports.insert(
            "readFileSync".to_string(),
            Type::Function {
                params: vec![Type::String, Type::String],
                return_type: Box::new(Type::String),
            },
        );

        // writeFileSync(path: string, data: string) => void
        exports.insert(
            "writeFileSync".to_string(),
            Type::Function {
                params: vec![Type::String, Type::String],
                return_type: Box::new(Type::Void),
            },
        );

        // existsSync(path: string) => boolean
        exports.insert(
            "existsSync".to_string(),
            Type::Function {
                params: vec![Type::String],
                return_type: Box::new(Type::Boolean),
            },
        );

        // mkdirSync(path: string) => void
        exports.insert(
            "mkdirSync".to_string(),
            Type::Function {
                params: vec![Type::String],
                return_type: Box::new(Type::Void),
            },
        );

        // rmdirSync(path: string) => void
        exports.insert(
            "rmdirSync".to_string(),
            Type::Function {
                params: vec![Type::String],
                return_type: Box::new(Type::Void),
            },
        );

        // unlinkSync(path: string) => void
        exports.insert(
            "unlinkSync".to_string(),
            Type::Function {
                params: vec![Type::String],
                return_type: Box::new(Type::Void),
            },
        );

        // readdirSync(path: string) => string[]
        exports.insert(
            "readdirSync".to_string(),
            Type::Function {
                params: vec![Type::String],
                return_type: Box::new(Type::Array(Box::new(Type::String))),
            },
        );

        // readFile(path: string, encoding: string, callback: Function) => void
        exports.insert(
            "readFile".to_string(),
            Type::Function {
                params: vec![Type::String, Type::String, Type::Any],
                return_type: Box::new(Type::Void),
            },
        );

        self.register_module("fs", exports);
    }

    fn register_path_module(&mut self) {
        let mut exports = HashMap::new();

        // join(path1: string, path2: string) => string
        exports.insert(
            "join".to_string(),
            Type::Function {
                params: vec![Type::String, Type::String],
                return_type: Box::new(Type::String),
            },
        );

        // resolve(...paths: string[]) => string
        exports.insert(
            "resolve".to_string(),
            Type::Function {
                params: vec![Type::Array(Box::new(Type::String))],
                return_type: Box::new(Type::String),
            },
        );

        // dirname(path: string) => string
        exports.insert(
            "dirname".to_string(),
            Type::Function {
                params: vec![Type::String],
                return_type: Box::new(Type::String),
            },
        );

        // basename(path: string) => string
        exports.insert(
            "basename".to_string(),
            Type::Function {
                params: vec![Type::String],
                return_type: Box::new(Type::String),
            },
        );

        // extname(path: string) => string
        exports.insert(
            "extname".to_string(),
            Type::Function {
                params: vec![Type::String],
                return_type: Box::new(Type::String),
            },
        );

        // isAbsolute(path: string) => boolean
        exports.insert(
            "isAbsolute".to_string(),
            Type::Function {
                params: vec![Type::String],
                return_type: Box::new(Type::Boolean),
            },
        );

        // normalize(path: string) => string
        exports.insert(
            "normalize".to_string(),
            Type::Function {
                params: vec![Type::String],
                return_type: Box::new(Type::String),
            },
        );

        self.register_module("path", exports);
    }

    fn register_process_module(&mut self) {
        let mut exports = HashMap::new();

        // exit(code: number) => void
        exports.insert(
            "exit".to_string(),
            Type::Function {
                params: vec![Type::Number],
                return_type: Box::new(Type::Void),
            },
        );

        // cwd() => string
        exports.insert(
            "cwd".to_string(),
            Type::Function {
                params: vec![],
                return_type: Box::new(Type::String),
            },
        );

        // env: object (simplified as Any for now)
        exports.insert("env".to_string(), Type::Any);

        // pid: number
        exports.insert("pid".to_string(), Type::Number);

        // platform: string
        exports.insert("platform".to_string(), Type::String);

        // arch: string
        exports.insert("arch".to_string(), Type::String);

        // argv: string[]
        exports.insert(
            "argv".to_string(),
            Type::Array(Box::new(Type::String)),
        );

        self.register_module("process", exports);
    }

    fn register_os_module(&mut self) {
        let mut exports = HashMap::new();

        // platform() => string
        exports.insert(
            "platform".to_string(),
            Type::Function {
                params: vec![],
                return_type: Box::new(Type::String),
            },
        );

        // arch() => string
        exports.insert(
            "arch".to_string(),
            Type::Function {
                params: vec![],
                return_type: Box::new(Type::String),
            },
        );

        // homedir() => string
        exports.insert(
            "homedir".to_string(),
            Type::Function {
                params: vec![],
                return_type: Box::new(Type::String),
            },
        );

        // tmpdir() => string
        exports.insert(
            "tmpdir".to_string(),
            Type::Function {
                params: vec![],
                return_type: Box::new(Type::String),
            },
        );

        // hostname() => string
        exports.insert(
            "hostname".to_string(),
            Type::Function {
                params: vec![],
                return_type: Box::new(Type::String),
            },
        );

        // cpus() => any[] (simplified)
        exports.insert(
            "cpus".to_string(),
            Type::Function {
                params: vec![],
                return_type: Box::new(Type::Array(Box::new(Type::Any))),
            },
        );

        // totalmem() => number
        exports.insert(
            "totalmem".to_string(),
            Type::Function {
                params: vec![],
                return_type: Box::new(Type::Number),
            },
        );

        // eol: string
        exports.insert("eol".to_string(), Type::String);

        self.register_module("os", exports);
    }

    fn register_http_module(&mut self) {
        let mut exports = HashMap::new();

        // get(url: string) => string
        exports.insert(
            "get".to_string(),
            Type::Function {
                params: vec![Type::String],
                return_type: Box::new(Type::String),
            },
        );

        // post(url: string, body: string, contentType: string) => string
        exports.insert(
            "post".to_string(),
            Type::Function {
                params: vec![Type::String, Type::String, Type::String],
                return_type: Box::new(Type::String),
            },
        );

        // put(url: string, body: string, contentType: string) => string
        exports.insert(
            "put".to_string(),
            Type::Function {
                params: vec![Type::String, Type::String, Type::String],
                return_type: Box::new(Type::String),
            },
        );

        // delete(url: string) => string
        exports.insert(
            "delete".to_string(),
            Type::Function {
                params: vec![Type::String],
                return_type: Box::new(Type::String),
            },
        );

        self.register_module("http", exports);
    }

    fn register_events_module(&mut self) {
        let mut exports = HashMap::new();

        // EventEmitter is class-based; register basic factory function for now
        // EventEmitter.new() => opaque pointer (represented as Any)
        exports.insert(
            "EventEmitter".to_string(),
            Type::Any,
        );

        self.register_module("events", exports);
    }

}

impl Default for BuiltinRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_module_recognition() {
        let registry = BuiltinRegistry::new();

        assert!(registry.is_builtin_module("fs"));
        assert!(registry.is_builtin_module("path"));
        assert!(registry.is_builtin_module("process"));
        assert!(registry.is_builtin_module("os"));
        assert!(registry.is_builtin_module("http"));
        assert!(registry.is_builtin_module("events"));
        assert!(!registry.is_builtin_module("unknown"));
    }

    #[test]
    fn test_fs_exports() {
        let registry = BuiltinRegistry::new();

        let read_file_sync = registry.get_export_type("fs", "readFileSync");
        assert!(read_file_sync.is_some());
        match read_file_sync {
            Some(Type::Function { params, return_type }) => {
                assert_eq!(params.len(), 2);
                assert_eq!(**return_type, Type::String);
            }
            _ => panic!("Expected function type"),
        }

        let exists_sync = registry.get_export_type("fs", "existsSync");
        assert!(exists_sync.is_some());
        match exists_sync {
            Some(Type::Function { return_type, .. }) => {
                assert_eq!(**return_type, Type::Boolean);
            }
            _ => panic!("Expected function type"),
        }
    }

    #[test]
    fn test_path_exports() {
        let registry = BuiltinRegistry::new();

        let join = registry.get_export_type("path", "join");
        assert!(join.is_some());

        let dirname = registry.get_export_type("path", "dirname");
        assert!(dirname.is_some());
    }

    #[test]
    fn test_process_exports() {
        let registry = BuiltinRegistry::new();

        let exit = registry.get_export_type("process", "exit");
        assert!(exit.is_some());

        let platform = registry.get_export_type("process", "platform");
        assert!(matches!(platform, Some(Type::String)));

        let argv = registry.get_export_type("process", "argv");
        assert!(matches!(argv, Some(Type::Array(_))));
    }

    #[test]
    fn test_os_exports() {
        let registry = BuiltinRegistry::new();

        let platform = registry.get_export_type("os", "platform");
        assert!(platform.is_some());

        let homedir = registry.get_export_type("os", "homedir");
        assert!(homedir.is_some());
    }

    #[test]
    fn test_http_exports() {
        let registry = BuiltinRegistry::new();

        let get = registry.get_export_type("http", "get");
        assert!(get.is_some());
        match get {
            Some(Type::Function { params, return_type }) => {
                assert_eq!(params.len(), 1);
                assert_eq!(**return_type, Type::String);
            }
            _ => panic!("Expected function type"),
        }

        let post = registry.get_export_type("http", "post");
        assert!(post.is_some());
        match post {
            Some(Type::Function { params, return_type }) => {
                assert_eq!(params.len(), 3);
                assert_eq!(**return_type, Type::String);
            }
            _ => panic!("Expected function type"),
        }
    }

    #[test]
    fn test_events_exports() {
        let registry = BuiltinRegistry::new();

        let emitter = registry.get_export_type("events", "EventEmitter");
        assert!(matches!(emitter, Some(Type::Any)));
    }

    #[test]
    fn test_unknown_export() {
        let registry = BuiltinRegistry::new();

        let unknown = registry.get_export_type("fs", "unknownFunction");
        assert!(unknown.is_none());

        let unknown_module = registry.get_export_type("unknown", "func");
        assert!(unknown_module.is_none());
    }
}
