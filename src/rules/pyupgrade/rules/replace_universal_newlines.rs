use crate::define_simple_autofix_violation;
use crate::violation::AlwaysAutofixableViolation;
use ruff_macros::derive_message_formats;
use rustpython_ast::{Expr, Keyword, Location};

use crate::ast::helpers::find_keyword;
use crate::ast::types::Range;
use crate::checkers::ast::Checker;
use crate::fix::Fix;
use crate::registry::Diagnostic;

define_simple_autofix_violation!(
    ReplaceUniversalNewlines,
    "`universal_newlines` is deprecated, use `text`",
    "Replace with `text` keyword argument"
);

/// UP021
pub fn replace_universal_newlines(checker: &mut Checker, expr: &Expr, kwargs: &[Keyword]) {
    if checker.resolve_call_path(expr).map_or(false, |call_path| {
        call_path.as_slice() == ["subprocess", "run"]
    }) {
        let Some(kwarg) = find_keyword(kwargs, "universal_newlines") else { return; };
        let range = Range::new(
            kwarg.location,
            Location::new(
                kwarg.location.row(),
                kwarg.location.column() + "universal_newlines".len(),
            ),
        );
        let mut diagnostic = Diagnostic::new(ReplaceUniversalNewlines, range);
        if checker.patch(diagnostic.kind.rule()) {
            diagnostic.amend(Fix::replacement(
                "text".to_string(),
                range.location,
                range.end_location,
            ));
        }
        checker.diagnostics.push(diagnostic);
    }
}
