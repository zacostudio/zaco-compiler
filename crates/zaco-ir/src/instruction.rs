//! IR instructions and control flow primitives.

use zaco_ast::Span;

use crate::{BlockId, IrType, Place, RValue, Value};

/// A single IR instruction within a basic block.
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    /// Assignment: dest = value
    Assign {
        dest: Place,
        value: RValue,
    },

    /// Function call: dest = func(args)
    Call {
        dest: Option<Place>,
        func: Value,
        args: Vec<Value>,
    },

    /// Return from function
    Return(Option<Value>),

    /// Conditional branch
    Branch {
        cond: Value,
        then_block: BlockId,
        else_block: BlockId,
    },

    /// Unconditional jump
    Jump(BlockId),

    /// Heap allocation
    Alloc {
        dest: Place,
        ty: IrType,
    },

    /// Deallocation (ownership drop)
    Free {
        value: Value,
    },

    /// Reference count adjustment (for GC)
    RefCount {
        value: Value,
        delta: i32,
    },

    /// Deep clone operation
    Clone {
        dest: Place,
        source: Value,
    },

    /// Store value to pointer
    Store {
        ptr: Value,
        value: Value,
    },

    /// Load value from pointer
    Load {
        dest: Place,
        ptr: Value,
    },
}

/// Terminator instruction that ends a basic block.
#[derive(Debug, Clone, PartialEq)]
pub enum Terminator {
    /// Return from function
    Return(Option<Value>),

    /// Conditional branch
    Branch {
        cond: Value,
        then_block: BlockId,
        else_block: BlockId,
    },

    /// Unconditional jump
    Jump(BlockId),

    /// Unreachable code (for optimization)
    Unreachable,
}

/// A basic block in the control flow graph.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// Unique identifier for this block
    pub id: BlockId,

    /// Instructions in this block (excluding terminator)
    pub instructions: Vec<Instruction>,

    /// Terminator instruction (control flow exit)
    pub terminator: Terminator,

    /// Optional source span for debugging
    pub span: Option<Span>,
}

impl Block {
    /// Creates a new basic block with the given ID.
    pub fn new(id: BlockId) -> Self {
        Block {
            id,
            instructions: Vec::new(),
            terminator: Terminator::Unreachable,
            span: None,
        }
    }

    /// Adds an instruction to this block.
    pub fn push_instruction(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }

    /// Sets the terminator for this block.
    pub fn set_terminator(&mut self, terminator: Terminator) {
        self.terminator = terminator;
    }

    /// Returns the successor block IDs (for CFG analysis).
    pub fn successors(&self) -> Vec<BlockId> {
        match &self.terminator {
            Terminator::Return(_) | Terminator::Unreachable => Vec::new(),
            Terminator::Jump(block) => vec![*block],
            Terminator::Branch { then_block, else_block, .. } => {
                vec![*then_block, *else_block]
            }
        }
    }
}
