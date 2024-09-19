use crate::collector::Collector;
pub use crate::db::ModuleDb;
use crate::resolver::Resolver;
pub use crate::settings::{Direction, GraphSettings};
use anyhow::Result;
use red_knot_python_semantic::SemanticModel;
use ruff_db::files::system_path_to_file;
use ruff_db::parsed::parsed_module;
use ruff_db::system::{SystemPath, SystemPathBuf};
use ruff_python_ast::helpers::to_module_path;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod collector;
mod db;
mod resolver;
mod settings;

#[derive(Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ModuleImports(BTreeSet<SystemPathBuf>);

impl ModuleImports {
    pub fn insert(&mut self, path: SystemPathBuf) {
        self.0.insert(path);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Convert the file paths to be relative to a given path.
    #[must_use]
    pub fn relative_to(&self, path: &SystemPath) -> Self {
        let mut imports = Self::default();
        for import in &self.0 {
            if let Ok(path) = import.strip_prefix(path) {
                imports.insert(path.to_path_buf());
            }
        }
        imports
    }
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ImportMap(BTreeMap<SystemPathBuf, ModuleImports>);

impl ImportMap {
    pub fn insert(&mut self, path: SystemPathBuf, imports: ModuleImports) {
        self.0.insert(path, imports);
    }

    #[must_use]
    pub fn reverse(imports: impl IntoIterator<Item = (SystemPathBuf, ModuleImports)>) -> Self {
        let mut reverse = ImportMap::default();
        for (path, imports) in imports {
            for import in imports.0 {
                reverse.0.entry(import).or_default().insert(path.clone());
            }
        }
        reverse
    }
}

impl FromIterator<(SystemPathBuf, ModuleImports)> for ImportMap {
    fn from_iter<I: IntoIterator<Item = (SystemPathBuf, ModuleImports)>>(iter: I) -> Self {
        let mut map = ImportMap::default();
        for (path, imports) in iter {
            map.0.entry(path).or_default().0.extend(imports.0);
        }
        map
    }
}

/// Generate the module imports for a given Python file.
pub fn generate(
    path: &SystemPath,
    package: Option<&SystemPath>,
    string_imports: bool,
    db: &ModuleDb,
) -> Result<ModuleImports> {
    // Read and parse the source code.
    let file = system_path_to_file(db, path)?;
    let parsed = parsed_module(db, file);
    let module_path =
        package.and_then(|package| to_module_path(package.as_std_path(), path.as_std_path()));
    let model = SemanticModel::new(db, file);

    // Collect the imports.
    let imports = Collector::new(string_imports).collect(parsed.syntax());

    // Resolve the imports.
    let mut resolved_imports = ModuleImports::default();
    for import in imports {
        let Some(resolved) = Resolver::new(&model, module_path.as_deref()).resolve(import) else {
            continue;
        };
        let Some(path) = resolved.as_system_path() else {
            continue;
        };
        resolved_imports.insert(path.to_path_buf());
    }

    Ok(resolved_imports)
}
