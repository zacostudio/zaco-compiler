//! Type environment (scoped symbol table)

use std::collections::HashMap;
use crate::types::Type;
use crate::ownership::{OwnershipState, VarInfo};

/// Type environment with scoped symbol tables
#[derive(Debug, Clone)]
pub struct TypeEnv {
    scopes: Vec<HashMap<String, VarInfo>>,
    type_aliases: HashMap<String, Type>,
    interfaces: HashMap<String, Type>,
    classes: HashMap<String, Type>,
    enums: HashMap<String, Type>,
    /// Exported symbols from this module
    exports: HashMap<String, Type>,
    /// Generic type parameter names for classes/interfaces (e.g., "Array" → ["T"])
    type_param_names: HashMap<String, Vec<String>>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            type_aliases: HashMap::new(),
            interfaces: HashMap::new(),
            classes: HashMap::new(),
            enums: HashMap::new(),
            exports: HashMap::new(),
            type_param_names: HashMap::new(),
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn declare(&mut self, name: String, var_info: VarInfo) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, var_info);
        }
    }

    /// Check if a binding exists in the current (innermost) scope only
    pub fn has_in_current_scope(&self, name: &str) -> bool {
        if let Some(scope) = self.scopes.last() {
            scope.contains_key(name)
        } else {
            false
        }
    }

    pub fn lookup(&self, name: &str) -> Option<&VarInfo> {
        for scope in self.scopes.iter().rev() {
            if let Some(info) = scope.get(name) {
                return Some(info);
            }
        }
        None
    }

    pub fn lookup_mut(&mut self, name: &str) -> Option<&mut VarInfo> {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                return scope.get_mut(name);
            }
        }
        None
    }

    pub fn update_ownership(&mut self, name: &str, state: OwnershipState) -> Result<(), String> {
        if let Some(var_info) = self.lookup_mut(name) {
            var_info.ownership = state;
            Ok(())
        } else {
            Err(format!("Variable '{}' not found", name))
        }
    }

    pub fn define_type_alias(&mut self, name: String, ty: Type) {
        self.type_aliases.insert(name, ty);
    }

    pub fn define_interface(&mut self, name: String, ty: Type) {
        self.interfaces.insert(name, ty);
    }

    pub fn define_class(&mut self, name: String, ty: Type) {
        self.classes.insert(name, ty);
    }

    pub fn define_enum(&mut self, name: String, ty: Type) {
        self.enums.insert(name, ty);
    }

    pub fn lookup_type(&self, name: &str) -> Option<&Type> {
        self.type_aliases
            .get(name)
            .or_else(|| self.interfaces.get(name))
            .or_else(|| self.classes.get(name))
            .or_else(|| self.enums.get(name))
    }

    /// Register generic type parameter names for a class/interface
    pub fn define_type_params(&mut self, name: String, params: Vec<String>) {
        self.type_param_names.insert(name, params);
    }

    /// Get the generic type parameter names for a class/interface
    pub fn get_type_params(&self, name: &str) -> Option<&Vec<String>> {
        self.type_param_names.get(name)
    }

    /// Register an exported symbol
    pub fn export_symbol(&mut self, name: String, ty: Type) {
        self.exports.insert(name, ty);
    }

    /// Get the type of an exported symbol
    pub fn get_export(&self, name: &str) -> Option<&Type> {
        self.exports.get(name)
    }

    /// Get all exports from this module
    pub fn get_all_exports(&self) -> &HashMap<String, Type> {
        &self.exports
    }
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}
