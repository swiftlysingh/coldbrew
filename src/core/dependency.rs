//! Dependency resolution

use crate::core::Formula;
use crate::error::{ColdbrewError, Result};
use std::collections::{HashMap, HashSet, VecDeque};

/// Dependency resolver for package installation
pub struct DependencyResolver {
    /// Available formulas (name -> formula)
    formulas: HashMap<String, Formula>,
}

/// A resolved dependency with installation order
#[derive(Debug, Clone)]
pub struct ResolvedDependency {
    /// Package name
    pub name: String,
    /// Version to install
    pub version: String,
    /// Whether this is a direct dependency or transitive
    pub is_direct: bool,
    /// Depth in the dependency tree (0 = root)
    pub depth: usize,
}

impl DependencyResolver {
    /// Create a new dependency resolver
    pub fn new() -> Self {
        Self {
            formulas: HashMap::new(),
        }
    }

    /// Add a formula to the resolver
    pub fn add_formula(&mut self, formula: Formula) {
        self.formulas.insert(formula.name.clone(), formula);
    }

    /// Add multiple formulas to the resolver
    pub fn add_formulas(&mut self, formulas: impl IntoIterator<Item = Formula>) {
        for formula in formulas {
            self.add_formula(formula);
        }
    }

    /// Resolve all dependencies for a package
    /// Returns a list of packages in installation order (dependencies first)
    pub fn resolve(&self, package_name: &str) -> Result<Vec<ResolvedDependency>> {
        let mut resolved = Vec::new();
        let mut visited = HashSet::new();
        let mut in_progress = HashSet::new();

        self.resolve_recursive(package_name, 0, true, &mut resolved, &mut visited, &mut in_progress)?;

        // Reverse to get installation order (dependencies first)
        resolved.reverse();
        Ok(resolved)
    }

    fn resolve_recursive(
        &self,
        package_name: &str,
        depth: usize,
        is_direct: bool,
        resolved: &mut Vec<ResolvedDependency>,
        visited: &mut HashSet<String>,
        in_progress: &mut HashSet<String>,
    ) -> Result<()> {
        // Check for circular dependencies
        if in_progress.contains(package_name) {
            return Err(ColdbrewError::CircularDependency(package_name.to_string()));
        }

        // Skip if already resolved
        if visited.contains(package_name) {
            return Ok(());
        }

        // Mark as in progress
        in_progress.insert(package_name.to_string());

        // Get the formula
        let formula = self.formulas.get(package_name).ok_or_else(|| {
            ColdbrewError::DependencyResolutionFailed {
                package: package_name.to_string(),
                dep: package_name.to_string(),
            }
        })?;

        // Resolve dependencies first
        for dep in &formula.dependencies {
            self.resolve_recursive(dep, depth + 1, false, resolved, visited, in_progress)?;
        }

        // Mark as visited and add to resolved list
        in_progress.remove(package_name);
        visited.insert(package_name.to_string());

        resolved.push(ResolvedDependency {
            name: package_name.to_string(),
            version: formula.versions.stable.clone(),
            is_direct,
            depth,
        });

        Ok(())
    }

    /// Get all dependencies for a package (without resolution order)
    pub fn get_dependencies(&self, package_name: &str) -> Result<Vec<String>> {
        let formula = self.formulas.get(package_name).ok_or_else(|| {
            ColdbrewError::PackageNotFound(package_name.to_string())
        })?;

        Ok(formula.dependencies.clone())
    }

    /// Get all packages that depend on a given package
    pub fn get_dependents(&self, package_name: &str) -> Vec<String> {
        self.formulas
            .iter()
            .filter(|(_, formula)| formula.dependencies.contains(&package_name.to_string()))
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Build a complete dependency tree (BFS)
    pub fn dependency_tree(&self, package_name: &str) -> Result<DependencyTree> {
        let formula = self.formulas.get(package_name).ok_or_else(|| {
            ColdbrewError::PackageNotFound(package_name.to_string())
        })?;

        let mut tree = DependencyTree {
            name: package_name.to_string(),
            version: formula.versions.stable.clone(),
            children: Vec::new(),
        };

        let mut visited = HashSet::new();
        visited.insert(package_name.to_string());

        self.build_tree(&formula.dependencies, &mut tree.children, &mut visited)?;

        Ok(tree)
    }

    fn build_tree(
        &self,
        deps: &[String],
        children: &mut Vec<DependencyTree>,
        visited: &mut HashSet<String>,
    ) -> Result<()> {
        for dep in deps {
            if visited.contains(dep) {
                // Skip circular references
                continue;
            }

            if let Some(formula) = self.formulas.get(dep) {
                visited.insert(dep.clone());

                let mut child = DependencyTree {
                    name: dep.clone(),
                    version: formula.versions.stable.clone(),
                    children: Vec::new(),
                };

                self.build_tree(&formula.dependencies, &mut child.children, visited)?;
                children.push(child);
            }
        }

        Ok(())
    }
}

impl Default for DependencyResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// A tree representation of dependencies
#[derive(Debug, Clone)]
pub struct DependencyTree {
    pub name: String,
    pub version: String,
    pub children: Vec<DependencyTree>,
}

impl DependencyTree {
    /// Get the total number of dependencies (including nested)
    pub fn total_count(&self) -> usize {
        self.children.iter().map(|c| 1 + c.total_count()).sum()
    }

    /// Get all unique package names in the tree
    pub fn all_packages(&self) -> HashSet<String> {
        let mut packages = HashSet::new();
        self.collect_packages(&mut packages);
        packages
    }

    fn collect_packages(&self, packages: &mut HashSet<String>) {
        packages.insert(self.name.clone());
        for child in &self.children {
            child.collect_packages(packages);
        }
    }

    /// Pretty print the tree
    pub fn pretty_print(&self) -> String {
        let mut output = String::new();
        self.pretty_print_recursive(&mut output, "", true);
        output
    }

    fn pretty_print_recursive(&self, output: &mut String, prefix: &str, is_last: bool) {
        let connector = if is_last { "└── " } else { "├── " };
        output.push_str(&format!("{}{}{} {}\n", prefix, connector, self.name, self.version));

        let child_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });
        for (i, child) in self.children.iter().enumerate() {
            child.pretty_print_recursive(output, &child_prefix, i == self.children.len() - 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::formula::{Formula, Versions};
    use crate::core::bottle::BottleSpec;
    use std::collections::HashMap;

    fn make_formula(name: &str, deps: Vec<&str>) -> Formula {
        Formula {
            name: name.to_string(),
            full_name: format!("homebrew/core/{}", name),
            tap: "homebrew/core".to_string(),
            desc: None,
            homepage: None,
            license: None,
            versions: Versions {
                stable: "1.0.0".to_string(),
                head: None,
                bottle: true,
            },
            bottle: BottleSpec::default(),
            dependencies: deps.into_iter().map(String::from).collect(),
            build_dependencies: vec![],
            optional_dependencies: vec![],
            test_dependencies: vec![],
            recommended_dependencies: vec![],
            keg_only: false,
            keg_only_reason: None,
            deprecated: false,
            deprecation_date: None,
            deprecation_reason: None,
            disabled: false,
            disable_date: None,
            disable_reason: None,
            caveats: None,
            urls: HashMap::new(),
            revision: 0,
            version_scheme: 0,
            link_overwrite: vec![],
            post_install_defined: false,
            service: None,
            analytics: None,
            analytics_install_on_request_30d: None,
        }
    }

    #[test]
    fn test_resolve_simple() {
        let mut resolver = DependencyResolver::new();
        resolver.add_formula(make_formula("jq", vec!["oniguruma"]));
        resolver.add_formula(make_formula("oniguruma", vec![]));

        let resolved = resolver.resolve("jq").unwrap();
        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0].name, "oniguruma");
        assert_eq!(resolved[1].name, "jq");
    }

    #[test]
    fn test_resolve_circular_dependency() {
        let mut resolver = DependencyResolver::new();
        resolver.add_formula(make_formula("a", vec!["b"]));
        resolver.add_formula(make_formula("b", vec!["a"]));

        let result = resolver.resolve("a");
        assert!(matches!(result, Err(ColdbrewError::CircularDependency(_))));
    }

    #[test]
    fn test_dependency_tree() {
        let mut resolver = DependencyResolver::new();
        resolver.add_formula(make_formula("a", vec!["b", "c"]));
        resolver.add_formula(make_formula("b", vec!["d"]));
        resolver.add_formula(make_formula("c", vec![]));
        resolver.add_formula(make_formula("d", vec![]));

        let tree = resolver.dependency_tree("a").unwrap();
        assert_eq!(tree.name, "a");
        assert_eq!(tree.children.len(), 2);
        assert_eq!(tree.total_count(), 3);
    }
}
