use super::helpers;
use crate::ast::types::Range;
use crate::checkers::ast::Checker;
use crate::define_simple_autofix_violation;
use crate::registry::Diagnostic;
use crate::rules::flake8_comprehensions::fixes;
use crate::violation::AlwaysAutofixableViolation;
use log::error;
use ruff_macros::derive_message_formats;
use rustpython_ast::{Expr, ExprKind};

define_simple_autofix_violation!(
    UnnecessaryListCall,
    "Unnecessary `list` call (remove the outer call to `list()`)",
    "Remove outer `list` call"
);

/// C411
pub fn unnecessary_list_call(checker: &mut Checker, expr: &Expr, func: &Expr, args: &[Expr]) {
    let Some(argument) = helpers::first_argument_with_matching_function("list", func, args) else {
        return;
    };
    if !checker.is_builtin("list") {
        return;
    }
    if !matches!(argument, ExprKind::ListComp { .. }) {
        return;
    }
    let mut diagnostic = Diagnostic::new(UnnecessaryListCall, Range::from_located(expr));
    if checker.patch(diagnostic.kind.rule()) {
        match fixes::fix_unnecessary_list_call(checker.locator, checker.stylist, expr) {
            Ok(fix) => {
                diagnostic.amend(fix);
            }
            Err(e) => error!("Failed to generate fix: {e}"),
        }
    }
    checker.diagnostics.push(diagnostic);
}
