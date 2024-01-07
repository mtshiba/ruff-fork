use ruff_diagnostics::{FixAvailability, Violation};
use ruff_macros::{derive_message_formats, violation};
use ruff_source_file::SourceRow;

/// ## What it does
/// Checks for variable definitions that redefine (or "shadow") unused
/// variables.
///
/// ## Why is this bad?
/// Redefinitions of unused names are unnecessary and often indicative of a
/// mistake.
///
/// ## Example
/// ```python
/// import foo
/// import bar
/// import foo  # Redefinition of unused `foo` from line 1
/// ```
///
/// Use instead:
/// ```python
/// import foo
/// import bar
/// ```
///
/// ## Fix safety
/// This rule's fix is marked as unsafe, as removing a redefinition across
/// branches or scopes may change the behavior of the program in subtle
/// ways.
///
/// For example:
/// ```python
/// import module
///
/// x = int(input())
///
/// if x > 0:
///     from package import module
/// ```
///
/// Removing the redefinition would change the `module` binding when `x > 0`.
#[violation]
pub struct RedefinedWhileUnused {
    pub name: String,
    pub row: SourceRow,
}

impl Violation for RedefinedWhileUnused {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    #[derive_message_formats]
    fn message(&self) -> String {
        let RedefinedWhileUnused { name, row } = self;
        format!("Redefinition of unused `{name}` from {row}")
    }

    fn fix_title(&self) -> Option<String> {
        let RedefinedWhileUnused { name, .. } = self;
        Some(format!("Remove definition: `{name}`"))
    }
}
