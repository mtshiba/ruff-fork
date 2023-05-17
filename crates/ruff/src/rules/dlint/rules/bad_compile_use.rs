use rustpython_parser::ast;
use rustpython_parser::ast::{Expr, Ranged};

use ruff_diagnostics::{Diagnostic, Violation};
use ruff_macros::{derive_message_formats, violation};

use crate::checkers::ast::Checker;

#[violation]
pub struct BadCompileUse;

impl Violation for BadCompileUse {
    #[derive_message_formats]
    fn message(&self) -> String {
        format!("Use of the `compile` command should be avoided")
    }
}

/// DUO110
pub(crate) fn bad_compile_use(checker: &mut Checker, expr: &Expr) {
    if let Expr::Call(ast::ExprCall { func, .. }) = expr {
        if let Some(call_path) = checker.ctx.resolve_call_path(func) {
            if call_path.as_slice() == ["", "compile"] {
                checker
                    .diagnostics
                    .push(Diagnostic::new(BadCompileUse, func.range()));
            }
        }
    }
}
