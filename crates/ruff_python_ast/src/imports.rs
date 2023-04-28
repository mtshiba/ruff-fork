<<<<<<< HEAD
use rustc_hash::{FxHashMap, FxHashSet};
use rustpython_parser::ast::Location;
use serde::{Deserialize, Serialize};
=======
use ruff_text_size::TextRange;
use rustc_hash::FxHashMap;
>>>>>>> upstream/main

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A representation of an individual name imported via any import statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnyImport<'a> {
    Import(Import<'a>),
    ImportFrom(ImportFrom<'a>),
}

/// A representation of an individual name imported via an `import` statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import<'a> {
    pub name: Alias<'a>,
}

/// A representation of an individual name imported via a `from ... import` statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportFrom<'a> {
    pub module: Option<&'a str>,
    pub name: Alias<'a>,
    pub level: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alias<'a> {
    pub name: &'a str,
    pub as_name: Option<&'a str>,
}

impl<'a> Import<'a> {
    pub fn module(name: &'a str) -> Self {
        Self {
            name: Alias {
                name,
                as_name: None,
            },
        }
    }
}

impl std::fmt::Display for AnyImport<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AnyImport::Import(import) => write!(f, "{import}"),
            AnyImport::ImportFrom(import_from) => write!(f, "{import_from}"),
        }
    }
}

impl std::fmt::Display for Import<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "import {}", self.name.name)?;
        if let Some(as_name) = self.name.as_name {
            write!(f, " as {as_name}")?;
        }
        Ok(())
    }
}

impl std::fmt::Display for ImportFrom<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "from ")?;
        if let Some(level) = self.level {
            write!(f, "{}", ".".repeat(level))?;
        }
        if let Some(module) = self.module {
            write!(f, "{module}")?;
        }
        write!(f, " import {}", self.name.name)?;
        Ok(())
    }
}

pub trait FutureImport {
    /// Returns `true` if this import is from the `__future__` module.
    fn is_future_import(&self) -> bool;
}

impl FutureImport for Import<'_> {
    fn is_future_import(&self) -> bool {
        self.name.name == "__future__"
    }
}

impl FutureImport for ImportFrom<'_> {
    fn is_future_import(&self) -> bool {
        self.module == Some("__future__")
    }
}

impl FutureImport for AnyImport<'_> {
    fn is_future_import(&self) -> bool {
        match self {
            AnyImport::Import(import) => import.is_future_import(),
            AnyImport::ImportFrom(import_from) => import_from.is_future_import(),
        }
    }
}

/// A representation of a module reference in an import statement.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ModuleImport {
    module: String,
    range: TextRange,
}

impl ModuleImport {
    pub fn new(module: String, range: TextRange) -> Self {
        Self { module, range }
    }
}

impl From<&ModuleImport> for TextRange {
    fn from(import: &ModuleImport) -> TextRange {
        import.range
    }
}

/// A representation of the import dependencies between modules.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ImportMap {
    /// A map from dot-delimited module name to the list of imports in that module.
    pub module_to_imports: FxHashMap<String, Vec<ModuleImport>>,
}

impl ImportMap {
    pub fn new(module_to_imports: FxHashMap<String, Vec<ModuleImport>>) -> Self {
        Self { module_to_imports }
    }

    pub fn insert(&mut self, module: String, imports_vec: Vec<ModuleImport>) {
        self.module_to_imports.insert(module, imports_vec);
    }

    pub fn extend(&mut self, other: Self) {
        self.module_to_imports.extend(other.module_to_imports);
    }
}

impl<'a> IntoIterator for &'a ImportMap {
    type Item = <&'a FxHashMap<String, Vec<ModuleImport>> as IntoIterator>::Item;
    type IntoIter = <&'a FxHashMap<String, Vec<ModuleImport>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.module_to_imports.iter()
    }
}

#[derive(Default)]
pub struct ModuleMapping<'a> {
    pub(super) module_to_id: FxHashMap<&'a str, u32>,
    pub(super) id_to_module: Vec<&'a str>,
    id: u32,
}

impl<'a> ModuleMapping<'a> {
    pub fn new() -> Self {
        Self {
            module_to_id: FxHashMap::default(),
            id_to_module: vec![],
            id: 0,
        }
    }

    pub(super) fn insert(&mut self, module: &'a str) {
        self.module_to_id.insert(module, self.id);
        self.id_to_module.push(module);
        self.id += 1;
    }

    pub fn to_id(&self, module: &str) -> Option<&u32> {
        self.module_to_id.get(module)
    }

    pub fn to_module(&self, id: &u32) -> Option<&&str> {
        self.id_to_module.get(*id as usize)
    }
}

#[derive(Default)]
pub struct CyclicImportHelper<'a> {
    pub cycles: FxHashMap<u32, FxHashSet<Vec<u32>>>,
    pub module_mapping: ModuleMapping<'a>,
}

impl<'a> CyclicImportHelper<'a> {
    pub fn new(import_map: &'a ImportMap) -> Self {
        let mut module_mapping = ModuleMapping::new();
        import_map.module_to_imports.keys().for_each(|module| {
            module_mapping.insert(module);
        });

        Self {
            cycles: FxHashMap::default(),
            module_mapping,
        }
    }
}
