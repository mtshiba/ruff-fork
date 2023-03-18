use ruff_diagnostics::{Diagnostic, Violation};
use ruff_macros::{derive_message_formats, violation};
use ruff_python_ast::types::Range;

/// ## What it does
/// Checks for variable names to be vague, non-descript values
///
/// ## Why is this bad?
/// Non-descript letter variable names do not help the reader to quickly
/// know what they contain.
///
/// ## Example
/// ```python
/// foo = "hi"
/// val = 12
/// ```
///
/// Use instead:
/// ```python
/// message = "hi"
/// valueForUse = 12
/// ```
#[violation]
pub struct NonDescriptVariableName(pub String);

impl Violation for NonDescriptVariableName {
    #[derive_message_formats]
    fn message(&self) -> String {
        let NonDescriptVariableName(name) = self;
        format!("Non-descript variable name: `{name}`")
    }
}

fn is_non_descript_variable(name: &str) -> bool {
    const BLOCKLIST: [&str; 17] = [
        "val", "vals", "var", "vars", "variable", "contents", "handle", "file", "objs", "some",
        "do", "no", "true", "false", "foo", "bar", "baz",
    ];

    if BLOCKLIST.contains(&name.to_lowercase().as_str()) {
        return true;
    }

    false
}

/// VN002
pub fn non_descript_variable_name(name: &str, range: Range) -> Option<Diagnostic> {
    if is_non_descript_variable(name) {
        Some(Diagnostic::new(
            NonDescriptVariableName(name.to_string()),
            range,
        ))
    } else {
        None
    }
}
