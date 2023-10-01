use ruff_python_ast::StmtImportFrom;

use ruff_diagnostics::{Diagnostic, Violation};
use ruff_macros::{derive_message_formats, violation};

use crate::checkers::ast::Checker;

/// ## What it does
/// Checks for the presence of the `from __future__ import annotations` import
/// statement in stub files.
///
/// Why is this bad?
/// Stub files already have the semantics of the annotations feature and thus
/// this import statement is not required.
#[violation]
pub struct FutureAnnotationsInStub;

impl Violation for FutureAnnotationsInStub {
    #[derive_message_formats]
    fn message(&self) -> String {
        format!("`from __future__ import annotations` has no effect in stub files, since type checkers automatically treat stubs as having those semantics")
    }
}

/// PYI044
pub(crate) fn from_future_import(checker: &mut Checker, target: &StmtImportFrom) {
    if let StmtImportFrom {
        range,
        module: Some(name),
        names,
        ..
    } = target
    {
        if name == "__future__" && names.iter().any(|alias| &*alias.name == "annotations") {
            checker
                .diagnostics
                .push(Diagnostic::new(FutureAnnotationsInStub, *range));
        }
    }
}
