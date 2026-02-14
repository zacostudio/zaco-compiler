//! NPM package resolution
//!
//! Implements Node.js module resolution algorithm for resolving
//! package names to their entry files.

use std::path::{Path, PathBuf};

use crate::package_json::parse_package_json;

pub struct NpmResolver {
    project_root: PathBuf,
}

impl NpmResolver {
    /// Create a new NPM resolver
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    /// Find the project root by searching for package.json
    pub fn find_project_root(start: &Path) -> Option<PathBuf> {
        let mut current = start.to_path_buf();

        loop {
            let package_json = current.join("package.json");
            if package_json.exists() {
                return Some(current);
            }

            // Try parent directory
            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                // Reached filesystem root
                return None;
            }
        }
    }

    /// Resolve a package name to its entry file
    ///
    /// Algorithm:
    /// 1. Parse package name and subpath (e.g., "lodash/fp" → "lodash" + "fp")
    /// 2. Start from the importing file's directory
    /// 3. Look for node_modules/<package_name>/package.json
    /// 4. If found, use "types" or "main" field to find entry
    /// 5. If not found, go up one directory and repeat
    /// 6. Stop at project root or filesystem root
    pub fn resolve(&self, package_name: &str, from_file: &Path) -> Result<PathBuf, String> {
        // Parse package name and subpath
        let (pkg_name, subpath) = Self::parse_package_specifier(package_name);

        // Start from the importing file's directory
        let mut current = from_file
            .parent()
            .ok_or_else(|| format!("Cannot resolve package from file: {}", from_file.display()))?
            .to_path_buf();

        loop {
            let node_modules = current.join("node_modules");
            let package_dir = node_modules.join(pkg_name);

            if package_dir.exists() && package_dir.is_dir() {
                // Found the package directory
                return self.resolve_package_entry(&package_dir, subpath);
            }

            // Stop at project root after checking it
            if current == self.project_root {
                break;
            }

            // Try parent directory
            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                // Reached filesystem root
                break;
            }
        }

        Err(format!(
            "Package '{}' not found in node_modules",
            package_name
        ))
    }

    /// Parse a package specifier into package name and subpath
    ///
    /// Examples:
    /// - "lodash" → ("lodash", None)
    /// - "lodash/fp" → ("lodash", Some("fp"))
    /// - "@types/node" → ("@types/node", None)
    /// - "@types/node/fs" → ("@types/node", Some("fs"))
    fn parse_package_specifier(specifier: &str) -> (&str, Option<&str>) {
        // Handle scoped packages (@scope/package)
        if specifier.starts_with('@') {
            // Find second slash for scoped packages
            let mut slash_count = 0;
            let mut split_pos = None;

            for (i, ch) in specifier.chars().enumerate() {
                if ch == '/' {
                    slash_count += 1;
                    if slash_count == 2 {
                        split_pos = Some(i);
                        break;
                    }
                }
            }

            if let Some(pos) = split_pos {
                let pkg_name = &specifier[..pos];
                let subpath = &specifier[pos + 1..];
                return (pkg_name, Some(subpath));
            } else {
                // No subpath
                return (specifier, None);
            }
        }

        // Handle regular packages
        if let Some(slash_pos) = specifier.find('/') {
            let pkg_name = &specifier[..slash_pos];
            let subpath = &specifier[slash_pos + 1..];
            (pkg_name, Some(subpath))
        } else {
            (specifier, None)
        }
    }

    /// Resolve the entry file for a package
    fn resolve_package_entry(
        &self,
        package_dir: &Path,
        subpath: Option<&str>,
    ) -> Result<PathBuf, String> {
        let package_json_path = package_dir.join("package.json");

        // If there's a subpath, try to resolve it directly
        if let Some(sub) = subpath {
            // Try common extensions
            let subpath_file = package_dir.join(sub);
            if let Ok(resolved) = self.try_resolve_file(&subpath_file) {
                return Ok(resolved);
            }
        }

        // Parse package.json
        if package_json_path.exists() {
            let pkg = parse_package_json(&package_json_path).map_err(|e| {
                format!(
                    "Failed to parse {}: {}",
                    package_json_path.display(),
                    e
                )
            })?;

            // If there's a subpath but we didn't find it, return error
            if subpath.is_some() {
                return Err(format!(
                    "Subpath '{}' not found in package '{}'",
                    subpath.unwrap(),
                    pkg.name
                ));
            }

            // Priority: types > module > main
            // For each field, try exact path first, then with extensions (.d.ts, .ts, etc.)
            if let Some(types_field) = &pkg.types {
                let types_path = package_dir.join(types_field);
                if types_path.exists() {
                    return Ok(types_path);
                }
                if let Ok(resolved) = self.try_resolve_file(&types_path) {
                    return Ok(resolved);
                }
            }

            if let Some(module_field) = &pkg.module {
                let module_path = package_dir.join(module_field);
                if module_path.exists() {
                    return Ok(module_path);
                }
                if let Ok(resolved) = self.try_resolve_file(&module_path) {
                    return Ok(resolved);
                }
            }

            if let Some(main_field) = &pkg.main {
                let main_path = package_dir.join(main_field);
                if main_path.exists() {
                    return Ok(main_path);
                }
                if let Ok(resolved) = self.try_resolve_file(&main_path) {
                    return Ok(resolved);
                }
            }

            // Try exports field
            if let Some(exports) = &pkg.exports {
                if let Some(types) = &exports.types {
                    let types_path = package_dir.join(types);
                    if types_path.exists() {
                        return Ok(types_path);
                    }
                    if let Ok(resolved) = self.try_resolve_file(&types_path) {
                        return Ok(resolved);
                    }
                }
                if let Some(default) = &exports.default {
                    let default_path = package_dir.join(default);
                    if default_path.exists() {
                        return Ok(default_path);
                    }
                    if let Ok(resolved) = self.try_resolve_file(&default_path) {
                        return Ok(resolved);
                    }
                }
            }
        }

        // Fallback: try index files
        self.try_resolve_file(&package_dir.join("index"))
    }

    /// Try to resolve a file with various extensions
    fn try_resolve_file(&self, target: &Path) -> Result<PathBuf, String> {
        // Try extensions in order: .d.ts, .ts, .tsx, .js, .jsx
        let extensions = ["d.ts", "ts", "tsx", "js", "jsx"];

        for ext in &extensions {
            let with_ext = if ext.contains('.') {
                // For .d.ts, don't use with_extension
                let mut path = target.to_path_buf();
                path.set_extension("");
                PathBuf::from(format!("{}.{}", path.display(), ext))
            } else {
                target.with_extension(ext)
            };

            if with_ext.exists() && with_ext.is_file() {
                return Ok(with_ext);
            }
        }

        // Try as directory with index file
        if target.is_dir() {
            for ext in &extensions {
                let index_path = if ext.contains('.') {
                    target.join(format!("index.{}", ext))
                } else {
                    target.join("index").with_extension(ext)
                };

                if index_path.exists() && index_path.is_file() {
                    return Ok(index_path);
                }
            }
        }

        Err(format!(
            "Module not found: {} (tried extensions: d.ts, ts, tsx, js, jsx)",
            target.display()
        ))
    }

    /// Get all dependencies from project's package.json
    pub fn get_dependencies(&self) -> Result<std::collections::HashMap<String, String>, String> {
        let package_json_path = self.project_root.join("package.json");

        if !package_json_path.exists() {
            return Ok(std::collections::HashMap::new());
        }

        let pkg = parse_package_json(&package_json_path)?;
        Ok(pkg.dependencies)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_package_specifier() {
        let (pkg, sub) = NpmResolver::parse_package_specifier("lodash");
        assert_eq!(pkg, "lodash");
        assert_eq!(sub, None);

        let (pkg, sub) = NpmResolver::parse_package_specifier("lodash/fp");
        assert_eq!(pkg, "lodash");
        assert_eq!(sub, Some("fp"));

        let (pkg, sub) = NpmResolver::parse_package_specifier("@types/node");
        assert_eq!(pkg, "@types/node");
        assert_eq!(sub, None);

        let (pkg, sub) = NpmResolver::parse_package_specifier("@types/node/fs");
        assert_eq!(pkg, "@types/node");
        assert_eq!(sub, Some("fs"));
    }

    #[test]
    fn test_find_project_root() {
        use std::fs;

        // Create a temporary directory structure
        let temp_dir = std::env::temp_dir().join(format!("zaco_npm_test_{}", std::process::id()));

        // Clean up if it exists
        let _ = fs::remove_dir_all(&temp_dir);

        // Create directories and files
        fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");
        let package_json = temp_dir.join("package.json");
        fs::write(&package_json, r#"{"name":"test","version":"1.0.0"}"#)
            .expect("Failed to write package.json");

        let nested = temp_dir.join("src/components");
        fs::create_dir_all(&nested).expect("Failed to create nested dir");

        let root = NpmResolver::find_project_root(&nested);
        assert!(root.is_some(), "Project root should be found");

        let found_root = root.unwrap();
        // Canonicalize both paths for comparison
        let expected = temp_dir.canonicalize().unwrap();
        let actual = found_root.canonicalize().unwrap_or(found_root);
        assert_eq!(actual, expected, "Project root should match temp dir");

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
