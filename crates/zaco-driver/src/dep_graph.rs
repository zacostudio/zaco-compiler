//! Dependency graph for multi-file compilation

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

/// Node in the dependency graph representing a module
#[derive(Debug, Clone)]
pub struct ModuleNode {
    /// Path to the module file
    pub path: PathBuf,
    /// Modules this module imports from (dependencies)
    pub dependencies: Vec<PathBuf>,
    /// Exported symbols from this module
    pub exports: HashSet<String>,
}

/// Dependency graph for tracking module dependencies
pub struct DepGraph {
    /// Map from file path to module node
    modules: HashMap<PathBuf, ModuleNode>,
    /// Entry point module
    entry: Option<PathBuf>,
}

impl DepGraph {
    /// Create a new empty dependency graph
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            entry: None,
        }
    }

    /// Set the entry point module
    pub fn set_entry(&mut self, path: PathBuf) {
        self.entry = Some(path);
    }

    /// Add a module to the graph
    pub fn add_module(&mut self, path: PathBuf, dependencies: Vec<PathBuf>, exports: HashSet<String>) {
        let node = ModuleNode {
            path: path.clone(),
            dependencies,
            exports,
        };
        self.modules.insert(path, node);
    }

    /// Get a module node by path
    pub fn get_module(&self, path: &PathBuf) -> Option<&ModuleNode> {
        self.modules.get(path)
    }

    /// Check if the graph contains a module
    pub fn contains(&self, path: &PathBuf) -> bool {
        self.modules.contains_key(path)
    }

    /// Detect cycles in the dependency graph
    pub fn detect_cycles(&self) -> Result<(), String> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for path in self.modules.keys() {
            if !visited.contains(path) {
                if let Some(cycle) = self.detect_cycle_util(path, &mut visited, &mut rec_stack) {
                    return Err(format!(
                        "Circular dependency detected: {}",
                        cycle
                            .iter()
                            .map(|p| p.display().to_string())
                            .collect::<Vec<_>>()
                            .join(" -> ")
                    ));
                }
            }
        }

        Ok(())
    }

    /// Utility function for cycle detection (DFS)
    fn detect_cycle_util(
        &self,
        current: &PathBuf,
        visited: &mut HashSet<PathBuf>,
        rec_stack: &mut HashSet<PathBuf>,
    ) -> Option<Vec<PathBuf>> {
        visited.insert(current.clone());
        rec_stack.insert(current.clone());

        if let Some(node) = self.modules.get(current) {
            for dep in &node.dependencies {
                if !visited.contains(dep) {
                    if let Some(mut cycle) = self.detect_cycle_util(dep, visited, rec_stack) {
                        cycle.insert(0, current.clone());
                        return Some(cycle);
                    }
                } else if rec_stack.contains(dep) {
                    // Cycle detected
                    return Some(vec![current.clone(), dep.clone()]);
                }
            }
        }

        rec_stack.remove(current);
        None
    }

    /// Perform topological sort to get compilation order
    /// Returns modules in the order they should be compiled (dependencies first)
    pub fn topological_sort(&self) -> Result<Vec<PathBuf>, String> {
        // First check for cycles
        self.detect_cycles()?;

        let mut in_degree: HashMap<PathBuf, usize> = HashMap::new();
        let mut dependents: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
        let mut result = Vec::new();

        // Initialize
        for path in self.modules.keys() {
            in_degree.insert(path.clone(), 0);
            dependents.insert(path.clone(), Vec::new());
        }

        // Calculate in-degree: count how many dependencies each node has
        // Build reverse mapping: for each dependency, track which nodes depend on it
        for (path, node) in &self.modules {
            let count = node.dependencies.iter()
                .filter(|d| self.modules.contains_key(*d))
                .count();
            in_degree.insert(path.clone(), count);

            for dep in &node.dependencies {
                if let Some(deps_list) = dependents.get_mut(dep) {
                    deps_list.push(path.clone());
                }
            }
        }

        // Queue of nodes with in-degree 0 (no unresolved dependencies — compile first)
        let mut queue: VecDeque<PathBuf> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(path, _)| path.clone())
            .collect();

        // Process queue (Kahn's algorithm)
        while let Some(current) = queue.pop_front() {
            result.push(current.clone());

            // For each node that depends on `current`, decrement its in-degree
            if let Some(deps) = dependents.get(&current) {
                for dependent in deps {
                    if let Some(degree) = in_degree.get_mut(dependent) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(dependent.clone());
                        }
                    }
                }
            }
        }

        // If not all nodes were processed, there's a cycle (shouldn't happen after cycle check)
        if result.len() != self.modules.len() {
            return Err("Topological sort failed: cycle detected".to_string());
        }

        Ok(result)
    }

    /// Get all modules in the graph
    pub fn all_modules(&self) -> Vec<&PathBuf> {
        self.modules.keys().collect()
    }

    /// Get the entry point
    pub fn entry(&self) -> Option<&PathBuf> {
        self.entry.as_ref()
    }
}

impl Default for DepGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_graph() {
        let mut graph = DepGraph::new();

        let a = PathBuf::from("a.ts");
        let b = PathBuf::from("b.ts");
        let c = PathBuf::from("c.ts");

        // c imports b, b imports a
        graph.add_module(a.clone(), vec![], HashSet::new());
        graph.add_module(b.clone(), vec![a.clone()], HashSet::new());
        graph.add_module(c.clone(), vec![b.clone()], HashSet::new());

        let order = graph.topological_sort().unwrap();

        // a should come before b, b before c
        let a_idx = order.iter().position(|p| p == &a).unwrap();
        let b_idx = order.iter().position(|p| p == &b).unwrap();
        let c_idx = order.iter().position(|p| p == &c).unwrap();

        assert!(a_idx < b_idx);
        assert!(b_idx < c_idx);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = DepGraph::new();

        let a = PathBuf::from("a.ts");
        let b = PathBuf::from("b.ts");

        // a imports b, b imports a (cycle)
        graph.add_module(a.clone(), vec![b.clone()], HashSet::new());
        graph.add_module(b.clone(), vec![a.clone()], HashSet::new());

        assert!(graph.detect_cycles().is_err());
    }

    #[test]
    fn test_diamond_dependency() {
        let mut graph = DepGraph::new();

        let a = PathBuf::from("a.ts");
        let b = PathBuf::from("b.ts");
        let c = PathBuf::from("c.ts");
        let d = PathBuf::from("d.ts");

        // Diamond: d -> b,c; b,c -> a
        graph.add_module(a.clone(), vec![], HashSet::new());
        graph.add_module(b.clone(), vec![a.clone()], HashSet::new());
        graph.add_module(c.clone(), vec![a.clone()], HashSet::new());
        graph.add_module(d.clone(), vec![b.clone(), c.clone()], HashSet::new());

        let order = graph.topological_sort().unwrap();

        // a should come before b and c, b and c before d
        let a_idx = order.iter().position(|p| p == &a).unwrap();
        let b_idx = order.iter().position(|p| p == &b).unwrap();
        let c_idx = order.iter().position(|p| p == &c).unwrap();
        let d_idx = order.iter().position(|p| p == &d).unwrap();

        assert!(a_idx < b_idx);
        assert!(a_idx < c_idx);
        assert!(b_idx < d_idx);
        assert!(c_idx < d_idx);
    }
}
