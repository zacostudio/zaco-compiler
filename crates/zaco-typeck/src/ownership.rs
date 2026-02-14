//! Ownership tracking

use crate::types::Type;

/// Ownership state for a variable
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipState {
    /// Variable owns the value
    Owned,
    /// Variable has an immutable reference
    Borrowed,
    /// Variable has a mutable reference
    MutBorrowed,
    /// Value has been moved (cannot be used)
    Moved,
    /// Value has been explicitly dropped
    Dropped,
}

/// Variable information in the symbol table
#[derive(Debug, Clone)]
pub struct VarInfo {
    pub ty: Type,
    pub ownership: OwnershipState,
    pub is_mutable: bool,
    pub is_initialized: bool,
}
