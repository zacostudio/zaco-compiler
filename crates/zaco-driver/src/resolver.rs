//! Module resolution logic for import statements

use std::path::{Path, PathBuf};

use crate::npm_resolver::NpmResolver;

/// Represents a resolved module
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResolvedModule {
    /// Local file path
    LocalFile(PathBuf),
    /// Built-in module (fs, path, http, etc.)
    Builtin(String),
    /// NPM package resolved to a file path
    Package(PathBuf),
    /// NPM package not found (compile error)
    PackageNotFound { name: String, reason: String },
}

/// Module resolver handles import path resolution
pub struct ModuleResolver {
    /// Base directory for resolving relative imports
    base_dir: PathBuf,
    /// NPM package resolver
    npm_resolver: Option<NpmResolver>,
}

impl ModuleResolver {
    /// Create a new module resolver with a base directory
    pub fn new(base_dir: PathBuf) -> Self {
        // Try to find project root for NPM resolution
        let npm_resolver = NpmResolver::find_project_root(&base_dir)
            .map(|root| NpmResolver::new(root));

        Self {
            base_dir,
            npm_resolver,
        }
    }

    /// Resolve an import specifier to a module
    pub fn resolve(&self, specifier: &str, from_file: &Path) -> Result<ResolvedModule, String> {
        // Check if it's a built-in module
        if Self::is_builtin(specifier) {
            return Ok(ResolvedModule::Builtin(specifier.to_string()));
        }

        // Check if it's a relative import (starts with ./ or ../)
        if specifier.starts_with("./") || specifier.starts_with("../") {
            return self.resolve_relative(specifier, from_file);
        }

        // Check if it's an absolute import (starts with /)
        if specifier.starts_with('/') {
            return self.resolve_absolute(specifier);
        }

        // Otherwise, it's a package import - try NPM resolution
        if let Some(ref npm_resolver) = self.npm_resolver {
            match npm_resolver.resolve(specifier, from_file) {
                Ok(path) => return Ok(ResolvedModule::Package(path)),
                Err(e) => {
                    // Package not found — propagate the resolution error
                    return Ok(ResolvedModule::PackageNotFound {
                        name: specifier.to_string(),
                        reason: e,
                    });
                }
            }
        }

        // No NPM resolver available — no node_modules directory found
        Ok(ResolvedModule::PackageNotFound {
            name: specifier.to_string(),
            reason: "no node_modules directory found".to_string(),
        })
    }

    /// Check if a specifier is a built-in module
    fn is_builtin(specifier: &str) -> bool {
        matches!(
            specifier,
            "fs" | "path" | "http" | "https" | "os" | "process" | "events"
                | "url" | "crypto" | "util" | "stream" | "buffer"
                | "child_process" | "net" | "tls" | "dns" | "querystring"
                | "assert" | "zlib"
        )
    }

    /// Resolve a relative import (./foo or ../bar)
    fn resolve_relative(&self, specifier: &str, from_file: &Path) -> Result<ResolvedModule, String> {
        let from_dir = from_file.parent().ok_or_else(|| {
            format!(
                "Cannot resolve relative import from file without parent: {}",
                from_file.display()
            )
        })?;

        let target = from_dir.join(specifier);
        self.try_resolve_file(&target)
    }

    /// Resolve an absolute import (/foo/bar)
    fn resolve_absolute(&self, specifier: &str) -> Result<ResolvedModule, String> {
        let target = PathBuf::from(specifier);
        self.try_resolve_file(&target)
    }

    /// Try to resolve a file path with various extensions
    fn try_resolve_file(&self, target: &Path) -> Result<ResolvedModule, String> {
        // Try extensions in order: .ts, .tsx, .js, .jsx
        let extensions = ["ts", "tsx", "js", "jsx"];

        // First, try the exact path with extensions
        for ext in &extensions {
            let with_ext = target.with_extension(ext);
            if with_ext.exists() && with_ext.is_file() {
                return Ok(ResolvedModule::LocalFile(
                    with_ext.canonicalize().map_err(|e| {
                        format!("Failed to canonicalize path {}: {}", with_ext.display(), e)
                    })?,
                ));
            }
        }

        // If that didn't work, try as a directory with index file
        if target.is_dir() {
            for ext in &extensions {
                let index_path = target.join("index").with_extension(ext);
                if index_path.exists() && index_path.is_file() {
                    return Ok(ResolvedModule::LocalFile(
                        index_path.canonicalize().map_err(|e| {
                            format!("Failed to canonicalize path {}: {}", index_path.display(), e)
                        })?,
                    ));
                }
            }
        }

        // Also try /index.* in case target is a path without the final index part
        let as_dir = target;
        for ext in &extensions {
            let index_path = as_dir.join("index").with_extension(ext);
            if index_path.exists() && index_path.is_file() {
                return Ok(ResolvedModule::LocalFile(
                    index_path.canonicalize().map_err(|e| {
                        format!("Failed to canonicalize path {}: {}", index_path.display(), e)
                    })?,
                ));
            }
        }

        Err(format!(
            "Module not found: {} (tried extensions: ts, tsx, js, jsx and index files)",
            target.display()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_builtin_detection() {
        assert!(ModuleResolver::is_builtin("fs"));
        assert!(ModuleResolver::is_builtin("path"));
        assert!(ModuleResolver::is_builtin("http"));
        assert!(!ModuleResolver::is_builtin("./local"));
        assert!(!ModuleResolver::is_builtin("my-package"));
    }

    #[test]
    fn test_package_detection() {
        let resolver = ModuleResolver::new(PathBuf::from("/tmp"));
        let from_file = PathBuf::from("/tmp/main.ts");

        // Should recognize as package (not builtin, not relative)
        // Since there's no node_modules, it should return PackageNotFound
        match resolver.resolve("my-package", &from_file) {
            Ok(ResolvedModule::PackageNotFound { name, .. }) => assert_eq!(name, "my-package"),
            other => panic!("Expected PackageNotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_relative_resolution() {
        // Create temp directory for testing
        let temp_dir = std::env::temp_dir().join("zaco_test_resolver");
        let _ = fs::create_dir_all(&temp_dir);

        let test_file = temp_dir.join("test.ts");
        fs::write(&test_file, "export const x = 1;").unwrap();

        let resolver = ModuleResolver::new(temp_dir.clone());
        let from_file = temp_dir.join("main.ts");

        match resolver.resolve("./test", &from_file) {
            Ok(ResolvedModule::LocalFile(path)) => {
                assert!(path.ends_with("test.ts"));
            }
            other => panic!("Expected LocalFile, got {:?}", other),
        }

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
