//! Intermediate Representation (IR) for the Zaco Compiler
//!
//! This module defines a low-level intermediate representation that sits between
//! the typed AST and native code generation. It's designed to be easily translated
//! to Cranelift IR for efficient native code generation.
//!
//! The IR uses a control flow graph (CFG) representation with basic blocks,
//! explicit memory operations, and a simple instruction set suitable for
//! compilation to native code.

pub mod lower;
pub mod types;
pub mod value;
pub mod instruction;
pub mod function;
pub mod module;

// ============================================================================
// ID Types (using newtype pattern for type safety)
// ============================================================================

/// Unique identifier for a basic block within a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockId(pub usize);

/// Unique identifier for a local variable within a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalId(pub usize);

/// Unique identifier for a temporary value within a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TempId(pub usize);

/// Unique identifier for a struct type within a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StructId(pub usize);

/// Unique identifier for a function within a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FuncId(pub usize);

// ============================================================================
// Display Implementations for ID types
// ============================================================================

impl std::fmt::Display for BlockId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bb{}", self.0)
    }
}

impl std::fmt::Display for LocalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "_local{}", self.0)
    }
}

impl std::fmt::Display for TempId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "_temp{}", self.0)
    }
}

impl std::fmt::Display for StructId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "struct{}", self.0)
    }
}

impl std::fmt::Display for FuncId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "func{}", self.0)
    }
}

// ============================================================================
// Re-exports (public API)
// ============================================================================

pub use types::*;
pub use value::*;
pub use instruction::*;
pub use function::*;
pub use module::*;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_creation() {
        let mut func = IrFunction::new(
            FuncId(0),
            "test".to_string(),
            vec![],
            IrType::Void,
        );

        let block_id = func.new_block();
        assert_eq!(block_id, BlockId(0));
        assert_eq!(func.blocks.len(), 1);
    }

    #[test]
    fn test_local_creation() {
        let mut func = IrFunction::new(
            FuncId(0),
            "test".to_string(),
            vec![],
            IrType::Void,
        );

        let local1 = func.add_local(IrType::I64);
        let local2 = func.add_local(IrType::Bool);

        assert_eq!(local1, LocalId(0));
        assert_eq!(local2, LocalId(1));
        assert_eq!(func.locals.len(), 2);
    }

    #[test]
    fn test_temp_creation() {
        let mut func = IrFunction::new(
            FuncId(0),
            "test".to_string(),
            vec![],
            IrType::Void,
        );

        let temp1 = func.add_temp(IrType::I64);
        let temp2 = func.add_temp(IrType::F64);

        assert_eq!(temp1, TempId(0));
        assert_eq!(temp2, TempId(1));
        assert_eq!(func.temps.len(), 2);
    }

    #[test]
    fn test_place_projections() {
        let place = Place::from_local(LocalId(0))
            .field(2)
            .index(Value::Const(Constant::I64(5)))
            .deref();

        assert_eq!(place.projections.len(), 3);
        assert!(matches!(place.projections[0], Projection::Field(2)));
        assert!(matches!(place.projections[1], Projection::Index(_)));
        assert!(matches!(place.projections[2], Projection::Deref));
    }

    #[test]
    fn test_module_string_interning() {
        let mut module = IrModule::new();

        let idx1 = module.intern_string("hello".to_string());
        let idx2 = module.intern_string("world".to_string());
        let idx3 = module.intern_string("hello".to_string());

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 0); // Same as idx1, deduplicated
        assert_eq!(module.string_literals.len(), 2);
    }

    #[test]
    fn test_struct_field_index() {
        let struct_def = IrStruct::new(
            StructId(0),
            "Point".to_string(),
            vec![
                ("x".to_string(), IrType::F64),
                ("y".to_string(), IrType::F64),
            ],
        );

        assert_eq!(struct_def.field_index("x"), Some(0));
        assert_eq!(struct_def.field_index("y"), Some(1));
        assert_eq!(struct_def.field_index("z"), None);
    }

    #[test]
    fn test_type_size() {
        assert_eq!(IrType::I64.size_bytes(), 8);
        assert_eq!(IrType::Bool.size_bytes(), 1);
        assert_eq!(IrType::Void.size_bytes(), 0);
        assert_eq!(IrType::Ptr.size_bytes(), 8);
    }

    #[test]
    fn test_type_heap_allocated() {
        assert!(!IrType::I64.is_heap_allocated());
        assert!(!IrType::Bool.is_heap_allocated());
        assert!(IrType::Str.is_heap_allocated());
        assert!(IrType::Array(Box::new(IrType::I64)).is_heap_allocated());
        assert!(IrType::Struct(StructId(0)).is_heap_allocated());
    }

    #[test]
    fn test_block_successors() {
        let mut block = Block::new(BlockId(0));

        // Test jump terminator
        block.set_terminator(Terminator::Jump(BlockId(1)));
        assert_eq!(block.successors(), vec![BlockId(1)]);

        // Test branch terminator
        block.set_terminator(Terminator::Branch {
            cond: Value::Const(Constant::Bool(true)),
            then_block: BlockId(2),
            else_block: BlockId(3),
        });
        assert_eq!(block.successors(), vec![BlockId(2), BlockId(3)]);

        // Test return terminator
        block.set_terminator(Terminator::Return(None));
        assert_eq!(block.successors(), Vec::<BlockId>::new());
    }
}
