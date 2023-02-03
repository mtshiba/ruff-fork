use crate::define_simple_autofix_violation;
use crate::violation::AlwaysAutofixableViolation;
use ruff_macros::derive_message_formats;
use rustpython_ast::Expr;

use crate::ast::types::Range;
use crate::checkers::ast::Checker;
use crate::fix::Fix;
use crate::registry::Diagnostic;

define_simple_autofix_violation!(
    TypingTextStrAlias,
    "`typing.Text` is deprecated, use `str`",
    "Replace with `str`"
);

/// UP019
pub fn typing_text_str_alias(checker: &mut Checker, expr: &Expr) {
    if checker.resolve_call_path(expr).map_or(false, |call_path| {
        call_path.as_slice() == ["typing", "Text"]
    }) {
        let mut diagnostic = Diagnostic::new(TypingTextStrAlias, Range::from_located(expr));
        if checker.patch(diagnostic.kind.rule()) {
            diagnostic.amend(Fix::replacement(
                "str".to_string(),
                expr.location,
                expr.end_location.unwrap(),
            ));
        }
        checker.diagnostics.push(diagnostic);
    }
}
