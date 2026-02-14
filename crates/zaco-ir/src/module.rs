//! IR module definition representing a compilation unit.

use std::collections::HashMap;

use crate::{Constant, FuncId, IrFunction, IrStruct, IrType, StructId};

/// An extern (imported) function declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct ExternFunction {
    /// Function name (as it appears in the object file)
    pub name: String,
    /// Parameter types
    pub params: Vec<IrType>,
    /// Return type
    pub return_type: IrType,
}

/// A complete IR module representing a compilation unit.
#[derive(Debug, Clone)]
pub struct IrModule {
    /// All functions in this module
    pub functions: Vec<IrFunction>,

    /// All struct type definitions
    pub structs: Vec<IrStruct>,

    /// Global variables (name, type, optional initializer)
    pub globals: Vec<(String, IrType, Option<Constant>)>,

    /// String literals used in the module (for deduplication)
    pub string_literals: Vec<String>,

    /// Extern function declarations (runtime or FFI)
    pub extern_functions: Vec<ExternFunction>,

    /// HashMap for O(1) string dedup lookups
    string_index_map: HashMap<String, usize>,

    /// Next available FuncId counter (set by the lowerer after lowering).
    /// Used by the driver to compute offsets for multi-module compilation.
    pub next_func_id: usize,

    /// Next available StructId counter (set by the lowerer after lowering).
    /// Used by the driver to compute offsets for multi-module compilation.
    pub next_struct_id: usize,
}

impl IrModule {
    /// Creates a new empty IR module.
    pub fn new() -> Self {
        IrModule {
            functions: Vec::new(),
            structs: Vec::new(),
            globals: Vec::new(),
            string_literals: Vec::new(),
            extern_functions: Vec::new(),
            string_index_map: HashMap::new(),
            next_func_id: 0,
            next_struct_id: 0,
        }
    }

    /// Adds an extern function declaration.
    pub fn add_extern_function(&mut self, name: String, params: Vec<IrType>, return_type: IrType) {
        self.extern_functions.push(ExternFunction {
            name,
            params,
            return_type,
        });
    }

    /// Adds a function to the module.
    pub fn add_function(&mut self, function: IrFunction) -> FuncId {
        let id = function.id;
        self.functions.push(function);
        id
    }

    /// Adds a struct type to the module.
    pub fn add_struct(&mut self, struct_def: IrStruct) -> StructId {
        let id = struct_def.id;
        self.structs.push(struct_def);
        id
    }

    /// Adds a global variable.
    pub fn add_global(&mut self, name: String, ty: IrType, init: Option<Constant>) {
        self.globals.push((name, ty, init));
    }

    /// Interns a string literal and returns its index.
    pub fn intern_string(&mut self, s: String) -> usize {
        if let Some(&index) = self.string_index_map.get(&s) {
            index
        } else {
            let index = self.string_literals.len();
            self.string_index_map.insert(s.clone(), index);
            self.string_literals.push(s);
            index
        }
    }

    /// Gets a function by ID.
    pub fn function(&self, id: FuncId) -> Option<&IrFunction> {
        self.functions.get(id.0)
    }

    /// Gets a mutable reference to a function by ID.
    pub fn function_mut(&mut self, id: FuncId) -> Option<&mut IrFunction> {
        self.functions.get_mut(id.0)
    }

    /// Gets a struct by ID.
    pub fn struct_def(&self, id: StructId) -> Option<&IrStruct> {
        self.structs.get(id.0)
    }

    /// Finds a function by name.
    pub fn find_function(&self, name: &str) -> Option<&IrFunction> {
        self.functions.iter().find(|f| f.name == name)
    }

    /// Finds a struct by name.
    pub fn find_struct(&self, name: &str) -> Option<&IrStruct> {
        self.structs.iter().find(|s| s.name == name)
    }
}

impl Default for IrModule {
    fn default() -> Self {
        Self::new()
    }
}
