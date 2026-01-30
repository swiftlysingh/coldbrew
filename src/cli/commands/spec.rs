//! Package spec resolution helpers

use crate::core::version::parse_package_spec;
use crate::error::Result;
use crate::registry::Index;
use crate::storage::Cellar;

/// Resolve a package spec for install, handling versioned formula names.
pub fn resolve_install_spec(spec: &str, index: &Index) -> Result<(String, Option<String>)> {
    if spec.contains('@') && index.get_formula(spec)?.is_some() {
        return Ok((spec.to_string(), None));
    }

    Ok(parse_package_spec(spec))
}

/// Resolve a package spec based on what's installed in the cellar.
pub fn resolve_installed_spec(spec: &str, cellar: &Cellar) -> Result<(String, Option<String>)> {
    if spec.contains('@') && !cellar.get_versions(spec)?.is_empty() {
        return Ok((spec.to_string(), None));
    }

    Ok(parse_package_spec(spec))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::bottle::BottleSpec;
    use crate::core::formula::{Formula, Versions};
    use crate::storage::Paths;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;

    fn formula(name: &str, version: &str) -> Formula {
        Formula {
            name: name.to_string(),
            full_name: format!("homebrew/core/{}", name),
            tap: "homebrew/core".to_string(),
            desc: None,
            homepage: None,
            license: None,
            versions: Versions {
                stable: version.to_string(),
                head: None,
                bottle: true,
            },
            bottle: BottleSpec::default(),
            dependencies: Vec::new(),
            build_dependencies: Vec::new(),
            optional_dependencies: Vec::new(),
            test_dependencies: Vec::new(),
            recommended_dependencies: Vec::new(),
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
            link_overwrite: Vec::new(),
            post_install_defined: false,
            service: None,
            analytics: None,
            analytics_install_on_request_30d: None,
        }
    }

    fn write_formula_index(paths: &Paths, formulas: Vec<Formula>) {
        let index_path = paths.formula_index();
        if let Some(parent) = index_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let json = serde_json::to_string(&formulas).unwrap();
        fs::write(index_path, json).unwrap();
    }

    #[test]
    fn test_resolve_install_spec_prefers_formula_name() {
        let temp = TempDir::new().unwrap();
        let paths = Paths::with_root(temp.path().to_path_buf());
        write_formula_index(&paths, vec![formula("node@22", "22.1.0")]);

        let index = Index::new(paths);
        let (name, version) = resolve_install_spec("node@22", &index).unwrap();
        assert_eq!(name, "node@22");
        assert_eq!(version, None);
    }

    #[test]
    fn test_resolve_install_spec_falls_back_to_version() {
        let temp = TempDir::new().unwrap();
        let paths = Paths::with_root(temp.path().to_path_buf());
        write_formula_index(&paths, vec![formula("jq", "1.7.1")]);

        let index = Index::new(paths);
        let (name, version) = resolve_install_spec("jq@1.7", &index).unwrap();
        assert_eq!(name, "jq");
        assert_eq!(version, Some("1.7".to_string()));
    }

    #[test]
    fn test_resolve_installed_spec_prefers_installed_name() {
        let temp = TempDir::new().unwrap();
        let paths = Paths::with_root(temp.path().to_path_buf());
        let cellar = Cellar::new(paths.clone());
        let pkg_dir = paths.cellar_package("node@22", "22.1.0");
        fs::create_dir_all(pkg_dir).unwrap();

        let (name, version) = resolve_installed_spec("node@22", &cellar).unwrap();
        assert_eq!(name, "node@22");
        assert_eq!(version, None);
    }
}
