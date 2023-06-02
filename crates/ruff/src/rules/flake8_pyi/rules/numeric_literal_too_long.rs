use ruff_text_size::TextSize;
use rustpython_parser::ast::{Expr, Ranged};

use ruff_diagnostics::{Diagnostic, Violation};
use ruff_macros::{derive_message_formats, violation};

use crate::checkers::ast::Checker;

#[violation]
pub struct NumericLiteralTooLong;

/// ## What it does
/// Checks for numeric literals with a string representation longer than ten
/// characters.
///
/// ## Why is this bad?
/// If a function has a default value where the literal representation is
/// greater than 50 characters, it is likely to be an implementation detail or
/// a constant that varies depending on the system you're running on.
///
/// Consider replacing such constants with ellipses (`...`).
///
/// ## Example
/// ```python
/// def foo(arg: int = 12345678901) -> None: ...
/// ```
///
/// Use instead:
/// ```python
/// def foo(arg: int = ...) -> None: ...
/// ```
impl Violation for NumericLiteralTooLong {
    #[derive_message_formats]
    fn message(&self) -> String {
        format!("Numeric literals with a string representation longer than ten characters are not permitted")
    }
}

/// PYI054
pub(crate) fn numeric_literal_too_long(checker: &mut Checker, expr: &Expr) {
    if expr.range().len() > TextSize::new(10) {
        checker
            .diagnostics
            .push(Diagnostic::new(NumericLiteralTooLong, expr.range()));
    }
}
