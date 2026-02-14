//! IR function and struct definitions.

use zaco_ast::Span;

use crate::{Block, BlockId, FuncId, FuncSignature, IrType, LocalId, StructId, TempId};

/// An IR function definition.
#[derive(Debug, Clone, PartialEq)]
pub struct IrFunction {
    /// Function identifier
    pub id: FuncId,

    /// Function name (mangled for uniqueness)
    pub name: String,

    /// Function parameters with their types
    pub params: Vec<(LocalId, IrType)>,

    /// Return type
    pub return_type: IrType,

    /// Local variables (including params) with their types
    pub locals: Vec<(LocalId, IrType)>,

    /// Temporary values with their types
    pub temps: Vec<(TempId, IrType)>,

    /// Basic blocks comprising the function body
    pub blocks: Vec<Block>,

    /// Entry block (first block to execute)
    pub entry_block: BlockId,

    /// Whether this function is public/exported
    pub is_public: bool,

    /// Optional source span for debugging
    pub span: Option<Span>,
}

impl IrFunction {
    /// Creates a new function with the given name and signature.
    pub fn new(id: FuncId, name: String, params: Vec<(LocalId, IrType)>, return_type: IrType) -> Self {
        IrFunction {
            id,
            name,
            params: params.clone(),
            return_type,
            locals: params,
            temps: Vec::new(),
            blocks: Vec::new(),
            entry_block: BlockId(0),
            is_public: false,
            span: None,
        }
    }

    /// Adds a new local variable.
    pub fn add_local(&mut self, ty: IrType) -> LocalId {
        let id = LocalId(self.locals.len());
        self.locals.push((id, ty));
        id
    }

    /// Adds a new temporary value.
    pub fn add_temp(&mut self, ty: IrType) -> TempId {
        let id = TempId(self.temps.len());
        self.temps.push((id, ty));
        id
    }

    /// Creates a new basic block.
    pub fn new_block(&mut self) -> BlockId {
        let id = BlockId(self.blocks.len());
        self.blocks.push(Block::new(id));
        id
    }

    /// Gets a mutable reference to a block.
    pub fn block_mut(&mut self, id: BlockId) -> &mut Block {
        &mut self.blocks[id.0]
    }

    /// Gets an immutable reference to a block.
    pub fn block(&self, id: BlockId) -> &Block {
        &self.blocks[id.0]
    }

    /// Returns the function signature.
    pub fn signature(&self) -> FuncSignature {
        FuncSignature {
            params: self.params.iter().map(|(_, ty)| ty.clone()).collect(),
            return_type: Box::new(self.return_type.clone()),
        }
    }
}

/// An IR struct type definition.
#[derive(Debug, Clone, PartialEq)]
pub struct IrStruct {
    /// Unique struct identifier
    pub id: StructId,

    /// Struct name
    pub name: String,

    /// Fields with their names and types
    pub fields: Vec<(String, IrType)>,

    /// Optional drop function (destructor) for cleanup
    pub drop_fn: Option<FuncId>,

    /// Optional source span for debugging
    pub span: Option<Span>,
}

impl IrStruct {
    /// Creates a new struct type.
    pub fn new(id: StructId, name: String, fields: Vec<(String, IrType)>) -> Self {
        IrStruct {
            id,
            name,
            fields,
            drop_fn: None,
            span: None,
        }
    }

    /// Returns the index of a field by name.
    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|(field_name, _)| field_name == name)
    }

    /// Returns the type of a field by index.
    pub fn field_type(&self, index: usize) -> Option<&IrType> {
        self.fields.get(index).map(|(_, ty)| ty)
    }

    /// Returns the total size of the struct in bytes.
    pub fn size_bytes(&self) -> usize {
        self.fields.iter().map(|(_, ty)| ty.size_bytes()).sum()
    }
}
