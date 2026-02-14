//! Zaco Compiler Driver Library
//!
//! Core compilation pipeline logic for the Zaco TypeScript compiler.
//! Provides module resolution, dependency graph management, and the
//! full compilation pipeline (lex → parse → typecheck → lower → codegen).

pub mod resolver;
pub mod dep_graph;
pub mod package_json;
pub mod npm_resolver;
pub mod dts_loader;

pub use resolver::{ModuleResolver, ResolvedModule};
pub use dep_graph::DepGraph;
